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
    /// Enter the directory under the cursor.
    Activate,
    /// Leave the current directory.
    GoParent,
    /// F5 — copy the entry under the cursor.
    Copy,
    /// F6 — move it, or rename it in place.
    Move,
    /// Shift+F6 — rename the row under the cursor, in the list itself.
    RenameInline,
    /// F7 — create a directory here.
    CreateDir,
    /// F8 / Del — delete to the trash, recoverably.
    Delete,
    /// Shift+F8 / Shift+Del — delete for good.
    DeletePermanently,
    /// Space — mark the row under the cursor, leaving the cursor where it is.
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
    .union(ModifierType::ALT_MASK);

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
        key: Key::F5,
        modifiers: PLAIN,
        action: Action::Copy,
    },
    Binding {
        key: Key::F6,
        modifiers: PLAIN,
        action: Action::Move,
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
    ("activate", Action::Activate),
    ("go_parent", Action::GoParent),
    ("copy", Action::Copy),
    ("move", Action::Move),
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
    ("insert_name", Action::InsertName),
    ("select_drive_left", Action::SelectDriveLeft),
    ("select_drive_right", Action::SelectDriveRight),
    ("clone_to_right", Action::CloneToRight),
    ("clone_to_left", Action::CloneToLeft),
    ("exchange_panes", Action::ExchangePanes),
    ("toggle_hidden", Action::ToggleHidden),
    ("quit", Action::Quit),
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
        Keymap {
            bindings: BINDINGS
                .iter()
                .map(|binding| ((binding.key, binding.modifiers), binding.action))
                .collect(),
        }
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
            let Some(stroke) = parse_key(spec) else {
                complaints.push(format!("{UNKNOWN_KEY}: {spec}"));
                continue;
            };
            // An empty action is how a key is taken away, which is not the
            // same as leaving it out — leaving it out keeps the default.
            if action.is_empty() {
                keymap.bindings.remove(&stroke);
                continue;
            }
            match action_named(action) {
                Some(action) => {
                    keymap.bindings.insert(stroke, action);
                }
                None => complaints.push(format!("{UNKNOWN_ACTION}: {action}")),
            }
        }
        (keymap, complaints)
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
    use super::*;

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
            (Key::Return, PLAIN, Action::Activate),
            (Key::KP_Enter, PLAIN, Action::Activate),
            (Key::BackSpace, PLAIN, Action::GoParent),
            (Key::F5, PLAIN, Action::Copy),
            (Key::F6, PLAIN, Action::Move),
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
            (Key::h, ModifierType::CONTROL_MASK, Action::ToggleHidden),
            (Key::q, ModifierType::CONTROL_MASK, Action::Quit),
        ];
        for (key, modifiers, action) in expected {
            assert_eq!(bound(key, modifiers), Some(action), "{key:?}");
        }
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
    fn paging_is_bound_with_shift_and_unbound_without_it() {
        // The asymmetry is deliberate: plain paging belongs to the widget,
        // which is the only thing that knows how tall the viewport is.
        for key in [Key::Page_Up, Key::Page_Down] {
            assert_eq!(bound(key, PLAIN), None, "{key:?}");
            assert!(bound(key, ModifierType::SHIFT_MASK).is_some(), "{key:?}");
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
