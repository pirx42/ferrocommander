//! One pane: a path bar above a column view of a directory.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use gtk::gio;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

use fc_core::branch;
use fc_core::config;
use fc_core::listing::{split_name, Arrival, Listing, Loading, Sort, SortKey, SortOrder};
use fc_core::ops::CancelToken;
use fc_core::sizes::{self, Sizes};
use fc_core::vfs::constants::SEPARATOR;
use fc_core::vfs::{VfsPath, VirtualFs};

use crate::constants::{
    BRANCH_MARKER, CELL_REPAINT, CLASS_FILTER_BAR, CLASS_MARKED, CLASS_OUTPUT, CLASS_PANE,
    CLASS_PANE_ACTIVE, CLASS_PATH_BAR, CLASS_STATUS_LINE, COLUMN_TITLE_ATTR, COLUMN_TITLE_DATE,
    COLUMN_TITLE_EXT, COLUMN_TITLE_NAME, COLUMN_TITLE_SIZE, COLUMN_WIDTH_ATTR, COLUMN_WIDTH_DATE,
    COLUMN_WIDTH_EXT, COLUMN_WIDTH_NAME, COLUMN_WIDTH_SIZE, DISK_SPACE, FILTER_PLACEHOLDER,
    PAGE_OVERLAP_ROWS, PAGE_ROWS_FALLBACK, PANE_PAGE_LIST, PANE_PAGE_PREVIEW, PANE_SPACING,
    PATH_BAR_ERROR_SEPARATOR, RIGHT_BUTTON, ROW_ICON_GAP, ROW_ICON_SIZE, ROW_REVISION,
    SCROLL_RESTORE_PRIORITY, SORT_MARKER_ASCENDING, SORT_MARKER_DESCENDING, TYPE_AHEAD_TIMEOUT,
    XALIGN_LEFT, XALIGN_RIGHT,
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

    /// This column's width in the settings, if it has one there.
    ///
    /// `None` for the name column, which has no stored width because it takes
    /// whatever is left.
    fn width_in(self, widths: &config::ColumnSettings) -> Option<i32> {
        Some(match self {
            Column::Name => return None,
            Column::Ext => widths.ext,
            Column::Size => widths.size,
            Column::Modified => widths.date,
            Column::Attributes => widths.attributes,
        })
    }

    /// Writes this column's width back into the settings.
    fn set_width_in(self, widths: &mut config::ColumnSettings, width: i32) {
        match self {
            Column::Name => {}
            Column::Ext => widths.ext = width,
            Column::Size => widths.size = width,
            Column::Modified => widths.date = width,
            Column::Attributes => widths.attributes = width,
        }
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
            // While a row is being renamed the entry holds the *whole* name,
            // extension and all, so an Ext column showing it too puts the
            // same three characters on screen twice — once editable and once
            // not, which reads as two different things to change.
            Column::Ext if row.renaming => "",
            Column::Ext => &row.ext,
            Column::Size => &row.size,
            Column::Modified => &row.modified,
            Column::Attributes => &row.attributes,
        }
    }
}

mod imp {
    use std::cell::{Cell, RefCell};
    use std::sync::OnceLock;

    use super::*;

    /// GObject wrapper so rendered rows can live in a `gio::ListStore`.
    #[derive(Default)]
    pub struct PaneEntry {
        pub row: RefCell<Option<Row>>,
        /// Bumped whenever the row is rewritten in place, which is what tells
        /// the cell showing it to paint itself again — see
        /// [`PaneEntry::rewrite`](super::PaneEntry::rewrite) for why a row is
        /// rewritten rather than replaced.
        pub revision: Cell<u32>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PaneEntry {
        const NAME: &'static str = "FcPaneEntry";
        type Type = super::PaneEntry;
    }

    impl ObjectImpl for PaneEntry {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: OnceLock<Vec<glib::ParamSpec>> = OnceLock::new();
            PROPERTIES.get_or_init(|| vec![glib::ParamSpecUInt::builder(ROW_REVISION).build()])
        }

        fn property(&self, _id: usize, _spec: &glib::ParamSpec) -> glib::Value {
            self.revision.get().to_value()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, _spec: &glib::ParamSpec) {
            self.revision
                .set(value.get().expect("the revision is the only property"));
        }
    }
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

    /// Puts a new `row` into this entry and tells its cell to repaint.
    ///
    /// **In place, never a replacement**, and that is not a micro-optimisation
    /// but the whole reason the revision exists. `GtkListBase` anchors the
    /// scroll position on an *item*; splice a new object over the anchored one
    /// and the anchor is gone, so the next `gtk_widget_allocate` reconfigures
    /// the adjustment from scratch and the list is at the top. That is a pane
    /// jumping to `..` because somebody pressed `Space`, and no amount of
    /// putting the offset back beats it — the reset happens inside GTK's own
    /// allocation, after every idle a caller could hook.
    fn rewrite(&self, row: Row) {
        self.imp().row.replace(Some(row));
        let next = self.imp().revision.get().wrapping_add(1);
        // Through `set_property` rather than the cell directly, because that
        // is what emits `notify` — and the cell may not exist: a `ColumnView`
        // keeps widgets only for the rows on screen.
        self.set_property(ROW_REVISION, next);
    }

    fn full_name(&self) -> String {
        self.imp()
            .row
            .borrow()
            .as_ref()
            .map(|row| row.full_name.clone())
            .unwrap_or_default()
    }

    /// The content type this row's icon comes from.
    fn content_type(&self) -> String {
        self.imp()
            .row
            .borrow()
            .as_ref()
            .map(Row::content_type)
            .unwrap_or_default()
    }

