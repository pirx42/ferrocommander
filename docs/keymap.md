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
| `Shift+F6` | Rename the row under the cursor, in the list itself |
| `F7` | Create a directory |
| `Shift+F4` | Create a file and open it in the editor |
| `F8`, `Delete` | Delete to the trash |
| `Shift+F8`, `Shift+Delete` | Delete permanently |
| `Space` | Mark the row under the cursor |
| `Insert`, `Shift+↓` | Mark it and step down |
| `Shift+↑` | Mark it and step up |
| `Shift+Home` / `Shift+End` | Mark from the cursor to the first / last row |
| `Shift+PgUp` / `Shift+PgDn` | The same, over one page |
| `Num +` | Mark everything matching a wildcard |
| `Num −` | Unmark everything matching a wildcard |
| `Num *` | Invert the marks on the **files** |
| `Shift+Num *` | Invert them on the directories too |
| `Alt+Num +` / `Alt+Num −` | Mark / unmark every file with the cursor row's extension |
| `Num /` | The selection from before the last operation |
| `Ctrl+A`, `Ctrl+Num +` | Mark everything visible |
| `Ctrl+Num −` | Unmark everything visible |
| `Ctrl+↓`, `Alt+F8` | The command history — see [command-line.md](command-line.md) |
| `Ctrl+Enter` | Put the name under the cursor into the command line |
| `Alt+F1` / `Alt+F2` | Send the left / right pane to a drive |
| `Ctrl+→` / `Ctrl+←` | Show the active pane's directory in the right / left pane |
| `Ctrl+U` | Exchange the two panes |
| `Ctrl+R` | Re-read the directory — see [watching.md](watching.md) |
| `Ctrl+F3` … `Ctrl+F6` | Sort by name / ext / date / size |
| `Ctrl+H` | Show or hide the dot-files |
| `Ctrl+S` | Narrow the pane as you type |
| `Esc` | Stop narrowing |
| `Ctrl+Q` | Quit |
| any unbound letter | Starts a command — see [command-line.md](command-line.md) |

Activating a *file* still does nothing — F3/F4 arrive in phase 4.

**Two keys for each delete**, because Total Commander has both and muscle
memory splits evenly between them. Shift is the only place in the keymap where
a modifier changes what survives, so it has a test of its own rather than
riding on the binding table.

## Marks decide what an operation acts on

F5–F8 act on **everything marked**, and fall back to the row under the cursor
when nothing is marked. That fallback is what keeps marks optional rather than
a mode: press F5 on a file and it copies, mark ten and it copies ten, and
there is no third thing to learn.

`Insert` steps down after marking so it can be held, which is how a run of
files gets selected; `Space` leaves the cursor where it is, for picking one
out of a list.

**`Shift`+cursor is the same behaviour under another key.** Total Commander
marks the row being *left* and then moves, so `Shift+↓` is `Insert` by another
name — and running back over a row toggles it a second time and takes its mark
off again. That is TC's own quirk: it does not mark the row you land on, so
someone coming from Explorer overshoots by one. Reproducing it is the point.

**A jump marks a range instead of toggling.** `Shift+Home`/`End`/`PgUp`/`PgDn`
have no direction to run back over, and "to the end" does not mean "flip
everything I passed". `..` still cannot be marked, so a range running over it
simply does not pick it up.

**`Num *` inverts the files only; `Shift+Num *` takes the directories too.**
TC's split, and the useful default — a person inverting a selection is nearly
always thinking about files. `Alt+Num ±` follows the same rule: a directory
called `photos.backup` is not one of "the `.backup` files".

**`Num /` restores the selection from before the last operation.** The marks
are put away when a job is *submitted*, not when it finishes: the job ends by
building a fresh listing, and by then the marks it consumed are gone.

### Ordinary keys stand in for the keypad

`+ − * /` reach their keypad bindings through an alias table rather than
through bindings of their own, so a keyboard with no numeric block runs every
marking command, modifiers included: `Ctrl+−` is `Ctrl+Num −`, `Alt++` is
`Alt+Num +`.

**The Shift that produced the character is dropped.** On most layouts `+` is
`Shift+=` and `*` is `Shift+8`, so the keystroke arrives as `plus` or
`asterisk` *carrying* `SHIFT_MASK` and matched nothing at all — the `+` twin
shipped in phase 3 was dead on arrival, and only a real key press could say
so. A modifier needed to type a character is a fact about the keyboard, not
something the user meant.

The cost: the ordinary `*` cannot also carry a *deliberate* Shift, so
`Shift+Num *` is reachable from the keypad only. That is a limitation of
layouts on which `*` cannot be typed without Shift, not a choice, and a plain
`*` that did nothing would be worse.

**Both are a toggle, not a set**: pressing either again on a marked row takes
the mark back, which is the only way to unmark one row out of many. Tested at
both layers — `toggle_selected` twice in `tc-core`, and both keys twice
through the real binary.

