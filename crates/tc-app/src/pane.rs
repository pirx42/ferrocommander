//! One pane: a path bar above a column view of a directory.

use gtk::gio;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

use tc_core::listing::Listing;
use tc_core::vfs::{VfsPath, VirtualFs};

use crate::constants::{
    CLASS_PANE, CLASS_PANE_ACTIVE, CLASS_PATH_BAR, COLUMN_TITLE_DATE, COLUMN_TITLE_EXT,
    COLUMN_TITLE_NAME, COLUMN_TITLE_SIZE, COLUMN_WIDTH_DATE, COLUMN_WIDTH_EXT, COLUMN_WIDTH_NAME,
    COLUMN_WIDTH_SIZE, PANE_SPACING, PATH_BAR_ERROR_SEPARATOR, XALIGN_LEFT, XALIGN_RIGHT,
};
use crate::navigation::{activation_target, focus_after_move, parent_target};
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
}

impl Column {
    pub const ALL: [Column; 4] = [Column::Name, Column::Ext, Column::Size, Column::Modified];

    fn title(self) -> &'static str {
        match self {
            Column::Name => COLUMN_TITLE_NAME,
            Column::Ext => COLUMN_TITLE_EXT,
            Column::Size => COLUMN_TITLE_SIZE,
            Column::Modified => COLUMN_TITLE_DATE,
        }
    }

    fn width(self) -> i32 {
        match self {
            Column::Name => COLUMN_WIDTH_NAME,
            Column::Ext => COLUMN_WIDTH_EXT,
            Column::Size => COLUMN_WIDTH_SIZE,
            Column::Modified => COLUMN_WIDTH_DATE,
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
    store: gio::ListStore,
    selection: gtk::SingleSelection,
    column_view: gtk::ColumnView,
    listing: Listing,
    fs: Box<dyn VirtualFs>,
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
    pub fn new(fs: Box<dyn VirtualFs>, dir: VfsPath) -> Self {
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

        let root = gtk::Box::new(gtk::Orientation::Vertical, PANE_SPACING);
        root.add_css_class(CLASS_PANE);
        root.append(&path_bar);
        root.append(&scroller);

        let mut pane = PaneView {
            root,
            path_bar,
            store,
            selection,
            column_view,
            listing,
            fs,
            error,
        };
        pane.refresh();
        pane
    }

    /// The widget to place in the window.
    pub fn widget(&self) -> &gtk::Widget {
        self.root.upcast_ref()
    }

    /// Rebuilds the rows from the listing and puts the selection back on the
    /// cursor. Called after anything that changes the model.
    pub fn refresh(&mut self) {
        let path = self.listing.dir().as_str();
        self.path_bar.set_text(&match &self.error {
            Some(reason) => format!("{path}{PATH_BAR_ERROR_SEPARATOR}{reason}"),
            None => path.to_string(),
        });

        self.store.remove_all();
        for index in 0..self.listing.len() {
            let entry = self
                .listing
                .get(index)
                .expect("indices below len() always resolve");
            let row = Row::from_entry(entry, self.listing.is_parent(index));
            self.store.append(&PaneEntry::new(row));
        }

        self.sync_cursor();
    }

    /// Moves the visible selection onto the listing's cursor.
    fn sync_cursor(&self) {
        if self.listing.is_empty() {
            return;
        }
        self.selection.set_selected(self.listing.cursor() as u32);
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
        let focus = focus_after_move(self.listing.dir(), &dir);
        match Listing::load(self.fs.as_ref(), dir.clone()) {
            Ok(mut listing) => {
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
        let label = gtk::Label::builder()
            .xalign(column.xalign())
            .ellipsize(gtk::pango::EllipsizeMode::Middle)
            .build();
        item.set_child(Some(&label));
    });

    factory.connect_bind(move |_, item| {
        let entry = item
            .item()
            .and_downcast::<PaneEntry>()
            .expect("the store holds PaneEntry values");
        let label = item
            .child()
            .and_downcast::<gtk::Label>()
            .expect("setup installed a Label");
        label.set_text(&entry.text(column));
    });

    let view_column = gtk::ColumnViewColumn::new(Some(column.title()), Some(factory));
    view_column.set_fixed_width(column.width());
    view_column.set_expand(column.expands());
    view_column
}
