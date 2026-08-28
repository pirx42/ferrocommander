# Keymap — what each key does

← Parent: [CLAUDE.md](CLAUDE.md)

## Phase 1 bindings

| Key | Action |
|---|---|
| `Tab` | Switch to the other pane |
| `↑` / `↓` | Move the cursor one row |
| `Home` / `End` | Move the cursor to the first / last row |
| `Enter`, keypad `Enter` | Enter the directory under the cursor |
| `Backspace` | Leave the current directory |
| `Ctrl+Q` | Quit |

Everything else is unbound. Activating a *file* does nothing in phase 1 —
F3/F4 arrive in phase 4.

## One table, no key names in the widgets

All bindings live in a single `BINDINGS` table in `keymap.rs`, and the GTK
controller knows no key names at all: it looks up an `Action` and dispatches
it. That is what makes the bindings testable without a display, and it gives
phase 3's configurable keymap exactly one place to replace.

**Unlisted modifiers are masked out before the lookup.** GTK reports Caps
Lock, Num Lock and held mouse buttons alongside the real modifiers; without
masking, a user with Caps Lock on would find every key unbound. Only Ctrl,
Shift and Alt take part in a binding.

**A bound key with the wrong modifier does nothing.** `Ctrl+↓` does not fall
through to plain `↓` — in phase 3 it will mean something else entirely, and a
binding that silently ignores its modifiers would make that impossible.

## Capture phase

The controller sits on the window in the **capture** phase, so the window sees
a keystroke before the `ColumnView` does. Otherwise the column view's built-in
focus and selection handling would consume `Tab` and the arrow keys and fight
the pane cursor.

## Where a keystroke leads

`navigation.rs` answers that as pure functions over a `Listing`, so the
decisions are testable without a window:

- `activation_target` — the directory the cursor row leads into, or `None` for
  a file. The `..` row needs no special case: it is a directory like any
  other, and `VfsPath` normalization makes its target the parent.
- `parent_target` — where `Backspace` leads, `None` at the root.
- `focus_after_move` — which entry the cursor lands on afterwards.

**Stepping up lands on the directory you just left**, not on `..`. Someone
who pressed Backspace is looking for where they were; dropping them at the
top of the list makes them hunt for it again, and makes walking a tree
several levels deep genuinely tedious. Every other move — descending, or
jumping somewhere unrelated — starts at the top, because there is no previous
position to restore.

If the directory just left is hidden and hidden entries are not shown, the
cursor stays at the top: the cursor cannot sit on a row that is not there.

## When a directory cannot be entered

The pane **stays where it is** and shows the reason next to the path:

```
/home/pirx   ⚠ locked: permission denied
```

A pane that cannot read a directory must not display it as empty — that would
strand the user somewhere that looks browsable but is not. Leaving them where
they were, with the reason visible, keeps the pane navigable. The notice
clears on the next successful navigation.

## Testing

The keymap table and the navigation targets are unit-tested headlessly,
including the cases that are easy to get wrong: an unbound key, a bound key
with the wrong modifier, and irrelevant modifiers that must not break a
binding.

The wiring *between* a physical keypress and those functions — the GTK
controller, its capture phase, the focus handling — is not covered by an
automated test. It was verified by hand on 2026-08-28: `Tab`, `Enter`,
`Backspace`, `Home`/`End` and the arrow cursors behave as specified. Keep that in mind when
touching the controller — nothing will fail the gate if it breaks.
