//! What each key does.
//!
//! Every default binding lives in one table, and the GTK controller knows no
//! key names at all — it looks up an [`Action`] and dispatches it. That is
//! what makes the bindings testable without a display, and it is what lets the
//! user's own bindings be laid over the defaults in one place.
//!
//! **The overlay is the shell's job, not `tc-core`'s.** The settings file
//! carries the `[keys]` table as plain strings because a key name is a GTK
//! keysym and an action is a command of this shell, neither of which the
//! engine knows anything about ([`docs/config.md`]).

use std::collections::HashMap;

use gtk::gdk::{Key, ModifierType};
use tc_core::listing::SortKey;

use crate::constants::{
    KEYPAD_PREFIX, KEYPAD_PREFIX_TITLED, KEY_NAME_SEPARATOR, KEY_SPEC_SEPARATOR, MODIFIER_NAMES,
    UNKNOWN_ACTION, UNKNOWN_KEY,
};

/// Something the user asked the shell to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Move the focus to the other pane.
    SwitchPane,
    CursorUp,
    CursorDown,
    CursorFirst,
    CursorLast,
    /// Page Up / Page Down — a screenful, less the row of overlap that makes
    /// the jump placeable.
    CursorPageUp,
    CursorPageDown,
    /// Enter the directory under the cursor.
    Activate,
    /// Leave the current directory.
    GoParent,
    /// F5 — copy the entry under the cursor.
    Copy,
    /// Alt+F5.
    Pack,
    /// F6 — move it, or rename it in place.
    Move,
    /// Ctrl+R — re-read the directory, keeping marks, cursor and position.
    Reread,
    /// Alt+F7 — find files below the active pane's directory.
    Search,
    /// Ctrl+M — rename what is marked, by a rule, with a preview first.
    MultiRename,
    /// Ctrl+Z — put the last multi-rename back.
    UndoRename,
    /// F3 — look inside the file under the cursor.
    View,
    /// F4 — hand it to the editor.
    Edit,
    /// Shift+F4 — create an empty file and open it in the editor.
    CreateFile,
    /// Shift+F6 — rename the row under the cursor, in the list itself.
    RenameInline,
    /// F7 — create a directory here.
    CreateDir,
    /// F8 / Del — delete to the trash, recoverably.
    Delete,
    /// Shift+F8 / Shift+Del — delete for good.
    DeletePermanently,
    /// Space — mark the row under the cursor, leaving the cursor where it is.
    /// Puts what is marked on the system clipboard, to be copied.
    ClipboardCopy,
    /// The same, to be moved.
    ClipboardCut,
    /// Copies or moves what is on the clipboard into the active pane.
    ClipboardPaste,
    /// Puts the keyboard in the command line.
    ///
    /// The way in, now that a letter searches the pane instead of typing.
    FocusCommandLine,
    ToggleMark,
    /// Insert / Shift+↓ — mark it and step down, so the key can be held.
    ToggleMarkAndAdvance,
    /// Shift+↑ — the same, upwards.
    ToggleMarkAndRetreat,
    /// Shift+Home — mark from the cursor to the first row, and go there.
    ExtendMarkToFirst,
    /// Shift+End — the same, to the last row.
    ExtendMarkToLast,
    /// Shift+PgUp — the same, one page up.
    ExtendMarkPageUp,
    /// Shift+PgDn — the same, one page down.
    ExtendMarkPageDown,
    /// Num + — mark everything matching a pattern.
    MarkByPattern,
    /// Num − — unmark everything matching a pattern.
    UnmarkByPattern,
    /// Num * — swap what is marked for what is not, files only.
    InvertMarks,
    /// Shift+Num * — the same, directories included.
    InvertMarksIncludingFolders,
    /// Alt+Num + — mark every file with the cursor row's extension.
    MarkSameExtension,
    /// Alt+Num − — unmark them.
    UnmarkSameExtension,
    /// Num / — the selection from before the last operation.
    RestoreMarks,
    /// Ctrl+A / Ctrl+Num + — mark everything visible.
    MarkAll,
    /// Ctrl+Num − — unmark everything visible.
    UnmarkAll,
    /// Ctrl+S — narrow the pane as you type.
    QuickFilter,
    /// Escape — stop narrowing.
    ClearFilter,
    /// Ctrl+F3…Ctrl+F6 — sort by a column, or flip it if it is already the
    /// one in force.
    SortBy(SortKey),
    /// Ctrl+↓ / Alt+F8 — the command lines that were run, to pick one from.
    CommandHistory,
    /// Ctrl+Enter — put the name under the cursor into the command line.
    InsertName,
    /// Alt+F1 — offer the left pane a list of places to go.
    /// `Ctrl+D`: the favourite directories, to pick one and go there.
    Favourites,
    /// `Ctrl+B`: every file below this pane, as one flat list.
    BranchView,
    /// `Alt+Shift+Enter`: count what the marked folders hold.
    FolderSizes,
    SelectDriveLeft,
    /// Alt+F2 — the same for the right pane.
    SelectDriveRight,
    /// Ctrl+→ — show the left pane's directory in the right one.
    CloneToRight,
    /// Ctrl+← — the other way round.
    CloneToLeft,
    /// Ctrl+U — exchange the two panes.
    ExchangePanes,
    /// Ctrl+H — show or hide the dot-files.
    ToggleHidden,
    Quit,
}
impl Action {
    /// Every action there is.
    ///
    /// Exists for the test below and says so: nothing the program *does*
    /// needs a list of actions — a key resolves to one and `dispatch` matches
    /// it — so shipping the array would be shipping something with no reader.
    ///
    /// Hand-written, and the one list here that the compiler does not check,
    /// so a new variant has to be added here as well as to the enum. What
    /// makes that worth it is `every_action_can_be_named`, which walks this
    /// list and insists each entry has a name in [`ACTION_NAMES`]: without
    /// it, an action added to the enum and to `dispatch` but to neither table
    /// compiles, runs, and can never be reached or rebound by anyone.
    ///
    /// `SortBy` is spelled out per key, because a sort key is part of the
    /// action rather than an argument to it.
    #[cfg(test)]
    const ALL: [Action; 61] = [
        Action::SwitchPane,
        Action::CursorUp,
        Action::CursorDown,
        Action::CursorFirst,
        Action::CursorLast,
        Action::CursorPageUp,
        Action::CursorPageDown,
        Action::Activate,
        Action::GoParent,
        Action::Copy,
        Action::Pack,
        Action::Move,
        Action::Reread,
        Action::Search,
        Action::MultiRename,
        Action::UndoRename,
        Action::View,
        Action::Edit,
        Action::CreateFile,
        Action::RenameInline,
        Action::CreateDir,
        Action::Delete,
        Action::DeletePermanently,
        Action::ClipboardCopy,
        Action::ClipboardCut,
        Action::ClipboardPaste,
        Action::FocusCommandLine,
        Action::ToggleMark,
        Action::ToggleMarkAndAdvance,
        Action::ToggleMarkAndRetreat,
        Action::ExtendMarkToFirst,
        Action::ExtendMarkToLast,
        Action::ExtendMarkPageUp,
        Action::ExtendMarkPageDown,
        Action::MarkByPattern,
        Action::UnmarkByPattern,
        Action::InvertMarks,
        Action::InvertMarksIncludingFolders,
        Action::MarkSameExtension,
        Action::UnmarkSameExtension,
        Action::RestoreMarks,
        Action::MarkAll,
        Action::UnmarkAll,
        Action::QuickFilter,
        Action::ClearFilter,
        Action::SortBy(SortKey::Name),
        Action::SortBy(SortKey::Ext),
        Action::SortBy(SortKey::Size),
        Action::SortBy(SortKey::Modified),
        Action::CommandHistory,
        Action::InsertName,
        Action::Favourites,
        Action::BranchView,
        Action::FolderSizes,
        Action::SelectDriveLeft,
        Action::SelectDriveRight,
        Action::CloneToRight,
        Action::CloneToLeft,
        Action::ExchangePanes,
        Action::ToggleHidden,
        Action::Quit,
    ];
}

struct Binding {
    key: Key,
    modifiers: ModifierType,
    action: Action,
}

/// No modifier held.
const PLAIN: ModifierType = ModifierType::empty();

/// Modifiers a binding can ask for.
///
/// Everything else GTK reports — Caps Lock, Num Lock, held mouse buttons — is
/// masked out before the lookup. Without that, a user with Caps Lock on would
/// find every key unbound.
const RELEVANT_MODIFIERS: ModifierType = ModifierType::CONTROL_MASK
    .union(ModifierType::SHIFT_MASK)
    .union(ModifierType::ALT_MASK)
    .union(ModifierType::META_MASK);

