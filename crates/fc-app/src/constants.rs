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

/// Rows a page key leaves on screen from the page before it.
///
/// One, and measured rather than picked: this is what the `ColumnView`'s own
/// paging did before the keys were bound, and reproducing it exactly was the
/// requirement. A jump that shares no row with the page before it gives the
/// reader nothing to place themselves by.
pub const PAGE_OVERLAP_ROWS: usize = 1;

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
///
/// `cmd` is the Command key: GTK on macOS reports it as `META_MASK`, which is
/// why the name maps there rather than to Super. It exists so a macOS keymap
/// layer is *expressible* — before it, `RELEVANT_MODIFIERS` masked META out
/// ahead of the lookup and a Cmd binding could never fire, a gap found by the
/// groundwork plan on a Linux box. On Linux the key that sends META (where
/// any does) is a keyboard-layout question; nothing default binds it.
pub const MODIFIER_NAMES: [(&str, gtk::gdk::ModifierType); 4] = [
    ("ctrl", gtk::gdk::ModifierType::CONTROL_MASK),
    ("shift", gtk::gdk::ModifierType::SHIFT_MASK),
    ("alt", gtk::gdk::ModifierType::ALT_MASK),
    ("cmd", gtk::gdk::ModifierType::META_MASK),
];

/// The shortcuts a text field owns, whatever the keymap says about them.
///
/// While the command line has the focus these belong to the *entry*: `Ctrl+V`
/// pastes a path into a command, it does not start a copy. Without this list
/// the shell dispatches any modified key the keymap claims, which was
/// harmless only for as long as the keymap claimed nothing a text field
/// wants — and then `Ctrl+C`, `Ctrl+X` and `Ctrl+V` were bound.
///
/// A **list of what a text field owns**, not a list of today's collisions.
/// That is the difference between a fix and the same bug again the next time
/// somebody binds a key: `Ctrl+Y` is here although nothing claims it.
///
/// All of them are Ctrl and nothing else, which the check relies on.
pub const TEXT_FIELD_SHORTCUTS: [gtk::gdk::Key; 6] = [
    gtk::gdk::Key::a, // select all
    gtk::gdk::Key::c, // copy
    gtk::gdk::Key::v, // paste
    gtk::gdk::Key::x, // cut
    gtk::gdk::Key::z, // undo
    gtk::gdk::Key::y, // redo
];

/// Where the generated action-name table lives, relative to this crate.
///
/// The table is what somebody rebinding a key needs and could only get by
/// reading `keymap.rs`. It is generated from `ACTION_NAMES` and `BINDINGS`
/// rather than typed, and a test compares the two — so the markers below are
/// load-bearing, not decoration (skill 53).
/// Where the end-to-end suite lives, relative to this crate.
#[cfg(test)]
pub const UI_SUITE: &str = "tests/ui.rs";

/// Where the harness lives — it presses keys of its own, `Ctrl+Q` among them.
#[cfg(test)]
pub const UI_HARNESS: &str = "tests/harness/mod.rs";

/// Keys the end-to-end suite may press by another spelling than the keysym.
///
/// `xdotool` takes X names, so `Page_Down` is `Next`; and the alias table lets
/// an ordinary `+` reach a keypad binding, which is how the suite presses one
/// on a keyboard that may have no numeric block.
#[cfg(test)]
pub const UI_KEY_SPELLINGS: [(gtk::gdk::Key, &str); 6] = [
    (gtk::gdk::Key::Page_Down, "Next"),
    (gtk::gdk::Key::Page_Up, "Prior"),
    (gtk::gdk::Key::KP_Add, "plus"),
    (gtk::gdk::Key::KP_Subtract, "minus"),
    (gtk::gdk::Key::KP_Multiply, "asterisk"),
    (gtk::gdk::Key::KP_Divide, "slash"),
];

