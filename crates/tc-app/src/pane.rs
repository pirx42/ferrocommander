//! One pane: a path bar above a column view of a directory.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use gtk::gio;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

use tc_core::listing::{split_name, Arrival, Listing, Loading, Sort, SortKey, SortOrder};
use tc_core::vfs::{VfsPath, VirtualFs};

use crate::constants::{
    CLASS_FILTER_BAR, CLASS_MARKED, CLASS_PANE, CLASS_PANE_ACTIVE, CLASS_PATH_BAR,
    CLASS_STATUS_LINE, COLUMN_TITLE_ATTR, COLUMN_TITLE_DATE, COLUMN_TITLE_EXT, COLUMN_TITLE_NAME,
    COLUMN_TITLE_SIZE, COLUMN_WIDTH_ATTR, COLUMN_WIDTH_DATE, COLUMN_WIDTH_EXT, COLUMN_WIDTH_NAME,
    COLUMN_WIDTH_SIZE, FILTER_PLACEHOLDER, PAGE_ROWS_FALLBACK, PANE_SPACING,
    PATH_BAR_ERROR_SEPARATOR, SORT_MARKER_ASCENDING, SORT_MARKER_DESCENDING, XALIGN_LEFT,
    XALIGN_RIGHT,
};
use crate::navigation::{activation_step, adopted_cursor, focus_after_move, parent_target, Step};
use crate::row::Row;

/// The columns a pane shows.
///
/// Titles, widths and alignment hang off the enum so the column set is
/// generated from one list — adding a column later means one variant, not
/// four scattered edits (skill 53).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Column {
    Name,
    Ext,
    Size,
    Modified,
    Attributes,
}

impl Column {
    pub const ALL: [Column; 5] = [
        Column::Name,
        Column::Ext,
        Column::Size,
        Column::Modified,
        Column::Attributes,
    ];

    /// Which ordering this column stands for, if any.
    pub fn sort_key(self) -> Option<SortKey> {
        Some(match self {
            Column::Name => SortKey::Name,
            Column::Ext => SortKey::Ext,
            Column::Size => SortKey::Size,
            Column::Modified => SortKey::Modified,
            // Nothing sorts by permissions, and nothing should: the question
            // it answers is "can I run this", not "where is it in the list".
            Column::Attributes => return None,
        })
    }

    fn title(self) -> &'static str {
        match self {
            Column::Name => COLUMN_TITLE_NAME,
            Column::Ext => COLUMN_TITLE_EXT,
            Column::Size => COLUMN_TITLE_SIZE,
            Column::Modified => COLUMN_TITLE_DATE,
            Column::Attributes => COLUMN_TITLE_ATTR,
        }
    }

    fn width(self) -> i32 {
        match self {
            Column::Name => COLUMN_WIDTH_NAME,
            Column::Ext => COLUMN_WIDTH_EXT,
            Column::Size => COLUMN_WIDTH_SIZE,
            Column::Modified => COLUMN_WIDTH_DATE,
            Column::Attributes => COLUMN_WIDTH_ATTR,
        }
    }

    /// The name column takes the leftover width; the rest stay fixed so the
    /// two panes line up with each other.
    fn expands(self) -> bool {
        matches!(self, Column::Name)
    }

    /// Sizes are right-aligned so digits line up by magnitude.
    fn xalign(self) -> f32 {
        match self {
            Column::Size => XALIGN_RIGHT,
            _ => XALIGN_LEFT,
        }
    }

    fn value(self, row: &Row) -> &str {
        match self {
            Column::Name => &row.name,
            Column::Ext => &row.ext,
            Column::Size => &row.size,
            Column::Modified => &row.modified,
            Column::Attributes => &row.attributes,
        }
    }
}

mod imp {
    use std::cell::RefCell;

    use super::*;

    /// GObject wrapper so rendered rows can live in a `gio::ListStore`.
    #[derive(Default)]
    pub struct PaneEntry {
        pub row: RefCell<Option<Row>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PaneEntry {
        const NAME: &'static str = "FcPaneEntry";
        type Type = super::PaneEntry;
    }

    impl ObjectImpl for PaneEntry {}
}

glib::wrapper! {
    pub struct PaneEntry(ObjectSubclass<imp::PaneEntry>);
}

impl PaneEntry {
    fn new(row: Row) -> Self {
        let entry: Self = glib::Object::new();
        entry.imp().row.replace(Some(row));
        entry
    }

    fn full_name(&self) -> String {
        self.imp()
            .row
            .borrow()
            .as_ref()
            .map(|row| row.full_name.clone())
            .unwrap_or_default()
    }

    /// Whether this entry would say something different with those flags.
    ///
    /// Asked rather than assigned: the answer is what keeps a mark toggle from
    /// replacing fifty thousand rows that did not change.
    fn differs(&self, selected: bool, renaming: bool) -> bool {
        self.imp()
            .row
            .borrow()
            .as_ref()
            .is_some_and(|row| row.selected != selected || row.renaming != renaming)
    }

    fn is_renaming(&self) -> bool {
        self.imp()
            .row
            .borrow()
            .as_ref()
            .is_some_and(|row| row.renaming)
    }

    fn is_marked(&self) -> bool {
        self.imp()
            .row
            .borrow()
            .as_ref()
            .is_some_and(|row| row.selected)
    }