Marking acts on what is **visible**: what the hidden-file flag or a filter is
holding back is not something the user can see to have meant. Details of the
model in [listing.md](listing.md).

**`..` is not something to operate on.** It is a navigation control, not an
entry, and copying or deleting "the parent directory" from inside it is never
what the user means. It cannot be marked, and with nothing else marked, F5–F8
on it do nothing. A mark elsewhere in the pane still counts — the `..` rule is
about the fallback, not about the marks.

## The drive selector

`Alt+F1` and `Alt+F2` open a list of mount points and send a pane to the one
chosen. The list opens focused on its first row, so the arrows walk it, Enter
takes it and Escape leaves without going anywhere — there is no Cancel button
to fall back on, which makes Escape load-bearing and it has a test.

**The F-key number *is* the pane number**, exactly as in Total Commander:
`Alt+F1` is the left pane whichever one has the keyboard. That is the opposite
rule to `Ctrl+←/→` below, and deliberately — an arrow has a direction to be
relative to, and a number does not. The test presses `Alt+F2` from the left
pane, since that is the only way to tell the two rules apart.

**A drive remembers the directory it was left in.** Switching away and back
is not a trip to the root and a walk down again — Total Commander's behaviour
with its default `AlwaysToRoot=0`. The memory is **shared between the panes**,
as it is there: leaving a drive in one pane is what the other finds when it
arrives, and whichever pane left a drive most recently is the one that decides.
It survives a restart, in the `[drives]` table of [config.md](config.md), and a
remembered directory that has since gone falls back to the drive itself rather
than leaving the pane showing an error about a path nobody asked for by name.

The drive-bar buttons go through the same code, so a drive remembers where it
was left however it was reached.

**What is *not* carried across**, because TC does not carry it either: the
marks are cleared and the cursor starts at the top. Leaving a directory drops
its selection, and coming back lands on the first row — the same in TC, which
is why this is a matching behaviour rather than a missing one.

**It is a modal window, where TC has a dropdown.** A GTK popover is not a
window the end-to-end suite can find or send keys to, and a drive selector
that cannot be tested through a real key press is the kind of thing that ships
broken. The drive bar's buttons still do the same job with the mouse.

## The two-pane commands

`Ctrl+←/→` are **relative to the active pane**, as in Total Commander: the
arrow points at the pane being *written*, and the active one supplies the
directory. Pressing an arrow toward the pane the keyboard is already in does
nothing, rather than guessing which of the two directions was meant.

"Nothing" is literal, and that is the part worth a test: sending a pane to the
directory it is already in looks identical from outside until you notice it
re-read it, which drops the marks and puts the cursor back at the top. The
end-to-end test marks a file, presses the self-pointing arrow, and then spends
the mark.

`Ctrl+U` exchanges the panes' **contents**, not their widgets — both are
children of a `Paned`, and reparenting them would be work for no reason. The
whole `Listing` is swapped, which is what makes the directory, the cursor and
the marks travel together; anything rebuilt field by field would quietly drop
one of them, and the test for it marks a file on one side and spends it on the
other. The keyboard stays in the same physical pane, now showing the other
side.

## Creating a file to edit

`Shift+F4` asks for a name, creates an **empty** file and hands it to the
editor — Total Commander's behaviour, including leaving the file empty: what an
editor makes of a zero-byte file is the editor's business.

Asked for rather than assumed, unlike TC's fixed `new.txt`: the name is the
first thing anybody changes, and a dialog they can accept with Enter costs
nothing. Which program opens it is the `editor` line in
[config.md](config.md).

**A name that is already taken is refused**, and that is the whole reason this
is a job rather than a bare VFS call: `create_file` truncates, so `Shift+F4` on
an existing name would empty the very file the user meant to open — and then
hand it to an editor, which would save the emptiness back.

**The editor starts only once the job reports success.** One opened on a file
that was never created shows an empty buffer that silently recreates it on
save, which is a worse answer than nothing.

## Renaming in the list

`Shift+F6` turns the name under the cursor into a field in the list itself,
rather than a dialog that covers the thing being renamed — Total Commander's
behaviour. Enter accepts, Escape abandons and hands the keyboard back.

**The field carries the whole filename**, extension and all. The name and ext
columns are a presentation split; renaming `notes` to `todo` while silently
keeping `.txt` in another column is not something the user can see to have
agreed to. The **stem is selected** when it opens, so typing replaces the name
and leaves the extension — the ordinary case, and retyping `.txt` every time
is the annoying one.

**A rename is a move whose destination is exact**, which is the same rule F6's
dialog follows, so it goes through the same queue and gets the same conflict
question when something is already called that. Accepting an unchanged name
does nothing at all: pressing Enter straight away is somebody deciding not to
rename, not a job to run.

The editable cell is a `Stack` per *recycled* cell, not per entry: a
`ColumnView` keeps widgets only for the rows on screen, so it is some forty
deep rather than fifty thousand ([performance.md](performance.md)).