/// Bindings no end-to-end test presses, each with the reason it need not.
///
/// The suite drives the real binary through a real X server, and it is where
/// every bug in this project has lived — so "which keys does it actually
/// press" is worth a test rather than a paragraph. It was a paragraph until
/// 2026-08-30, and the paragraph was missing `Num −`, which read as covered
/// because its twin had a test and nothing counted.
///
/// **An alias earns a place here; a command does not.** A key that resolves to
/// the same action as one that *is* pressed adds only the lookup, which the
/// keymap's own tests cover exhaustively. A key that reaches an action nothing
/// else reaches does not belong here — it belongs in the suite.
#[cfg(test)]
pub const UI_UNPRESSED: [(&str, &str); 8] = [
    (
        "ctrl+F3",
        "sorting is covered headlessly; ctrl+F6 is pressed for the wiring",
    ),
    ("ctrl+F4", "as ctrl+F3"),
    ("ctrl+F5", "as ctrl+F3"),
    ("KP_Enter", "an alias for Return, which is pressed"),
    (
        "ctrl+KP_Enter",
        "an alias for ctrl+Return, which is pressed",
    ),
    (
        "shift+alt+KP_Enter",
        "an alias for shift+alt+Return, which is pressed",
    ),
    ("ctrl+KP_Add", "an alias for ctrl+a, which is pressed"),
    ("shift+F8", "an alias for shift+Delete, which is pressed"),
];

/// Where the hand-written bindings table lives, and the heading above it.
///
/// That table stays prose — what `Insert` *means* is not derivable from the
/// code and a generated version would lose the rows that pair two keys. What
/// is checked is the facts: a test parses the keys out of it and asserts they
/// are exactly the keys in `BINDINGS`, so a binding added, removed or moved
/// without the document following fails the gate.
#[cfg(test)]
pub const BINDINGS_TABLE_HEADING: &str = "## Bindings";

/// How `docs/keymap.md` spells a key that GDK spells differently.
///
/// Only the irregular ones: a document writes `↑` where a keysym says `Up`,
/// and `Num +` where it says `KP_Add`. Everything regular — the letters, the
/// F-keys, `Home`, `Insert` — goes through `key_named`, which already knows
/// how to read a name in whatever case it arrives.
///
/// **Longest match wins**, because `Num Enter` ends with `Enter`.
#[cfg(test)]
pub const DOC_KEY_NAMES: [(&str, gtk::gdk::Key); 14] = [
    ("Num Enter", gtk::gdk::Key::KP_Enter),
    ("Backspace", gtk::gdk::Key::BackSpace),
    ("Enter", gtk::gdk::Key::Return),
    ("Esc", gtk::gdk::Key::Escape),
    ("PgUp", gtk::gdk::Key::Page_Up),
    ("PgDn", gtk::gdk::Key::Page_Down),
    ("Num +", gtk::gdk::Key::KP_Add),
    // U+2212, the minus sign the table is written with — not a hyphen.
    ("Num −", gtk::gdk::Key::KP_Subtract),
    ("Num *", gtk::gdk::Key::KP_Multiply),
    ("Num /", gtk::gdk::Key::KP_Divide),
    ("↑", gtk::gdk::Key::Up),
    ("↓", gtk::gdk::Key::Down),
    ("←", gtk::gdk::Key::Left),
    ("→", gtk::gdk::Key::Right),
];

/// What separates two keys that mean **the same command** in the table, and
/// what separates the two halves of a row that is *about* a pair.
///
/// The table already used them that way throughout — `` `F8`, `Delete` `` is
/// one command reachable two ways, `` `↑` / `↓` `` is two commands on one
/// row — so the punctuation is load-bearing and a test reads it. That is why
/// there is no list of exceptions here: the document says which rows are
/// which, in the character it already used.
#[cfg(test)]
pub const SAME_ACTION_SEPARATOR: &str = ",";
#[cfg(test)]
pub const PAIRED_ROW_SEPARATOR: &str = " / ";

///
/// Only the test that renders the table reads them, so they are `cfg(test)`.
/// They live here rather than beside it because this is where every string
/// the shell owns lives, and a marker written into a document is exactly
/// that kind of string — the renderer itself is test code and sits with its
/// caller in `keymap.rs`.
#[cfg(test)]
pub const ACTION_TABLE_DOC: &str = "../../docs/keymap.md";
#[cfg(test)]
pub const ACTION_TABLE_BEGIN: &str = "<!-- generated: action names -->";
#[cfg(test)]
pub const ACTION_TABLE_END: &str = "<!-- /generated: action names -->";

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
/// How big a row's leading icon is, and how far the name sits from it.
///
/// Sixteen pixels is what the screenshot of Total Commander shows and what
/// every desktop icon theme has at native size — asking for anything else
/// means a scaled bitmap on the row that is redrawn most.
pub const ROW_ICON_SIZE: i32 = 16;
pub const ROW_ICON_GAP: i32 = 4;