    fn text(&self, column: Column) -> String {
        self.imp()
            .row
            .borrow()
            .as_ref()
            .map(|row| column.value(row).to_string())
            .unwrap_or_default()
    }
}

/// What an inline rename ended with.
pub enum Renamed {
    /// A name was typed and accepted.
    To(String),
    /// Escape. The row goes back to being a label and nothing is renamed.
    Abandoned,
}

/// Where the pane sends the end of an inline rename, once the shell has said
/// what to do with it.
///
/// A slot filled after construction rather than a constructor argument,
/// because the shell that has to turn a typed name into a job does not exist
/// until both panes do.
type RenameHook = Rc<RefCell<Option<Box<dyn Fn(Renamed)>>>>;

/// A pane and the directory model behind it.
pub struct PaneView {
    root: gtk::Box,
    path_bar: gtk::Label,
    filter_bar: gtk::Entry,
    status: gtk::Label,
    store: gio::ListStore,
    selection: gtk::SingleSelection,
    column_view: gtk::ColumnView,
    listing: Listing,
    /// Watches the directory this pane is showing, so a change made by
    /// anything else reaches it without being asked.
    ///
    /// Replaced on every navigation, and `None` when the directory cannot be
    /// watched at all — a pane that does not refresh itself, not one that
    /// fails to open.
    watch: Option<tc_core::watch::Watch>,
    /// The directory a read is in flight for, so an answer that arrives after
    /// the pane has moved on can be recognised and dropped.
    wanted: Option<VfsPath>,
    /// Which entry to put the cursor on when that read arrives — the directory
    /// just left, when stepping up.
    focus_on_arrival: Option<String>,
    /// Which directory [`watch`](Self::watch) is about, so the shell can tell
    /// when navigation has left it behind.
    watched: Option<VfsPath>,
    /// The full name of the row being renamed in place, if any.
    ///
    /// By name rather than by index, like everything else that has to survive
    /// a listing being rebuilt.
    renaming: Option<String>,
    /// What to do when an inline rename is accepted or abandoned.
    ///
    /// Installed by the shell after construction, the way the filter bar is
    /// wired: the pane knows when a name was typed, and the shell is the only
    /// thing that can turn that into a job.
    rename_hook: RenameHook,
    /// Kept so the two page keys that mark can measure a page; the model has
    /// no idea how tall the viewport is.
    scroller: gtk::ScrolledWindow,
    /// What was marked when the last job was submitted, for `Num /`.
    ///
    /// Names rather than rows, and held by the pane rather than the listing,
    /// because the listing this refers to no longer exists: every finished job
    /// builds a new one.
    remembered_marks: Vec<String>,
    /// How this pane orders and filters, which belongs to the *pane* and not
    /// to the directory it happens to be showing.
    ///
    /// Every navigation and every finished job builds a fresh `Listing`, so
    /// without keeping these here they would reset to the defaults each time
    /// — sorting by size and then copying a file put the order back to name.
    sort: Sort,
    show_hidden: bool,
    /// Shared rather than owned: a running job holds the same backend on its
    /// worker thread while the pane goes on using it.
    fs: Arc<dyn VirtualFs>,
    /// The archives this pane has walked into, outermost first.
    ///
    /// The one piece of state entering an archive adds to the shell, and the
    /// phase 6 plan said so out loud so the audit could check that nothing
    /// else crept in; it had not. It is what `..` at an archive's root needs: which backend to
    /// go back to, and which file to put the cursor on.
    ///
    /// A stack rather than one slot, because an archive inside an archive is
    /// then not a case anybody has to think about.
    entered: Vec<Entered>,
    /// What the read in flight will do to that stack when it lands.
    ///
    /// Applied on arrival rather than when the key was pressed: opening an
    /// archive happens on a worker thread and can fail, and a pane that had
    /// already changed backends would be pointing at something it cannot show.
    transition: Transition,
    /// Why the last navigation attempt failed, shown beside the path.
    error: Option<String>,
}

impl PaneView {
    /// Builds a pane showing `dir`.
    ///
    /// The pane owns its filesystem, because navigation re-reads through it
    /// and walking into an archive swaps it while the pane lives on.
    ///
    /// A directory that cannot be read falls back to the nearest ancestor that
    /// can, the way a re-read does. The remembered directory is somebody
    /// else's filesystem by the time it is used again — deleted, unmounted, or
    /// a path inside an archive that only means something with the archive
    /// open — and a pane that opened showing an error would strand the user
    /// somewhere they cannot navigate out of on the one screen where they have
    /// not done anything yet.
    pub fn new(fs: Arc<dyn VirtualFs>, dir: VfsPath) -> Self {
        let listing = Listing::load_nearest(fs.as_ref(), dir);
        let error = None;

        let store = gio::ListStore::new::<PaneEntry>();
        let selection = gtk::SingleSelection::new(Some(store.clone()));
        let column_view = gtk::ColumnView::new(Some(selection.clone()));

        let rename_hook: RenameHook = Rc::new(RefCell::new(None));
        for column in Column::ALL {
            // Only the name column is editable, so only it is handed the hook.
            let hook = (column == Column::Name).then(|| rename_hook.clone());
            column_view.append_column(&build_column(column, hook));
        }

        let scroller = gtk::ScrolledWindow::builder()
            .child(&column_view)
            .vexpand(true)
            .hscrollbar_policy(gtk::PolicyType::Automatic)
            .build();

        let path_bar = gtk::Label::builder()
            .xalign(0.0)
            .ellipsize(gtk::pango::EllipsizeMode::Start)
            .build();
        path_bar.add_css_class(CLASS_PATH_BAR);

        let status = gtk::Label::builder().xalign(0.0).build();
        status.add_css_class(CLASS_STATUS_LINE);

        // Hidden until Ctrl+S asks for it, so a pane that is not being
        // filtered looks exactly as it did.
        let filter_bar = gtk::Entry::builder()
            .placeholder_text(FILTER_PLACEHOLDER)
            .visible(false)
            .build();
        filter_bar.add_css_class(CLASS_FILTER_BAR);

        let root = gtk::Box::new(gtk::Orientation::Vertical, PANE_SPACING);
        root.add_css_class(CLASS_PANE);
        root.append(&path_bar);
        root.append(&filter_bar);
        root.append(&scroller);
        root.append(&status);

        let mut pane = PaneView {
            sort: listing.sort(),
            show_hidden: listing.show_hidden(),
            root,
            path_bar,
            filter_bar,
            status,
            store,
            selection,
            column_view,
            listing,
            scroller,
            wanted: None,
            focus_on_arrival: None,
            entered: Vec::new(),
            transition: Transition::Stay,
            watch: None,
            watched: None,
            renaming: None,
            rename_hook,
            remembered_marks: Vec::new(),
            fs,
            error,
        };
        pane.update_headers();
        pane.refresh();
        pane
    }

    /// The widget to place in the window.
    pub fn widget(&self) -> &gtk::Widget {
        self.root.upcast_ref()
    }

    /// The model behind the pane, for the pure functions that decide what a
    /// keystroke acts on.
    pub fn listing(&self) -> &Listing {
        &self.listing
    }

