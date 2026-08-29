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
use std::sync::Arc;

use crate::glob;
use crate::vfs::constants::PARENT;
use crate::vfs::{Attributes, Entry, EntryKind, VfsError, VfsPath, VirtualFs};

use constants::{DEFAULT_SHOW_HIDDEN, DEFAULT_SORT_KEY, DEFAULT_SORT_ORDER, PARENT_MODIFIED};

pub use name::split_name;
pub use sort::{Sort, SortKey, SortOrder};

/// Where a directory read started by [`Listing::spawn_load`] arrives.
///
/// Named here so the shell can hold one without naming the channel crate.
pub type Loading = async_channel::Receiver<Arrival>;

/// What comes back from a read: the listing, and **the backend it was read
/// from**.
///
/// The backend travels with the listing because a step can change it. Entering
/// an archive opens a new [`VirtualFs`] on a worker thread — opening one is a
/// full parse, which is the same unbounded wait a directory read is — and a
/// pane that adopted the new backend before the listing arrived would be
/// pointing at an archive it could not yet show, or at one that turned out not
/// to open at all.
pub type Arrival = Result<(Arc<dyn VirtualFs>, Listing), VfsError>;

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
    /// The quick filter. Empty means everything is shown.
    ///
    /// Applied in `rebuild` beside the hidden-file rule, so narrowing costs
    /// one pass over the loaded entries and never re-reads the directory.
    filter: String,
}

impl Listing {
    /// Reads a directory and builds the model.
    pub fn load(fs: &dyn VirtualFs, dir: VfsPath) -> Result<Self, VfsError> {
        let entries = fs.read_dir(&dir)?;
        Ok(Listing::new(dir, entries))
    }

    /// Reads a directory on a thread of its own, and says where the answer
    /// will arrive.
    ///
    /// A directory read is the one thing the shell does that has no bound on
    /// how long it takes — a cold cache, a network mount, a hundred thousand
    /// files — and doing it on the UI thread froze the window for all of it.
    ///
    /// The whole listing arrives at once rather than streaming in. That is not
    /// laziness: the view is **sorted**, so an entry read late belongs in the
    /// middle, and showing rows as they arrive would shove everything below
    /// them down while the user is looking at it. One swap moves the list
    /// once (`docs/listing.md`).
    pub fn spawn_load(fs: Arc<dyn VirtualFs>, dir: VfsPath) -> Loading {
        spawn(move || {
            let listing = Listing::load(fs.as_ref(), dir)?;
            Ok((fs, listing))
        })
    }

    /// Reads `dir` on a thread, falling back to the nearest readable ancestor.
    ///
    /// What a pane needs after a job or a re-read, for the reason
    /// [`load_nearest`](Self::load_nearest) gives.
    pub fn spawn_load_nearest(fs: Arc<dyn VirtualFs>, dir: VfsPath) -> Loading {
        spawn(move || {
            let listing = Listing::load_nearest(fs.as_ref(), dir);
            Ok((fs, listing))
        })
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
        let parent = dir.parent().map(|_| parent_row());
        let mut listing = Listing {
            selected: vec![false; entries.len()],
            dir,
            entries,
            parent,
            view: Vec::new(),
            cursor: 0,
            sort: Sort::new(DEFAULT_SORT_KEY, DEFAULT_SORT_ORDER),
            show_hidden: DEFAULT_SHOW_HIDDEN,
            filter: String::new(),
        };
        listing.rebuild(None);
        listing
    }