**While a rename is open the cursor sync stops asking for the focus.** It
belongs to the editor, and the column view taking it back would close the
field under the user's fingers. That the rename worked at all before that was
ordering luck — a cell further down happens to bind and grab the focus after
the scroll, and the first row did not.

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

**With Shift they are bound, and the pane measures a page itself.** The
scrolled window's adjustment knows the content height and the viewport height,
and the rows are uniform, so the row count falls out of the ratio; before the
first layout there is no height to divide by and a fallback constant stands
in. That measurement is the one part of this that could quietly be wrong, so
it has an end-to-end test on a screen tall enough to hold every row, where one
`Shift+PgDn` has to mark all of them.

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

All default bindings live in a single `BINDINGS` table in `keymap.rs`, and the
GTK controller knows no key names at all: it looks up an `Action` and
dispatches it. That is what makes the bindings testable without a display, and
it is what lets the user's own bindings be laid over the defaults in one place.

**Every binding above can be changed** — see the `[keys]` table in
[config.md](config.md). The overlay is built once at startup into a hash map,
so a lookup stays a hash of one key rather than a walk down a table a
configurable keymap would otherwise make arbitrarily long
([performance.md](performance.md)).

**Unlisted modifiers are masked out before the lookup.** GTK reports Caps
Lock, Num Lock and held mouse buttons alongside the real modifiers; without
masking, a user with Caps Lock on would find every key unbound. Only Ctrl,
Shift and Alt take part in a binding.

**A bound key with the wrong modifier does nothing.** `Ctrl+↓` does not fall
through to plain `↓` — in phase 3 it will mean something else entirely, and a
binding that silently ignores its modifiers would make that impossible.

## Sorting

`Ctrl+F3`…`Ctrl+F6` are Total Commander's sort keys, in its order: name,
extension, date, size. **The same key again flips the direction**; a different
one starts ascending. That rule is what makes one key mean both "sort by this"
and "the other way round", and it is a pure function on `Sort` with its own
test rather than something the widget layer decides.

Note that `F5` copies and `Ctrl+F5` sorts by date. The keymap's rule that a
bound key with the wrong modifier does nothing is what keeps both possible;
there is a test for exactly that pair.

The header of the active column carries ▲ or ▼. The marker is put into the
header *text*, because this shell sorts in the model and there is no GTK
sorter whose arrow GTK would draw for us.

**The ordering belongs to the pane, not to the directory.** Every navigation
and every finished job builds a fresh listing, so the pane re-applies its own
sort, hidden-file flag and filter to whatever it just read. Without that,
sorting by size and then copying one file quietly put the order back to name.

## The quick filter takes the keyboard back

`Ctrl+S` opens a field above the rows; typing narrows the pane live; `Esc`
clears it and hides the field; `Enter` keeps the narrowed view and hands the
keyboard back to the rows.

While that field has the focus the shell **does not dispatch anything**. Its
controller sits in the capture phase so the column view cannot swallow Tab and
the arrows, and that puts it ahead of the field too — every letter would
become a command and `Enter` would open a directory instead of accepting the
filter. So the controller checks whether the focus is inside a text widget and
stands down if it is.

The field's own handler is *also* in the capture phase, and for a different
reason: `GtkText` consumes `Return` to emit its own activate signal, so a
bubble-phase handler never sees it and the keyboard stays trapped in the
field. `Esc` arrives either way, which is why only half of it looked wired up
until a test pressed `Enter`.

**A filter belongs to the directory it was typed in** and is dropped on
navigation. Carrying it into the next directory would show an empty pane and
no reason why.

**`..` survives every filter**, so a filter that matches nothing can still be
left.

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
automated coverage at all. It is now covered by the end-to-end suite in
`crates/tc-app/tests/ui.rs`, where a real X server delivers real key events to
the real binary and the checks are on the filesystem afterwards. The cursor
keys and Tab are used by every test to get anywhere at all, and `Ctrl+Q` is
how the harness closes the app.

**What is deliberately not exercised end to end**, and why:

- `Backspace`, and `Ctrl+F3`/`F4`/`F5` — covered headlessly, and reaching them
  through a real window would say nothing the unit tests do not.
- The **second** drive key: `Alt+F1` and `Alt+F2` differ only in which pane
  they name, and the test that presses `Alt+F2` covers exactly that.
- The **aliases**: `Ctrl+Num +` for `Ctrl+A`, keypad `Enter` for `Enter`,
  `Delete` for `F8`. They resolve to the same action as a key that *is*
  pressed for real, so the second press only tests the lookup, which the
  keymap's own tests cover exhaustively.

Everything a physical press could get wrong on its own — a binding that
matches nothing, a modifier that arrives uninvited, a dialog with no focused
button, an action reaching the wrong pane — is pressed for real. That is not
theoretical: the `+` binding shipped in phase 3 was dead on arrival, and only
a real key press found it.

It is part of `cargo test --workspace`, so touching the controller, the keymap
or a dialog cannot silently break them. How it works and what it needs
installed: [ui-shell.md](ui-shell.md).