    /// A handle on this pane's backend, for a job that reads or writes here.
    pub fn fs(&self) -> Arc<dyn VirtualFs> {
        Arc::clone(&self.fs)
    }

    /// Re-reads the directory after a job may have changed it.
    ///
    /// Uses [`Listing::load_nearest`] rather than a plain reload, because the
    /// job may have deleted or moved the very directory this pane is standing
    /// in. Showing an error where a listing belongs would strand the user
    /// somewhere they cannot navigate out of; landing on the nearest
    /// surviving ancestor keeps the pane usable.
    pub fn reload_after_job(&mut self) {
        let focused = self.listing.current().map(|entry| entry.name.clone());
        let mut listing = Listing::load_nearest(self.fs.as_ref(), self.listing.dir().clone());
        self.adopt(&mut listing);
        if let Some(name) = focused {
            listing.focus_entry(&name);
        }
        self.listing = listing;
        self.error = None;
        self.refresh();
    }

    /// Starts watching whatever directory this pane is now showing.
    ///
    /// Called after every navigation, because the old watch is about a
    /// directory nobody is looking at any more. Returns where the nudges will
    /// arrive, for the shell to await on the main loop.
    pub fn rewatch(&mut self) -> Option<tc_core::watch::Changes> {
        // Set even when the watch could not be started, so a directory that
        // cannot be watched is not retried on every keystroke.
        self.watched = Some(self.listing.dir().clone());
        // Inside an archive there is nothing to watch: the path is one this
        // backend understands and the operating system does not, and starting
        // an inotify watch on it would either fail or, worse, register a real
        // directory that happens to have the same name. `Ctrl+R` still
        // re-reads (`docs/archives.md`).
        if self.in_archive() {
            return None;
        }
        // The old one goes on a thread of its own. Dropping a watcher joins
        // the worker inside it, and that worker sits in a poll with a timeout
        // — so letting the drop happen here stalled every navigation by up to
        // a fifth of a second, measured. Nothing waits for it: an inotify
        // registration that outlives its pane by a few milliseconds costs
        // nothing at all.
        if let Some(previous) = self.watch.take() {
            std::thread::spawn(move || drop(previous));
        }
        self.watch = tc_core::watch::Watch::start(self.listing.dir());
        self.watch.as_ref().map(|watch| watch.changes())
    }

    /// Whether the watch is about somewhere this pane has since left.
    pub fn watch_is_stale(&self) -> bool {
        self.watched.as_ref() != Some(self.listing.dir())
    }

    /// The directory this pane is showing, for a watcher to check it is still
    /// the one it was started for.
    pub fn directory(&self) -> VfsPath {
        self.listing.dir().clone()
    }

    /// Re-reads the directory, keeping everything the user put there.
    ///
    /// `Ctrl+R`, and what a directory watcher calls. Through
    /// [`Listing::reload`] rather than a fresh load, because that is the one
    /// that keeps the **marks** by name — a re-read that silently dropped a
    /// selection somebody spent a minute building would be worse than not
    /// re-reading at all. The cursor follows the same way, and the scroll
    /// offset is put back around it.
    ///
    /// A directory that has gone falls back to the nearest ancestor that can
    /// still be read, the same as after a job: showing an error where a
    /// listing belongs strands the user somewhere they cannot navigate out of.
    pub fn reread(&mut self) {
        // The pixel offset, not the row: a change elsewhere in the directory
        // must not move what is being looked at, and rows above the viewport
        // coming and going is the rarer case.
        let scroll = self.scroller.vadjustment().value();
        if self.listing.reload(self.fs.as_ref()).is_err() {
            self.reload_after_job();
            return;
        }
        self.error = None;
        self.refresh();
        // After `refresh`, which scrolls to the cursor: this is the one that
        // has to win.
        self.scroller.vadjustment().set_value(scroll);
    }

    /// Updates the marks on the rows already in the store, without rebuilding.
    ///
    /// For the marking commands, which change what a row *says* rather than
    /// which rows there are — and which are the ones pressed over and over.
    ///
    /// **Not** the rename editor, though it changes a row the same way: a
    /// spliced row does not end up with the keyboard focus the way a rebuilt
    /// one does, and a rename field that opens without the focus is no field
    /// at all. Renaming happens once in a while and can afford the rebuild;
    /// marking cannot.
    ///
    /// The distinction is not tidiness. Rebuilding a fifty-thousand entry
    /// store costs 69 ms and allocates a `PaneEntry` per row; updating one row
    /// costs 3 µs (`docs/performance.md`). Pressing Space used to pay the
    /// former — and rebuilding also empties the store, which drops the scroll
    /// adjustment to zero before `sync_cursor` puts it back, so the view
    /// visibly moved for a keystroke that changed one row's colour.
    fn refresh_marks(&mut self) {
        // Which rows actually say something different now. Everything else is
        // left exactly as it is, object identity included.
        let changed: Vec<usize> = (0..self.listing.len())
            .filter(|&index| {
                self.store
                    .item(index as u32)
                    .and_downcast::<PaneEntry>()
                    .is_some_and(|entry| {
                        let renaming = self.renaming.as_deref() == Some(entry.full_name().as_str());
                        entry.differs(self.listing.is_selected(index), renaming)
                    })
            })
            .collect();

        if let (Some(&first), Some(&last)) = (changed.first(), changed.last()) {
            // The span between the first and last change, replaced in one
            // splice. **Replaced**, not mutated: a `ListView` rebinds a cell
            // when its item is a different object, and mutating one behind the
            // model's back leaves the row on screen saying what it used to —
            // the mark would not repaint and the rename field would not open.
            //
            // One splice rather than a remove-all and refill, because emptying
            // the store drops the scroll adjustment to zero and the view jumps
            // for a keystroke that changed one row's colour.
            let replacements: Vec<PaneEntry> =
                (first..=last).map(|index| self.entry_at(index)).collect();
            self.store
                .splice(first as u32, replacements.len() as u32, &replacements);
        }

        self.status
            .set_text(&crate::jobs::selection_status(&self.listing));
        self.sync_cursor();
    }