/// The keymap.
static BINDINGS: &[Binding] = &[
    Binding {
        key: Key::Tab,
        modifiers: PLAIN,
        action: Action::SwitchPane,
    },
    Binding {
        key: Key::Up,
        modifiers: PLAIN,
        action: Action::CursorUp,
    },
    Binding {
        key: Key::Down,
        modifiers: PLAIN,
        action: Action::CursorDown,
    },
    Binding {
        key: Key::Home,
        modifiers: PLAIN,
        action: Action::CursorFirst,
    },
    Binding {
        key: Key::End,
        modifiers: PLAIN,
        action: Action::CursorLast,
    },
    Binding {
        key: Key::Return,
        modifiers: PLAIN,
        action: Action::Activate,
    },
    // The numeric keypad's Enter is a different keysym but the same intent.
    Binding {
        key: Key::KP_Enter,
        modifiers: PLAIN,
        action: Action::Activate,
    },
    Binding {
        key: Key::BackSpace,
        modifiers: PLAIN,
        action: Action::GoParent,
    },
    Binding {
        key: Key::F7,
        modifiers: ModifierType::ALT_MASK,
        action: Action::Search,
    },
    Binding {
        key: Key::m,
        modifiers: ModifierType::CONTROL_MASK,
        action: Action::MultiRename,
    },
    // Total Commander undoes a multi-rename from a button inside the tool.
    // This one closes when it runs, so the undo is a key instead — and the key
    // everybody already knows.
    Binding {
        key: Key::z,
        modifiers: ModifierType::CONTROL_MASK,
        action: Action::UndoRename,
    },
    Binding {
        key: Key::F3,
        modifiers: PLAIN,
        action: Action::View,
    },
    Binding {
        key: Key::F4,
        modifiers: PLAIN,
        action: Action::Edit,
    },
    Binding {
        key: Key::F5,
        modifiers: PLAIN,
        action: Action::Copy,
    },
    // Total Commander's own key for it, and the reason Alt+F5 rather than a
    // letter: it sits beside the copy it is a kind of.
    Binding {
        key: Key::F5,
        modifiers: ModifierType::ALT_MASK,
        action: Action::Pack,
    },
    Binding {
        key: Key::F6,
        modifiers: PLAIN,
        action: Action::Move,
    },
    Binding {
        key: Key::F4,
        modifiers: ModifierType::SHIFT_MASK,
        action: Action::CreateFile,
    },
    Binding {
        key: Key::F6,
        modifiers: ModifierType::SHIFT_MASK,
        action: Action::RenameInline,
    },
    Binding {
        key: Key::F7,
        modifiers: PLAIN,
        action: Action::CreateDir,
    },
    // Two keys for each delete, because Total Commander has both and muscle
    // memory splits evenly between them.
    Binding {
        key: Key::F8,
        modifiers: PLAIN,
        action: Action::Delete,
    },
    Binding {
        key: Key::Delete,
        modifiers: PLAIN,
        action: Action::Delete,
    },
    Binding {
        key: Key::F8,
        modifiers: ModifierType::SHIFT_MASK,
        action: Action::DeletePermanently,
    },
    Binding {
        key: Key::Delete,
        modifiers: ModifierType::SHIFT_MASK,
        action: Action::DeletePermanently,
    },
    Binding {
        key: Key::space,
        modifiers: PLAIN,
        action: Action::ToggleMark,
    },
    Binding {
        key: Key::Insert,
        modifiers: PLAIN,
        action: Action::ToggleMarkAndAdvance,
    },
    // Shift+cursor is Total Commander's other way of marking a run, and it is
    // the same behaviour Insert has: mark the row being *left*, then move. So
    // Shift+↓ is Insert under another name, and reversing direction runs back
    // over a row and takes its mark off again.
    Binding {
        key: Key::Down,
        modifiers: ModifierType::SHIFT_MASK,
        action: Action::ToggleMarkAndAdvance,
    },
    Binding {
        key: Key::Up,
        modifiers: ModifierType::SHIFT_MASK,
        action: Action::ToggleMarkAndRetreat,
    },
    // A jump has no direction to run back over, so these mark a range rather
    // than toggling: "to the end" does not mean "flip everything I passed".
    Binding {
        key: Key::Home,
        modifiers: ModifierType::SHIFT_MASK,
        action: Action::ExtendMarkToFirst,
    },
    Binding {
        key: Key::End,
        modifiers: ModifierType::SHIFT_MASK,
        action: Action::ExtendMarkToLast,
    },
    // The one pair that is bound with Shift and unbound without it: plain
    // paging is the widget's job, because only it knows how tall the viewport
    // is. With Shift the pane measures a page and marks what it crosses.
    // Plain paging was the `ColumnView`'s own until the model started needing
    // to know where the cursor is — type-ahead searches from it, and a widget
    // that moved the selection without saying so left the search behind. The
    // step and the scroll are the widget's own, measured; see `page_step`.
    Binding {
        key: Key::Page_Up,
        modifiers: PLAIN,
        action: Action::CursorPageUp,
    },
    Binding {
        key: Key::Page_Down,
        modifiers: PLAIN,
        action: Action::CursorPageDown,
    },
    Binding {
        key: Key::Page_Up,
        modifiers: ModifierType::SHIFT_MASK,
        action: Action::ExtendMarkPageUp,
    },
    Binding {
        key: Key::Page_Down,
        modifiers: ModifierType::SHIFT_MASK,
        action: Action::ExtendMarkPageDown,
    },
    // The keypad keys Total Commander uses. Their ordinary twins reach the
    // same bindings through `KEYPAD_TWINS` rather than through entries of
    // their own.
    Binding {
        key: Key::KP_Add,
        modifiers: PLAIN,
        action: Action::MarkByPattern,
    },
    Binding {
        key: Key::KP_Subtract,
        modifiers: PLAIN,
        action: Action::UnmarkByPattern,
    },
    Binding {
        key: Key::KP_Multiply,
        modifiers: PLAIN,
        action: Action::InvertMarks,
    },
    Binding {
        key: Key::KP_Multiply,
        modifiers: ModifierType::SHIFT_MASK,
        action: Action::InvertMarksIncludingFolders,
    },
    Binding {
        key: Key::KP_Divide,
        modifiers: PLAIN,
        action: Action::RestoreMarks,
    },
    Binding {
        key: Key::KP_Add,
        modifiers: ModifierType::ALT_MASK,
        action: Action::MarkSameExtension,
    },
    Binding {
        key: Key::KP_Subtract,
        modifiers: ModifierType::ALT_MASK,
        action: Action::UnmarkSameExtension,
    },
    Binding {
        key: Key::KP_Add,
        modifiers: ModifierType::CONTROL_MASK,
        action: Action::MarkAll,
    },
    Binding {
        key: Key::KP_Subtract,
        modifiers: ModifierType::CONTROL_MASK,
        action: Action::UnmarkAll,
    },
    Binding {
        key: Key::a,
        modifiers: ModifierType::CONTROL_MASK,
        action: Action::MarkAll,
    },
    Binding {
        key: Key::d,
        modifiers: ModifierType::CONTROL_MASK,
        action: Action::Favourites,
    },
    Binding {
        key: Key::b,
        modifiers: ModifierType::CONTROL_MASK,
        action: Action::BranchView,
    },
    Binding {
        key: Key::Return,
        modifiers: ModifierType::ALT_MASK.union(ModifierType::SHIFT_MASK),
        action: Action::FolderSizes,
    },
    Binding {
        key: Key::KP_Enter,
        modifiers: ModifierType::ALT_MASK.union(ModifierType::SHIFT_MASK),
        action: Action::FolderSizes,
    },
    Binding {
        key: Key::s,
        modifiers: ModifierType::CONTROL_MASK,
        action: Action::QuickFilter,
    },
    Binding {
        key: Key::Escape,
        modifiers: PLAIN,
        action: Action::ClearFilter,
    },
    // Total Commander's sort keys, in its order: name, extension, date, size.
    Binding {
        key: Key::F3,
        modifiers: ModifierType::CONTROL_MASK,
        action: Action::SortBy(SortKey::Name),
    },
    Binding {
        key: Key::F4,
        modifiers: ModifierType::CONTROL_MASK,
        action: Action::SortBy(SortKey::Ext),
    },
    Binding {
        key: Key::F5,
        modifiers: ModifierType::CONTROL_MASK,
        action: Action::SortBy(SortKey::Modified),
    },
    Binding {
        key: Key::F6,
        modifiers: ModifierType::CONTROL_MASK,
        action: Action::SortBy(SortKey::Size),
    },
    // The one shortcut that makes a command line in a file manager worth
    // having: the name you are looking at, without typing it.
    Binding {
        key: Key::Return,
        modifiers: ModifierType::CONTROL_MASK,
        action: Action::InsertName,
    },
    Binding {
        key: Key::KP_Enter,
        modifiers: ModifierType::CONTROL_MASK,
        action: Action::InsertName,
    },
    // Total Commander's two ways to the same list, and both are muscle memory.
    Binding {
        key: Key::Down,
        modifiers: ModifierType::CONTROL_MASK,
        action: Action::CommandHistory,
    },
    Binding {
        key: Key::F8,
        modifiers: ModifierType::ALT_MASK,
        action: Action::CommandHistory,
    },
    // Absolute, as in Total Commander: the F-key number *is* the pane number,
    // so which pane has the keyboard makes no difference. Unlike Ctrl+arrow
    // below, there is nothing relative for these to be consistent with.
    Binding {
        key: Key::F1,
        modifiers: ModifierType::ALT_MASK,
        action: Action::SelectDriveLeft,
    },
    Binding {
        key: Key::F2,
        modifiers: ModifierType::ALT_MASK,
        action: Action::SelectDriveRight,
    },
    // Relative to the active pane, as in Total Commander: the arrow points at
    // the pane being *written*, so pressing it toward the pane the keyboard is
    // already in does nothing rather than guessing.
    Binding {
        key: Key::Right,
        modifiers: ModifierType::CONTROL_MASK,
        action: Action::CloneToRight,
    },
    // The three every desktop shares. Nothing in this program claimed them,
    // and a file manager that did not answer them would be the odd one out.
    Binding {
        key: Key::c,
        modifiers: ModifierType::CONTROL_MASK,
        action: Action::ClipboardCopy,
    },
    Binding {
        key: Key::x,
        modifiers: ModifierType::CONTROL_MASK,
        action: Action::ClipboardCut,
    },
    Binding {
        key: Key::v,
        modifiers: ModifierType::CONTROL_MASK,
        action: Action::ClipboardPaste,
    },
    // Plain Right, which nothing claimed: a pane has no horizontal movement
    // to spend it on, and the command line needs a way in from the keyboard.
    Binding {
        key: Key::Right,
        modifiers: PLAIN,
        action: Action::FocusCommandLine,
    },
    Binding {
        key: Key::Left,
        modifiers: ModifierType::CONTROL_MASK,
        action: Action::CloneToLeft,
    },
    Binding {
        key: Key::u,
        modifiers: ModifierType::CONTROL_MASK,
        action: Action::ExchangePanes,
    },
    Binding {
        key: Key::r,
        modifiers: ModifierType::CONTROL_MASK,
        action: Action::Reread,
    },
    Binding {
        key: Key::h,
        modifiers: ModifierType::CONTROL_MASK,
        action: Action::ToggleHidden,
    },
    Binding {
        key: Key::q,
        modifiers: ModifierType::CONTROL_MASK,
        action: Action::Quit,
    },
];

