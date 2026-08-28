# Keymap — what each key does

← Parent: [CLAUDE.md](CLAUDE.md)

## Bindings

| Key | Action |
|---|---|
| `Tab` | Switch to the other pane |
| `↑` / `↓` | Move the cursor one row |
| `Home` / `End` | Move the cursor to the first / last row |
| `Enter`, keypad `Enter` | Enter the directory under the cursor |
| `Backspace` | Leave the current directory |
| `F5` | Copy the entry under the cursor |
| `F6` | Move it, or rename it in place |
| `F7` | Create a directory |
| `F8`, `Delete` | Delete to the trash |
| `Shift+F8`, `Shift+Delete` | Delete permanently |
| `Ctrl+Q` | Quit |

Activating a *file* still does nothing — F3/F4 arrive in phase 4.

**Two keys for each delete**, because Total Commander has both and muscle
memory splits evenly between them. Shift is the only place in the keymap where
a modifier changes what survives, so it has a test of its own rather than
riding on the binding table.

**`..` is not something to operate on.** It is a navigation control, not an
entry, and copying or deleting "the parent directory" from inside it is never
what the user means. F5-F8 on the `..` row do nothing.

## What the target field means

F5 and F6 open the same dialog, prefilled with the *other* pane's directory
**and a trailing separator**. That slash is the rule, shown rather than
hidden:

| What is in the field | Where the entry lands |
|---|---|
| `/mnt/backup/` | into that directory, keeping its name |
| `/mnt/backup/holiday.txt` | at exactly that path |
| `holiday.txt` | beside the source, under that name |

The last row is the whole of "F6 renames in place" and "F5 duplicates a
file" - no separate rename command, and no dialog that has to guess. A rename
*is* a move whose destination is exact, which is why `ops` has no `Rename`
job.

## Page Up / Page Down are the widget's job

They are deliberately **not** in the table. Paging depends on how many rows
fit on screen, and the model has no idea how tall the viewport is — the
`ColumnView` does. So the page keys fall through to the widget, which moves
its own selection and scrolls.

That leaves the widget's selection ahead of the model's cursor, so the pane
**adopts the selection before acting on any bound key**. Without that step the
two drift apart and the next Enter opens whatever row the cursor was on
before the page, not the row the user is looking at.

The same mechanism covers anything else the widget handles on its own, and is
what mouse selection will ride on in a later phase.

The traffic runs the other way too: when the shell moves the cursor it also
moves the widget's *focus*, because the widget pages from its own focus. If
focus did not follow, a Page Down after some arrow keys would page from
wherever the widget last was rather than from the cursor.

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

What the text in the target field means is unit-tested too, in `jobs.rs`,
including that the prefilled value round-trips back to "into that directory".

The wiring *between* a physical keypress and those functions used to have no
automated coverage at all. It now has
[`scripts/smoke-keys.sh`](../scripts/smoke-keys.sh): a real X server delivers
real key events to the real binary, and the checks are on the filesystem
afterwards. It covers Tab, the cursor keys and Enter in passing, and F5, F7
and F8 with their dialogs directly.

Run it after touching the controller, the keymap or a dialog:

```bash
cargo build --release && scripts/smoke-keys.sh
```