    /// One row of the store, built from the listing.
    fn entry_at(&self, index: usize) -> PaneEntry {
        let entry = self
            .listing
            .get(index)
            .expect("indices below len() always resolve");
        let mut row = Row::from_entry(
            entry,
            self.listing.is_parent(index),
            self.listing.is_selected(index),
        );
        row.renaming = self.renaming.as_deref() == Some(row.full_name.as_str());
        PaneEntry::new(row)
    }

    /// Rebuilds the rows from the listing and puts the selection back on the
    /// cursor.
    ///
    /// For when the *set* of rows changed — a navigation, a sort, a filter, a
    /// re-read. Anything that only changes what a row says goes through
    /// [`refresh_marks`](Self::refresh_marks) instead.
    pub fn refresh(&mut self) {
        let shown = self.shown_dir();
        let path = shown.as_str();
        self.path_bar.set_text(&match &self.error {
            Some(reason) => format!("{path}{PATH_BAR_ERROR_SEPARATOR}{reason}"),
            None => path.to_string(),
        });

        // Emptying and refilling the store makes the widget move its own
        // selection, which `adopt_selection` would then read back as the
        // user's intent. The model's cursor is restored afterwards.
        let cursor = self.listing.cursor();
        self.store.remove_all();
        for index in 0..self.listing.len() {
            self.store.append(&self.entry_at(index));
        }

        self.listing.set_cursor(cursor);
        self.status
            .set_text(&crate::jobs::selection_status(&self.listing));
        self.sync_cursor();
    }

    /// The field the quick filter is typed into, so the shell can wire its
    /// own key handling to it.
    pub fn filter_bar(&self) -> &gtk::Entry {
        &self.filter_bar
    }

    /// Shows the filter field and puts the cursor in it.
    pub fn begin_filter(&self) {
        self.filter_bar.set_visible(true);
        self.filter_bar.grab_focus();
    }

    /// Applies whatever is in the field.
    pub fn apply_filter(&mut self) {
        let text = self.filter_bar.text().to_string();
        self.listing.set_filter(&text);
        self.refresh();
    }

    /// Stops filtering, hides the field and hands the keyboard back to the
    /// rows.
    pub fn reset_filter(&mut self) {
        self.filter_bar.set_text("");
        self.filter_bar.set_visible(false);
        self.listing.set_filter("");
        self.refresh();
        self.grab_focus();
    }

    /// Leaves the narrowed view in place but hands the keyboard back to the
    /// rows, which is what Enter in the filter field means.
    pub fn leave_filter(&self) {
        self.grab_focus();
    }

    /// Sorts by `key`, flipping the direction when it is already the one in
    /// force.
    pub fn sort_by(&mut self, key: SortKey) {
        self.sort = self.sort.cycled(key);
        self.listing.set_sort(self.sort);
        self.update_headers();
        self.refresh();
    }

    /// Shows or hides the dot-files.
    pub fn toggle_hidden(&mut self) {
        self.show_hidden = !self.show_hidden;
        self.listing.toggle_hidden();
        self.refresh();
    }

    /// Opens the pane with a remembered ordering and hidden-file flag.
    pub fn restore(&mut self, sort: Sort, show_hidden: bool) {
        self.sort = sort;
        self.show_hidden = show_hidden;
        let mut listing =
            std::mem::replace(&mut self.listing, Listing::new(VfsPath::root(), Vec::new()));
        self.adopt(&mut listing);
        self.listing = listing;
        self.update_headers();
        self.refresh();
    }

    /// What this pane would want back next time.
    ///
    /// A pane inside an archive records the path it is actually showing, which
    /// is a path only that archive understands. Restoring it is
    /// [`Listing::load_nearest`]'s ordinary business: reading a directory
    /// inside an archive off the local filesystem fails, and it walks up until
    /// something reads — which is the directory holding the archive. So a
    /// restart lands beside the archive rather than inside it, without a
    /// special case for saying so (`docs/archives.md`).
    pub fn state(&self) -> (VfsPath, Sort, bool) {
        (self.shown_dir(), self.sort, self.show_hidden)
    }

    /// Where this pane is, spelled so a person can read it.
    ///
    /// Inside an archive the listing's own directory is `/` — the archive's
    /// root, which is all the archive backend knows about. What the user is
    /// looking at is `…/bundle.zip/deeper`, and that is what the path bar and
    /// the settings file want: one is unreadable without the archive in it,
    /// and the other would send the next start to the filesystem root.
    ///
    /// Composed across the whole stack, so an archive inside an archive reads
    /// the way it looks.
    ///
    /// **Not** what an operation uses. A job addresses its backend, and this
    /// path means nothing to one — [`target_dir`](Self::target_dir) is that.
    fn shown_dir(&self) -> VfsPath {
        if self.entered.is_empty() {
            return self.target_dir();
        }
        let mut path = VfsPath::root();
        for step in self
            .entered
            .iter()
            .map(|entered| &entered.archive)
            .chain(std::iter::once(&self.target_dir()))
        {
            for component in step.components() {
                path = path.child(component);
            }
        }
        path
    }

    /// Puts this pane's ordering, hidden-file flag and filter onto a listing
    /// that has just been read.
    fn adopt(&self, listing: &mut Listing) {
        listing.set_sort(self.sort);
        if listing.show_hidden() != self.show_hidden {
            listing.toggle_hidden();
        }
        listing.set_filter(&self.filter_bar.text());
    }

    /// Marks the column the listing is ordered by, and which way.
    ///
    /// The header text is the marker: this shell sorts in the model, so there
    /// is no GTK sorter whose arrow GTK would draw for us.
    fn update_headers(&self) {
        let sort = self.listing.sort();
        for (position, column) in Column::ALL.iter().enumerate() {
            let Some(view_column) = self
                .column_view
                .columns()
                .item(position as u32)
                .and_downcast::<gtk::ColumnViewColumn>()
            else {
                continue;
            };
            let marker = match (column.sort_key(), sort.key, sort.order) {
                (Some(key), active, SortOrder::Ascending) if key == active => SORT_MARKER_ASCENDING,
                (Some(key), active, SortOrder::Descending) if key == active => {
                    SORT_MARKER_DESCENDING
                }
                _ => "",
            };
            view_column.set_title(Some(&format!("{}{marker}", column.title())));
        }
    }