/// Ordinary keys that stand in for keypad ones, because not every keyboard
/// has a numeric block.
///
/// An alias table rather than four more bindings: the twins are the *same*
/// commands, and duplicating each of them once per modifier would be eight
/// entries that have to be kept in step with the keypad ones.
const KEYPAD_TWINS: [(Key, Key); 4] = [
    (Key::plus, Key::KP_Add),
    (Key::minus, Key::KP_Subtract),
    (Key::asterisk, Key::KP_Multiply),
    (Key::slash, Key::KP_Divide),
];

/// The names actions are written under in the settings file, and the only
/// place that mapping lives.
///
/// Spelled out rather than derived from the enum, for the reason the sort-key
/// table in `tc-core` is: renaming a variant would otherwise silently change
/// the file format under everybody's existing bindings. A test walks this
/// table against every default binding, so an action nobody named here is
/// caught rather than being quietly unbindable.
const ACTION_NAMES: &[(&str, Action)] = &[
    ("switch_pane", Action::SwitchPane),
    ("cursor_up", Action::CursorUp),
    ("cursor_down", Action::CursorDown),
    ("cursor_first", Action::CursorFirst),
    ("cursor_last", Action::CursorLast),
    ("cursor_page_up", Action::CursorPageUp),
    ("cursor_page_down", Action::CursorPageDown),
    ("activate", Action::Activate),
    ("go_parent", Action::GoParent),
    ("copy", Action::Copy),
    ("pack", Action::Pack),
    ("move", Action::Move),
    ("reread", Action::Reread),
    ("search", Action::Search),
    ("multi_rename", Action::MultiRename),
    ("undo_rename", Action::UndoRename),
    ("view", Action::View),
    ("edit", Action::Edit),
    ("create_file", Action::CreateFile),
    ("rename_inline", Action::RenameInline),
    ("create_dir", Action::CreateDir),
    ("delete", Action::Delete),
    ("delete_permanently", Action::DeletePermanently),
    ("toggle_mark", Action::ToggleMark),
    ("toggle_mark_and_advance", Action::ToggleMarkAndAdvance),
    ("toggle_mark_and_retreat", Action::ToggleMarkAndRetreat),
    ("extend_mark_to_first", Action::ExtendMarkToFirst),
    ("extend_mark_to_last", Action::ExtendMarkToLast),
    ("extend_mark_page_up", Action::ExtendMarkPageUp),
    ("extend_mark_page_down", Action::ExtendMarkPageDown),
    ("mark_by_pattern", Action::MarkByPattern),
    ("unmark_by_pattern", Action::UnmarkByPattern),
    ("invert_marks", Action::InvertMarks),
    (
        "invert_marks_including_folders",
        Action::InvertMarksIncludingFolders,
    ),
    ("mark_same_extension", Action::MarkSameExtension),
    ("unmark_same_extension", Action::UnmarkSameExtension),
    ("restore_marks", Action::RestoreMarks),
    ("mark_all", Action::MarkAll),
    ("unmark_all", Action::UnmarkAll),
    ("quick_filter", Action::QuickFilter),
    ("clear_filter", Action::ClearFilter),
    ("sort_by_name", Action::SortBy(SortKey::Name)),
    ("sort_by_ext", Action::SortBy(SortKey::Ext)),
    ("sort_by_size", Action::SortBy(SortKey::Size)),
    ("sort_by_date", Action::SortBy(SortKey::Modified)),
    ("command_history", Action::CommandHistory),
    ("focus_command_line", Action::FocusCommandLine),
    ("clipboard_copy", Action::ClipboardCopy),
    ("clipboard_cut", Action::ClipboardCut),
    ("clipboard_paste", Action::ClipboardPaste),
    ("insert_name", Action::InsertName),
    ("favourites", Action::Favourites),
    ("branch_view", Action::BranchView),
    ("folder_sizes", Action::FolderSizes),
    ("select_drive_left", Action::SelectDriveLeft),
    ("select_drive_right", Action::SelectDriveRight),
    ("clone_to_right", Action::CloneToRight),
    ("clone_to_left", Action::CloneToLeft),
    ("exchange_panes", Action::ExchangePanes),
    ("toggle_hidden", Action::ToggleHidden),
    ("quit", Action::Quit),
];

/// The macOS layer: what Command means there, laid over the defaults.
///
/// **Data, in exactly the `[keys]` shape** — each entry is what a user could
/// have written, run through the same parser, so shipping it costs no second
/// mechanism. Applied between the defaults and the user's own table, which
/// keeps the precedence obvious: platform under person.
///
/// **Additive and unjudged.** Every `Ctrl` binding keeps working; these add
/// the spellings a Mac hand expects for the commands whose Cmd form is
/// universal there (copy, cut, paste, select all, undo, quit, refresh).
/// Whether more of the `Ctrl` table should *move* to Cmd is a taste question
/// nobody can answer from a Linux box — this is the deliberate minimum, and
/// the groundwork plan lists it among the things a real Mac reviews first.
#[cfg(any(target_os = "macos", test))]
pub(crate) const MACOS_LAYER: [(&str, &str); 7] = [
    ("cmd+c", "clipboard_copy"),
    ("cmd+x", "clipboard_cut"),
    ("cmd+v", "clipboard_paste"),
    ("cmd+a", "mark_all"),
    ("cmd+z", "undo_rename"),
    ("cmd+q", "quit"),
    ("cmd+r", "reread"),
];

/// The defaults, with the user's own bindings laid over them.
///
/// An overlay rather than a replacement: a key nobody mentions keeps what it
/// always did, so a binding added in a later version reaches people who
/// already have a settings file. An entry with an empty action unbinds its
/// key.
///
/// Built once at startup and then only read, so a lookup stays a hash of one
/// key rather than a walk down a table that a configurable keymap would
/// otherwise make arbitrarily long (`docs/performance.md`).
#[derive(Clone)]
pub struct Keymap {
    bindings: HashMap<(Key, ModifierType), Action>,
}

impl Default for Keymap {
    fn default() -> Self {
        // `mut` is for the layer application below it, which only macOS
        // compiles — the attribute says so instead of silencing broadly.
        #[cfg_attr(not(target_os = "macos"), allow(unused_mut))]
        let mut keymap = Keymap {
            bindings: BINDINGS
                .iter()
                .map(|binding| ((binding.key, binding.modifiers), binding.action))
                .collect(),
        };
        // The platform's own layer goes under the user's [keys], and a
        // complaint from it is a programming error rather than user input —
        // a test walks the layer on every platform, so it cannot get here
        // broken without the gate saying so.
        #[cfg(target_os = "macos")]
        for (spec, action) in MACOS_LAYER {
            debug_assert!(keymap.apply(spec, action).is_none(), "{spec}");
        }
        keymap
    }
}

