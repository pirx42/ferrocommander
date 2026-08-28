//! What each key does.
//!
//! Every binding lives in one table. The GTK controller does not know any key
//! names — it looks up an [`Action`] and dispatches it — which is what makes
//! the bindings testable without a display and gives phase 3's configurable
//! keymap one place to replace.

use gtk::gdk::{Key, ModifierType};

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

/// The phase-1 keymap.
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
        key: Key::q,
        modifiers: ModifierType::CONTROL_MASK,
        action: Action::Quit,
    },
];

/// The action a keystroke triggers, or `None` when nothing is bound.
pub fn action_for(key: Key, modifiers: ModifierType) -> Option<Action> {
    let modifiers = modifiers & RELEVANT_MODIFIERS;
    BINDINGS
        .iter()
        .find(|binding| binding.key == key && binding.modifiers == modifiers)
        .map(|binding| binding.action)
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
            (Key::q, ModifierType::CONTROL_MASK, Action::Quit),
        ];
        for (key, modifiers, action) in expected {
            assert_eq!(action_for(key, modifiers), Some(action), "{key:?}");
        }
    }

    #[test]
    fn an_unbound_key_triggers_nothing() {
        for key in [Key::F5, Key::Escape, Key::a, Key::Delete] {
            assert_eq!(action_for(key, PLAIN), None, "{key:?}");
        }
    }

    #[test]
    fn a_bound_key_with_the_wrong_modifier_triggers_nothing() {
        // Ctrl+Down must not fall through to the unmodified CursorDown — in
        // phase 3 it will mean something else entirely.
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
