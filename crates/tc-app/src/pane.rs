//! One pane: a path bar above a column view of a directory.

use std::sync::Arc;

use gtk::gio;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

use tc_core::listing::{Listing, Sort, SortKey, SortOrder};
use tc_core::vfs::{VfsPath, VirtualFs};

use crate::constants::{
    CLASS_FILTER_BAR, CLASS_MARKED, CLASS_PANE, CLASS_PANE_ACTIVE, CLASS_PATH_BAR,
    CLASS_STATUS_LINE, COLUMN_TITLE_ATTR, COLUMN_TITLE_DATE, COLUMN_TITLE_EXT, COLUMN_TITLE_NAME,
    COLUMN_TITLE_SIZE, COLUMN_WIDTH_ATTR, COLUMN_WIDTH_DATE, COLUMN_WIDTH_EXT, COLUMN_WIDTH_NAME,
    COLUMN_WIDTH_SIZE, FILTER_PLACEHOLDER, PAGE_ROWS_FALLBACK, PANE_SPACING,
    PATH_BAR_ERROR_SEPARATOR, SORT_MARKER_ASCENDING, SORT_MARKER_DESCENDING, XALIGN_LEFT,
    XALIGN_RIGHT,
};
use crate::navigation::{activation_target, adopted_cursor, focus_after_move, parent_target};
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
    /// Why the last navigation attempt failed, shown beside the path.
    error: Option<String>,
}

impl PaneView {
    /// Builds a pane showing `dir`.
    ///
    /// The pane owns its filesystem, because navigation re-reads through it
    /// and phase 6 swaps it for an archive backend while the pane lives on.
    ///
    /// A directory that cannot be read yields an empty pane rather than
    /// failing construction — a window with one broken pane is still a usable
    /// program, and the reason is shown in the path bar.
    pub fn new(fs: Arc<dyn VirtualFs>, dir: VfsPath) -> Self {
        let (listing, error) = match Listing::load(fs.as_ref(), dir.clone()) {
            Ok(listing) => (listing, None),
            Err(reason) => (Listing::new(dir, Vec::new()), Some(reason.to_string())),
        };

        let store = gio::ListStore::new::<PaneEntry>();
        let selection = gtk::SingleSelection::new(Some(store.clone()));
        let column_view = gtk::ColumnView::new(Some(selection.clone()));

        for column in Column::ALL {
            column_view.append_column(&build_column(column));
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

    /// Rebuilds the rows from the listing and puts the selection back on the
    /// cursor. Called after anything that changes the model.
    pub fn refresh(&mut self) {
        let path = self.listing.dir().as_str();
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
            let entry = self
                .listing
                .get(index)
                .expect("indices below len() always resolve");
            let row = Row::from_entry(
                entry,
                self.listing.is_parent(index),
                self.listing.is_selected(index),
            );
            self.store.append(&PaneEntry::new(row));
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
    pub fn state(&self) -> (VfsPath, Sort, bool) {
        (self.listing.dir().clone(), self.sort, self.show_hidden)
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
        self.refresh();
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
        self.refresh();
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
        self.refresh();
    }

    /// Flips the visible files, leaving directories alone — Total Commander's
    /// `Num *`. `including_folders` is its `Shift+Num *`.
    pub fn invert_marks(&mut self, including_folders: bool) {
        if including_folders {
            self.listing.invert_selection();
        } else {
            self.listing.invert_selection_files();
        }
        self.refresh();
    }

    pub fn mark_all(&mut self) {
        self.listing.select_all();
        self.refresh();
    }

    pub fn unmark_all(&mut self) {
        self.listing.clear_selection();
        self.refresh();
    }

    /// `Alt+Num ±`: every visible file sharing the cursor row's extension.
    pub fn mark_same_extension(&mut self, selected: bool) {
        self.adopt_selection();
        self.listing.select_same_extension(selected);
        self.refresh();
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
        self.refresh();
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
        let row = self.listing.cursor() as u32;
        self.column_view.scroll_to(
            row,
            None,
            gtk::ListScrollFlags::FOCUS | gtk::ListScrollFlags::SELECT,
            None,
        );
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
    pub fn go_to(&mut self, dir: VfsPath) {
        self.navigate_to(dir);
    }

    /// Enters the directory under the cursor. Does nothing on a file — F3/F4
    /// arrive in phase 4.
    pub fn activate(&mut self) {
        if let Some(target) = activation_target(&self.listing) {
            self.navigate_to(target);
        }
    }

    /// Leaves the current directory. Does nothing at the root.
    pub fn go_parent(&mut self) {
        if let Some(target) = parent_target(&self.listing) {
            self.navigate_to(target);
        }
    }

    /// Shows `dir`, or stays put and reports why it could not.
    ///
    /// A pane that cannot read a directory must not end up displaying it as
    /// empty: leaving the user where they were, with the reason next to the
    /// path, keeps the pane in a state they can navigate out of.
    fn navigate_to(&mut self, dir: VfsPath) {
        // A filter belongs to the directory it was typed in. Carrying it into
        // the next one would show an empty pane and no reason why.
        self.filter_bar.set_text("");
        self.filter_bar.set_visible(false);
        let focus = focus_after_move(self.listing.dir(), &dir);
        match Listing::load(self.fs.as_ref(), dir.clone()) {
            Ok(mut listing) => {
                self.adopt(&mut listing);
                if let Some(name) = focus {
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
fn build_column(column: Column) -> gtk::ColumnViewColumn {
    let factory = gtk::SignalListItemFactory::new();

    factory.connect_setup(move |_, item| {
        let item = list_item(item);
        let label = gtk::Label::builder()
            .xalign(column.xalign())
            .ellipsize(gtk::pango::EllipsizeMode::Middle)
            .build();
        item.set_child(Some(&label));
    });

    factory.connect_bind(move |_, item| {
        let item = list_item(item);
        let entry = item
            .item()
            .and_downcast::<PaneEntry>()
            .expect("the store holds PaneEntry values");
        let label = item
            .child()
            .and_downcast::<gtk::Label>()
            .expect("setup installed a Label");
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