    /// Whether this entry already says exactly that.
    ///
    /// The whole row, not just the mark: [`sync_rows`](PaneView::sync_rows)
    /// uses it to leave untouched every position a re-read did not change.
    fn says(&self, row: &Row) -> bool {
        self.imp().row.borrow().as_ref() == Some(row)
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

/// What the right mouse button did to a row.
///
/// Total Commander's button, not Explorer's: a click marks the row and
/// puts the cursor on it, a held press asks for the context menu there.
/// The pane reports which; what either means is the shell's, because the
/// menu needs the shell and the mark needs nothing but the pane — and one
/// hook rather than two keeps "which pane was pressed" in one place.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowGesture {
    /// The button went down and up on the row.
    Click(usize),
    /// The button was held on the row long enough to mean "menu".
    Hold(usize),
}

/// Where the pane sends a right-button gesture. A slot, like [`RenameHook`]
/// and for the same reason.
type RowHook = Rc<RefCell<Option<Box<dyn Fn(RowGesture)>>>>;

/// A pane and the directory model behind it.
pub struct PaneView {
    root: gtk::Box,
    path_bar: gtk::Label,
    filter_bar: gtk::Entry,
    status: gtk::Label,
    space: gtk::Label,
    /// The view columns, so their widths can be read and written.
    columns: Vec<(Column, gtk::ColumnViewColumn)>,
    store: gio::ListStore,
    selection: gtk::SingleSelection,
    column_view: gtk::ColumnView,
    /// The pane's two faces: its listing, and the quick-view preview
    /// ([`docs/viewer.md`]). A stack rather than a swap, so the listing keeps
    /// its selection, scroll position and watch while it is hidden.
    pages: gtk::Stack,
    preview: gtk::Label,
    /// The folder this pane's *preview* is counting, and the token that stops
    /// it. Separate from `measuring`, which belongs to this pane's own
    /// `Alt+Shift+Enter`: one is about what this pane was asked to count, the
    /// other about what the other pane's cursor is passing over.
    ///
    /// The path is carried so a redraw can tell "the cursor moved to another
    /// folder" from "the cursor is still on this one".
    previewing: Option<(VfsPath, CancelToken)>,
    /// Everything that belongs to *what this pane is showing* rather than to
    /// the pane itself — and therefore everything `Ctrl+U` carries across.
    shown: Contents,
    /// Watches the directory this pane is showing, so a change made by
    /// anything else reaches it without being asked.
    ///
    /// Replaced on every navigation, and `None` when the directory cannot be
    /// watched at all — a pane that does not refresh itself, not one that
    /// fails to open.
    watch: Option<fc_core::watch::Watch>,
    /// The directory a read is in flight for, so an answer that arrives after
    /// the pane has moved on can be recognised and dropped.
    wanted: Option<VfsPath>,
    /// Which entry to put the cursor on when that read arrives — the directory
    /// just left, when stepping up.
    focus_on_arrival: Option<String>,
    /// What has been typed at the rows so far, and when the last key came.
    ///
    /// Per pane, because it is a position in *this* pane's list; dropped when
    /// the pane changes directory, since a search is about what is on screen.
    type_ahead: String,
    type_ahead_at: std::time::Instant,
    /// Where each directory was scrolled to when this pane left it.
    ///
    /// A session's memory, not a saved one: it needs no cap and no settings
    /// file that grows with every directory ever visited, and coming back to
    /// where you were matters within a session in a way it does not across a
    /// restart.
    scrolled_to: std::collections::HashMap<String, f64>,
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
    row_hook: RowHook,
    /// Kept so the two page keys that mark can measure a page; the model has
    /// no idea how tall the viewport is.
    scroller: gtk::ScrolledWindow,
    /// Stops the branch walk in flight, when there is one.
    ///
    /// Held here rather than passed around because Escape has to reach it
    /// from the keymap, which knows only the pane.
    walking: Option<CancelToken>,
    /// Stops the folder-size scan in flight, when there is one.
    measuring: Option<CancelToken>,
    /// Folder names `Space` has asked to count, in the order it asked.
    ///
    /// `Space` counts the folder it marks; `Insert` and `Shift+↓` only mark
    /// ([`docs/keymap.md`]). So the set a scan should cover is **not**
    /// "everything marked" — it is what this one key asked for, and nothing
    /// else in the model records that.
    ///
    /// Kept because a second `Space` restarts the scan: the folders already
    /// counted are filtered out by [`Listing::is_measured`], and the ones
    /// still owed are these. Cleared with the listing in [`Self::adopt`], a
    /// name meaning nothing in a directory it was not typed in.
    counting: Vec<String>,
    /// What the read in flight will do to that stack when it lands.
    ///
    /// Applied on arrival rather than when the key was pressed: opening an
    /// archive happens on a worker thread and can fail, and a pane that had
    /// already changed backends would be pointing at something it cannot show.
    transition: Transition,
    /// Why the last navigation attempt failed, shown beside the path.
    error: Option<String>,
}

/// Records that a folder has answered, so `Space` is owed no further count.
///
/// The list of folders still owed is kept by **removing** from it rather than
/// by asking the listing which rows carry a size. Asking cost a scan of the
/// view per name: 100 marked folders in a directory of 50 000 took **18.7 ms**
/// on the main loop, and 1000 took 179 ms — measured, after a first benchmark
/// put the names at the front of the listing and reported a hundredth of that
/// ([`docs/performance.md`]). This is O(names) and touches no rows.
///
/// A name whose row is not showing is forgotten just the same: a filter can
/// hide a row while its scan runs, and the count still happened.
fn forget_counted(counting: &mut Vec<String>, name: &str) {
    counting.retain(|owed| owed != name);
}

/// How far a page key moves when `visible` rows fit on screen.
///
/// Split out of [`PaneView::page_step`] because the rule is the part that can
/// be checked without a display server — what a viewport measures is GTK's
/// business, what is done with the number is ours. At least one row, or a
/// viewport too short for two would page nowhere.
fn page_step_for(visible: usize) -> usize {
    visible.saturating_sub(PAGE_OVERLAP_ROWS).max(1)
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
        let row_hook: RowHook = Rc::new(RefCell::new(None));
        let mut columns = Vec::new();
        for column in Column::ALL {
            // Only the name column is editable, so only it is handed the hook.
            let hook = (column == Column::Name).then(|| rename_hook.clone());
            let built = build_column(column, hook, row_hook.clone());
            columns.push((column, built.clone()));
            column_view.append_column(&built);
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

        let status = gtk::Label::builder().xalign(0.0).hexpand(true).build();
        status.add_css_class(CLASS_STATUS_LINE);
        // The disk figure sits at the other end of the same line: the two grow
        // from opposite ends, so a long selection summary and a long size
        // cannot push each other off.
        let space = gtk::Label::builder().xalign(1.0).build();
        space.add_css_class(CLASS_STATUS_LINE);
        let status_line = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        status_line.append(&status);
        status_line.append(&space);

        // Hidden until Ctrl+S asks for it, so a pane that is not being
        // filtered looks exactly as it did.
        let filter_bar = gtk::Entry::builder()
            .placeholder_text(FILTER_PLACEHOLDER)
            .visible(false)
            .build();
        filter_bar.add_css_class(CLASS_FILTER_BAR);

        let preview = gtk::Label::builder()
            .xalign(XALIGN_LEFT)
            .yalign(XALIGN_LEFT)
            .selectable(true)
            .build();
        preview.add_css_class(CLASS_OUTPUT);
        let preview_scroller = gtk::ScrolledWindow::builder()
            .child(&preview)
            .vexpand(true)
            .hexpand(true)
            .build();

        let pages = gtk::Stack::new();
        pages.add_named(&scroller, Some(PANE_PAGE_LIST));
        pages.add_named(&preview_scroller, Some(PANE_PAGE_PREVIEW));

        let root = gtk::Box::new(gtk::Orientation::Vertical, PANE_SPACING);
        root.add_css_class(CLASS_PANE);
        root.append(&path_bar);
        root.append(&filter_bar);
        root.append(&pages);
        root.append(&status_line);

        let mut pane = PaneView {
            shown: Contents {
                sort: listing.sort(),
                show_hidden: listing.show_hidden(),
                listing,
                entered: Vec::new(),
                remembered_marks: Vec::new(),
                fs,
            },
            root,
            path_bar,
            filter_bar,
            status,
            space,
            columns,
            store,
            selection,
            column_view,
            pages,
            preview,
            previewing: None,
            scroller,
            wanted: None,
            focus_on_arrival: None,
            type_ahead: String::new(),
            type_ahead_at: std::time::Instant::now(),
            scrolled_to: std::collections::HashMap::new(),
            walking: None,
            measuring: None,
            counting: Vec::new(),
            transition: Transition::Stay,
            watch: None,
            watched: None,
            renaming: None,
            rename_hook,
            row_hook,
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

    /// The rows themselves, for something that has to sit on them — the
    /// context menu, which points at a row and is a child of the widget
    /// that holds it.
    pub fn rows(&self) -> &gtk::ColumnView {
        &self.column_view
    }

    /// The model behind the pane, for the pure functions that decide what a
    /// keystroke acts on.
    pub fn listing(&self) -> &Listing {
        &self.shown.listing
    }

    /// A handle on this pane's backend, for a job that reads or writes here.
    pub fn fs(&self) -> Arc<dyn VirtualFs> {
        Arc::clone(&self.shown.fs)
    }

    /// Re-reads the directory after a job may have changed it.
    ///
    /// Uses [`Listing::load_nearest`] rather than a plain reload, because the
    /// job may have deleted or moved the very directory this pane is standing
    /// in. Showing an error where a listing belongs would strand the user
    /// somewhere they cannot navigate out of; landing on the nearest
    /// surviving ancestor keeps the pane usable.
    pub fn reload_after_job(&mut self) {
        // A job that said where the cursor should end up wins over where the
        // cursor is now. After a rename the name it is on does not exist any
        // more, so keeping it means focusing nothing and clamping to the top
        // row — leaving the file somebody just named off screen under a name
        // they then have to go and find.
        let focused = self
            .focus_on_arrival
            .take()
            .or_else(|| self.shown.listing.current().map(|entry| entry.name.clone()));
        // Where the view is, because a job that changed a file somewhere is
        // not a reason to move what somebody is looking at. This ran without
        // any restore at all until 2026-09-01, which is why finishing a copy
        // yanked *both* panes onto their cursors — the one in the other pane
        // included, which had nothing to do with the job.
        let offset = self.scroller.vadjustment().value();
        let mut listing = match self.is_branch() {
            true => self.walk_again(),
            false => {
                Listing::load_nearest(self.shown.fs.as_ref(), self.shown.listing.dir().clone())
            }
        };
        self.adopt(&mut listing);
        if let Some(name) = focused {
            listing.focus_entry(&name);
        }
        self.shown.listing = listing;
        self.error = None;
        self.refresh_keeping_view(offset);
    }

    /// Walks the tree again, keeping this pane a branch view.
    ///
    /// **On this thread**, unlike the walk `Ctrl+B` starts. That is not an
    /// oversight: the re-read it replaces — `Listing::load_nearest` — is
    /// synchronous too, and a branch view of a tree costs about what a plain
    /// listing of the same number of files costs
    /// (`docs/performance.md`). So this is the same exposure the plain path
    /// already has, on a list of the same size, and moving it to a worker
    /// means giving `reload_all` somewhere to await — worth doing when the
    /// synchronous re-read moves, not before.
    ///
    /// Uncancellable for the same reason: there is no keystroke in flight to
    /// cancel it with.
    fn walk_again(&mut self) -> Listing {
        let root = self.shown.listing.dir().clone();
        branch::listing(self.shown.fs.as_ref(), root, &CancelToken::new())
    }

    /// Starts watching whatever directory this pane is now showing.
    ///
    /// Called after every navigation, because the old watch is about a
    /// directory nobody is looking at any more. Returns where the nudges will
    /// arrive, for the shell to await on the main loop.
    pub fn rewatch(&mut self) -> Option<fc_core::watch::Changes> {
        // Set even when the watch could not be started, so a directory that
        // cannot be watched is not retried on every keystroke.
        self.watched = Some(self.shown.listing.dir().clone());
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
        self.watch = fc_core::watch::Watch::start(self.shown.listing.dir());
        self.watch.as_ref().map(|watch| watch.changes())
    }

    /// Whether the watch is about somewhere this pane has since left.
    pub fn watch_is_stale(&self) -> bool {
        self.watched.as_ref() != Some(self.shown.listing.dir())
    }

    /// The directory this pane is showing, for a watcher to check it is still
    /// the one it was started for.
    pub fn directory(&self) -> VfsPath {
        self.shown.listing.dir().clone()
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
        // `Listing::reload` re-reads one directory, which for a branch view
        // would quietly replace the whole tree with the root's own files.
        if self.is_branch() {
            // `reload_after_job` puts the offset back itself, and the one it
            // reads is this one — nothing has moved the view in between.
            self.reload_after_job();
            return;
        }
        if self.shown.listing.reload(self.shown.fs.as_ref()).is_err() {
            self.reload_after_job();
            return;
        }
        self.error = None;
        self.refresh_keeping_view(scroll);
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
    fn refresh_marks(&mut self, moved: bool) {
        // Which rows actually say something different now. Everything else is
        // left exactly as it is, object identity included.
        let changed: Vec<usize> = (0..self.shown.listing.len())
            .filter(|&index| {
                self.store
                    .item(index as u32)
                    .and_downcast::<PaneEntry>()
                    .is_some_and(|entry| {
                        let renaming = self.renaming.as_deref() == Some(entry.full_name().as_str());
                        entry.differs(self.shown.listing.is_selected(index), renaming)
                    })
            })
            .collect();

        // Rewritten where they stand, not spliced over. The model does not
        // change at all, so the list keeps its scroll anchor and its
        // selection — and each row is exactly the ones that differ rather
        // than the whole span between the first and the last.
        for index in changed {
            self.rewrite_row(index);
        }

        self.status
            .set_text(&crate::jobs::selection_status(&self.shown.listing));
        // Only when the cursor actually moved. `Insert` and `Shift+↓` mark
        // *and advance*, and a held-down key has to keep its cursor on
        // screen; `Space` marks where it stands, and a view somebody scrolled
        // away from the cursor was scrolled there on purpose.
        if moved {
            self.sync_cursor();
        }
    }

    /// Brings the store in line with the listing, keeping every row object
    /// it can.
    ///
    /// **Emptying and refilling was what made a pane jump.** `GtkListBase`
    /// anchors its scroll position on an *item*, and `remove_all` takes every
    /// item away — so the next allocation reconfigures the adjustment from an
    /// anchor that no longer exists, and nothing set before that allocation
    /// survives it. A re-read after a job, or a watcher nudge, therefore
    /// moved the view however carefully the offset was put back.
    ///
    /// So the objects stay and their contents are rewritten. Positions that
    /// say the same thing are not touched at all; the tail is spliced only
    /// when the listing actually got longer or shorter. It is also cheaper by
    /// the same stroke — a re-read of fifty thousand rows used to allocate
    /// fifty thousand `PaneEntry` values ([`docs/performance.md`]).
    ///
    /// The *marking* path does not come through here, and
    /// [`refresh_marks`](Self::refresh_marks) says why: it is the same
    /// question asked more cheaply, for the one case where the answer is
    /// known to be two flags.
    fn sync_rows(&self) {
        let wanted = self.shown.listing.len();
        let held = self.store.n_items() as usize;
        for index in 0..wanted.min(held) {
            let row = self.row_at(index);
            let entry = self
                .store
                .item(index as u32)
                .and_downcast::<PaneEntry>()
                .expect("the store holds PaneEntry values");
            if !entry.says(&row) {
                entry.rewrite(row);
            }
        }
        match wanted.cmp(&held) {
            std::cmp::Ordering::Greater => {
                let added: Vec<PaneEntry> =
                    (held..wanted).map(|index| self.entry_at(index)).collect();
                self.store.splice(held as u32, 0, &added);
            }
            std::cmp::Ordering::Less => {
                let none: [PaneEntry; 0] = [];
                self.store
                    .splice(wanted as u32, (held - wanted) as u32, &none);
            }
            std::cmp::Ordering::Equal => {}
        }
    }

    /// Rewrites the store's row at `index` from the listing.
    fn rewrite_row(&self, index: usize) {
        let Some(entry) = self.store.item(index as u32).and_downcast::<PaneEntry>() else {
            return;
        };
        entry.rewrite(self.row_at(index));
    }

    /// One row of the store, built from the listing.
    fn row_at(&self, index: usize) -> Row {
        let entry = self
            .shown
            .listing
            .get(index)
            .expect("indices below len() always resolve");
        let mut row = Row::from_entry(
            entry,
            self.shown.listing.is_parent(index),
            self.shown.listing.is_selected(index),
            self.shown.listing.measured_at(index),
        );
        row.renaming = self.renaming.as_deref() == Some(row.full_name.as_str());
        row
    }

    fn entry_at(&self, index: usize) -> PaneEntry {
        PaneEntry::new(self.row_at(index))
    }

    /// Rebuilds the rows from the listing and puts the selection back on the
    /// cursor.
    ///
    /// For when the *set* of rows changed — a navigation, a sort, a filter, a
    /// re-read. Anything that only changes what a row says goes through
    /// [`refresh_marks`](Self::refresh_marks) instead.
    pub fn refresh(&mut self) {
        self.rebuild(true);
    }

    /// The same, but leaving the viewport to the caller.
    ///
    /// **The cursor is not scrolled to.** Emptying the store costs the list
    /// its scroll anchor, and the `scroll_to` that `sync_cursor` issues then
    /// puts the cursor row at the *top* rather than scrolling minimally to
    /// it — and it wins over an offset put back on an idle, because GTK
    /// applies it during the frame's own layout. That is the pane jumping
    /// after a re-read or a job, with the cursor row landing at the top of
    /// the list, which is what the second testing round reported twice.
    ///
    /// So a caller that means to restore an offset asks for no scroll at all
    /// and then restores it. The selection still moves to the cursor; only
    /// the viewport is left alone.
    fn refresh_keeping_view(&mut self, offset: f64) {
        self.rebuild(false);
        self.restore_offset(offset);
    }

    fn rebuild(&mut self, scroll_to_cursor: bool) {
        let shown = self.shown_dir();
        // A branch view is not the directory it was walked from, and a path
        // bar that said only the root would be the pane lying about what is
        // in it.
        let path = match self.is_branch() {
            true => format!("{}{BRANCH_MARKER}", shown.as_str()),
            false => shown.as_str().to_string(),
        };
        self.path_bar.set_text(&match &self.error {
            Some(reason) => format!("{path}{PATH_BAR_ERROR_SEPARATOR}{reason}"),
            None => path,
        });
        self.show_space();

        // Emptying and refilling the store makes the widget move its own
        // selection, and `wire_selection` hears every such move — but this one
        // is ours and arrives while the shell is borrowed, so the handler
        // stands down and the model's cursor is never touched. `sync_cursor`
        // below then puts the widget back on it.
        //
        // This used to save and restore the cursor across the rebuild. That
        // was a no-op: nothing between the two lines could change it, because
        // reaching `refresh` at all means holding the borrow that keeps the
        // handler out (skill 19).
        self.sync_rows();

        self.status
            .set_text(&crate::jobs::selection_status(&self.shown.listing));
        if scroll_to_cursor {
            self.sync_cursor();
        } else if !self.shown.listing.is_empty() {
            // The selection follows the cursor; the viewport does not,
            // because the caller is about to put it back where it was.
            self.selection
                .set_selected(self.shown.listing.cursor() as u32);
        }
    }

    /// The selection the widget keeps, so the shell can hear it move.
    ///
    /// Handed out for the same reason the filter bar is: the pane knows what
    /// happened, and only the shell can reach the pane to act on it.
    pub fn selection(&self) -> &gtk::SingleSelection {
        &self.selection
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
        self.shown.listing.set_filter(&text);
        self.refresh();
    }

    /// Stops filtering, hides the field and hands the keyboard back to the
    /// rows.
    pub fn reset_filter(&mut self) {
        self.filter_bar.set_text("");
        self.filter_bar.set_visible(false);
        self.shown.listing.set_filter("");
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
        self.shown.sort = self.shown.sort.cycled(key);
        self.shown.listing.set_sort(self.shown.sort);
        self.update_headers();
        self.refresh();
    }

    /// Shows or hides the dot-files.
    pub fn toggle_hidden(&mut self) {
        self.shown.show_hidden = !self.shown.show_hidden;
        self.shown.listing.toggle_hidden();
        self.refresh();
    }

    /// Opens the pane with a remembered ordering and hidden-file flag.
    pub fn restore(&mut self, sort: Sort, show_hidden: bool) {
        self.shown.sort = sort;
        self.shown.show_hidden = show_hidden;
        let mut listing = std::mem::replace(
            &mut self.shown.listing,
            Listing::new(VfsPath::root(), Vec::new()),
        );
        self.adopt(&mut listing);
        self.shown.listing = listing;
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
        (self.shown_dir(), self.shown.sort, self.shown.show_hidden)
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
    /// Everything a *person* reads uses this: the path bar, the command
    /// line's prompt, and the settings file.
    pub fn shown_dir(&self) -> VfsPath {
        if self.shown.entered.is_empty() {
            return self.target_dir();
        }
        let mut path = VfsPath::root();
        for step in self
            .shown
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
    fn adopt(&mut self, listing: &mut Listing) {
        // A different directory: the names `Space` asked to count are not in
        // it, and `measured` is being replaced along with them.
        self.counting.clear();
        listing.set_sort(self.shown.sort);
        if listing.show_hidden() != self.shown.show_hidden {
            listing.toggle_hidden();
        }
        listing.set_filter(&self.filter_bar.text());
    }

    /// Marks the column the listing is ordered by, and which way.
    ///
    /// The header text is the marker: this shell sorts in the model, so there
    /// is no GTK sorter whose arrow GTK would draw for us.
    fn update_headers(&self) {
        let sort = self.shown.listing.sort();
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
    /// `Space`: mark the row under the cursor, and count it if it is a folder.
    ///
    /// Total Commander's behaviour, and it is what makes the status line's
    /// marked-bytes total true — a directory's size is zero until something
    /// counts it, so marking folders otherwise reports `0 B`.
    ///
    /// Returns the folders a scan should now cover, or `None` when there is
    /// nothing new to count: a file, the `..` row, a folder already counted,
    /// or a press that took a mark *off* — none of which makes a total truer.
    pub fn toggle_mark_counting(&mut self) -> Option<Vec<String>> {
        let cursor = self.shown.listing.cursor();
        self.toggle_mark(0);

        let listing = &self.shown.listing;
        let counts = listing.is_selected(cursor)
            && !listing.is_parent(cursor)
            && !listing.is_measured(cursor)
            && listing.get(cursor).is_some_and(|entry| entry.is_dir());
        if !counts {
            return None;
        }
        let name = listing.get(cursor)?.name.clone();
        if !self.counting.contains(&name) {
            self.counting.push(name);
        }
        let owed = self.owed();
        Some(owed)
    }

    /// The folders `Space` asked for that no answer has arrived for yet.
    ///
    /// A restart covers these rather than everything marked, so a folder
    /// counted before the last press is not walked twice and a folder somebody
    /// marked with `Insert` is not walked at all.
    fn owed(&self) -> Vec<String> {
        self.counting.clone()
    }

    pub fn toggle_mark(&mut self, step: isize) {
        self.marking(|listing| {
            listing.toggle_selected(listing.cursor());
            listing.move_cursor_by(step);
        });
    }

    /// Marks every row between the cursor and `target`, then goes there.
    ///
    /// `Shift+Home`/`End`/`PgUp`/`PgDn`: a range, not a toggle, because a
    /// jump has no direction to run back over and "flip everything I passed"
    /// is not what a person asking for "to the end" means.
    pub fn extend_mark_to(&mut self, target: usize) {
        self.marking(|listing| {
            listing.select_range(listing.cursor(), target, true);
            listing.set_cursor(target);
        });
    }

    /// Exchanges everything this pane is showing with another's.
    ///
    /// The contents, not the widgets: both panes are children of a `Paned`,
    /// and reparenting them would be work for no reason. Swapping the whole
    /// `Listing` is what makes the directory, the cursor and the marks come
    /// along together — anything reconstructed field by field would quietly
    /// drop one of them.
    pub fn exchange_with(&mut self, other: &mut PaneView) {
        // The *other* pane, and it is not the redundant call it looks like:
        // dispatch adopts the active pane, and this is the one it does not
        // touch. A click gives a pane's widget the focus and a selection of
        // its own without making it active, so its model can be a row behind
        // when the exchange arrives.
        other.adopt_selection();
        // One swap, not six. Naming the fields one at a time is what let the
        // backend and the archive stack be forgotten when archives arrived,
        // and every field that joins `Contents` from now on travels by
        // construction — `Contents` itself says what is deliberately not in
        // it, and why.
        std::mem::swap(&mut self.shown, &mut other.shown);
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
        if self.shown.listing.is_parent(self.shown.listing.cursor()) {
            return;
        }
        // A branch view's rows are named by their path, and this edits a
        // name: opening on `nested/inner.txt` would offer the whole path for
        // editing and then either fail or move the file. Refused per row
        // rather than per view, because a row at the walk's own root — plain
        // `notes.txt` — is an ordinary name and renames perfectly well.
        //
        // Unlike the `..` guard above, this one is reachable, and the test
        // that proves it bites when it is removed.
        if self
            .current_name()
            .is_some_and(|name| name.contains(SEPARATOR))
        {
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
    /// Tells the pane where to send what the right mouse button did.
    pub fn on_row_gesture(&self, hook: impl Fn(RowGesture) + 'static) {
        *self.row_hook.borrow_mut() = Some(Box::new(hook));
    }

    /// The right button clicked on `row`: the cursor goes there and the
    /// row's mark is toggled — Total Commander's mouse.
    ///
    /// Through `marking`, like every key that marks, so the status line's
    /// count and the view's scroll follow the same rule they do for `Space`.
    pub fn right_click(&mut self, row: usize) {
        self.marking(|listing| {
            listing.set_cursor(row);
            listing.toggle_selected(row);
        });
    }

    /// Puts the cursor on `row` and nothing else — what a held right button
    /// does before the menu opens on it. The marks are not touched: the menu
    /// acts on them, and a hold that changed them would act on something
    /// other than what the user saw when they pressed.
    pub fn point_cursor_at(&mut self, row: usize) {
        self.shown.listing.set_cursor(row);
        self.sync_cursor();
    }

    pub fn on_rename(&self, hook: impl Fn(Renamed) + 'static) {
        *self.rename_hook.borrow_mut() = Some(Box::new(hook));
    }

    /// The name of the row under the cursor.
    pub fn current_name(&self) -> Option<String> {
        self.shown.listing.current().map(|entry| entry.name.clone())
    }

    /// The path of the row under the cursor, when it is a **file**.
    ///
    /// `None` on `..` and on a directory: what F3 and F4 do with one is
    /// nothing, which is what Total Commander does too.
    pub fn current_file(&self) -> Option<VfsPath> {
        let entry = self.shown.listing.current()?;
        if entry.is_dir() || self.shown.listing.is_parent(self.shown.listing.cursor()) {
            return None;
        }
        self.shown.listing.current_path()
    }

    /// Whether the last navigation failed and left the pane where it was.
    pub fn went_wrong(&self) -> bool {
        self.error.is_some()
    }

    /// Where the cursor is, and the last row it could be on.
    pub fn cursor(&self) -> usize {
        self.shown.listing.cursor()
    }

    pub fn last_row(&self) -> usize {
        self.shown.listing.len().saturating_sub(1)
    }

    /// How far a page key moves: a screenful, less one row of overlap.
    ///
    /// **The overlap is what makes it match**, and it was measured rather than
    /// chosen. Paging used to be the `ColumnView`'s own business, and its
    /// scroll settled at thirteen rows where a screenful held fourteen — so it
    /// leaves the last visible row on screen as the first of the next page,
    /// which is what stops a reader losing their place across a jump. Moving
    /// by the full screenful instead lands one row further every time, and the
    /// difference compounds.
    ///
    /// At least one row, or a viewport too short for two rows would page
    /// nowhere.
    pub fn page_step(&self) -> usize {
        page_step_for(self.page_rows())
    }

    /// How many rows fit on screen.
    ///
    /// Measured rather than assumed: the adjustment knows the content height
    /// and the viewport height, and the rows are uniform, so the row count
    /// falls out of the ratio. Before the first layout there is no height to
    /// divide by and the fallback stands in.
    fn page_rows(&self) -> usize {
        let adjustment = self.scroller.vadjustment();
        let (content, viewport) = (adjustment.upper(), adjustment.page_size());
        let rows = self.shown.listing.len() as f64;
        if content <= 0.0 || viewport <= 0.0 || rows <= 0.0 {
            return PAGE_ROWS_FALLBACK;
        }
        let row_height = content / rows;
        ((viewport / row_height) as usize).max(1)
    }

    pub fn mark_matching(&mut self, pattern: &str, selected: bool) {
        self.marking(|listing| listing.select_matching(pattern, selected));
    }

    /// Flips the visible files, leaving directories alone — Total Commander's
    /// `Num *`. `including_folders` is its `Shift+Num *`.
    pub fn invert_marks(&mut self, including_folders: bool) {
        self.marking(|listing| match including_folders {
            true => listing.invert_selection(),
            false => listing.invert_selection_files(),
        });
    }

    pub fn mark_all(&mut self) {
        self.marking(Listing::select_all);
    }

    pub fn unmark_all(&mut self) {
        self.marking(Listing::clear_selection);
    }

    /// `Alt+Num ±`: every visible file sharing the cursor row's extension.
    pub fn mark_same_extension(&mut self, selected: bool) {
        self.marking(|listing| listing.select_same_extension(selected));
    }

    /// Puts away what is marked, so `Num /` can bring it back.
    ///
    /// Called when a job is submitted rather than when it finishes: the job
    /// ends with a fresh listing, and by then the marks it consumed are gone.
    pub fn remember_marks(&mut self) {
        self.shown.remembered_marks = self.shown.listing.selected_names();
    }

    /// `Num /`: the selection from before the last operation.
    pub fn restore_marks(&mut self) {
        let remembered = std::mem::take(&mut self.shown.remembered_marks);
        self.marking(|listing| listing.set_selected_names(&remembered));
        self.shown.remembered_marks = remembered;
    }

    /// Changes what is marked, and repaints exactly the rows that now say
    /// something different.
    ///
    /// Every mark operation goes through here, so the repaint is not an
    /// obligation an author can forget. It has been forgotten once: marks
    /// stopped repainting because a mutated-in-place row was never re-bound,
    /// and a mark operation that skips [`refresh_marks`](Self::refresh_marks)
    /// does not fail — it silently changes nothing on screen.
    ///
    /// **No `adopt_selection` here.** The widget's selection is adopted once
    /// per dispatched action, before the action runs, and nothing reaches a
    /// mark operation any other way.
    fn marking(&mut self, change: impl FnOnce(&mut Listing)) {
        // Whether the cursor moved is the only thing that entitles a marking
        // key to move the view, and one comparison tells the keys apart — so
        // no caller has to know which kind it is.
        let before = self.shown.listing.cursor();
        change(&mut self.shown.listing);
        let moved = self.shown.listing.cursor() != before;
        self.refresh_marks(moved);
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
        if self.shown.listing.is_empty() {
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
        let row = self.shown.listing.cursor() as u32;
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
    ///
    /// **Nobody has to call this.** `wire_selection` connects it to the
    /// widget's own `selection-changed`, so a move the *user* made is in the
    /// model before anything reads the cursor, and a new route into a pane
    /// owes nothing.
    ///
    /// It used to be a discipline: "adopted once per dispatched action". That
    /// rule was kept perfectly and stopped being sufficient anyway, twice in
    /// one week — when paging belonged to the widget, and when the letter keys
    /// became type-ahead, which is not an action. Neither author knew there
    /// was a call to make, and nothing failed to tell them. A rule that fails
    /// silently on the paths nobody thought of is the argument for a signal
    /// over a convention.
    ///
    /// One caller remains, and it is outside the signal by nature: the pane
    /// exchange adopts the **other** pane before swapping, which is not a move
    /// the widget just made.
    pub fn adopt_selection(&mut self) {
        if let Some(cursor) = adopted_cursor(self.selection.selected()) {
            self.shown.listing.set_cursor(cursor);
        }
    }

    /// Moves the cursor by `delta` rows.
    pub fn move_cursor_by(&mut self, delta: isize) {
        self.shown.listing.move_cursor_by(delta);
        self.sync_cursor();
    }

    /// Moves the cursor one page, clamping at either end.
    pub fn move_by_page(&mut self, direction: isize) {
        let page = self.page_step() as isize;
        self.move_cursor_by(direction * page);
    }

    /// Marks across one page and lands there.
    pub fn extend_mark_by_page(&mut self, direction: isize) {
        let page = self.page_step() as isize;
        let target = (self.cursor() as isize + direction * page).max(0) as usize;
        self.extend_mark_to(target);
    }

    pub fn move_cursor_to_first(&mut self) {
        self.shown.listing.move_cursor_to_first();
        self.sync_cursor();
    }

    pub fn move_cursor_to_last(&mut self) {
        self.shown.listing.move_cursor_to_last();
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
        let outermost = self.shown.entered.first().map_or_else(
            || Arc::clone(&self.shown.fs),
            |entered| Arc::clone(&entered.fs),
        );
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
        let (fs, dir) = (Arc::clone(&other.shown.fs), other.target_dir());
        self.start(dir.clone(), None);
        self.transition = Transition::Adopt(other.shown.entered.clone());
        Listing::spawn_load(fs, dir)
    }

    /// Enters what is under the cursor: a directory, or an archive as if it
    /// were one. Does nothing on any other file, which is what F3 and F4 are
    /// for.
    #[must_use = "the caller has to await the listing, or the pane never moves"]
    pub fn activate(&mut self) -> Option<Loading> {
        match activation_step(&self.shown.listing)? {
            Step::Into(target) => Some(self.navigate_to(target)),
            Step::Enter(archive) => Some(self.enter_archive(archive)),
            Step::Out => self.go_parent(),
        }
    }

    /// Opens the archive at `archive` and shows its root.
    #[must_use = "the caller has to await the listing, or the pane never moves"]
    fn enter_archive(&mut self, archive: VfsPath) -> Loading {
        let outer = Arc::clone(&self.shown.fs);
        let mut stack = self.shown.entered.clone();
        stack.push(Entered {
            fs: Arc::clone(&outer),
            archive: archive.clone(),
        });
        self.start(VfsPath::root(), None);
        self.transition = Transition::Adopt(stack);
        fc_core::archive::spawn_enter(outer, archive)
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
        if let Some(target) = parent_target(&self.shown.listing) {
            return Some(self.navigate_to(target));
        }
        let entered = self.shown.entered.last()?;
        let (outer, archive) = (Arc::clone(&entered.fs), entered.archive.clone());
        let containing = archive.parent()?;
        let mut stack = self.shown.entered.clone();
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
        !self.shown.entered.is_empty()
    }

    /// Fills this pane with every file below where it is — `Ctrl+B`.
    ///
    /// The pane keeps showing what it has until the walk lands, exactly as it
    /// does for a slow directory read, and
    /// [`abandon_background`](Self::abandon_background) is why the key is not
    /// a trap on a huge tree.
    #[must_use = "the caller has to await the listing, or the pane never changes"]
    pub fn branch(&mut self) -> Loading {
        let root = self.target_dir();
        let cancel = CancelToken::new();
        self.walking = Some(cancel.clone());
        // The cursor row is worth keeping: a branch view of where you are
        // should start on the file you were looking at, which is now spelled
        // as a relative path of one component.
        let focus = self.current_name();
        self.start(root.clone(), focus);
        self.transition = Transition::Stay;
        branch::spawn(Arc::clone(&self.shown.fs), root, cancel)
    }

    /// Counts what the marked folders hold, or the one under the cursor.
    ///
    /// Hands back where the answers will arrive; the pane stays usable while
    /// they do, unlike a directory read, because nothing about the rows is in
    /// doubt — each folder simply gains a number it did not have.
    #[must_use = "the caller has to await the answers, or no size ever appears"]
    pub fn measure_folders(&mut self, folders: Vec<String>) -> Option<Sizes> {
        if folders.is_empty() {
            return None;
        }
        // A second press re-counts, so the one already running is stopped
        // first rather than left racing the new one for the same rows.
        self.abandon_background();
        let cancel = CancelToken::new();
        self.measuring = Some(cancel.clone());
        Some(sizes::spawn(
            Arc::clone(&self.shown.fs),
            self.shown.listing.dir().clone(),
            folders,
            cancel,
        ))
    }

    /// Records one folder's answer and repaints just that row.
    ///
    /// One row, not the span the marking commands repaint: the answer names
    /// exactly one folder, and re-deriving which rows differ would be work
    /// proportional to the listing for a change proportional to nothing.
    pub fn measured(&mut self, name: &str, bytes: u64, complete: bool) {
        forget_counted(&mut self.counting, name);
        let Some(index) = self.shown.listing.set_measured(name, bytes, complete) else {
            return;
        };
        self.rewrite_row(index);
        // The total the accumulation was for, which the status line renders
        // from the marks and now includes this folder in.
        self.status
            .set_text(&crate::jobs::selection_status(&self.shown.listing));
    }

    /// Shows this pane's own listing again, and stops any walk its preview
    /// had started.
    ///
    /// Called on the active pane after every keystroke without first asking
    /// whether it is a preview: the stack ignores a page it is already on,
    /// and a question whose answer changes nothing is a question worth not
    /// asking.
    pub fn show_listing(&mut self) {
        self.pages.set_visible_child_name(PANE_PAGE_LIST);
        self.stop_previewing();
    }

    /// Shows `text` in place of this pane's listing.
    ///
    /// The listing is not touched — it is the stack's other page, keeping its
    /// selection, its scroll position and its watch — so leaving quick view
    /// is a page change and nothing else.
    pub fn show_preview(&self, text: &str) {
        self.preview.set_text(text);
        self.pages.set_visible_child_name(PANE_PAGE_PREVIEW);
    }

    /// Starts counting `folder` for the preview, stopping whatever the last
    /// cursor row started.
    ///
    /// **Cancellation, not delay** — the rule the measurement settled
    /// (`docs/performance.md`): a walk costs 0.058 ms to start and runs on
    /// behind the keystroke, so what keeps a held-down arrow key from leaving
    /// a queue of walks churning the disk is that each move stops the last.
    ///
    /// `None` when this is the folder already being previewed, which is not
    /// the same nothing: the preview is redrawn after *every* keystroke, not
    /// only the ones that moved the cursor, so restarting unconditionally
    /// would throw away a finished count and go back to "counting…" every
    /// time somebody pressed a key.
    pub fn preview_folder(&mut self, dir: VfsPath, folder: String) -> Option<Sizes> {
        let target = dir.child(&folder);
        if self
            .previewing
            .as_ref()
            .is_some_and(|(counting, _)| *counting == target)
        {
            return None;
        }
        self.stop_previewing();
        let cancel = CancelToken::new();
        self.previewing = Some((target, cancel.clone()));
        Some(sizes::spawn(
            Arc::clone(&self.shown.fs),
            dir,
            vec![folder],
            cancel,
        ))
    }

    /// The folder this pane's preview is counting, if it is counting one.
    ///
    /// What a walk's answer is checked against before it is drawn. The pane
    /// is asked rather than the listing re-read: the pane *is* where "which
    /// folder is being counted" lives, and deriving it a second way is how
    /// two answers to one question start to disagree.
    pub fn previewing(&self) -> Option<&VfsPath> {
        self.previewing.as_ref().map(|(folder, _)| folder)
    }

    /// Stops the walk this pane's preview started, if one is running.
    ///
    /// Not part of `abandon_background`, which is `Escape`'s: that acts on
    /// the pane with the keyboard, and the preview is by definition the other
    /// one. A walk nobody asked for is also not what "stop what you are
    /// doing" means, and stopping it would only make the next keystroke start
    /// it again.
    fn stop_previewing(&mut self) {
        if let Some((_, cancel)) = self.previewing.take() {
            cancel.cancel();
        }
    }

    /// Stops whatever this pane has running in the background.
    ///
    /// One method rather than one per kind, because the caller — `Escape` —
    /// means "stop what you are doing", not "stop the walk specifically".
    /// Returns whether there was anything to stop, which is what lets Escape
    /// fall through to clearing a filter when there was not.
    pub fn abandon_background(&mut self) -> bool {
        let scanning = self.measuring.take().inspect(|cancel| cancel.cancel());
        // The sizes already on screen stay. Unlike a half-finished walk, each
        // folder's number is its own and complete — there is nothing
        // misleading about having counted three of five.
        self.abandon_walk() || scanning.is_some()
    }

    /// Stops a branch walk and leaves the pane exactly where it was.
    ///
    /// Both halves matter. Cancelling alone would still land a partial
    /// listing, which is worse than none — a tree half shown looks like a
    /// tree. Forgetting `wanted` is what makes [`arrived`](Self::arrived)
    /// drop the answer when it comes.
    fn abandon_walk(&mut self) -> bool {
        let Some(cancel) = self.walking.take() else {
            return false;
        };
        cancel.cancel();
        self.wanted = None;
        true
    }

    /// Whether this pane is showing a walk rather than one directory.
    pub fn is_branch(&self) -> bool {
        self.shown.listing.is_branch()
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
        let focus = focus_after_move(self.shown.listing.dir(), &dir);
        self.start(dir.clone(), focus);
        self.transition = Transition::Stay;
        Listing::spawn_load(Arc::clone(&self.shown.fs), dir)
    }

    /// Records that a move to `dir` is in flight, whatever backend answers it.
    fn start(&mut self, dir: VfsPath, focus: Option<String>) {
        // A search is a position in the list on screen, and the list is about
        // to be a different one.
        self.forget_type_ahead();
        // Where this directory was left, so coming back to it lands where you
        // were rather than at the cursor's row. Recorded on the way out
        // because that is the last moment the offset is still the one the
        // user was looking at.
        self.remember_scroll();
        // A filter belongs to the directory it was typed in. Carrying it into
        // the next one would show an empty pane and no reason why.
        self.filter_bar.set_text("");
        self.filter_bar.set_visible(false);
        self.focus_on_arrival = focus;
        self.wanted = Some(dir);
    }

    /// Applies the widths from the settings to this pane's columns.
    ///
    /// The name column is left alone: it expands into what is left over, so
    /// it has no width of its own to set.
    pub fn set_column_widths(&self, widths: &config::ColumnSettings) {
        for (column, view_column) in &self.columns {
            if let Some(width) = column.width_in(widths) {
                view_column.set_fixed_width(width);
            }
        }
    }

    /// What this pane's columns are currently wide.
    pub fn column_widths(&self) -> config::ColumnSettings {
        let mut widths = config::ColumnSettings::default();
        for (column, view_column) in &self.columns {
            column.set_width_in(&mut widths, view_column.fixed_width());
        }
        widths
    }

    /// Runs `changed` whenever a column is dragged to a new width.
    pub fn on_column_resized(&self, changed: impl Fn() + 'static) {
        let changed = Rc::new(changed);
        for (column, view_column) in &self.columns {
            if column.expands() {
                continue;
            }
            let changed = changed.clone();
            view_column.connect_fixed_width_notify(move |_| changed());
        }
    }

    /// Puts how much room is left on the disk at the end of the status line.
    ///
    /// Asked of the *backend*, so an archive — which has no free space of its
    /// own — leaves the figure empty rather than reporting the disk the
    /// archive file happens to sit on, which would be an answer to a question
    /// nobody asked. A filesystem that has gone leaves it empty too: an empty
    /// status line is honest where "0 B free" is a lie.
    fn show_space(&self) {
        let space = self.shown.fs.space(self.shown.listing.dir());
        self.space.set_text(&match space {
            Some(space) => DISK_SPACE
                .replace("{free}", &crate::format::human_bytes(space.free))
                .replace("{total}", &crate::format::human_bytes(space.total)),
            None => String::new(),
        });
    }

    /// Takes one typed character and moves the cursor to what it spells.
    ///
    /// Two rules, and they differ in one detail that matters more than it
    /// looks. A **continuation** — a character typed while the buffer is
    /// still warm — searches from the cursor *inclusive*, so typing more
    /// letters narrows onto the row you are already on rather than jumping
    /// off it. A **fresh** buffer searches from the row *after* the cursor,
    /// so pressing the same letter again walks to the next match instead of
    /// sitting still.
    ///
    /// A search that matches nothing leaves the cursor alone and keeps the
    /// buffer, so one mistyped letter does not throw away what came before
    /// it — the next character may well complete a name that exists.
    pub fn type_ahead(&mut self, typed: char) {
        let now = std::time::Instant::now();
        let expired = now.duration_since(self.type_ahead_at) > TYPE_AHEAD_TIMEOUT;
        if expired {
            self.type_ahead.clear();
        }
        self.type_ahead_at = now;

        // The same character again is "show me the next one", not a longer
        // needle: `nn` matches nothing in most directories, so a second `n`
        // would sit still exactly when somebody is pressing it to move on.
        // The cost is that a name is not reachable by typing its doubled
        // letter — `aa` walks the `a`s rather than finding `aardvark` — which
        // is the trade every list-search in every file manager makes.
        let repeated = !self.type_ahead.is_empty() && self.type_ahead.chars().all(|c| c == typed);
        let fresh = self.type_ahead.is_empty();
        match repeated {
            true => self.type_ahead = typed.to_string(),
            false => self.type_ahead.push(typed),
        }

        let cursor = self.shown.listing.cursor();
        // A fresh needle, or the same one again, looks *past* the cursor, so
        // the search moves. A needle being extended starts at the cursor, so
        // typing more letters narrows onto the row already found.
        let from = match fresh || repeated {
            true => (cursor + 1) % self.shown.listing.len().max(1),
            false => cursor,
        };
        if let Some(found) = self.shown.listing.find_from(from, &self.type_ahead) {
            self.shown.listing.set_cursor(found);
            self.sync_cursor();
        }
    }

    /// Forgets what was typed at the rows.
    ///
    /// Called when the pane changes directory: the buffer is a position in a
    /// list, and the list is gone.
    pub fn forget_type_ahead(&mut self) {
        self.type_ahead.clear();
    }

    /// Writes down where the directory on screen is scrolled to.
    fn remember_scroll(&mut self) {
        let offset = self.scroller.vadjustment().value();
        self.scrolled_to
            .insert(self.shown.listing.dir().as_str().to_string(), offset);
    }

    /// Puts the view back where this directory was left, if it has been here
    /// before.
    ///
    /// After the rows exist and after the cursor has been placed: GTK scrolls
    /// to keep the focused row visible, so restoring first would be undone by
    /// the thing it is meant to override. The offset is clamped by the
    /// adjustment itself, which is what makes a directory that shrank while
    /// you were below it land at its end rather than past it.
    ///
    /// **The priority is what stops it flickering.** Deferred it must be —
    /// before the rows are laid out there is no height to clamp against — but
    /// a plain idle runs after the paint as well, and the frame in between is
    /// the one showing the top of the list. Logging every painted frame's
    /// offset says it outright: coming back out of a directory used to paint
    /// a frame at 39 px, the cursor row, before landing at the remembered
    /// 828; at [`SCROLL_RESTORE_PRIORITY`] it goes straight to 828.
    fn restore_scroll(&self) {
        let Some(&offset) = self.scrolled_to.get(self.shown.listing.dir().as_str()) else {
            return;
        };
        self.restore_offset(offset);
    }

    /// Puts the view at `offset` once the rows are laid out and before they
    /// are painted.
    ///
    /// **The one place that knows when an offset may be put back**, and there
    /// are four callers now: arriving in a directory, re-reading one, and
    /// reloading after a job in either pane. Three of them used to do it
    /// themselves and two got it wrong in different ways — one set the value
    /// on the line after `refresh`, where the adjustment is still collapsed,
    /// and one never restored at all.
    fn restore_offset(&self, offset: f64) {
        let adjustment = self.scroller.vadjustment();
        glib::idle_add_local_full(glib::Priority::from(SCROLL_RESTORE_PRIORITY), move || {
            adjustment.set_value(offset);
            glib::ControlFlow::Break
        });
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
            .unwrap_or_else(|| self.shown.listing.dir().clone())
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
        self.walking = None;
        let transition = std::mem::replace(&mut self.transition, Transition::Stay);
        match arrival {
            Ok((fs, mut listing)) => {
                // The backend arrives with the listing, so a step that changed
                // it — into an archive, or back out of one — takes effect at
                // the moment there is something to show, and a step that
                // failed leaves the pane exactly where it was.
                self.shown.fs = fs;
                if let Transition::Adopt(entered) = transition {
                    self.shown.entered = entered;
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
                self.shown.listing = listing;
                self.error = None;
                self.restore_scroll();
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
fn bind_name_cell(
    cell: &gtk::Box,
    entry: &PaneEntry,
    icons: &RefCell<HashMap<String, gio::Icon>>,
) -> gtk::Label {
    let icon = cell
        .first_child()
        .and_downcast::<gtk::Image>()
        .expect("setup put the icon first");
    icon.set_from_gicon(&themed_icon(entry, icons));
    let stack = icon
        .next_sibling()
        .and_downcast::<gtk::Stack>()
        .expect("setup put the stack after the icon");
    let stack = &stack;
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
fn build_column(
    column: Column,
    rename: Option<RenameHook>,
    rows: RowHook,
) -> gtk::ColumnViewColumn {
    let factory = gtk::SignalListItemFactory::new();
    // One cache per column — which in practice means one, since only the name
    // column shows icons. Shared by every cell of it, because forty cells
    // asking the theme for `text/plain` forty times is thirty-nine answers
    // nobody needed ([`docs/performance.md`]).
    let bind_icons: Rc<RefCell<HashMap<String, gio::Icon>>> = Rc::default();

    let setup_rename = rename.clone();
    factory.connect_setup(move |_, item| {
        let item = list_item(item);
        // The right button, on every cell of every column: a row is what the
        // user sees, and which column the pointer happened to be over is not
        // something they meant. The row is read from the `ListItem` at press
        // time rather than captured here, because a recycled cell shows a
        // different row every time it scrolls back in.
        let click = gtk::GestureClick::builder().button(RIGHT_BUTTON).build();
        let (hook, pressed) = (rows.clone(), item.clone());
        click.connect_released(move |_, _, _, _| {
            if let Some(hook) = hook.borrow().as_ref() {
                hook(RowGesture::Click(pressed.position() as usize));
            }
        });
        let hold = gtk::GestureLongPress::builder()
            .button(RIGHT_BUTTON)
            .build();
        let (hook, pressed) = (rows.clone(), item.clone());
        hold.connect_pressed(move |_, _, _| {
            if let Some(hook) = hook.borrow().as_ref() {
                hook(RowGesture::Hold(pressed.position() as usize));
            }
        });

        let label = gtk::Label::builder()
            .xalign(column.xalign())
            .ellipsize(gtk::pango::EllipsizeMode::Middle)
            .build();
        let Some(hook) = setup_rename.clone() else {
            label.add_controller(click);
            label.add_controller(hold);
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

        // The icon sits *outside* the stack, so it stays put while a row is
        // being renamed — the editor replaces the name, not the row.
        let icon = gtk::Image::builder().pixel_size(ROW_ICON_SIZE).build();
        let cell = gtk::Box::new(gtk::Orientation::Horizontal, ROW_ICON_GAP);
        cell.append(&icon);
        cell.append(&stack);
        cell.add_controller(click);
        cell.add_controller(hold);
        item.set_child(Some(&cell));
    });

    factory.connect_bind(move |_, item| {
        let item = list_item(item);
        let entry = item
            .item()
            .and_downcast::<PaneEntry>()
            .expect("the store holds PaneEntry values");
        let child = item.child().expect("setup installed a child");

        // One closure, run now and run again whenever the row is rewritten
        // under this cell. The two must do the same thing: a rewrite is how a
        // mark repaints, how a counted size appears, *and* how the rename
        // editor opens — and the editor is in the name column's stack, so a
        // repaint that only touched the label would leave a rename with
        // nowhere to type.
        //
        // A weak reference to the cell, so the entry's handler does not keep
        // a recycled one alive; the handler itself is disconnected on unbind
        // below, because that cell will be showing another row by then.
        let paint = {
            let child = child.downgrade();
            let icons = Rc::clone(&bind_icons);
            move |entry: &PaneEntry| {
                let Some(child) = child.upgrade() else {
                    return;
                };
                let label = match child.downcast_ref::<gtk::Box>() {
                    Some(cell) => bind_name_cell(cell, entry, &icons),
                    None => child
                        .downcast_ref::<gtk::Label>()
                        .expect("setup installed a Label")
                        .clone(),
                };
                paint_cell(&label, entry, column);
            }
        };
        paint(&entry);
        let id = entry.connect_notify_local(Some(ROW_REVISION), move |entry, _| paint(entry));
        // SAFETY: the key is this module's own, the value is put on the very
        // `ListItem` that `connect_unbind` takes it off again, and the type
        // there matches the type here.
        unsafe { item.set_data(CELL_REPAINT, id) };
    });

    factory.connect_unbind(|_, item| {
        let item = list_item(item);
        // SAFETY: as above — the same key, the same item, the same type.
        let Some(id) = (unsafe { item.steal_data::<glib::SignalHandlerId>(CELL_REPAINT) }) else {
            return;
        };
        if let Some(entry) = item.item().and_downcast::<PaneEntry>() {
            entry.disconnect(id);
        }
    });

    let view_column = gtk::ColumnViewColumn::new(Some(column.title()), Some(factory));
    view_column.set_fixed_width(column.width());
    view_column.set_expand(column.expands());
    // Draggable, except the name column, which takes the leftover width: a
    // column that both expands and has a dragged width is two answers to how
    // wide it is, and GTK picks the one nobody asked for.
    view_column.set_resizable(!column.expands());
    view_column
}

/// The icon for a row's type, from the cache or from the theme.
///
/// GIO's content type carries the theme's whole fallback chain, so a desktop
/// without an icon for `text/plain` still finds `text-x-generic` and one
/// without either still finds the generic file — which is why the row asks
/// for a *type* rather than an icon name.
fn themed_icon(entry: &PaneEntry, icons: &RefCell<HashMap<String, gio::Icon>>) -> gio::Icon {
    let content_type = entry.content_type();
    if let Some(icon) = icons.borrow().get(&content_type) {
        return icon.clone();
    }
    let icon = gio::content_type_get_icon(&content_type);
    icons.borrow_mut().insert(content_type, icon.clone());
    icon
}

/// Writes one cell from its row: the column's text, and the colour a marked
/// row carries.
///
/// Marked rows are coloured, which is how Total Commander shows them and the
/// only cue that survives the row also being the cursor.
fn paint_cell(label: &gtk::Label, entry: &PaneEntry, column: Column) {
    label.set_text(&entry.text(column));
    if entry.is_marked() {
        label.add_css_class(CLASS_MARKED);
    } else {
        label.remove_css_class(CLASS_MARKED);
    }
}

/// From GTK 4.12 the factory hands over a plain `Object`, because a factory
/// can also produce header and cell items. A column's factory only ever makes
/// list items.
fn list_item(item: &glib::Object) -> &gtk::ListItem {
    item.downcast_ref::<gtk::ListItem>()
        .expect("a column view factory always yields ListItems")
}

/// What a pane is showing, as opposed to the pane itself.
///
/// The grouping exists because `Ctrl+U` swaps exactly this and nothing else.
/// It used to name six fields one at a time, and that shape has already cost
/// a real bug: when archives arrived, `fs` and `entered` were added to the
/// pane and forgotten in the swap, so each pane went on reading the other's
/// entries through its own backend. Every field added here now travels by
/// construction, and a field added to `PaneView` deliberately does not.
///
/// **What is not here, and why.** The directory watch is rebuilt by the shell
/// after any move, so swapping it would be work undone a moment later. A read
/// in flight (`wanted`, `focus_on_arrival`, `transition`) is addressed to the
/// pane that asked for it, not to the contents. An open inline rename belongs
/// to the widget the user is typing into. And the quick filter's text lives in
/// a `gtk::Entry`, which stays with the pane — so `exchange_with` moves that
/// one by hand, and it is the only exception.
struct Contents {
    listing: Listing,
    /// How this pane orders and filters, which belongs to what is shown and
    /// not to the directory: every navigation and every finished job builds a
    /// fresh `Listing`, so without keeping these the order would reset each
    /// time — sorting by size and then copying a file put it back to name.
    sort: Sort,
    show_hidden: bool,
    /// Shared rather than owned: a running job holds the same backend on its
    /// worker thread while the pane goes on using it.
    fs: Arc<dyn VirtualFs>,
    /// The archives this pane has walked into, outermost first.
    ///
    /// What `..` at an archive's root needs: which backend to go back to, and
    /// which file to put the cursor on. A stack rather than one slot, because
    /// an archive inside an archive is then not a case anybody has to think
    /// about.
    entered: Vec<Entered>,
    /// What was marked when the last job was submitted, for `Num /`.
    ///
    /// Names rather than rows, because the listing this refers to no longer
    /// exists: every finished job builds a new one.
    remembered_marks: Vec<String>,
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

    /// An answer takes exactly its own folder off the owed list.
    ///
    /// This is the accumulation the feature turns on: a second `Space`
    /// restarts the scan over the folders that have not answered, so one that
    /// has must drop out and the rest must not. The end-to-end test cannot see
    /// it — the fixtures it can afford finish counting before the next
    /// keystroke arrives, so it passes either way, which a probe showed.
    #[test]
    fn an_answer_stops_its_folder_being_owed_and_leaves_the_others() {
        let mut counting = vec![
            String::from("big"),
            String::from("small"),
            String::from("middling"),
        ];

        forget_counted(&mut counting, "small");
        assert_eq!(
            counting,
            ["big", "middling"],
            "the wrong folder was forgotten"
        );

        // A folder nobody asked about changes nothing — an answer can arrive
        // for a row `Alt+Shift+Enter` asked for rather than `Space`.
        forget_counted(&mut counting, "elsewhere");
        assert_eq!(counting, ["big", "middling"]);

        forget_counted(&mut counting, "big");
        forget_counted(&mut counting, "middling");
        assert!(
            counting.is_empty(),
            "everything answered, something still owed"
        );
    }

    /// A page moves a screenful less the row that carries the reader over.
    ///
    /// The numbers are the ones measured off the running app on 2026-08-30,
    /// when plain paging still belonged to the `ColumnView`: a 39-pixel row
    /// in a 579-pixel viewport is fourteen rows visible, and the widget's own
    /// paging scrolled thirteen of them — offsets 0, 474, 981, 1488, which a
    /// thirteen-row step with the ordinary scroll-into-view reproduces
    /// exactly. Reproducing it was the requirement, so the figure it turns on
    /// is pinned here rather than left to a comment.
    ///
    /// **What this does not pin is the scroll itself.** Where a row ends up on
    /// screen is not something the end-to-end suite can see — it asserts on
    /// the filesystem — and a down-and-back-up test cannot see the overlap
    /// either, because both directions use the same step and cancel. This
    /// holds the arithmetic; the appearance is checked by eye and recorded in
    /// [`docs/keymap.md`].
    #[test]
    fn a_page_is_a_screenful_less_the_row_that_carries_over() {
        assert_eq!(page_step_for(14), 13, "the measured case");
        // A viewport that fits one row still has to move, or the key is dead.
        assert_eq!(page_step_for(1), 1);
        assert_eq!(page_step_for(0), 1);
    }

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
    fn the_ext_column_stands_down_while_the_row_is_renamed() {
        // Reported from the field: Shift+F6 puts the whole name in the entry,
        // extension and all, and the Ext column went on showing it too — the
        // same three characters twice, once editable and once not.
        let editing = row(false, true);
        assert_eq!(
            Column::Ext.value(&editing),
            "",
            "the extension is shown twice"
        );
        assert_eq!(
            Column::Name.value(&editing),
            "notes",
            "the name column is the entry's business, not blanked here"
        );

        // Every other column goes on saying what it said: the row is being
        // renamed, not hidden.
        assert_eq!(Column::Size.value(&editing), "0");
        assert_eq!(Column::Ext.value(&row(false, false)), "txt");
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