    /// Flips the mark on the cursor row, then steps `step` rows.
    ///
    /// One method for four keys, because in Total Commander they are one
    /// behaviour: `Space` marks without moving, `Insert` and `Shift+↓` mark
    /// and step down so the key can be held, and `Shift+↑` does the same
    /// upwards. Marking the row being *left* rather than the one arrived at
    /// is TC's own rule, and it is what makes running back over a row take
    /// its mark off again.
    pub fn toggle_mark(&mut self, step: isize) {
        self.adopt_selection();
        self.listing.toggle_selected(self.listing.cursor());
        self.listing.move_cursor_by(step);
        self.refresh_marks();
    }

    /// Marks every row between the cursor and `target`, then goes there.
    ///
    /// `Shift+Home`/`End`/`PgUp`/`PgDn`: a range, not a toggle, because a
    /// jump has no direction to run back over and "flip everything I passed"
    /// is not what a person asking for "to the end" means.
    pub fn extend_mark_to(&mut self, target: usize) {
        self.adopt_selection();
        self.listing
            .select_range(self.listing.cursor(), target, true);
        self.listing.set_cursor(target);
        self.refresh_marks();
    }

    /// Exchanges everything this pane is showing with another's.
    ///
    /// The contents, not the widgets: both panes are children of a `Paned`,
    /// and reparenting them would be work for no reason. Swapping the whole
    /// `Listing` is what makes the directory, the cursor and the marks come
    /// along together — anything reconstructed field by field would quietly
    /// drop one of them.
    pub fn exchange_with(&mut self, other: &mut PaneView) {
        self.adopt_selection();
        other.adopt_selection();
        // The backend and the archive stack travel with the listing. Swapping
        // only the listing would leave each pane showing the other's entries
        // through its own backend — which, once one of them can be an archive,
        // is two panes both looking at the wrong filesystem.
        std::mem::swap(&mut self.fs, &mut other.fs);
        std::mem::swap(&mut self.entered, &mut other.entered);
        std::mem::swap(&mut self.listing, &mut other.listing);
        std::mem::swap(&mut self.sort, &mut other.sort);
        std::mem::swap(&mut self.show_hidden, &mut other.show_hidden);
        std::mem::swap(&mut self.remembered_marks, &mut other.remembered_marks);
        let filter = self.filter_bar.text();
        self.set_filter_text(&other.filter_bar.text());
        other.set_filter_text(&filter);
        for pane in [self, other] {
            pane.update_headers();
            pane.refresh();
        }
    }

    /// Puts text into the quick-filter field, showing or hiding it to match.
    fn set_filter_text(&self, filter: &str) {
        self.filter_bar.set_text(filter);
        self.filter_bar.set_visible(!filter.is_empty());
    }

    /// Starts an inline rename of the row under the cursor.
    ///
    /// Total Commander's `Shift+F6`: the name turns into a field in the list
    /// itself, rather than a dialog that covers the thing being renamed.
    ///
    /// `..` is not a file and cannot be renamed, so it does nothing there.
    ///
    /// That guard is belt-and-braces and is recorded as such: a probe removing
    /// it could not get an editor to open on `..` either, so nothing today
    /// reaches it. It stays because of what is on the other side — `..`
    /// resolves to the *parent directory*, and a rename accepted there would
    /// be a move of the directory you are standing in. Three lines against
    /// that is a trade worth making even when the case is unreachable.
    pub fn begin_rename(&mut self) {
        self.adopt_selection();
        if self.listing.is_parent(self.listing.cursor()) {
            return;
        }
        self.renaming = self.current_name();
        self.refresh();
    }

    /// Puts the row back to being a label. The rename itself is the shell's.
    pub fn end_rename(&mut self) {
        if self.renaming.take().is_some() {
            self.refresh();
            self.grab_focus();
        }
    }

    /// Whether a rename is in progress, and of what.
    pub fn renaming(&self) -> Option<&str> {
        self.renaming.as_deref()
    }

    /// Installs what happens when an inline rename ends.
    pub fn on_rename(&self, hook: impl Fn(Renamed) + 'static) {
        *self.rename_hook.borrow_mut() = Some(Box::new(hook));
    }

    /// The name of the row under the cursor.
    pub fn current_name(&self) -> Option<String> {
        self.listing.current().map(|entry| entry.name.clone())
    }

    /// The path of the row under the cursor, when it is a **file**.
    ///
    /// `None` on `..` and on a directory: what F3 and F4 do with one is
    /// nothing, which is what Total Commander does too.
    pub fn current_file(&self) -> Option<VfsPath> {
        let entry = self.listing.current()?;
        if entry.is_dir() || self.listing.is_parent(self.listing.cursor()) {
            return None;
        }
        self.listing.current_path()
    }

    /// Whether the last navigation failed and left the pane where it was.
    pub fn went_wrong(&self) -> bool {
        self.error.is_some()
    }

    /// Where the cursor is, and the last row it could be on.
    pub fn cursor(&self) -> usize {
        self.listing.cursor()
    }

    pub fn last_row(&self) -> usize {
        self.listing.len().saturating_sub(1)
    }

    /// How many rows fit on screen, for the two page keys that mark.
    ///
    /// Measured rather than assumed: the model has no idea how tall the
    /// viewport is, which is exactly why plain Page Up/Down are left to the
    /// widget ([`docs/keymap.md`]). The adjustment knows the content height
    /// and the viewport height, and the rows are uniform, so the row count
    /// falls out of the ratio. Before the first layout there is no height to
    /// divide by and the fallback stands in.
    pub fn page_rows(&self) -> usize {
        let adjustment = self.scroller.vadjustment();
        let (content, viewport) = (adjustment.upper(), adjustment.page_size());
        let rows = self.listing.len() as f64;
        if content <= 0.0 || viewport <= 0.0 || rows <= 0.0 {
            return PAGE_ROWS_FALLBACK;
        }
        let row_height = content / rows;
        ((viewport / row_height) as usize).max(1)
    }

    pub fn mark_matching(&mut self, pattern: &str, selected: bool) {
        self.listing.select_matching(pattern, selected);
        self.refresh_marks();
    }