impl Keymap {
    /// Lays the user's bindings over the defaults, reporting the entries it
    /// could not make sense of.
    ///
    /// A bad entry is skipped and named, never fatal: nothing about the
    /// settings may stop the program starting ([`docs/config.md`]), and a
    /// misspelling in one line must not cost the other nineteen.
    pub fn with_overrides<'a>(
        overrides: impl IntoIterator<Item = (&'a String, &'a String)>,
    ) -> (Keymap, Vec<String>) {
        let mut keymap = Keymap::default();
        let mut complaints = Vec::new();
        for (spec, action) in overrides {
            complaints.extend(keymap.apply(spec, action));
        }
        (keymap, complaints)
    }

    /// Lays one binding over what is there, reporting what made no sense.
    ///
    /// One function for the user's `[keys]` lines and the platform layer,
    /// so "what a line may say" cannot drift between the two.
    fn apply(&mut self, spec: &str, action: &str) -> Option<String> {
        let Some(stroke) = parse_key(spec) else {
            return Some(format!("{UNKNOWN_KEY}: {spec}"));
        };
        // An empty action is how a key is taken away, which is not the
        // same as leaving it out — leaving it out keeps the default.
        if action.is_empty() {
            self.bindings.remove(&stroke);
            return None;
        }
        match action_named(action) {
            Some(action) => {
                self.bindings.insert(stroke, action);
                None
            }
            None => Some(format!("{UNKNOWN_ACTION}: {action}")),
        }
    }

    /// The action a keystroke triggers, or `None` when nothing is bound.
    pub fn action_for(&self, key: Key, modifiers: ModifierType) -> Option<Action> {
        let stroke = normalize(key, modifiers & RELEVANT_MODIFIERS);
        self.bindings.get(&stroke).copied()
    }
}

/// The action written under `name`.
fn action_named(name: &str) -> Option<Action> {
    ACTION_NAMES
        .iter()
        .find(|(candidate, _)| *candidate == name)
        .map(|(_, action)| *action)
}

/// Reads `"ctrl+shift+kp_add"` into the keystroke it names.
///
/// Case-insensitive and order-insensitive, because a person writing a settings
/// file by hand should not have to guess either. The key itself is whatever
/// GDK knows by that name, so every keysym is reachable without this file
/// carrying a list of them ([`docs/config.md`]).
fn parse_key(spec: &str) -> Option<(Key, ModifierType)> {
    let mut modifiers = ModifierType::empty();
    let mut key = None;
    for part in spec.split(KEY_SPEC_SEPARATOR) {
        let part = part.trim();
        match MODIFIER_NAMES
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(part))
        {
            Some((_, modifier)) => modifiers |= *modifier,
            // Not a modifier, so it must be the key — and there is only one.
            None if key.is_none() => key = Some(key_named(part)?),
            None => return None,
        }
    }
    Some(normalize(key?, modifiers))
}

/// The key GDK knows by `name`, without insisting on GDK's own capitalisation.
///
/// GDK's keysym names are case-sensitive and follow no rule a person could
/// guess — `space` is lower, `Insert` is capitalised, `F8` is upper and
/// `KP_Add` is all three at once. Making somebody get that right in a
/// hand-written settings file, with a silently dead binding as the penalty, is
/// not a trade worth making, so the spellings GDK uses are tried in turn.
///
/// A single letter is folded to lower case, because upper-case letters are
/// *different* keysyms that only arrive with Shift held: someone writing
/// `ctrl+U` means Ctrl+U, and `shift+` is how Shift is asked for here.
fn key_named(name: &str) -> Option<Key> {
    if name.len() == 1
        && name
            .chars()
            .all(|character| character.is_ascii_alphabetic())
    {
        return Key::from_name(name.to_ascii_lowercase());
    }
    let titled = title_case(name);
    [
        name.to_string(),
        titled.clone(),
        name.to_ascii_uppercase(),
        name.to_ascii_lowercase(),
        // `KP_Add` and its neighbours, the one family with a shouted prefix.
        titled.replacen(KEYPAD_PREFIX_TITLED, KEYPAD_PREFIX, 1),
    ]
    .iter()
    .find_map(Key::from_name)
}

