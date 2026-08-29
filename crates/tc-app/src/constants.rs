//! Constants of the GTK shell, collected here instead of inline at the use
//! site (skills 16/17).

/// Product name as shown to the user.
///
/// A macro because [`APP_TITLE`] is built with `concat!`, which takes literals
/// — this keeps the spelling in one place instead of once here and once
/// inside the concatenation.
macro_rules! app_name {
    () => {
        "FerroCommander"
    };
}

/// What the window is called: `FerroCommander #51 (62fddd9)`.
///
/// The build is on the title bar because that is the one part of the window
/// that survives into a screenshot or a bug report, and "which build were you
/// running" is the first question either raises.
///
/// A constant, not a function: `build.rs` asks git and hands over the finished
/// tail through `TC_BUILD_STAMP`, so the whole title is fixed at compile time
/// and nothing assembles it at run time. The tail is empty on a build with no
/// git to ask, and the title is then the bare product name — the rule, and its
/// tests, live in `build_stamp.rs`.
pub const APP_TITLE: &str = concat!(app_name!(), env!("TC_BUILD_STAMP"));

/// Reverse-DNS application id GTK identifies the process by.
pub const APP_ID: &str = "st.rose.Ferrocommander";

/// How long the settings wait after a change before being written.
///
/// Not zero, because dragging a window edge changes the size continuously and
/// each step would be a write. Not long either: the point of saving on change
/// rather than only on exit is that a killed or crashed program keeps what you
/// did, and anything longer than about a second stops being true in practice.
pub const SETTINGS_SAVE_DELAY: std::time::Duration = std::time::Duration::from_millis(500);

/// What a run with no readable settings reports, once, on stderr. The program
/// starts either way.
pub const SETTINGS_UNREADABLE: &str = "settings could not be read, using defaults";
pub const SETTINGS_UNWRITABLE: &str = "settings could not be saved";

/// Rows a page key moves when the pane has not been laid out yet and there is
/// no viewport height to divide by.
///
/// Only reachable before the first frame, since a laid-out pane measures its
/// own page. A screenful on a small window, so the one keystroke that could
/// land here does something sensible rather than nothing.
pub const PAGE_ROWS_FALLBACK: usize = 20;

/// Both panes start equally wide — neither side is the "main" one.
pub const PANE_SPLIT_RATIO: f32 = 0.5;

/// What separates the parts of a key name in the settings file:
/// `ctrl+shift+kp_add`.
pub const KEY_SPEC_SEPARATOR: char = '+';

/// What separates the words inside one key name: `page_down`, `kp_add`.
pub const KEY_NAME_SEPARATOR: char = '_';

/// The keypad keys are the one family GDK shouts the prefix of.
pub const KEYPAD_PREFIX: &str = "KP_";
pub const KEYPAD_PREFIX_TITLED: &str = "Kp_";

/// The modifier names a settings file may use, and the only place that
/// mapping lives. Matched case-insensitively.
pub const MODIFIER_NAMES: [(&str, gtk::gdk::ModifierType); 3] = [
    ("ctrl", gtk::gdk::ModifierType::CONTROL_MASK),
    ("shift", gtk::gdk::ModifierType::SHIFT_MASK),
    ("alt", gtk::gdk::ModifierType::ALT_MASK),
];

/// What a settings file with a binding nobody can make sense of is told,
/// once, on stderr. The rest of the table still applies.
pub const UNKNOWN_KEY: &str = "no such key";
pub const UNKNOWN_ACTION: &str = "no such action";

/// The two panes by position, for the commands that name a side rather than
/// "the active one": Ctrl+← / Ctrl+→ and the pane exchange.
pub const LEFT_PANE: usize = 0;
pub const RIGHT_PANE: usize = 1;

/// Panes in the window. Switching cycles through them, so a third pane
/// later would need no new switching logic.
pub const PANE_COUNT: usize = 2;

/// Appended to the header of the column the listing is ordered by.
pub const SORT_MARKER_ASCENDING: &str = " \u{25b2}";
pub const SORT_MARKER_DESCENDING: &str = " \u{25bc}";