    /// Flips the visible files, leaving directories alone — Total Commander's
    /// `Num *`. `including_folders` is its `Shift+Num *`.
    pub fn invert_marks(&mut self, including_folders: bool) {
        if including_folders {
            self.listing.invert_selection();
        } else {
            self.listing.invert_selection_files();
        }
        self.refresh_marks();
    }

    pub fn mark_all(&mut self) {
        self.listing.select_all();
        self.refresh_marks();
    }

    pub fn unmark_all(&mut self) {
        self.listing.clear_selection();
        self.refresh_marks();
    }

    /// `Alt+Num ±`: every visible file sharing the cursor row's extension.
    pub fn mark_same_extension(&mut self, selected: bool) {
        self.adopt_selection();
        self.listing.select_same_extension(selected);
        self.refresh_marks();
    }

    /// Puts away what is marked, so `Num /` can bring it back.
    ///
    /// Called when a job is submitted rather than when it finishes: the job
    /// ends with a fresh listing, and by then the marks it consumed are gone.
    pub fn remember_marks(&mut self) {
        self.remembered_marks = self.listing.selected_names();
    }

    /// `Num /`: the selection from before the last operation.
    pub fn restore_marks(&mut self) {
        let remembered = std::mem::take(&mut self.remembered_marks);
        self.listing.set_selected_names(&remembered);
        self.remembered_marks = remembered;
        self.refresh_marks();
    }

    /// Selects the listing's cursor row, focuses it, and scrolls it into
    /// view — the three halves of "the cursor is here" as far as the widget
    /// is concerned.
    ///
    /// Setting the selection alone moves nothing: only the widget's own key
    /// handling scrolls, and this shell binds the cursor keys itself, so the
    /// selection would silently walk off screen. `FOCUS` matters as much as
    /// the scrolling — Page Up/Down are handled by the widget and page from
    /// *its* focus, so the focus has to follow our cursor or paging resumes
    /// from wherever it last was.
    fn sync_cursor(&self) {
        if self.listing.is_empty() {
            return;
        }
        // While a row is being renamed the focus belongs to its editor, and
        // asking the column view for it back would take it away. That the
        // rename worked at all before this was ordering luck: the cell happens
        // to bind and grab the focus *after* the scroll for a row further
        // down, and did not for the first row — which is how a probe on the
        // `..` guard came to pass with the guard removed.
        let focus = match self.renaming {
            Some(_) => gtk::ListScrollFlags::empty(),
            None => gtk::ListScrollFlags::FOCUS,
        };
        let row = self.listing.cursor() as u32;
        self.column_view
            .scroll_to(row, None, focus | gtk::ListScrollFlags::SELECT, None);
    }

    /// Takes over a selection the widget moved on its own.
    ///
    /// Keys this shell does not bind — Page Up/Down above all — fall through
    /// to the `ColumnView`, which moves its selection without telling the
    /// model. Adopting that selection before acting keeps the two from
    /// drifting apart, so the next Enter opens the row the user is actually
    /// looking at.
    ///
    /// Paging is deliberately left to the widget: it knows the height of the
    /// viewport, and the model has no idea how many rows are on screen.
    pub fn adopt_selection(&mut self) {
        if let Some(cursor) = adopted_cursor(self.selection.selected()) {
            self.listing.set_cursor(cursor);
        }
    }

    /// Moves the cursor by `delta` rows.
    pub fn move_cursor_by(&mut self, delta: isize) {
        self.listing.move_cursor_by(delta);
        self.sync_cursor();
    }

    pub fn move_cursor_to_first(&mut self) {
        self.listing.move_cursor_to_first();
        self.sync_cursor();
    }

    pub fn move_cursor_to_last(&mut self) {
        self.listing.move_cursor_to_last();
        self.sync_cursor();
    }

    /// Sends this pane somewhere, which is what the drive bar does.
    #[must_use = "the caller has to await the listing, or the pane never moves"]
    pub fn go_to(&mut self, dir: VfsPath) -> Loading {
        self.navigate_to(dir)
    }

    /// Sends this pane to `dir` on the **outermost** backend, leaving behind
    /// any archive it had walked into.
    ///
    /// What the drive bar needs. Navigating on the current backend would send
    /// the *archive* to `/mnt/backup`, which is a path it has never heard of —
    /// so the pane would show an error and still be inside the archive, with
    /// no way out but Backspace.
    #[must_use = "the caller has to await the listing, or the pane never moves"]
    pub fn leave_for(&mut self, dir: VfsPath) -> Loading {
        let outermost = self
            .entered
            .first()
            .map_or_else(|| Arc::clone(&self.fs), |entered| Arc::clone(&entered.fs));
        self.start(dir.clone(), None);
        self.transition = Transition::Adopt(Vec::new());
        Listing::spawn_load(outermost, dir)
    }

    /// Sends this pane to wherever `other` is — same backend, same archives.
    ///
    /// What `Ctrl+←` and `Ctrl+→` mean. Copying only the *path* would send
    /// this pane's own backend to a path that belongs to the other's, which
    /// inside an archive is a path on the disk that has nothing to do with it.
    #[must_use = "the caller has to await the listing, or the pane never moves"]
    pub fn follow(&mut self, other: &PaneView) -> Loading {
        let (fs, dir) = (Arc::clone(&other.fs), other.target_dir());
        self.start(dir.clone(), None);
        self.transition = Transition::Adopt(other.entered.clone());
        Listing::spawn_load(fs, dir)
    }

    /// Enters what is under the cursor: a directory, or an archive as if it
    /// were one. Does nothing on any other file, which is what F3 and F4 are
    /// for.
    #[must_use = "the caller has to await the listing, or the pane never moves"]
    pub fn activate(&mut self) -> Option<Loading> {
        match activation_step(&self.listing)? {
            Step::Into(target) => Some(self.navigate_to(target)),
            Step::Enter(archive) => Some(self.enter_archive(archive)),
            Step::Out => self.go_parent(),
        }
    }

    /// Opens the archive at `archive` and shows its root.
    #[must_use = "the caller has to await the listing, or the pane never moves"]
    fn enter_archive(&mut self, archive: VfsPath) -> Loading {
        let outer = Arc::clone(&self.fs);
        let mut stack = self.entered.clone();
        stack.push(Entered {
            fs: Arc::clone(&outer),
            archive: archive.clone(),
        });
        self.start(VfsPath::root(), None);
        self.transition = Transition::Adopt(stack);
        Listing::spawn_enter(outer, archive)
    }

