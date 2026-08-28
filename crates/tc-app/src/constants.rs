//! Constants of the GTK shell, collected here instead of inline at the use
//! site (skills 16/17).

/// Product name as shown to the user.
pub const APP_NAME: &str = "Ferrocommander";

/// Reverse-DNS application id GTK identifies the process by.
pub const APP_ID: &str = "st.rose.Ferrocommander";

pub const WINDOW_WIDTH: i32 = 1200;
pub const WINDOW_HEIGHT: i32 = 700;

/// Both panes start equally wide — neither side is the "main" one.
pub const PANE_SPLIT_RATIO: f32 = 0.5;

/// Panes in the window. Switching cycles through them, so a third pane
/// later would need no new switching logic.
pub const PANE_COUNT: usize = 2;

/// Column headers.
pub const COLUMN_TITLE_NAME: &str = "Name";
pub const COLUMN_TITLE_EXT: &str = "Ext";
pub const COLUMN_TITLE_SIZE: &str = "Size";
pub const COLUMN_TITLE_DATE: &str = "Date";

/// Column widths. The name column expands into leftover space; the others
/// stay fixed so the two panes line up with each other.
pub const COLUMN_WIDTH_NAME: i32 = 260;
pub const COLUMN_WIDTH_EXT: i32 = 70;
pub const COLUMN_WIDTH_SIZE: i32 = 120;
pub const COLUMN_WIDTH_DATE: i32 = 140;

/// Cell alignment: sizes right-aligned so digits line up by magnitude,
/// everything else left.
pub const XALIGN_LEFT: f32 = 0.0;
pub const XALIGN_RIGHT: f32 = 1.0;

/// Shown in the size column for directories, whose byte size is meaningless.
pub const DIR_SIZE_LABEL: &str = "<DIR>";

/// Separates thousands in file sizes, as Total Commander groups them.
pub const THOUSANDS_SEPARATOR: char = ' ';

/// Digits per group in a file size.
pub const THOUSANDS_GROUP: usize = 3;

/// `glib::DateTime` format for the date column: sortable, unambiguous, and
/// free of locale surprises in a column of fixed width.
pub const DATE_FORMAT: &str = "%Y-%m-%d %H:%M";

/// Separates the directory path from an error notice in the path bar.
pub const PATH_BAR_ERROR_SEPARATOR: &str = "   ⚠ ";

/// Spacing and padding of the pane's own widgets.
pub const PANE_SPACING: i32 = 0;

/// Style classes the stylesheet keys off.
pub const CLASS_PANE: &str = "pane";
pub const CLASS_PANE_ACTIVE: &str = "pane-active";
pub const CLASS_PATH_BAR: &str = "path-bar";

/// The shell's stylesheet.
///
/// The active pane is marked on its path bar rather than by dimming the whole
/// widget: in a dual-pane manager the inactive side must stay fully readable,
/// since its only job at that moment is to show you where a copy would land.
pub const STYLESHEET: &str = "
.path-bar {
    padding: 4px 8px;
    font-family: monospace;
}
.pane-active .path-bar {
    background-color: #3584e4;
    color: #ffffff;
    font-weight: bold;
}
";