/// Column headers.
pub const COLUMN_TITLE_NAME: &str = "Name";
pub const COLUMN_TITLE_EXT: &str = "Ext";
pub const COLUMN_TITLE_SIZE: &str = "Size";
pub const COLUMN_TITLE_DATE: &str = "Date";
pub const COLUMN_TITLE_ATTR: &str = "Attr";

/// Column widths. The name column expands into leftover space; the others
/// stay fixed so the two panes line up with each other.
pub const COLUMN_WIDTH_NAME: i32 = 260;
pub const COLUMN_WIDTH_EXT: i32 = 70;
pub const COLUMN_WIDTH_SIZE: i32 = 120;
pub const COLUMN_WIDTH_DATE: i32 = 140;
/// Wide enough for `rwxr-xr-x`, which is the longest form either platform
/// produces.
pub const COLUMN_WIDTH_ATTR: i32 = 90;

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

/// Gap between the drive buttons.
pub const DRIVE_BAR_SPACING: i32 = 2;

/// Spacing and padding of the pane's own widgets.
pub const PANE_SPACING: i32 = 0;

/// Style classes the stylesheet keys off.
pub const CLASS_PANE: &str = "pane";
pub const CLASS_PANE_ACTIVE: &str = "pane-active";
pub const CLASS_PATH_BAR: &str = "path-bar";