    /// Leaves the current directory. Does nothing at the root of the outermost
    /// filesystem.
    ///
    /// At the root of an *archive* it leaves the archive: back to the backend
    /// the archive was opened from, in the directory holding it, with the
    /// cursor on the archive file. That is what `..` has always meant here —
    /// where you came from ([`focus_after_move`]) — and it is why the pane
    /// remembers what it entered.
    #[must_use = "the caller has to await the listing, or the pane never moves"]
    pub fn go_parent(&mut self) -> Option<Loading> {
        if let Some(target) = parent_target(&self.listing) {
            return Some(self.navigate_to(target));
        }
        let entered = self.entered.last()?;
        let (outer, archive) = (Arc::clone(&entered.fs), entered.archive.clone());
        let containing = archive.parent()?;
        let mut stack = self.entered.clone();
        stack.pop();
        self.start(containing.clone(), archive.file_name().map(str::to_string));
        self.transition = Transition::Adopt(stack);
        Some(Listing::spawn_load(outer, containing))
    }

    /// Whether this pane is looking inside an archive.
    ///
    /// Asked by the things that only make sense on a real filesystem — the
    /// directory watcher and the command line — because an archive has no path
    /// the operating system knows (`docs/archives.md`).
    pub fn in_archive(&self) -> bool {
        !self.entered.is_empty()
    }

    /// Shows `dir`, or stays put and reports why it could not.
    ///
    /// A pane that cannot read a directory must not end up displaying it as
    /// empty: leaving the user where they were, with the reason next to the
    /// path, keeps the pane in a state they can navigate out of.
    /// Asks for a directory, and hands back where the answer will arrive.
    ///
    /// The pane keeps showing what it has until the new listing turns up:
    /// blanking it first would flash an empty pane on every step, and there is
    /// nothing to put there that is more true than what is already on screen.
    #[must_use = "the caller has to await the listing, or the pane never moves"]
    fn navigate_to(&mut self, dir: VfsPath) -> Loading {
        let focus = focus_after_move(self.listing.dir(), &dir);
        self.start(dir.clone(), focus);
        self.transition = Transition::Stay;
        Listing::spawn_load(Arc::clone(&self.fs), dir)
    }

    /// Records that a move to `dir` is in flight, whatever backend answers it.
    fn start(&mut self, dir: VfsPath, focus: Option<String>) {
        // A filter belongs to the directory it was typed in. Carrying it into
        // the next one would show an empty pane and no reason why.
        self.filter_bar.set_text("");
        self.filter_bar.set_visible(false);
        self.focus_on_arrival = focus;
        self.wanted = Some(dir);
    }

    /// Puts the cursor on `path` when the listing being read arrives.
    ///
    /// For a search result: going to a file's directory is only half of what
    /// was asked for, and hunting for the row afterwards is the other half
    /// nobody wants to do by hand.
    pub fn focus_on_arrival(&mut self, path: &VfsPath) {
        self.focus_on_arrival = path.file_name().map(str::to_string);
    }

    /// The directory a read is in flight for.
    pub fn awaiting(&self) -> Option<VfsPath> {
        self.wanted.clone()
    }

    /// The directory this pane is **about**: where it is, or where it is on
    /// its way to.
    ///
    /// Reading the directory off the listing is not the same thing now that a
    /// read happens on a worker thread. Keys arrive faster than listings: press
    /// Enter and then F7 quickly enough and both are dispatched before the
    /// first read lands, so a pane asked where it *is* would answer with the
    /// directory it is leaving — and F7 would make the directory in the wrong
    /// place. A pane that has been told to go somewhere is already about that
    /// somewhere.
    ///
    /// Everything that acts *in* a directory uses this. Anything acting on
    /// what is **marked** keeps reading the listing, because the marks belong
    /// to the rows the user was looking at when they made them.
    pub fn target_dir(&self) -> VfsPath {
        self.wanted
            .clone()
            .unwrap_or_else(|| self.listing.dir().clone())
    }

    /// Takes a listing that was read on a worker thread.
    ///
    /// `dir` is what was asked for. A pane that has since been sent somewhere
    /// else drops it on the floor: two quick steps would otherwise land in
    /// whichever order the reads happened to finish, which is a pane that
    /// sometimes goes back.
    pub fn arrived(&mut self, dir: &VfsPath, arrival: Arrival) {
        if self.wanted.as_ref() != Some(dir) {
            return;
        }
        self.wanted = None;
        let transition = std::mem::replace(&mut self.transition, Transition::Stay);
        match arrival {
            Ok((fs, mut listing)) => {
                // The backend arrives with the listing, so a step that changed
                // it — into an archive, or back out of one — takes effect at
                // the moment there is something to show, and a step that
                // failed leaves the pane exactly where it was.
                self.fs = fs;
                if let Transition::Adopt(entered) = transition {
                    self.entered = entered;
                }
                // At the root of an archive there is no parent inside it, and
                // still somewhere to go: back out. The row is offered here
                // rather than by the listing, which has no idea it is an
                // archive's root rather than a filesystem's.
                if self.in_archive() && listing.dir().is_root() {
                    listing.offer_parent();
                }
                self.adopt(&mut listing);
                if let Some(name) = self.focus_on_arrival.take() {
                    listing.focus_entry(&name);
                }
                self.listing = listing;
                self.error = None;
            }
            Err(reason) => {
                let name = dir.file_name().unwrap_or(dir.as_str()).to_string();
                self.error = Some(format!("{name}: {reason}"));
            }
        }
        self.refresh();
    }

    /// Gives this pane the keyboard focus, so its cursor row is drawn as the
    /// focused selection rather than a dim one.
    pub fn grab_focus(&self) {
        self.column_view.grab_focus();
    }