/// `page_down` and `PAGE_DOWN` alike → `Page_Down`: GDK's usual shape for a
/// multi-word key, reached from however the user shouted it.
fn title_case(name: &str) -> String {
    name.split(KEY_NAME_SEPARATOR)
        .map(|word| {
            let mut characters = word.chars();
            match characters.next() {
                Some(first) => {
                    first.to_ascii_uppercase().to_string() + &characters.as_str().to_lowercase()
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(&KEY_NAME_SEPARATOR.to_string())
}

/// Resolves a keypad twin, and drops the Shift that produced it.
///
/// On most layouts `+` is Shift and `*` is Shift too, so the keystroke arrives
/// as `plus` or `asterisk` *with* `SHIFT_MASK` and matched nothing at all —
/// the `+` binding shipped in phase 3 was dead on arrival for that reason.
/// A modifier that was needed to type a character is a fact about the
/// keyboard, not something the user meant, so it does not take part in the
/// lookup.
///
/// The cost is that the ordinary `*` cannot also carry a *deliberate* Shift:
/// `Shift+Num *` (invert including folders) is reachable from the keypad
/// only. That is a limitation of layouts on which `*` cannot be typed without
/// Shift, not a choice — and a plain `*` that does nothing would be worse.
fn normalize(key: Key, modifiers: ModifierType) -> (Key, ModifierType) {
    match KEYPAD_TWINS.iter().find(|(twin, _)| *twin == key) {
        Some((_, keypad)) => (*keypad, modifiers - ModifierType::SHIFT_MASK),
        None => (key, modifiers),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::constants::{
        ACTION_TABLE_BEGIN, ACTION_TABLE_DOC, ACTION_TABLE_END, BINDINGS_TABLE_HEADING,
        DOC_KEY_NAMES, PAIRED_ROW_SEPARATOR, SAME_ACTION_SEPARATOR, UI_HARNESS, UI_KEY_SPELLINGS,
        UI_SUITE, UI_UNPRESSED,
    };

    /// What a keystroke does with nobody's settings laid over the defaults.
    fn bound(key: Key, modifiers: ModifierType) -> Option<Action> {
        Keymap::default().action_for(key, modifiers)
    }

    /// A keymap with one line of settings file laid over it.
    fn overridden(spec: &str, action: &str) -> (Keymap, Vec<String>) {
        let (spec, action) = (spec.to_string(), action.to_string());
        Keymap::with_overrides(std::iter::once((&spec, &action)))
    }

    #[test]
    fn every_documented_binding_resolves() {
        let expected = [
            (Key::Tab, PLAIN, Action::SwitchPane),
            (Key::Up, PLAIN, Action::CursorUp),
            (Key::Down, PLAIN, Action::CursorDown),
            (Key::Home, PLAIN, Action::CursorFirst),
            (Key::End, PLAIN, Action::CursorLast),
            (Key::Page_Up, PLAIN, Action::CursorPageUp),
            (Key::Page_Down, PLAIN, Action::CursorPageDown),
            (Key::Return, PLAIN, Action::Activate),
            (Key::KP_Enter, PLAIN, Action::Activate),
            (Key::BackSpace, PLAIN, Action::GoParent),
            (Key::F7, ModifierType::ALT_MASK, Action::Search),
            (Key::m, ModifierType::CONTROL_MASK, Action::MultiRename),
            (Key::z, ModifierType::CONTROL_MASK, Action::UndoRename),
            (Key::F3, PLAIN, Action::View),
            (Key::F4, PLAIN, Action::Edit),
            (Key::F5, PLAIN, Action::Copy),
            (Key::F5, ModifierType::ALT_MASK, Action::Pack),
            (Key::F6, PLAIN, Action::Move),
            (Key::F4, ModifierType::SHIFT_MASK, Action::CreateFile),
            (Key::F6, ModifierType::SHIFT_MASK, Action::RenameInline),
            (Key::F7, PLAIN, Action::CreateDir),
            (Key::F8, PLAIN, Action::Delete),
            (Key::Delete, PLAIN, Action::Delete),
            (Key::F8, ModifierType::SHIFT_MASK, Action::DeletePermanently),
            (
                Key::Delete,
                ModifierType::SHIFT_MASK,
                Action::DeletePermanently,
            ),
            (Key::space, PLAIN, Action::ToggleMark),
            (Key::Insert, PLAIN, Action::ToggleMarkAndAdvance),
            (
                Key::Down,
                ModifierType::SHIFT_MASK,
                Action::ToggleMarkAndAdvance,
            ),
            (
                Key::Up,
                ModifierType::SHIFT_MASK,
                Action::ToggleMarkAndRetreat,
            ),
            (
                Key::Home,
                ModifierType::SHIFT_MASK,
                Action::ExtendMarkToFirst,
            ),
            (Key::End, ModifierType::SHIFT_MASK, Action::ExtendMarkToLast),
            (
                Key::Page_Up,
                ModifierType::SHIFT_MASK,
                Action::ExtendMarkPageUp,
            ),
            (
                Key::Page_Down,
                ModifierType::SHIFT_MASK,
                Action::ExtendMarkPageDown,
            ),
            (Key::KP_Add, PLAIN, Action::MarkByPattern),
            (Key::KP_Subtract, PLAIN, Action::UnmarkByPattern),
            (Key::KP_Multiply, PLAIN, Action::InvertMarks),
            (
                Key::KP_Multiply,
                ModifierType::SHIFT_MASK,
                Action::InvertMarksIncludingFolders,
            ),
            (Key::KP_Divide, PLAIN, Action::RestoreMarks),
            (
                Key::KP_Add,
                ModifierType::ALT_MASK,
                Action::MarkSameExtension,
            ),
            (
                Key::KP_Subtract,
                ModifierType::ALT_MASK,
                Action::UnmarkSameExtension,
            ),
            (Key::KP_Add, ModifierType::CONTROL_MASK, Action::MarkAll),
            (
                Key::KP_Subtract,
                ModifierType::CONTROL_MASK,
                Action::UnmarkAll,
            ),
            (Key::a, ModifierType::CONTROL_MASK, Action::MarkAll),
            (Key::d, ModifierType::CONTROL_MASK, Action::Favourites),
            (Key::b, ModifierType::CONTROL_MASK, Action::BranchView),
            (
                Key::Return,
                ModifierType::ALT_MASK.union(ModifierType::SHIFT_MASK),
                Action::FolderSizes,
            ),
            (Key::s, ModifierType::CONTROL_MASK, Action::QuickFilter),
            (Key::Escape, PLAIN, Action::ClearFilter),
            (
                Key::F3,
                ModifierType::CONTROL_MASK,
                Action::SortBy(SortKey::Name),
            ),
            (
                Key::F4,
                ModifierType::CONTROL_MASK,
                Action::SortBy(SortKey::Ext),
            ),
            (
                Key::F5,
                ModifierType::CONTROL_MASK,
                Action::SortBy(SortKey::Modified),
            ),
            (
                Key::F6,
                ModifierType::CONTROL_MASK,
                Action::SortBy(SortKey::Size),
            ),
            (
                Key::Down,
                ModifierType::CONTROL_MASK,
                Action::CommandHistory,
            ),
            (Key::F8, ModifierType::ALT_MASK, Action::CommandHistory),
            (Key::Return, ModifierType::CONTROL_MASK, Action::InsertName),
            (
                Key::KP_Enter,
                ModifierType::CONTROL_MASK,
                Action::InsertName,
            ),
            (Key::F1, ModifierType::ALT_MASK, Action::SelectDriveLeft),
            (Key::F2, ModifierType::ALT_MASK, Action::SelectDriveRight),
            (Key::Right, ModifierType::CONTROL_MASK, Action::CloneToRight),
            (Key::Left, ModifierType::CONTROL_MASK, Action::CloneToLeft),
            (Key::u, ModifierType::CONTROL_MASK, Action::ExchangePanes),
            (Key::r, ModifierType::CONTROL_MASK, Action::Reread),
            (Key::h, ModifierType::CONTROL_MASK, Action::ToggleHidden),
            (Key::q, ModifierType::CONTROL_MASK, Action::Quit),
            (Key::Right, PLAIN, Action::FocusCommandLine),
            (Key::c, ModifierType::CONTROL_MASK, Action::ClipboardCopy),
            (Key::x, ModifierType::CONTROL_MASK, Action::ClipboardCut),
            (Key::v, ModifierType::CONTROL_MASK, Action::ClipboardPaste),
            (
                Key::KP_Enter,
                ModifierType::SHIFT_MASK.union(ModifierType::ALT_MASK),
                Action::FolderSizes,
            ),
        ];
        // The list is written out rather than read from `BINDINGS`, because a
        // test that takes its expectations from the code it checks can only
        // agree with it. The count is the other half of that bargain: without
        // it, a binding added to `BINDINGS` and forgotten here is checked by
        // nothing and nothing says so — which is exactly what happened to the
        // five entries above, all added in one week and none noticed.
        assert_eq!(
            expected.len(),
            BINDINGS.len(),
            "every binding is expected here, and only bindings are",
        );
        for (key, modifiers, action) in expected {
            assert_eq!(bound(key, modifiers), Some(action), "{key:?}");
        }
    }

    #[test]
    fn every_action_can_be_named() {
        // The gap the other two tables leave between them: one walks
        // BINDINGS and one walks ACTION_NAMES, so an action in *neither* —
        // added to the enum and to `dispatch`, and nowhere else — passes both
        // while being unreachable and unnameable. This one walks the actions.
        for action in Action::ALL {
            assert!(
                ACTION_NAMES.iter().any(|(_, named)| *named == action),
                "{action:?} has no name in ACTION_NAMES, so nobody can bind it"
            );
        }
    }

    #[test]
    fn the_action_list_names_each_action_once() {
        // `ALL` is hand-written, so it can go wrong in the other direction
        // too: a variant listed twice would make the walk above look thorough
        // while covering one fewer action than it appears to.
        let mut seen = Vec::new();
        for action in Action::ALL {
            assert!(!seen.contains(&action), "{action:?} is in ALL twice");
            seen.push(action);
        }
        assert_eq!(
            seen.len(),
            ACTION_NAMES.len(),
            "ALL and ACTION_NAMES disagree about how many actions there are"
        );
    }

    #[test]
    fn every_default_binding_has_a_name_to_write_it_under() {
        // Walking the bindings rather than listing the actions is what makes
        // this exhaustive: an action that reaches a key but has no name is
        // one the user can see working and cannot rebind, which is worse than
        // one that does not exist.
        for binding in BINDINGS {
            assert!(
                ACTION_NAMES
                    .iter()
                    .any(|(_, action)| *action == binding.action),
                "{:?} has no name in ACTION_NAMES",
                binding.action
            );
        }
    }

    #[test]
    fn every_name_reaches_the_action_it_spells() {
        // The other direction: a name that resolves to the wrong action, or
        // two names for one action, would both pass the walk above.
        for (name, action) in ACTION_NAMES {
            assert_eq!(action_named(name), Some(*action), "{name}");
        }
        let mut names: Vec<&str> = ACTION_NAMES.iter().map(|(name, _)| *name).collect();
        names.sort_unstable();
        let count = names.len();
        names.dedup();
        assert_eq!(names.len(), count, "two actions share a name");
    }

    #[test]
    fn a_users_binding_wins_over_the_default() {
        let (keymap, complaints) = overridden("ctrl+e", "exchange_panes");

        assert!(complaints.is_empty(), "{complaints:?}");
        assert_eq!(
            keymap.action_for(Key::e, ModifierType::CONTROL_MASK),
            Some(Action::ExchangePanes)
        );
        // And Ctrl+U still means it too: an override adds a way in, it does
        // not take the old one away.
        assert_eq!(
            keymap.action_for(Key::u, ModifierType::CONTROL_MASK),
            Some(Action::ExchangePanes)
        );
    }

    #[test]
    fn a_key_nobody_mentions_keeps_its_default() {
        // The whole point of an overlay: a binding added in a later version
        // reaches people who already have a settings file.
        let (keymap, _) = overridden("ctrl+e", "exchange_panes");

        assert_eq!(keymap.action_for(Key::F5, PLAIN), Some(Action::Copy));
        assert_eq!(keymap.action_for(Key::Tab, PLAIN), Some(Action::SwitchPane));
    }

    #[test]
    fn an_empty_action_takes_a_key_away() {
        // Not the same as leaving the line out, which keeps the default.
        let (keymap, complaints) = overridden("f8", "");

        assert!(complaints.is_empty(), "{complaints:?}");
        assert_eq!(keymap.action_for(Key::F8, PLAIN), None);
        // Delete is a separate binding for the same action and is untouched.
        assert_eq!(keymap.action_for(Key::Delete, PLAIN), Some(Action::Delete));
    }

    /// Every action name, with the keys bound to it by default.
    ///
    /// Generated from [`ACTION_NAMES`] and [`BINDINGS`] rather than written down,
    /// because a list of names kept by hand beside the table it describes is a
    /// list that drifts — which is exactly what happened to
    /// `docs/keymap.md` before this existed (skill 53). The test below renders
    /// it and fails when the document and the code disagree.
    ///
    /// An action with no default binding still appears, with an empty key list:
    /// it is bindable, which is the question this answers.
    fn action_catalogue() -> Vec<(&'static str, Vec<String>)> {
        ACTION_NAMES
            .iter()
            .map(|(name, action)| {
                let keys = BINDINGS
                    .iter()
                    .filter(|binding| binding.action == *action)
                    .map(|binding| key_spec(binding.key, binding.modifiers))
                    .collect();
                (*name, keys)
            })
            .collect()
    }

    /// A keystroke written the way the `[keys]` table spells one.
    ///
    /// The inverse of [`parse_key`], and only meaningful because it is: GDK's
    /// own keysym name is the first spelling `key_named` tries, so what this
    /// writes is always something that reads back. The test below asserts that
    /// round trip over every default binding rather than trusting this
    /// sentence.
    fn key_spec(key: Key, modifiers: ModifierType) -> String {
        let mut spec = String::new();
        for (name, modifier) in MODIFIER_NAMES {
            if modifiers.contains(modifier) {
                spec.push_str(name);
                spec.push(KEY_SPEC_SEPARATOR);
            }
        }
        // Every key in `BINDINGS` is a GDK keysym constant, so it has a name.
        spec.push_str(&key.name().expect("a keysym from BINDINGS has a name"));
        spec
    }

    /// The bindings table as `docs/keymap.md` holds it, one entry per **group**
    /// of keys the table says are the same command.
    ///
    /// A row's first cell is split on [`PAIRED_ROW_SEPARATOR`] — the slash that
    /// means "this row is about two things" — and what is left in each part is
    /// the keys joined by [`SAME_ACTION_SEPARATOR`], which the table uses for
    /// one command reachable more than one way. So `` `↑` / `↓` `` is two
    /// groups of one and `` `F8`, `Delete` `` is one group of two, read out of
    /// the punctuation rather than out of a list kept here.
    ///
    /// A part with no backticked key is prose — "a letter, digit or symbol" is
    /// type-ahead, which is what happens when *no* binding matches and so has
    /// none to check.
    fn documented_groups() -> Vec<Vec<(String, (Key, ModifierType))>> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(ACTION_TABLE_DOC);
        let document = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        let section = document
            .split_once(BINDINGS_TABLE_HEADING)
            .expect("docs/keymap.md has lost its bindings heading")
            .1;

        let mut groups = Vec::new();
        for line in section.lines().skip_while(|line| !line.starts_with('|')) {
            let Some(rest) = line.strip_prefix('|') else {
                // The table ends at the first line that is not one.
                break;
            };
            let cell = rest.split('|').next().expect("a row has a first cell");
            for part in cell.split(PAIRED_ROW_SEPARATOR) {
                let group: Vec<_> = part
                    .split('`')
                    // A backticked token sits at an odd index once the part is
                    // split on the backtick, which is what tells a key from the
                    // prose around it.
                    .skip(1)
                    .step_by(2)
                    .map(|key| {
                        let parsed = parse_doc_key(key)
                            .unwrap_or_else(|| panic!("docs/keymap.md: `{key}` names no key"));
                        (key.to_string(), parsed)
                    })
                    .collect();
                if !group.is_empty() {
                    groups.push(group);
                }
            }
        }
        assert!(!groups.is_empty(), "no bindings table found");
        groups
    }

    /// A keystroke written the way the table writes one: `Alt+Num +`, `Ctrl+↓`.
    ///
    /// The irregular spellings are [`DOC_KEY_NAMES`], longest first because
    /// `Num Enter` ends with `Enter`; everything else goes to `key_named`,
    /// which is the same function a `[keys]` line goes through.
    fn parse_doc_key(spec: &str) -> Option<(Key, ModifierType)> {
        let (prefix, key) = DOC_KEY_NAMES
            .iter()
            .filter(|(name, _)| spec.ends_with(name))
            .max_by_key(|(name, _)| name.len())
            .map(|(name, key)| (&spec[..spec.len() - name.len()], *key))
            .or_else(|| match spec.rsplit_once(KEY_SPEC_SEPARATOR) {
                Some((prefix, name)) => Some((&spec[..prefix.len() + 1], key_named(name)?)),
                None => Some(("", key_named(spec)?)),
            })?;

        let mut modifiers = ModifierType::empty();
        for part in prefix
            .split(KEY_SPEC_SEPARATOR)
            .filter(|part| !part.is_empty())
        {
            let (_, modifier) = MODIFIER_NAMES
                .iter()
                .find(|(name, _)| name.eq_ignore_ascii_case(part))?;
            modifiers |= *modifier;
        }
        Some(normalize(key, modifiers))
    }

    /// `docs/keymap.md`'s table names exactly the keys that are bound.
    ///
    /// The table stays prose — what `Insert` *means* is not in the code — so
    /// what is checked is the half that is: every binding appears, and nothing
    /// appears that is not a binding. That is the drift the 2026-08-30 review
    /// found and could only find by reading.
    #[test]
    fn the_bindings_table_names_exactly_the_keys_that_are_bound() {
        // A keystroke is `Hash` but not `Ord` — the keymap's own lookup is a
        // hash map for the same reason — so the sets are hashed and only the
        // rendered difference is sorted, which is what makes a failure read
        // the same twice.
        let documented: HashSet<_> = documented_groups()
            .into_iter()
            .flatten()
            .map(|(_, stroke)| stroke)
            .collect();
        let bound: HashSet<_> = BINDINGS
            .iter()
            .map(|binding| normalize(binding.key, binding.modifiers))
            .collect();

        let mut undocumented: Vec<_> = bound
            .difference(&documented)
            .map(|(key, modifiers)| key_spec(*key, *modifiers))
            .collect();
        let mut invented: Vec<_> = documented
            .difference(&bound)
            .map(|(key, modifiers)| key_spec(*key, *modifiers))
            .collect();
        undocumented.sort();
        invented.sort();

        assert!(
            undocumented.is_empty() && invented.is_empty(),
            "docs/keymap.md and BINDINGS disagree.\n  \
             bound but not in the table: {undocumented:?}\n  \
             in the table but not bound: {invented:?}",
        );
    }

    /// Keys a row joins with a comma are one command; a slash is a pair.
    ///
    /// The table already used the two that way everywhere, so the punctuation
    /// is read rather than a list of exceptions kept beside it. It catches the
    /// row that quietly stops being true: rebind `Delete` and
    /// `` `F8`, `Delete` `` still reads as one command reachable two ways.
    #[test]
    fn keys_a_row_joins_with_a_comma_are_the_same_command() {
        let keymap = Keymap::default();
        for group in documented_groups() {
            let actions: Vec<_> = group
                .iter()
                .map(|(spelling, (key, modifiers))| {
                    let action = keymap
                        .action_for(*key, *modifiers)
                        .expect("every documented key is bound");
                    (spelling, action)
                })
                .collect();
            let (first, rest) = actions.split_first().expect("a group holds a key");
            for (spelling, action) in rest {
                assert_eq!(
                    *action, first.1,
                    "`{}` and `{spelling}` are joined by `{SAME_ACTION_SEPARATOR}` in \
                     docs/keymap.md, which says they are one command — a row about two \
                     separates them with `{PAIRED_ROW_SEPARATOR}`",
                    first.0,
                );
            }
        }
    }

    /// Every default binding is pressed by the end-to-end suite, or excused.
    ///
    /// The suite drives the real binary with real key events and is where
    /// every bug in this project has lived, so which keys it actually presses
    /// is a fact worth holding rather than describing. Before this, the answer
    /// lived in a paragraph in `keymap.md` — and the paragraph had been wrong
    /// since `Num −` arrived, which nobody noticed because its twin `Num +`
    /// has a test and nothing was counting.
    ///
    /// **Pressed is not the same as proven.** This says a key reaches the
    /// program in some test, not that its own behaviour is asserted there —
    /// `Tab` is pressed by nearly every test just to get somewhere. It is the
    /// half that can be checked mechanically, and the half whose absence is
    /// silent.
    #[test]
    fn every_binding_is_pressed_end_to_end_or_says_why_not() {
        let crate_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let tests: String = [UI_SUITE, UI_HARNESS]
            .iter()
            .map(|name| {
                let path = crate_root.join(name);
                std::fs::read_to_string(&path)
                    .unwrap_or_else(|error| panic!("{}: {error}", path.display()))
            })
            .collect();

        let excused: HashMap<_, _> = UI_UNPRESSED.iter().copied().collect();
        let mut unpressed = Vec::new();
        let mut excused_but_pressed = Vec::new();
        for binding in BINDINGS {
            let spec = key_spec(binding.key, binding.modifiers);
            match (
                pressed_somewhere(&tests, binding),
                excused.contains_key(spec.as_str()),
            ) {
                (false, false) => unpressed.push(spec),
                (true, true) => excused_but_pressed.push(spec),
                _ => {}
            }
        }
        unpressed.sort();
        excused_but_pressed.sort();

        assert!(
            unpressed.is_empty(),
            "no end-to-end test presses these, and UI_UNPRESSED does not excuse them: \
             {unpressed:?}",
        );
        // The other direction, so the list cannot quietly outlive its reasons:
        // a key that has since been given a test does not stay excused.
        assert!(
            excused_but_pressed.is_empty(),
            "UI_UNPRESSED excuses keys the suite does press — drop them from it: \
             {excused_but_pressed:?}",
        );
    }

    /// Whether the suite's text presses this binding, by any of its spellings.
    ///
    /// Modifiers are tried in every order, because `xdotool` takes them in
    /// whichever one the test author wrote.
    fn pressed_somewhere(tests: &str, binding: &Binding) -> bool {
        let names = std::iter::once(
            binding
                .key
                .name()
                .expect("a keysym from BINDINGS has a name")
                .to_string(),
        )
        .chain(
            UI_KEY_SPELLINGS
                .iter()
                .filter(|(key, _)| *key == binding.key)
                .map(|(_, spelling)| (*spelling).to_string()),
        );
        let present: Vec<_> = MODIFIER_NAMES
            .iter()
            .filter(|(_, modifier)| binding.modifiers.contains(*modifier))
            .map(|(name, _)| *name)
            .collect();

        names.into_iter().any(|name| {
            orderings(&present)
                .into_iter()
                .any(|order| tests.contains(&format!("\"{}\"", order.join("+") + "+" + &name)))
                // No modifiers: the key stands on its own.
                || (present.is_empty() && tests.contains(&format!("\"{name}\"")))
        })
    }

    /// Every ordering of the modifiers, since a spec may be written either way.
    fn orderings(present: &[&'static str]) -> Vec<Vec<&'static str>> {
        match present {
            [] => Vec::new(),
            [only] => vec![vec![*only]],
            [a, b] => vec![vec![*a, *b], vec![*b, *a]],
            _ => {
                let mut all = Vec::new();
                for (index, first) in present.iter().enumerate() {
                    let mut rest: Vec<_> = present.to_vec();
                    rest.remove(index);
                    for tail in orderings(&rest) {
                        let mut one = vec![*first];
                        one.extend(tail);
                        all.push(one);
                    }
                }
                all
            }
        }
    }

    /// The generated key specs are specs this file can read back.
    ///
    /// [`key_spec`] is only worth anything as the inverse of [`parse_key`],
    /// and "GDK's own name is the first spelling tried" is a claim about
    /// `key_named`'s list rather than a guarantee. So every default binding
    /// goes out as text and comes back as a keystroke, and the two must be the
    /// same one — a round trip rather than a table of expected spellings,
    /// which would only agree with whatever the code did on the day.
    #[test]
    fn every_generated_key_spec_reads_back_as_the_key_it_names() {
        for binding in BINDINGS {
            let spec = key_spec(binding.key, binding.modifiers);
            assert_eq!(
                parse_key(&spec),
                Some(normalize(binding.key, binding.modifiers)),
                "{spec} did not read back",
            );
        }
    }

    /// `docs/keymap.md`'s action-name table is the one this code generates.
    ///
    /// The table is every name the user can rebind, and it was missing from
    /// the documentation entirely — somebody wanting a different key had to
    /// read this file. Writing it out by hand would have made it the
    /// project's next stale table, so it is generated and this test is what
    /// makes "generated" true: the block between the markers in the document
    /// must be exactly what [`action_catalogue`] renders.
    ///
    /// On a mismatch the correct block is printed, so the fix is a paste
    /// rather than a hunt.
    #[test]
    fn the_action_name_table_in_the_docs_is_the_one_the_code_generates() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(ACTION_TABLE_DOC);
        let document = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));

        let rendered = render_action_table();
        let (before, rest) = document
            .split_once(ACTION_TABLE_BEGIN)
            .expect("docs/keymap.md has lost its generated-table marker");
        let (found, _) = rest
            .split_once(ACTION_TABLE_END)
            .expect("docs/keymap.md has lost its end marker");
        let _ = before;

        assert_eq!(
            found.trim(),
            rendered.trim(),
            "\n\ndocs/keymap.md's action table is out of date. Replace the \
             block between the markers with:\n\n{ACTION_TABLE_BEGIN}\n\
             {rendered}\n{ACTION_TABLE_END}\n",
        );
    }

    /// The action table as `docs/keymap.md` carries it.
    fn render_action_table() -> String {
        let mut table = String::from("| Action name | Default keys |\n|---|---|\n");
        for (name, keys) in action_catalogue() {
            let keys = match keys.is_empty() {
                // An action nothing binds by default is still bindable, which
                // is the question this table answers.
                true => String::from("—"),
                false => keys
                    .iter()
                    .map(|key| format!("`{key}`"))
                    .collect::<Vec<_>>()
                    .join(", "),
            };
            table.push_str(&format!("| `{name}` | {keys} |\n"));
        }
        table
    }

    /// A keymap with the macOS layer applied, built the way a Mac builds it.
    ///
    /// On this box `Default` does not apply the layer, so the test applies it
    /// the same way `Default` does there — through the same `apply` — which
    /// is what makes the layer reviewable without a Mac.
    fn with_macos_layer() -> Keymap {
        let mut keymap = Keymap::default();
        for (spec, action) in MACOS_LAYER {
            assert!(
                keymap.apply(spec, action).is_none(),
                "the shipped layer must parse: {spec} = {action}"
            );
        }
        keymap
    }

    #[test]
    fn the_macos_layer_adds_cmd_without_taking_ctrl() {
        // Additive is the deliberate minimum: a Mac hand gets Cmd+C, and
        // everything the Linux table promised still works. The layer moving
        // beyond additive is a decision for somebody at a real Mac.
        let keymap = with_macos_layer();
        for (key, action) in [
            (Key::c, Action::ClipboardCopy),
            (Key::x, Action::ClipboardCut),
            (Key::v, Action::ClipboardPaste),
            (Key::a, Action::MarkAll),
            (Key::z, Action::UndoRename),
            (Key::q, Action::Quit),
            (Key::r, Action::Reread),
        ] {
            assert_eq!(
                keymap.action_for(key, ModifierType::META_MASK),
                Some(action),
                "{key:?} under Cmd"
            );
            assert_eq!(
                keymap.action_for(key, ModifierType::CONTROL_MASK),
                Some(action),
                "{key:?} under Ctrl still"
            );
        }
    }

    #[test]
    fn a_users_binding_wins_over_the_macos_layer() {
        // Platform under person: the layer is applied before the [keys]
        // table, so somebody who rebinds cmd+q gets their binding, exactly as
        // they would over a plain default.
        let mut keymap = with_macos_layer();
        assert!(keymap.apply("cmd+q", "reread").is_none());
        assert_eq!(
            keymap.action_for(Key::q, ModifierType::META_MASK),
            Some(Action::Reread)
        );
        // And an empty action takes a layer key away like any other.
        assert!(keymap.apply("cmd+r", "").is_none());
        assert_eq!(keymap.action_for(Key::r, ModifierType::META_MASK), None);
    }

    #[test]
    fn cmd_is_a_modifier_a_binding_can_carry() {
        // The Command key, as a macOS keymap layer needs it. Before META
        // joined RELEVANT_MODIFIERS this could parse and still never fire —
        // the mask stripped it ahead of the lookup — so the assert on
        // *firing* is the half that matters.
        let (keymap, complaints) = overridden("cmd+j", "quit");
        assert!(complaints.is_empty(), "{complaints:?}");
        assert_eq!(
            keymap.action_for(Key::j, ModifierType::META_MASK),
            Some(Action::Quit),
            "a cmd binding fires under META"
        );
        assert_eq!(
            keymap.action_for(Key::j, ModifierType::CONTROL_MASK),
            None,
            "and not under Ctrl — cmd is not a spelling of ctrl"
        );
        assert_eq!(
            keymap.action_for(Key::j, PLAIN),
            None,
            "nor bare — the wrong-modifier rule holds for META too"
        );
    }

    #[test]
    fn meta_on_an_unclaimed_key_is_still_irrelevant_noise() {
        // META joining the relevant set must not break the other direction:
        // a plain binding still fires when META arrives uninvited only if
        // something *made* META relevant to that stroke — it did not, so the
        // stroke is a different one and nothing fires. That is the same rule
        // Ctrl already follows ("a bound key with the wrong modifier does
        // nothing"), asserted here for the mask that just joined.
        assert_eq!(bound(Key::F5, ModifierType::META_MASK), None);
    }

    #[test]
    fn a_key_name_reads_the_same_in_any_case_or_order() {
        let (canonical, _) = overridden("ctrl+shift+f5", "quit");
        for spelling in ["CTRL+SHIFT+F5", "shift+ctrl+F5", "Ctrl + Shift + F5"] {
            let (keymap, complaints) = overridden(spelling, "quit");
            assert!(complaints.is_empty(), "{spelling}: {complaints:?}");
            assert_eq!(
                keymap.action_for(
                    Key::F5,
                    ModifierType::CONTROL_MASK | ModifierType::SHIFT_MASK
                ),
                canonical.action_for(
                    Key::F5,
                    ModifierType::CONTROL_MASK | ModifierType::SHIFT_MASK
                ),
                "{spelling}"
            );
        }
    }

    #[test]
    fn a_key_name_does_not_have_to_match_gdks_capitalisation() {
        // GDK's names follow no rule a person could guess: `space` is lower,
        // `Insert` is capitalised, `F8` is upper and `KP_Add` is all three at
        // once. A settings file that silently dropped a binding over that
        // would be a trap.
        for (written, key) in [
            ("f8", Key::F8),
            ("F8", Key::F8),
            ("insert", Key::Insert),
            ("page_down", Key::Page_Down),
            ("PAGE_DOWN", Key::Page_Down),
            ("kp_add", Key::KP_Add),
            ("KP_Add", Key::KP_Add),
            ("SPACE", Key::space),
            ("space", Key::space),
        ] {
            assert_eq!(key_named(written), Some(key), "{written}");
        }
    }

    #[test]
    fn a_letter_key_is_written_lower_case_and_shift_is_asked_for_by_name() {
        // Upper-case letters are *different* keysyms that only arrive with
        // Shift held, so `ctrl+U` taken literally would bind a stroke that
        // never comes. Someone writing it means Ctrl+U.
        let (keymap, complaints) = overridden("ctrl+U", "quit");

        assert!(complaints.is_empty(), "{complaints:?}");
        assert_eq!(
            keymap.action_for(Key::u, ModifierType::CONTROL_MASK),
            Some(Action::Quit)
        );
    }

    #[test]
    fn a_binding_nobody_can_make_sense_of_is_named_and_skipped() {
        // Nothing about the settings may stop the program starting, and a
        // misspelling in one line must not cost the other nineteen.
        for (spec, action) in [("no_such_key", "quit"), ("ctrl+e", "no_such_action")] {
            let (keymap, complaints) = overridden(spec, action);
            assert_eq!(complaints.len(), 1, "{spec} = {action}");
            assert_eq!(
                keymap.action_for(Key::F5, PLAIN),
                Some(Action::Copy),
                "the rest of the keymap went with it"
            );
        }
    }

    #[test]
    fn a_user_can_bind_an_ordinary_key_to_a_keypad_command() {
        // Key specs go through the same twin resolution a real keystroke
        // does, so a settings file may say `*` and mean Num *. Without that,
        // a binding written the obvious way would be unreachable on every
        // layout where `*` needs Shift.
        let (keymap, complaints) = overridden("asterisk", "quit");

        assert!(complaints.is_empty(), "{complaints:?}");
        assert_eq!(
            keymap.action_for(Key::KP_Multiply, PLAIN),
            Some(Action::Quit)
        );
    }

    #[test]
    fn an_unbound_key_triggers_nothing() {
        // F5 and Delete left this list in phase 2, and Insert and Escape in
        // phase 3, each when it was bound. The contract is unchanged; only
        // the witnesses are. Plain `a` and `s` are still unbound — only their
        // Ctrl forms mean anything.
        for key in [Key::a, Key::s, Key::F9, Key::F12] {
            assert_eq!(bound(key, PLAIN), None, "{key:?}");
        }
    }

    #[test]
    fn ctrl_turns_an_operation_key_into_a_sort_key() {
        // F5 copies and Ctrl+F5 sorts by date. A binding that ignored its
        // modifiers would make one of those impossible.
        assert_eq!(bound(Key::F5, PLAIN), Some(Action::Copy));
        assert_eq!(
            bound(Key::F5, ModifierType::CONTROL_MASK),
            Some(Action::SortBy(SortKey::Modified))
        );
        assert_eq!(bound(Key::F6, PLAIN), Some(Action::Move));
        assert_eq!(
            bound(Key::F6, ModifierType::CONTROL_MASK),
            Some(Action::SortBy(SortKey::Size))
        );
    }

    #[test]
    fn f4_means_three_things_by_its_modifier() {
        // Edit, create-and-edit, sort by extension. F3 and F4 stopped being
        // unbound in phase 4, which is also why the modifier tests name them.
        assert_eq!(bound(Key::F4, PLAIN), Some(Action::Edit));
        assert_eq!(
            bound(Key::F4, ModifierType::SHIFT_MASK),
            Some(Action::CreateFile)
        );
        assert_eq!(
            bound(Key::F4, ModifierType::CONTROL_MASK),
            Some(Action::SortBy(SortKey::Ext))
        );
    }

    #[test]
    fn f6_means_three_things_by_its_modifier() {
        // Move, rename in place, sort by size. The busiest key here, and a
        // lookup ignoring modifiers would collapse all three.
        assert_eq!(bound(Key::F6, PLAIN), Some(Action::Move));
        assert_eq!(
            bound(Key::F6, ModifierType::SHIFT_MASK),
            Some(Action::RenameInline)
        );
        assert_eq!(
            bound(Key::F6, ModifierType::CONTROL_MASK),
            Some(Action::SortBy(SortKey::Size))
        );
    }

    #[test]
    fn shift_tells_a_recoverable_delete_from_a_final_one() {
        // The one place in the keymap where a modifier changes what survives,
        // so it gets its own test rather than riding on the table above.
        for key in [Key::F8, Key::Delete] {
            assert_eq!(bound(key, PLAIN), Some(Action::Delete), "{key:?}");
            assert_eq!(
                bound(key, ModifierType::SHIFT_MASK),
                Some(Action::DeletePermanently),
                "{key:?}"
            );
        }
    }

    #[test]
    fn an_ordinary_key_reaches_its_keypad_twin_shifted_or_not() {
        // `+` and `*` need Shift on most layouts, so they arrive carrying one.
        // A modifier that was needed to type a character is a fact about the
        // keyboard, not something the user meant — and the phase-3 `+` binding
        // was dead on arrival for exactly this reason.
        for (twin, keypad) in KEYPAD_TWINS {
            let expected = bound(keypad, PLAIN);
            assert!(expected.is_some(), "{keypad:?} is not bound at all");
            for modifiers in [PLAIN, ModifierType::SHIFT_MASK] {
                assert_eq!(bound(twin, modifiers), expected, "{twin:?}");
            }
        }
    }

    #[test]
    fn a_twin_carries_the_modifiers_that_were_meant() {
        // Only Shift is dropped. Ctrl and Alt on an ordinary `+` still reach
        // the keypad meanings, which is what makes a keyboard without a
        // numeric block able to run every marking command.
        assert_eq!(
            bound(Key::plus, ModifierType::CONTROL_MASK),
            Some(Action::MarkAll)
        );
        assert_eq!(
            bound(Key::minus, ModifierType::ALT_MASK),
            Some(Action::UnmarkSameExtension)
        );
    }

    #[test]
    fn one_keypad_key_means_four_things_by_its_modifier() {
        // Num ± carry the most meanings of any key here, and each modifier
        // has to reach its own. A lookup that ignored modifiers would collapse
        // all four into whichever came first in the table.
        for (modifiers, add, subtract) in [
            (PLAIN, Action::MarkByPattern, Action::UnmarkByPattern),
            (
                ModifierType::ALT_MASK,
                Action::MarkSameExtension,
                Action::UnmarkSameExtension,
            ),
            (
                ModifierType::CONTROL_MASK,
                Action::MarkAll,
                Action::UnmarkAll,
            ),
        ] {
            assert_eq!(bound(Key::KP_Add, modifiers), Some(add), "{modifiers:?}");
            assert_eq!(
                bound(Key::KP_Subtract, modifiers),
                Some(subtract),
                "{modifiers:?}"
            );
        }
    }

    #[test]
    fn shift_tells_the_two_inversions_apart() {
        // Files only, or directories too — Total Commander's split, and the
        // one place in the marking keys where Shift changes what is touched
        // rather than only where the cursor ends up.
        assert_eq!(bound(Key::KP_Multiply, PLAIN), Some(Action::InvertMarks));
        assert_eq!(
            bound(Key::KP_Multiply, ModifierType::SHIFT_MASK),
            Some(Action::InvertMarksIncludingFolders)
        );
    }

    #[test]
    fn shift_turns_a_cursor_key_into_a_marking_key() {
        // Every cursor key that marks with Shift still navigates without it.
        // A binding that ignored its modifiers would lose one of the two.
        for (key, plain, shifted) in [
            (Key::Down, Action::CursorDown, Action::ToggleMarkAndAdvance),
            (Key::Up, Action::CursorUp, Action::ToggleMarkAndRetreat),
            (Key::Home, Action::CursorFirst, Action::ExtendMarkToFirst),
            (Key::End, Action::CursorLast, Action::ExtendMarkToLast),
        ] {
            assert_eq!(bound(key, PLAIN), Some(plain), "{key:?}");
            assert_eq!(
                bound(key, ModifierType::SHIFT_MASK),
                Some(shifted),
                "{key:?}"
            );
        }
    }

    #[test]
    fn paging_is_bound_with_and_without_shift_and_means_different_things() {
        // This test used to assert the opposite for the plain keys: that they
        // were *unbound*, because "plain paging belongs to the widget, which
        // is the only thing that knows how tall the viewport is". That reason
        // expired when `page_rows` was built for the marking twins — the
        // measurement it called impossible is a method on `PaneView` — and the
        // keys were bound on 2026-08-30 so the model hears about the move.
        // Kept and inverted rather than deleted: the asymmetry is still the
        // thing worth pinning, it is just no longer an asymmetry of binding.
        for key in [Key::Page_Up, Key::Page_Down] {
            let plain = bound(key, PLAIN).expect("plain paging moves the cursor");
            let shifted = bound(key, ModifierType::SHIFT_MASK).expect("shift marks");
            assert_ne!(plain, shifted, "{key:?}");
        }
    }

    #[test]
    fn a_bound_key_with_the_wrong_modifier_triggers_nothing() {
        // Ctrl+End is the witness because Ctrl+Down stopped being one: it
        // took on the command history in phase 3c. The contract is unchanged,
        // only which key demonstrates it — End is bound plain and with Shift,
        // and must still mean nothing with Ctrl.
        assert_eq!(bound(Key::End, ModifierType::CONTROL_MASK), None);
        assert_eq!(bound(Key::Tab, ModifierType::ALT_MASK), None);
        assert_eq!(bound(Key::q, PLAIN), None);
    }

    #[test]
    fn ctrl_does_not_fall_through_to_a_plain_cursor_key() {
        // What Ctrl+Down used to witness, now that it means something: a
        // lookup ignoring modifiers would move the cursor instead of opening
        // the history, and the two are not close.
        assert_eq!(bound(Key::Down, PLAIN), Some(Action::CursorDown));
        assert_eq!(
            bound(Key::Down, ModifierType::CONTROL_MASK),
            Some(Action::CommandHistory)
        );
    }

    #[test]
    fn irrelevant_modifiers_do_not_break_a_binding() {
        // Caps Lock and Num Lock are reported by GTK but must not disable
        // navigation.
        for noise in [ModifierType::LOCK_MASK, ModifierType::BUTTON1_MASK] {
            assert_eq!(bound(Key::Down, noise), Some(Action::CursorDown));
            assert_eq!(
                bound(Key::q, ModifierType::CONTROL_MASK | noise),
                Some(Action::Quit)
            );
        }
    }
}
