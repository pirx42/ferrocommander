//! What each key does.
//!
//! Every binding lives in one table. The GTK controller does not know any key
//! names — it looks up an [`Action`] and dispatches it — which is what makes
//! the bindings testable without a display and gives phase 3's configurable
//! keymap one place to replace.

use gtk::gdk::{Key, ModifierType};
use tc_core::listing::SortKey;

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

/// The action a keystroke triggers, or `None` when nothing is bound.
pub fn action_for(key: Key, modifiers: ModifierType) -> Option<Action> {
    let (key, modifiers) = normalize(key, modifiers & RELEVANT_MODIFIERS);
    BINDINGS
        .iter()
        .find(|binding| binding.key == key && binding.modifiers == modifiers)
        .map(|binding| binding.action)
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
            (Key::Right, ModifierType::CONTROL_MASK, Action::CloneToRight),
            (Key::Left, ModifierType::CONTROL_MASK, Action::CloneToLeft),
            (Key::u, ModifierType::CONTROL_MASK, Action::ExchangePanes),
            (Key::h, ModifierType::CONTROL_MASK, Action::ToggleHidden),
            (Key::q, ModifierType::CONTROL_MASK, Action::Quit),
        ];
        for (key, modifiers, action) in expected {
            assert_eq!(action_for(key, modifiers), Some(action), "{key:?}");
        }
    }

    #[test]
    fn an_unbound_key_triggers_nothing() {
        // F5 and Delete left this list in phase 2, and Insert and Escape in
        // phase 3, each when it was bound. The contract is unchanged; only
        // the witnesses are. Plain `a` and `s` are still unbound — only their
        // Ctrl forms mean anything.
        for key in [Key::a, Key::s, Key::F9, Key::F12] {
            assert_eq!(action_for(key, PLAIN), None, "{key:?}");
        }
    }

    #[test]
    fn ctrl_turns_an_operation_key_into_a_sort_key() {
        // F5 copies and Ctrl+F5 sorts by date. A binding that ignored its
        // modifiers would make one of those impossible.
        assert_eq!(action_for(Key::F5, PLAIN), Some(Action::Copy));
        assert_eq!(
            action_for(Key::F5, ModifierType::CONTROL_MASK),
            Some(Action::SortBy(SortKey::Modified))
        );
        assert_eq!(action_for(Key::F6, PLAIN), Some(Action::Move));
        assert_eq!(
            action_for(Key::F6, ModifierType::CONTROL_MASK),
            Some(Action::SortBy(SortKey::Size))
        );
    }

    #[test]
    fn shift_tells_a_recoverable_delete_from_a_final_one() {
        // The one place in the keymap where a modifier changes what survives,
        // so it gets its own test rather than riding on the table above.
        for key in [Key::F8, Key::Delete] {
            assert_eq!(action_for(key, PLAIN), Some(Action::Delete), "{key:?}");
            assert_eq!(
                action_for(key, ModifierType::SHIFT_MASK),
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
            let expected = action_for(keypad, PLAIN);
            assert!(expected.is_some(), "{keypad:?} is not bound at all");
            for modifiers in [PLAIN, ModifierType::SHIFT_MASK] {
                assert_eq!(action_for(twin, modifiers), expected, "{twin:?}");
            }
        }
    }

    #[test]
    fn a_twin_carries_the_modifiers_that_were_meant() {
        // Only Shift is dropped. Ctrl and Alt on an ordinary `+` still reach
        // the keypad meanings, which is what makes a keyboard without a
        // numeric block able to run every marking command.
        assert_eq!(
            action_for(Key::plus, ModifierType::CONTROL_MASK),
            Some(Action::MarkAll)
        );
        assert_eq!(
            action_for(Key::minus, ModifierType::ALT_MASK),
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
            assert_eq!(
                action_for(Key::KP_Add, modifiers),
                Some(add),
                "{modifiers:?}"
            );
            assert_eq!(
                action_for(Key::KP_Subtract, modifiers),
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
        assert_eq!(
            action_for(Key::KP_Multiply, PLAIN),
            Some(Action::InvertMarks)
        );
        assert_eq!(
            action_for(Key::KP_Multiply, ModifierType::SHIFT_MASK),
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
            assert_eq!(action_for(key, PLAIN), Some(plain), "{key:?}");
            assert_eq!(
                action_for(key, ModifierType::SHIFT_MASK),
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
            assert_eq!(action_for(key, PLAIN), None, "{key:?}");
            assert!(
                action_for(key, ModifierType::SHIFT_MASK).is_some(),
                "{key:?}"
            );
        }
    }

    #[test]
    fn a_bound_key_with_the_wrong_modifier_triggers_nothing() {
        // Ctrl+Down must not fall through to the unmodified CursorDown.
        assert_eq!(action_for(Key::Down, ModifierType::CONTROL_MASK), None);
        assert_eq!(action_for(Key::Tab, ModifierType::ALT_MASK), None);
        assert_eq!(action_for(Key::q, PLAIN), None);
    }

    #[test]
    fn irrelevant_modifiers_do_not_break_a_binding() {
        // Caps Lock and Num Lock are reported by GTK but must not disable
        // navigation.
        for noise in [ModifierType::LOCK_MASK, ModifierType::BUTTON1_MASK] {
            assert_eq!(action_for(Key::Down, noise), Some(Action::CursorDown));
            assert_eq!(
                action_for(Key::q, ModifierType::CONTROL_MASK | noise),
                Some(Action::Quit)
            );
        }
    }
}