/// What GIO calls a directory. Not guessed from the name: a folder called
/// `archive.tar.gz` is not a gzip, and it is the one row whose type is known
/// without asking.
pub const DIRECTORY_CONTENT_TYPE: &str = "inode/directory";

/// What a directory's name is wrapped in, Total Commander's own spelling —
/// `[Documents]`, and `[..]` for the parent row.
///
/// Display only. The name a rename, a mark, the type-ahead or a job uses is
/// `Row::full_name`, which is what the filesystem calls it.
pub const DIR_NAME_OPEN: &str = "[";
pub const DIR_NAME_CLOSE: &str = "]";

pub const DIR_SIZE_LABEL: &str = "<DIR>";

/// Separates thousands in file sizes, as Total Commander groups them.
pub const THOUSANDS_SEPARATOR: char = ' ';

/// Digits per group in a file size.
pub const THOUSANDS_GROUP: usize = 3;

/// `glib::DateTime` format for the date column: sortable, unambiguous, and
/// free of locale surprises in a column of fixed width.
pub const DATE_FORMAT: &str = "%Y-%m-%d %H:%M";

/// What the path bar adds when a pane is showing a branch view rather than
/// one directory.
///
/// The shell glob for "everything below here", because that is what the pane
/// is showing and what somebody would have typed to mean it.
pub const BRANCH_MARKER: &str = "/**";

/// Marks a counted folder size as a lower bound.
///
/// A subdirectory that refused to be read, or a scan that was stopped: the
/// number is the best answer there is, and a size nobody can trust looking
/// exactly like one they can is the failure worth one character to avoid.
pub const SIZE_PARTIAL_MARKER: &str = "+";

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

