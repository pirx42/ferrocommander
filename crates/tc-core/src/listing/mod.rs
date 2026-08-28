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

use std::collections::HashSet;

use crate::glob;
use crate::vfs::constants::PARENT;
use crate::vfs::{Entry, EntryKind, VfsError, VfsPath, VirtualFs};

use constants::{DEFAULT_SHOW_HIDDEN, DEFAULT_SORT_KEY, DEFAULT_SORT_ORDER, PARENT_MODIFIED};

pub use name::split_name;
pub use sort::{Sort, SortKey, SortOrder};

/// A count of rows and the bytes they hold.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Selection {
    pub count: usize,
    pub bytes: u64,
}

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
    /// Which entries are marked, parallel to `entries` rather than keyed by
    /// name.
    ///
    /// Selecting everything is then a fill and reading the selection is a
    /// scan, and neither sorting nor filtering costs anything, because
    /// neither touches `entries`. A set of names would allocate a string per
    /// marked file for a question the pane asks on every redraw
    /// (`docs/performance.md`). The `..` row is not in `entries`, so it
    /// cannot be marked by any route.
    selected: Vec<bool>,
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

    /// Reads `dir`, or the nearest ancestor that can still be read.
    ///
    /// What a pane needs after a job finishes: the directory it was standing
    /// in may have been moved or deleted by that very job, and showing an
    /// error where a listing belongs strands the user somewhere they cannot
    /// navigate out of. Walking up lands them somewhere real instead.
    ///
    /// Always returns a listing. The root is the last stop, and a root that
    /// cannot be read yields an empty one rather than no pane at all.
    pub fn load_nearest(fs: &dyn VirtualFs, dir: VfsPath) -> Self {
        let mut current = dir;
        loop {
            match Listing::load(fs, current.clone()) {
                Ok(listing) => return listing,
                Err(_) => match current.parent() {
                    Some(parent) => current = parent,
                    None => return Listing::new(current, Vec::new()),
                },
            }
        }
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
            selected: vec![false; entries.len()],
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
        // Marks survive by name, exactly as the cursor does: a job that
        // changed one file must not silently drop the marks on the others.
        // Names that are gone fall out; names that are new arrive unmarked.
        let marked: HashSet<String> = self
            .entries
            .iter()
            .zip(&self.selected)
            .filter(|(_, &selected)| selected)
            .map(|(entry, _)| entry.name.clone())
            .collect();

        self.entries = fs.read_dir(&self.dir)?;
        self.selected = self
            .entries
            .iter()
            .map(|entry| marked.contains(&entry.name))
            .collect();
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

    /// Whether the row at `index` is marked. The `..` row never is.
    pub fn is_selected(&self, index: usize) -> bool {
        self.entry_index(index)
            .is_some_and(|position| self.selected[position])
    }

    /// Marks or unmarks the row at `index`. Does nothing on `..`.
    pub fn set_selected(&mut self, index: usize, selected: bool) {
        if let Some(position) = self.entry_index(index) {
            self.selected[position] = selected;
        }
    }

    /// Flips the row at `index`.
    pub fn toggle_selected(&mut self, index: usize) {
        self.set_selected(index, !self.is_selected(index));
    }

    /// Marks every **visible** row.
    ///
    /// Visible, not every loaded entry: what a filter or the hidden-file flag
    /// is hiding is not something the user can see to have meant.
    pub fn select_all(&mut self) {
        self.set_visible(|_| true);
    }

    pub fn clear_selection(&mut self) {
        self.set_visible(|_| false);
    }

    /// Flips every visible row.
    pub fn invert_selection(&mut self) {
        for &position in &self.view {
            self.selected[position] = !self.selected[position];
        }
    }

    /// Marks or unmarks every visible row whose name matches `pattern`.
    pub fn select_matching(&mut self, pattern: &str, selected: bool) {
        let matching: Vec<usize> = self
            .view
            .iter()
            .copied()
            .filter(|&position| glob::matches(pattern, &self.entries[position].name))
            .collect();
        for position in matching {
            self.selected[position] = selected;
        }
    }

    /// The marked rows, in the order they are shown.
    pub fn selected_paths(&self) -> Vec<VfsPath> {
        self.view
            .iter()
            .filter(|&&position| self.selected[position])
            .map(|&position| self.dir.child(&self.entries[position].name))
            .collect()
    }

    /// How many rows are marked, and how many bytes they hold.
    ///
    /// What the status line under a pane shows, and what a person checks
    /// before pressing F5.
    pub fn selection_summary(&self) -> Selection {
        let mut summary = Selection::default();
        for &position in &self.view {
            if self.selected[position] {
                summary.count += 1;
                summary.bytes += self.entries[position].size;
            }
        }
        summary
    }

    /// Total of everything visible, for the "n of m" the status line pairs
    /// the selection with.
    pub fn visible_summary(&self) -> Selection {
        Selection {
            count: self.view.len(),
            bytes: self.view.iter().map(|&p| self.entries[p].size).sum(),
        }
    }

    /// The position in `entries` a visible row refers to, or `None` for `..`.
    fn entry_index(&self, index: usize) -> Option<usize> {
        if index < self.parent_rows() {
            return None;
        }
        self.view.get(index - self.parent_rows()).copied()
    }

    fn set_visible(&mut self, decide: impl Fn(usize) -> bool) {
        for &position in &self.view {
            self.selected[position] = decide(position);
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