/// Style classes GTK itself renders: the affirmative button and the dangerous
/// one. Here rather than beside their use, so every class name in the shell
/// is in one list (skill 17).
pub const CLASS_SUGGESTED: &str = "suggested-action";
pub const CLASS_DESTRUCTIVE: &str = "destructive-action";

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
.marked {
    /* Colour only, no bold: bold text is wider, and it pushed the date out
       of its fixed-width column so a marked row read `2026-0... 8 17:20`.
       Total Commander marks in red alone for the same reason. */
    color: #c01c28;
}
.drive-bar {
    padding: 2px 4px;
}
.dim {
    opacity: 0.55;
    font-family: monospace;
}
.command-line {
    padding: 2px 4px;
}
.command-line entry {
    font-family: monospace;
}
.command-prompt {
    padding: 0 4px;
    font-family: monospace;
    opacity: 0.7;
}
.output {
    /* Program output, where the columns mean something. */
    font-family: monospace;
    padding: 4px;
}
.drive-bar button {
    padding: 1px 8px;
    min-height: 0;
}
.filter-bar {
    margin: 2px 4px;
}
.status-line {
    padding: 2px 8px;
    font-size: 90%;
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
pub const DELETE_PROMPT_TRASH: &str = "Move the {subject} to the trash?";
pub const DELETE_PROMPT_PERMANENT: &str =
    "Delete the {subject} permanently? This cannot be undone.";

/// How several entries are described instead of being listed. A list of forty
/// names is not a question anyone reads.
pub const SUBJECT_MANY: &str = "{count} selected entries";

/// What the status line under a pane says.
pub const SELECTION_STATUS: &str =
    "{marked} of {total} selected \u{2014} {marked_bytes} of {total_bytes}";

/// Style class marking a row the user has selected.
pub const CLASS_MARKED: &str = "marked";
pub const CLASS_STATUS_LINE: &str = "status-line";
pub const CLASS_FILTER_BAR: &str = "filter-bar";
pub const CLASS_DRIVE_BAR: &str = "drive-bar";

/// Secondary text that should not compete with what it sits beside — the
/// mount path behind a drive's label.
pub const CLASS_DIM: &str = "dim";

/// The command line across the bottom, and the window a command's output
/// lands in.
pub const CLASS_COMMAND_LINE: &str = "command-line";
pub const CLASS_COMMAND_PROMPT: &str = "command-prompt";
pub const CLASS_OUTPUT: &str = "output";

/// What the quick-filter field says when it is empty.
pub const FILTER_PLACEHOLDER: &str = "Filter\u{2026}  (Esc to clear)";

/// Prompt of the dialog that asks for a select-by-pattern wildcard.
pub const TITLE_MARK_PATTERN: &str = "Select by pattern";
pub const TITLE_UNMARK_PATTERN: &str = "Deselect by pattern";
pub const PROMPT_PATTERN: &str = "Pattern (* and ? are wildcards):";

/// What that dialog starts with — everything, which is the common case and
/// one keystroke from what anyone else wants.
pub const PATTERN_DEFAULT: &str = "*.";

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

/// Titles of the windows a running job puts up.
pub const TITLE_PROGRESS: &str = "Working";
pub const TITLE_FAILURES: &str = "Some items were not processed";

/// The Shift+F4 dialog, and the name it offers to start from.
pub const TITLE_CREATE_FILE: &str = "New file";
pub const PROMPT_CREATE_FILE: &str = "Create and open:";
pub const NEW_FILE_DEFAULT: &str = "new.txt";

/// The viewer's title: the file being looked at, and where in it.
pub const TITLE_VIEWER: &str = "{name} \u{2014} {percent}%";

/// How tall and wide the viewer opens. Bigger than a dialog, because it is
/// there to be read rather than answered.
pub const VIEWER_WIDTH: i32 = 900;
pub const VIEWER_HEIGHT: i32 = 700;

/// What a file with nothing in it says, rather than an empty window that looks
/// like it failed to load.
pub const VIEWER_EMPTY: &str = "(empty file)";

/// The search dialog: what it is called, what it asks, and how it reports.
pub const TITLE_SEARCH: &str = "Find files";
pub const PROMPT_SEARCH_NAME: &str = "File name:";

/// What the name field starts with: everything.
///
/// Its own constant rather than the mark dialog's `PATTERN_DEFAULT`, which is
/// `*.` — a starting point to be edited there, and a pattern that matches
/// almost nothing here.
pub const SEARCH_NAME_DEFAULT: &str = "*";
pub const PROMPT_SEARCH_CONTENT: &str = "Containing text (optional):";
pub const SEARCH_START: &str = "Search";
pub const SEARCH_STOP: &str = "Stop";
pub const SEARCH_FOUND: &str = "{count} found";
pub const SEARCH_SEARCHING: &str = "{count} found, searching\u{2026}";
pub const SEARCH_NOTHING: &str = "Nothing found";

/// How tall the result list may grow before it scrolls.
pub const SEARCH_LIST_HEIGHT: i32 = 360;

/// How many results the list shows.
///
/// A `ListBox` is not virtualised — every row is a widget — so a search that
/// found a hundred thousand files would build a hundred thousand of them and
/// take the window down with it. Counting carries on past the cap and the
/// status says so, which is the honest version of a limit.
pub const SEARCH_LIST_LIMIT: usize = 5_000;

/// What the status says once the list has stopped growing.
pub const SEARCH_CAPPED: &str = "{count} found, showing the first {shown}";

/// The multi-rename tool: its title, its fields, and how a preview row reads.
pub const TITLE_RENAME: &str = "Multi-rename";
pub const PROMPT_RENAME_TEMPLATE: &str = "Name template ([N] name, [E] extension, [C] counter):";
pub const PROMPT_RENAME_COUNTER: &str = "Counter starts at:";
pub const PROMPT_RENAME_FIND: &str = "Replace:";
pub const PROMPT_RENAME_WITH: &str = "With:";
pub const BUTTON_RENAME: &str = "Rename";

/// A preview row, and what a refused one says instead of a new name.
pub const RENAME_ROW: &str = "{from}  \u{2192}  {to}";
pub const RENAME_EMPTY: &str = "(no name)";
pub const RENAME_COLLIDES: &str = "(name already taken in this batch)";
pub const RENAME_SEPARATOR: &str = "(a name cannot contain a path)";
pub const RENAME_UNCHANGED: &str = "(unchanged)";

/// How tall the preview may grow before it scrolls.
pub const RENAME_LIST_HEIGHT: i32 = 320;

/// Title of the command history Ctrl+Down opens.
pub const TITLE_HISTORY: &str = "Command history";

/// Title of the window a command's output lands in.
pub const TITLE_OUTPUT: &str = "Command output";

/// Alt+F5: the title, the prompt, and what the name field starts as.
pub const TITLE_PACK: &str = "Pack";
pub const PROMPT_PACK: &str = "Pack into archive (the extension picks the format):";
/// What Alt+F5 appends when it makes up a name. Zip because it is the format
/// every other program on both target platforms can open.
pub const PACK_DEFAULT_EXTENSION: &str = ".zip";
/// What the offered name falls back to when there is nothing to name it after
/// — packing from a filesystem root, which has no name of its own.
pub const PACK_FALLBACK_NAME: &str = "archive";

/// What the command line says when the active pane is inside an archive.
///
/// A path inside an archive is not somewhere a process can run, and running
/// the command against whatever that path means on the real filesystem is how
/// something meant for an archive acts on a home directory instead.
pub const COMMAND_IN_ARCHIVE: &str =
    "This pane is inside an archive, which is not a directory a command can run in.";

/// Separates the prompt from the entry: the directory a command will run in,
/// then the usual shell mark.
pub const COMMAND_PROMPT_SUFFIX: &str = "$";

/// Title of the drive selector Alt+F1 / Alt+F2 open.
pub const TITLE_DRIVES: &str = "Drives";

pub const BUTTON_CLOSE: &str = "Close";

/// Shown while a job is still scanning and no total exists yet.
pub const PROGRESS_SCANNING: &str = "Scanning\u{2026}";

/// How the progress window reports where a job has got to.
pub const PROGRESS_FORMAT: &str = "{done} of {total}";

/// Between the parts of the caption: bytes, then rate, then what is left.
pub const PROGRESS_SEPARATOR: &str = " \u{b7} ";
pub const PROGRESS_RATE_SUFFIX: &str = "/s";
pub const PROGRESS_ETA_SUFFIX: &str = " left";

/// How far back the rate is measured over.
///
/// Not the whole job: a run of small files followed by one big one would leave
/// the average saying something that stopped being true minutes ago, and an
/// estimate built on it would be wrong for the rest of the job. Not one sample
/// either — that jumps around with every buffer. A few seconds is long enough
/// to be steady and short enough to still be about now.
pub const PROGRESS_RATE_WINDOW: std::time::Duration = std::time::Duration::from_secs(3);

/// How long a job must have been running before a rate is shown at all.
///
/// A number computed from the first fifty milliseconds is noise, and one that
/// appears and then halves reads as a program that does not know what it is
/// doing.
pub const PROGRESS_RATE_DELAY: std::time::Duration = std::time::Duration::from_millis(750);

/// Seconds in a minute and minutes in an hour, for writing a time left.
pub const SECONDS_PER_MINUTE: u64 = 60;
pub const MINUTES_PER_HOUR: u64 = 60;

/// Units byte counts are rendered in. Binary, because file managers count in
/// what the filesystem allocates rather than in what a marketing department
/// prints on a box.
pub const BYTE_UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];

