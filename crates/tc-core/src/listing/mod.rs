//! The directory model behind a pane: what was loaded, how it is ordered,
//! what is visible, and where the cursor sits.
//!
//! A `Listing` is a plain value with no filesystem handle of its own. The
//! `VirtualFs` is passed in for the two operations that actually read
//! (`load`, `reload`), because a pane swaps its backend when the user steps
//! into an archive — the pane owns the filesystem, the listing owns the
//! model. That also keeps the whole type constructible from a `Vec<Entry>`
//! in tests and, later, from streaming search results.

pub mod constants;
mod name;
mod sort;

use crate::vfs::constants::PARENT;
use crate::vfs::{Entry, EntryKind, VfsError, VfsPath, VirtualFs};

use constants::{DEFAULT_SHOW_HIDDEN, DEFAULT_SORT_KEY, DEFAULT_SORT_ORDER, PARENT_MODIFIED};

pub use name::split_name;
pub use sort::{Sort, SortKey, SortOrder};

/// One directory as the UI sees it.
pub struct Listing {
    dir: VfsPath,
    /// Everything the backend returned: never sorted, never filtered. The
    /// hidden-file toggle rearranges the view over this, so flipping it costs
    /// no filesystem access.
    entries: Vec<Entry>,
    /// The synthetic `..` row, absent at the root.
    parent: Option<Entry>,
    /// Indices into `entries`, sorted and filtered — the visible rows after
    /// `parent`.
    view: Vec<usize>,
    cursor: usize,
    sort: Sort,
    show_hidden: bool,
}

impl Listing {
    /// Reads a directory and builds the model.
    pub fn load(fs: &dyn VirtualFs, dir: VfsPath) -> Result<Self, VfsError> {
        let entries = fs.read_dir(&dir)?;
        Ok(Listing::new(dir, entries))
    }

    /// Builds the model from entries that are already in hand.
    pub fn new(dir: VfsPath, entries: Vec<Entry>) -> Self {
        let parent = dir.parent().map(|_| Entry {
            name: PARENT.to_string(),
            kind: EntryKind::Dir,
            size: 0,
            modified: PARENT_MODIFIED,
            hidden: false,
        });
        let mut listing = Listing {
            dir,
            entries,
            parent,
            view: Vec::new(),
            cursor: 0,
            sort: Sort::new(DEFAULT_SORT_KEY, DEFAULT_SORT_ORDER),
            show_hidden: DEFAULT_SHOW_HIDDEN,
        };
        listing.rebuild(None);
        listing
    }

    /// Re-reads the directory, keeping the cursor on the same entry when it
    /// still exists.
    pub fn reload(&mut self, fs: &dyn VirtualFs) -> Result<(), VfsError> {
        let focused = self.current().map(|entry| entry.name.clone());
        self.entries = fs.read_dir(&self.dir)?;
        self.rebuild(focused);
        Ok(())
    }

    pub fn dir(&self) -> &VfsPath {
        &self.dir
    }

    pub fn sort(&self) -> Sort {
        self.sort
    }

    pub fn show_hidden(&self) -> bool {
        self.show_hidden
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Number of visible rows, `..` included.
    pub fn len(&self) -> usize {
        self.parent_rows() + self.view.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The visible row at `index`, where row 0 is `..` when present.
    pub fn get(&self, index: usize) -> Option<&Entry> {
        if index < self.parent_rows() {
            return self.parent.as_ref();
        }
        self.view
            .get(index - self.parent_rows())
            .map(|&position| &self.entries[position])
    }

    /// The visible rows in order.
    pub fn iter(&self) -> impl Iterator<Item = &Entry> {
        (0..self.len()).filter_map(|index| self.get(index))
    }

    /// The row under the cursor.
    pub fn current(&self) -> Option<&Entry> {
        self.get(self.cursor)
    }

    /// Whether the row at `index` is the synthetic `..`.
    pub fn is_parent(&self, index: usize) -> bool {
        self.parent.is_some() && index == 0
    }

    /// Where activating the row at `index` leads.
    ///
    /// `..` needs no special case: `VfsPath` normalizes lexically, so
    /// `dir.child("..")` *is* the parent.
    pub fn path_at(&self, index: usize) -> Option<VfsPath> {
        self.get(index).map(|entry| self.dir.child(&entry.name))
    }

    /// Where activating the cursor row leads.
    pub fn current_path(&self) -> Option<VfsPath> {
        self.path_at(self.cursor)
    }

    /// Puts the cursor on the entry called `name`.
    ///
    /// Does nothing when that entry is not visible — a hidden directory is
    /// not somewhere the cursor can go while it is hidden.
    pub fn focus_entry(&mut self, name: &str) {
        if let Some(index) = self.index_of(name) {
            self.cursor = index;
        }
    }

    pub fn set_sort(&mut self, sort: Sort) {
        self.sort = sort;
        self.refocus();
    }

    fn set_show_hidden(&mut self, show_hidden: bool) {
        self.show_hidden = show_hidden;
        self.refocus();
    }

    /// Flips the hidden-file filter. No filesystem access — the entries are
    /// already loaded.
    pub fn toggle_hidden(&mut self) {
        self.set_show_hidden(!self.show_hidden);
    }

    /// Moves the cursor by `delta` rows, stopping at either end.
    pub fn move_cursor_by(&mut self, delta: isize) {
        let target = self.cursor as isize + delta;
        self.set_cursor(target.max(0) as usize);
    }

    pub fn move_cursor_to_first(&mut self) {
        self.set_cursor(0);
    }

    pub fn move_cursor_to_last(&mut self) {
        self.set_cursor(self.len().saturating_sub(1));
    }

    /// Places the cursor, clamped to the visible rows.
    pub fn set_cursor(&mut self, index: usize) {
        self.cursor = index.min(self.len().saturating_sub(1));
    }

    fn parent_rows(&self) -> usize {
        usize::from(self.parent.is_some())
    }

    /// The visible row holding `name`, if it is visible at all.
    fn index_of(&self, name: &str) -> Option<usize> {
        (0..self.len()).find(|&index| self.get(index).is_some_and(|entry| entry.name == name))
    }

    /// Rebuilds the view after a sort or filter change, keeping the cursor on
    /// the entry it was on.
    fn refocus(&mut self) {
        let focused = self.current().map(|entry| entry.name.clone());
        self.rebuild(focused);
    }

    /// Recomputes the visible rows and restores the cursor.
    ///
    /// The cursor follows the *named* entry when it is still visible — a file
    /// that is re-sorted or a directory that is reloaded should not slip out
    /// from under the user — and otherwise clamps to the row range.
    fn rebuild(&mut self, focused: Option<String>) {
        self.view = (0..self.entries.len())
            .filter(|&position| self.show_hidden || !self.entries[position].hidden)
            .collect();
        let sort = self.sort;
        let entries = &self.entries;
        self.view
            .sort_by(|&left, &right| sort.compare(&entries[left], &entries[right]));

        if let Some(name) = focused {
            self.cursor = self.index_of(&name).unwrap_or(self.cursor);
        }
        self.set_cursor(self.cursor);
    }
}
