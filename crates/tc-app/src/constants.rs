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

/// Quotation marks around a name inside a prompt. Typographic, because the
/// prompt is prose and a name may itself contain an ASCII quote.
pub const QUOTE_OPEN: &str = "\u{201c}";
pub const QUOTE_CLOSE: &str = "\u{201d}";

/// What the delete confirmation calls the thing at stake.
pub const KIND_FILE: &str = "file";
pub const KIND_DIRECTORY: &str = "directory";

/// The two delete questions. Separate strings rather than one with a word
/// swapped: they are different questions, and only one of them is final.
pub const DELETE_PROMPT_TRASH: &str = "Move the {kind} {name} to the trash?";
pub const DELETE_PROMPT_PERMANENT: &str =
    "Delete the {kind} {name} permanently? This cannot be undone.";

/// Dialog titles.
pub const TITLE_COPY: &str = "Copy";
pub const TITLE_MOVE: &str = "Move / Rename";
pub const TITLE_CREATE_DIR: &str = "New directory";
pub const TITLE_DELETE: &str = "Confirm delete";
pub const TITLE_CONFLICT: &str = "Target already exists";

/// Prompts above the entry field of the input dialogs.
pub const PROMPT_COPY: &str = "Copy to:";
pub const PROMPT_MOVE: &str = "Move to:";
pub const PROMPT_CREATE_DIR: &str = "Name of the new directory:";

/// Button labels.
pub const BUTTON_OK: &str = "OK";
pub const BUTTON_CANCEL: &str = "Cancel";
pub const BUTTON_DELETE: &str = "Delete";
pub const BUTTON_OVERWRITE: &str = "Overwrite";
pub const BUTTON_SKIP: &str = "Skip";
pub const BUTTON_KEEP_BOTH: &str = "Keep both";
pub const BUTTON_ABORT: &str = "Abort";

/// Label of the checkbox that turns one answer into a policy.
pub const CHECK_APPLY_TO_ALL: &str = "Apply to all";

/// Spacing and margins shared by every dialog, so they look like one family.
pub const DIALOG_SPACING: i32 = 12;
pub const DIALOG_MARGIN: i32 = 16;
pub const DIALOG_WIDTH: i32 = 520;

/// Characters the entry field is at least wide enough for.
pub const ENTRY_WIDTH_CHARS: i32 = 48;

/// What the conflict dialog says above the buttons.
pub const CONFLICT_PROMPT: &str = "{name} already exists in the target.";