/// What a stylesheet rule GTK cannot parse is reported as, on stderr.
///
/// GTK drops such a rule and carries on silently, so without this the only
/// symptom is a style that never applied.
pub const STYLESHEET_REJECTED: &str = "stylesheet rule rejected";

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
.pane columnview listview > row {
    /* As compact as the font allows. GTK's default row is padded for touch
       and the pane showed about half as many files as Total Commander does
       in the same window — the second testing round's screenshot is 17 px a
       row, and this is how close a desktop font gets to it. `min-height: 0`
       matters as much as the padding: without it GTK keeps its own floor and
       the padding change does nothing. */
    padding: 0;
    min-height: 0;
    /* And a notch smaller than the desktop's default, which is set for
       reading prose rather than for scanning a directory. The screenshot is
       17 px a row; padding alone reaches 23, and this reaches 20 without
       making the names harder to read than the date column beside them. */
    font-size: 0.9em;
}
.pane columnview listview > row cell {
    padding: 0 4px;
    min-height: 0;
}
.pane columnview header button {
    /* The header follows the rows down, or it is taller than three of them. */
    padding: 1px 4px;
    min-height: 0;
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
.job-indicator {
    /* The running-job bar at the end of the command line's row. Flat and
       narrow, because it lives in the chrome rather than in a dialog. */
    padding: 0 4px;
    min-height: 0;
}
.job-indicator progress, .job-indicator trough {
    min-height: 14px;
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
/* The inactive pane's cursor is the same rectangle, unfilled.
   Two solid blue bars say nothing about which one the keys reach, and a
   pane with no cursor at all forgets where it was. An outline keeps the
   row readable — which is why the inactive side was never dimmed — while
   still saying where a Tab would land you.
   `inset` rather than a border: a border changes the row's height, and a
   row that grew when it lost focus would move every row under it. */
.pane:not(.pane-active) columnview listview > row:selected {
    background-image: none;
    background-color: transparent;
    color: inherit;
    box-shadow: inset 0 0 0 1px #3584e4;
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
/// What the right end of the status line says about the disk.
///
/// Both figures: "18.2 GB free" alone says nothing about whether that is a
/// nearly empty disk or a nearly full one.
pub const DISK_SPACE: &str = "{free} free of {total}";

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
/// The running-job bar in the corner ([`docs/ops.md`]).
pub const CLASS_JOB_INDICATOR: &str = "job-indicator";

/// How wide that bar is. Wide enough for `42 % — 1.2 GB of 3.0 GB` to be
/// legible, narrow enough that the command line keeps the row.
pub const INDICATOR_WIDTH: i32 = 220;

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

/// What sends a running job on without watching it.
///
/// Not "Close": the window is not what is being dismissed, the *watching* is,
/// and the job carries on either way. Total Commander calls it Background.
pub const BUTTON_BACKGROUND: &str = "Background";
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

/// The `PaneEntry` property a cell watches to know its row changed.
///
/// A row is rewritten in place rather than replaced, so nothing about the
/// *model* changes when a mark is toggled — see `PaneEntry::rewrite`.
pub const ROW_REVISION: &str = "revision";

/// Where a bound cell keeps the handler that repaints it, so unbinding can
/// take it off again. A `ListItem` is recycled across rows, and a handler
/// left connected would paint the wrong row's text into it.
pub const CELL_REPAINT: &str = "fc-cell-repaint";

/// How much unscrolled room still counts as room, in pixels.
///
/// An adjustment's arithmetic is floating point, so "is there anything left
/// to scroll" is never exactly zero at the end of a document. Less than a
/// pixel is nothing anybody can see.
pub const SCROLL_ROOM_EPSILON: f64 = 1.0;

/// When the remembered scroll offset is put back, as a main-loop priority.
///
/// **Between GTK's two idle passes, and that is the whole point.** GTK lays
/// out at `GTK_PRIORITY_RESIZE` (`HIGH_IDLE + 10`) and paints at
/// `GDK_PRIORITY_REDRAW` (`HIGH_IDLE + 20`). Restoring before the first
/// clamps the offset to zero, because the adjustment does not know how tall
/// the rows are yet; restoring after the second — which is what a plain
/// `idle_add_local_once` at `DEFAULT_IDLE` does — paints one frame at the top
/// of the list and corrects it on the next. That frame is the flicker a
/// person sees coming back out of a directory.
///
/// The number is a documented ordering rather than an API contract, and
/// `scripts/check-scroll-memory.sh` is the only thing that would notice it
/// changing.
pub const SCROLL_RESTORE_PRIORITY: i32 = 115;

/// What a file with nothing in it says, rather than an empty window that looks
/// like it failed to load.
pub const VIEWER_EMPTY: &str = "(empty file)";

/// The pane's two faces while quick view is on ([`docs/viewer.md`]): its own
/// listing, or a preview of what the *other* pane's cursor is on. Named
/// pages of a stack rather than a widget swapped in and out, so the listing
/// keeps its selection, its scroll position and its watch while hidden —
/// leaving quick view restores a pane rather than rebuilding one.
pub const PANE_PAGE_LIST: &str = "list";
pub const PANE_PAGE_PREVIEW: &str = "preview";

/// What the preview says when the cursor is on something with no content to
/// show. A line rather than a blank: a pane that has gone empty reads as
/// broken, and a pane still showing the last file reads as stale.
pub const PREVIEW_PARENT: &str = "(the parent directory)";
pub const PREVIEW_UNREADABLE: &str = "(cannot be read)";
pub const PREVIEW_EMPTY_DIR: &str = "(nothing here)";

/// A folder's summary while its walk is still running, and once it lands —
/// the owner's decision that a directory shows what it holds rather than a
/// placeholder. The first is what a cursor passing through sees, and is why
/// the walk is never waited for.
pub const PREVIEW_FOLDER_COUNTING: &str = "{name}\n\ncounting…";
pub const PREVIEW_FOLDER_COUNTED: &str = "{name}\n\n{bytes}";
pub const PREVIEW_FOLDER_PARTIAL: &str = "{name}\n\n{bytes} (some of it unreadable)";

/// The compare window's title: both files, so a person with several open
/// knows which pair is which — and how the end-to-end suite finds it.
pub const TITLE_COMPARE: &str = "{left} \u{2194} {right}";

/// Sized like the viewer, for the viewer's reason: it is there to be read.
pub const COMPARE_WIDTH: i32 = 900;
pub const COMPARE_HEIGHT: i32 = 700;

/// What the byte verdict says when the row view does not apply
/// ([`docs/compare.md`]): the files were binary or over the engine's
/// ceiling, and only their bytes were compared.
pub const TITLE_COMPARE_VERDICT: &str = "Compare";
pub const COMPARE_IDENTICAL: &str = "The two files are identical.";
pub const COMPARE_DIFFER_AT: &str =
    "The two files differ. The first difference is at byte {offset}.";

/// What the compare key says when a pair cannot be picked: nothing under
/// the cursor to compare, and no marked pair to fall back on.
pub const COMPARE_NOTHING: &str = "Nothing to compare: mark two files, or put the cursor on one.";

/// Why a configured compare tool refuses inside an archive — the editor's
/// reason, one key over: the tool takes operating-system paths, and an
/// archive entry has none.
pub const COMPARE_IN_ARCHIVE: &str =
    "A file in this pair is inside an archive. The compare tool needs real paths; unpack it first.";

/// The tints of the compare view, one per row kind and one stronger mark
/// for the characters inside a changed pair that actually differ.
/// Translucent, so they read as background under either theme's text.
pub const COMPARE_TINT_LEFT_ONLY: &str = "rgba(192, 28, 40, 0.18)";
pub const COMPARE_TINT_RIGHT_ONLY: &str = "rgba(38, 162, 105, 0.18)";
pub const COMPARE_TINT_CHANGED: &str = "rgba(229, 165, 10, 0.18)";
pub const COMPARE_TINT_SPAN: &str = "rgba(229, 165, 10, 0.45)";

/// The tag names inside the compare view's buffers.
pub const TAG_LEFT_ONLY: &str = "left-only";
pub const TAG_RIGHT_ONLY: &str = "right-only";
pub const TAG_CHANGED: &str = "changed";
pub const TAG_SPAN: &str = "span";

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
/// What `F4` says when the file is inside an archive.
///
/// An editor takes an operating-system path, and a file in an archive has
/// none. `F3` reads through the backend and works.
pub const EDIT_IN_ARCHIVE: &str =
    "This file is inside an archive. F3 shows it; editing it needs it unpacked first.";

/// What `Ctrl+C` and `Ctrl+X` say inside an archive, and what `Ctrl+V` says
/// when the pane being pasted into is one.
///
/// The first is the reason `F4` and Enter are refused there: an entry has no
/// operating-system path, so a URI naming one would point at a file on the
/// disk that merely shares its name. The second is simply that an archive is
/// read-only.
pub const CLIPBOARD_IN_ARCHIVE: &str =
    "These are inside an archive. Unpack them first — the clipboard carries paths on the disk.";
pub const PASTE_INTO_ARCHIVE: &str =
    "This is inside an archive, which is read-only. Unpack it somewhere and paste there.";

/// How much of a clipboard payload is read.
///
/// A clipboard is somebody else's data and its size is their choice, so it is
/// bounded: a megabyte is tens of thousands of paths, and a file manager
/// pasting more than that has a different problem.
pub const CLIPBOARD_READ_LIMIT: usize = 1024 * 1024;

/// What `Enter` says when the file is inside an archive.
///
/// Separate from [`EDIT_IN_ARCHIVE`] rather than shared: they are different
/// questions with different ways out, and a message naming F4 when the user
/// pressed Enter sends them to the wrong key.
pub const OPEN_IN_ARCHIVE: &str =
    "This file is inside an archive. F3 shows it; opening it needs it unpacked first.";

/// Why the favourites list refuses to keep where a pane is standing.
///
/// A path inside an archive belongs to that archive's own store, where the
/// same spelling means a completely different file — so a favourite made of
/// one would either fail on the next run or, worse, resolve against the real
/// filesystem.
pub const FAVOURITE_IN_ARCHIVE: &str =
    "This pane is inside an archive, which is not a directory to come back to.";

pub const COMMAND_IN_ARCHIVE: &str =
    "This pane is inside an archive, which is not a directory a command can run in.";

/// Separates the prompt from the entry: the directory a command will run in,
/// then the usual shell mark.
pub const COMMAND_PROMPT_SUFFIX: &str = "$";

/// Title of the drive selector Alt+F1 / Alt+F2 open.
pub const TITLE_DRIVES: &str = "Drives";

/// The `Ctrl+D` window.
pub const TITLE_FAVOURITES: &str = "Favourite directories";

/// The row that keeps where the active pane is, and what it teaches.
///
/// The hint is where `Delete` is documented: a key that removes something
/// and is written down nowhere the user looks is a key nobody finds.
pub const FAVOURITES_ADD: &str = "Add the current directory";
pub const FAVOURITES_ADD_HINT: &str = "Del removes the row under the cursor";

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
/// How long a type-ahead search stays warm.
///
/// Long enough to type a name at speaking pace, short enough that a letter
/// pressed after a pause starts a new search rather than continuing one
/// nobody remembers beginning. The same order as every other program's
/// list-search timeout.
pub const TYPE_AHEAD_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(1000);

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