    /// Gives this listing a `..` row even though it is at a root.
    ///
    /// What a pane at the **root of an archive** needs. The archive's root has
    /// no parent inside the archive, and yet there is somewhere to go from it:
    /// back out to the directory holding the archive file. A pane that offered
    /// no `..` there would leave Backspace as the only way out of an archive,
    /// which is not how anybody navigates (`docs/archives.md`).
    ///
    /// Where the row *leads* is the pane's business, not the listing's.
    pub fn offer_parent(&mut self) {
        if self.parent.is_some() {
            return;
        }
        let focused = self.current().map(|entry| entry.name.clone());
        self.parent = Some(parent_row());
        self.rebuild(focused);
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

    pub fn filter(&self) -> &str {
        &self.filter
    }

    /// Narrows the visible rows to the names containing `filter`.
    ///
    /// The cursor follows its entry while that entry is still visible and
    /// clamps when it is not — the same rule as the hidden-file toggle,
    /// because it is the same code.
    pub fn set_filter(&mut self, filter: &str) {
        if self.filter == filter {
            return;
        }
        self.filter = filter.to_string();
        self.refocus();
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

    /// Flips every visible row, directories included.
    ///
    /// What Total Commander binds to `Shift+Num *`; its plain `Num *` is
    /// [`invert_selection_files`](Self::invert_selection_files).
    pub fn invert_selection(&mut self) {
        self.invert(|_| true);
    }

    /// Flips every visible **file**, leaving directories as they are.
    ///
    /// The split is Total Commander's, and it is the useful default: a person
    /// inverting a selection is nearly always thinking about files, and having
    /// every directory in the pane join in is a surprise that costs a second
    /// `Num *` to undo.
    pub fn invert_selection_files(&mut self) {
        self.invert(|entry| !entry.is_dir());
    }

    fn invert(&mut self, include: impl Fn(&Entry) -> bool) {
        let flipping: Vec<usize> = self
            .view
            .iter()
            .copied()
            .filter(|&position| include(&self.entries[position]))
            .collect();
        for position in flipping {
            self.selected[position] = !self.selected[position];
        }
    }

    /// Marks or unmarks every visible file sharing the cursor row's extension.
    ///
    /// Files only, like the inversion above and like Total Commander: a
    /// directory that happens to be called `photos.backup` is not one of "the
    /// `.backup` files". A cursor row with no extension picks out the other
    /// extension-less files, which is the same rule applied honestly rather
    /// than a special case.
    ///
    /// Does nothing on `..`, which has no extension to go by.
    pub fn select_same_extension(&mut self, selected: bool) {
        let Some(entry) = self.current() else {
            return;
        };
        if entry.is_dir() {
            return;
        }
        let wanted = split_name(&entry.name).1.to_string();
        let matching: Vec<usize> = self
            .view
            .iter()
            .copied()
            .filter(|&position| {
                let entry = &self.entries[position];
                !entry.is_dir() && split_name(&entry.name).1 == wanted
            })
            .collect();
        for position in matching {
            self.selected[position] = selected;
        }
    }

    /// The names of the marked rows, in the order they are shown.
    ///
    /// Names rather than indices, because that is the only form of a selection
    /// that survives anything: every finished job builds a fresh listing and
    /// every sort reorders the one that is there, so an index restored later
    /// points at a different file than the one it was taken from.
    pub fn selected_names(&self) -> Vec<String> {
        self.view
            .iter()
            .filter(|&&position| self.selected[position])
            .map(|&position| self.entries[position].name.clone())
            .collect()
    }

    /// Marks exactly the named rows, unmarking everything else visible.
    ///
    /// A name that is no longer here — the file it referred to was moved or
    /// deleted by the very operation this restores the selection from — is
    /// silently not marked. There is nothing else it could mean.
    pub fn set_selected_names(&mut self, names: &[String]) {
        let wanted: Vec<usize> = self
            .view
            .iter()
            .copied()
            .filter(|&position| {
                names
                    .iter()
                    .any(|name| *name == self.entries[position].name)
            })
            .collect();
        self.clear_selection();
        for position in wanted {
            self.selected[position] = true;
        }
    }

    /// Marks or unmarks every row between `a` and `b`, both ends included.
    ///
    /// Order-independent, because the two ends are a cursor and a destination
    /// and either can be the higher one. Out-of-range ends are clamped rather
    /// than rejected: `Shift+End` names the last row by asking for one past
    /// it, and that is not an error to report anywhere.
    pub fn select_range(&mut self, a: usize, b: usize, selected: bool) {
        let last = self.len().saturating_sub(1);
        for index in a.min(b)..=a.max(b).min(last) {
            self.set_selected(index, selected);
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
            .filter(|&position| {
                let entry = &self.entries[position];
                (self.show_hidden || !entry.hidden)
                    && name::contains_ignoring_case(&entry.name, &self.filter)
            })
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

/// Runs `read` on a thread of its own and says where its answer will arrive.
///
/// One place rather than several, so a read that forgot to be a thread cannot
/// happen: everything a pane waits for goes through here.
///
/// Public because **which backend to read from is not this module's
/// business**. Walking into an archive opens one on the way — the slow half,
/// a zip's central directory or a whole `.tar.gz` decompressed, which belongs
/// on this thread rather than in front of it on the UI thread — and putting
/// that here would make the directory model name a concrete backend, which is
/// the one thing `VirtualFs` exists to prevent. The caller supplies the
/// opening; this supplies the thread.
pub fn spawn(read: impl FnOnce() -> Arrival + Send + 'static) -> Loading {
    let (sender, receiver) = async_channel::bounded(1);
    std::thread::spawn(move || {
        let _ = sender.send_blocking(read());
    });
    receiver
}

/// The `..` row, which is a directory as far as everything above here knows.
fn parent_row() -> Entry {
    Entry {
        name: PARENT.to_string(),
        kind: EntryKind::Dir,
        size: 0,
        modified: PARENT_MODIFIED,
        attributes: Attributes::default(),
        hidden: false,
    }
}