/// Step between two byte units.
pub const BYTE_STEP: f64 = 1024.0;

/// Decimals shown once a size has left plain bytes behind.
pub const BYTE_DECIMALS: usize = 1;

/// How long a job may run before it gets a progress window.
///
/// Below this it would flash up and vanish, which is more distracting than no
/// window at all. Measured from the job's start and checked as events arrive,
/// so no timer is needed.
pub const PROGRESS_DELAY: std::time::Duration = std::time::Duration::from_millis(300);

/// How a single failure is written in the summary.
pub const FAILURE_FORMAT: &str = "{path}: {reason}";

/// Failures listed before the summary stops and counts the rest.
pub const FAILURES_SHOWN: usize = 12;

/// How the summary reports the ones it did not list.
pub const FAILURES_MORE: &str = "\u{2026} and {count} more";

/// Height the failure list grows to before it starts scrolling.
pub const FAILURE_LIST_HEIGHT: i32 = 240;

/// How tall the drive list may grow before it scrolls. Like the failure list
/// it grows with its contents, so two mount points do not open a window mostly
/// full of nothing.
pub const DRIVE_LIST_HEIGHT: i32 = 320;

/// How tall a command's output may grow before it scrolls.
pub const OUTPUT_HEIGHT: i32 = 400;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn this_binary_was_stamped() {
        // The stamp comes from `build.rs`, so this asserts the build script
        // ran and found the repository — which no test of the formatting rule
        // can, and which is the half that actually breaks. How the stamp is
        // shaped is tested in `build_stamp.rs`.
        assert!(APP_TITLE.starts_with(app_name!()), "{APP_TITLE}");
        assert_ne!(
            APP_TITLE,
            app_name!(),
            "build.rs stamped nothing: no build number or commit hash"
        );
    }
}