    /// Marks this pane as the one keystrokes go to.
    pub fn set_active(&self, active: bool) {
        if active {
            self.root.add_css_class(CLASS_PANE_ACTIVE);
        } else {
            self.root.remove_css_class(CLASS_PANE_ACTIVE);
        }
    }
}

/// Builds one column with a label-per-cell factory.
/// Names of the two pages in a name cell's stack.
const STACK_LABEL: &str = "label";
const STACK_EDITOR: &str = "editor";

/// Shows the label or the editor, and hands back the label either way.
///
/// The editor carries the **whole** filename, extension included: the name and
/// ext columns are a presentation split, and renaming `notes` to `todo` while
/// silently keeping `.txt` in another column is not something the user can see
/// to have agreed to.
fn bind_name_cell(stack: &gtk::Stack, entry: &PaneEntry) -> gtk::Label {
    let label = stack
        .child_by_name(STACK_LABEL)
        .and_downcast::<gtk::Label>()
        .expect("setup named the label");
    let editor = stack
        .child_by_name(STACK_EDITOR)
        .and_downcast::<gtk::Entry>()
        .expect("setup named the editor");

    if !entry.is_renaming() {
        stack.set_visible_child_name(STACK_LABEL);
        return label;
    }

    let full_name = entry.full_name();
    editor.set_text(&full_name);
    stack.set_visible_child_name(STACK_EDITOR);
    // Only when it is not already ours: a cell rebinds when the list scrolls,
    // and grabbing the focus again would fight whoever is typing.
    if !editor.has_focus() {
        editor.grab_focus();
        // The stem, not the whole name — changing `notes.txt` to `todo.txt`
        // is the ordinary case, and retyping the extension every time is the
        // annoying one.
        let stem = split_name(&full_name).0.chars().count() as i32;
        editor.select_region(0, stem);
    }
    label
}

/// Builds one column. `rename` is `Some` only for the name column, which is
/// the one that can turn into an editable field.
///
/// The editor is a `Stack` per *recycled cell*, not per entry: a `ColumnView`
/// keeps widgets only for the rows on screen, so this is some forty entries
/// deep rather than fifty thousand (`docs/performance.md`).
fn build_column(column: Column, rename: Option<RenameHook>) -> gtk::ColumnViewColumn {
    let factory = gtk::SignalListItemFactory::new();

    let setup_rename = rename.clone();
    factory.connect_setup(move |_, item| {
        let item = list_item(item);
        let label = gtk::Label::builder()
            .xalign(column.xalign())
            .ellipsize(gtk::pango::EllipsizeMode::Middle)
            .build();
        let Some(hook) = setup_rename.clone() else {
            item.set_child(Some(&label));
            return;
        };

        let editor = gtk::Entry::builder().has_frame(false).build();
        let accepting = hook.clone();
        editor.connect_activate(move |editor| {
            if let Some(hook) = accepting.borrow().as_ref() {
                hook(Renamed::To(editor.text().to_string()));
            }
        });
        // Capture phase, for the reason the command line's handler is:
        // `GtkText` consumes Escape itself, so a bubble-phase handler never
        // sees it and there is no way out of the field but the mouse.
        let controller = gtk::EventControllerKey::new();
        controller.set_propagation_phase(gtk::PropagationPhase::Capture);
        let abandoning = hook.clone();
        controller.connect_key_pressed(move |_, key, _, _| {
            if key != gtk::gdk::Key::Escape {
                return glib::Propagation::Proceed;
            }
            if let Some(hook) = abandoning.borrow().as_ref() {
                hook(Renamed::Abandoned);
            }
            glib::Propagation::Stop
        });
        editor.add_controller(controller);

        let stack = gtk::Stack::new();
        stack.add_named(&label, Some(STACK_LABEL));
        stack.add_named(&editor, Some(STACK_EDITOR));
        item.set_child(Some(&stack));
    });

    factory.connect_bind(move |_, item| {
        let item = list_item(item);
        let entry = item
            .item()
            .and_downcast::<PaneEntry>()
            .expect("the store holds PaneEntry values");
        let child = item.child().expect("setup installed a child");

        let label = match child.downcast_ref::<gtk::Stack>() {
            Some(stack) => bind_name_cell(stack, &entry),
            None => child
                .downcast::<gtk::Label>()
                .expect("setup installed a Label"),
        };
        label.set_text(&entry.text(column));
        // Marked rows are coloured, which is how Total Commander shows them
        // and the only cue that survives the row also being the cursor.
        if entry.is_marked() {
            label.add_css_class(CLASS_MARKED);
        } else {
            label.remove_css_class(CLASS_MARKED);
        }
    });

    let view_column = gtk::ColumnViewColumn::new(Some(column.title()), Some(factory));
    view_column.set_fixed_width(column.width());
    view_column.set_expand(column.expands());
    view_column
}

/// From GTK 4.12 the factory hands over a plain `Object`, because a factory
/// can also produce header and cell items. A column's factory only ever makes
/// list items.
fn list_item(item: &glib::Object) -> &gtk::ListItem {
    item.downcast_ref::<gtk::ListItem>()
        .expect("a column view factory always yields ListItems")
}

/// An archive this pane walked into, and where it came from.
#[derive(Clone)]
struct Entered {
    /// The backend the archive file itself lives on.
    fs: Arc<dyn VirtualFs>,
    /// Where the archive file is on that backend.
    archive: VfsPath,
}

/// What the read in flight will do to the pane's archive stack.
enum Transition {
    /// An ordinary move: the backend does not change.
    Stay,
    /// Every other case is "the stack becomes this" — walking into an archive,
    /// back out of one, following the other pane, or leaving for a drive. One
    /// variant rather than four, because they differ only in the list.
    Adopt(Vec<Entered>),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(selected: bool, renaming: bool) -> Row {
        Row {
            name: "notes".to_string(),
            ext: "txt".to_string(),
            full_name: "notes.txt".to_string(),
            size: "0".to_string(),
            modified: String::new(),
            attributes: String::new(),
            is_dir: false,
            selected,
            renaming,
        }
    }

    #[test]
    fn a_row_that_did_not_move_is_not_replaced() {
        // The guard that makes marking cheap. Without it, pressing Space in a
        // fifty-thousand entry directory replaces every row: 70 ms and a
        // scroll adjustment reset, for one row changing colour
        // (`docs/performance.md`).
        let entry = PaneEntry::new(row(false, false));
        assert!(!entry.differs(false, false), "nothing moved");
        assert!(entry.differs(true, false), "the mark moved");
        assert!(entry.differs(false, true), "the rename moved");
    }
}
