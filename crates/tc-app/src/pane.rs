//! One pane: a path bar above a column view of a directory.

use gtk::gio;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

use tc_core::listing::Listing;
use tc_core::vfs::{VfsPath, VirtualFs};

use crate::constants::{CLASS_PANE, CLASS_PANE_ACTIVE, CLASS_PATH_BAR, PANE_SPACING};
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
            Column::Name => "Name",
            Column::Ext => "Ext",
            Column::Size => "Size",
            Column::Modified => "Date",
        }
    }

    fn width(self) -> i32 {
        match self {
            Column::Name => 260,
            Column::Ext => 70,
            Column::Size => 120,
            Column::Modified => 140,
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
            Column::Size => 1.0,
            _ => 0.0,
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
    listing: Listing,
}

impl PaneView {
    /// Builds a pane showing `dir`.
    ///
    /// The filesystem is borrowed rather than owned: nothing in this phase
    /// re-reads after construction, and the pane will take ownership in
    /// phase D, where navigation gives it a caller.
    ///
    /// A directory that cannot be read yields an empty pane rather than
    /// failing construction — a window with one broken pane is still a usable
    /// program. Phase D reports the reason in the path bar.
    pub fn new(fs: &dyn VirtualFs, dir: VfsPath) -> Self {
        let listing =
            Listing::load(fs, dir.clone()).unwrap_or_else(|_| Listing::new(dir, Vec::new()));

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
            listing,
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
        self.path_bar.set_text(self.listing.dir().as_str());

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
    pub fn sync_cursor(&self) {
        if self.listing.is_empty() {
            return;
        }
        self.selection.set_selected(self.listing.cursor() as u32);
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
