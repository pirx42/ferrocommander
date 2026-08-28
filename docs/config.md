# Config — what the program remembers

← Parent: [CLAUDE.md](CLAUDE.md)

`tc-core::config` reads and writes the settings file. It lives in `tc-core`
because the UI never touches a filesystem directly, and the program's own
settings are no exception — everything here goes through a `VirtualFs`,
including the atomic write.

| | |
|---|---|
| Unix | `$XDG_CONFIG_HOME/ferrocommander/config.toml`, or `~/.config/…` |
| Windows | `%APPDATA%\ferrocommander\config.toml` |

What is remembered: the window size, and per pane the directory, the sort key
and direction, and the hidden-file flag — plus which pane had the keyboard.

## It is written as it changes, not on the way out

Every action that could change something worth remembering calls `remember`,
which schedules a write. Saving only from the close handler loses everything a
session learned the moment that handler does not run — a `kill -9`, a crash, a
session that ends with the machine — and a file manager that forgets where you
were every time it dies is one nobody trusts to remember anything. The close
handler still writes, so a change made in the last half-second is not lost to
the delay.

Two things keep this off the hot path, which matters because `remember` runs
after *every* keystroke that reaches an action
([performance.md](performance.md)):

- **A write that would change nothing is skipped.** Moving the cursor down a
  directory listing produces the same settings as before, and never touches
  the disk.
- **Writes are debounced** by `SETTINGS_SAVE_DELAY` (500 ms) and coalesced:
  one pending write picks up whatever the settings are when it runs, so
  dragging a window to a new size costs one file write rather than one per
  intermediate size.

The window size is the one change that arrives from outside the keymap, so it
is watched separately, through GTK's own `default-width` / `default-height`
notifications.

Both halves are covered end to end by tests that **kill** the app rather than
closing it (`settings_survive_the_app_being_killed`,
`a_resized_window_is_remembered_without_being_closed` in
`crates/tc-app/tests/ui.rs`) — with a close handler in play, a save-on-exit
implementation would pass a test that closed politely.

## `[drives]` — where each drive was last showing

A mount path to the directory a pane last had open on it, so `Alt+F1` lands
where you were rather than at the root ([keymap.md](keymap.md)). Written by
the app like everything but `[keys]`.

Shared by both panes and keyed by the **longest** mount a path is under, since
mounts nest: a file under `/mnt/backup` belongs to the backup drive, not to
`/`. `vfs::mount_for` decides that, and it takes the mount list as an argument
so the rule is tested against a made-up machine rather than whatever the test
host has mounted.

## `[keys]` — the bindings, which belong to the user

The defaults are the keymap in [keymap.md](keymap.md); a `[keys]` table in
`config.toml` lays the user's own bindings over them:

```toml
[keys]
# F9 makes a directory
"f9" = "create_dir"
# and F7 stops doing anything
"f7" = ""
"ctrl+e" = "exchange_panes"
```

- **An overlay, not a replacement.** A key nobody mentions keeps its default,
  so a binding added in a later version reaches people who already have a
  settings file. A full replacement would make every new key invisible to
  anyone who ever wrote this table.
- **An empty action unbinds the key**, which is not the same as leaving the
  line out — that keeps the default.
- **Names may be written in any case, and modifiers in any order.** GDK's own
  keysym names follow no rule a person could guess (`space` is lower, `Insert`
  is capitalised, `F8` is upper, `KP_Add` is all three at once), so the
  spellings it uses are tried in turn. A single letter is folded to lower case:
  upper-case letters are *different* keysyms that only arrive with Shift held,
  and `shift+` is how Shift is asked for.
- **A binding nobody can read is named on stderr and skipped.** A misspelling
  in one line must not cost the other nineteen.
- Action names are the `ACTION_NAMES` table in `crates/tc-app/src/keymap.rs`.
  A test walks every default binding against it, so an action that reaches a
  key but has no name — one the user could see working and could not rebind —
  fails the tests rather than shipping.

**`tc-core` carries this table and never interprets it.** A key name is a GTK
keysym and an action is a command of the shell, neither of which the engine
knows anything about ([crates/CLAUDE.md](../crates/CLAUDE.md)); the strings
cross the file and the shell makes sense of them.

## The file is edited, never regenerated

`save` parses the existing `config.toml` as a document, updates the tables the
app owns, and writes it back. Comments, blank lines, the order things were put
in and the whole `[keys]` table come through exactly as they were typed.

This is what makes a hand-edited file safe now that saving happens on every
change: serializing the settings struct reproduces the *data* and nothing else,
so a user's comments would be gone about half a second after they opened the
app. The end-to-end test writes a commented `[keys]` table, lets the app save,
kills it, and checks the comments are still there — and a probe that swaps the
document edit back for a struct serialization fails it.

The app **never writes `[keys]` itself**, even though `Settings` carries it.
Writing it back would reformat and reorder lines nobody asked it to touch.

A file that is not TOML at all is started over rather than repaired: there is
nothing in it worth preserving that could be found reliably, and refusing to
save would mean one bad character costs every setting from then on.

## Writing it is atomic

The new settings go to `config.toml.new` beside the real file and are then
**renamed over it**. A rename within one directory is the only write a
filesystem promises to do all-or-nothing, so a crash leaves either the old
file or the new one and never half of either. A settings file truncated
mid-write is a program that starts up wrong.

The test for this asserts the *mechanism*, because the outcome cannot be
observed: it records which paths are opened for writing and checks that the
real file is never one of them. Checking that no temporary file is left behind
afterwards does not catch a direct write — that leaves none either, which a
mutation probe demonstrated before this test existed.

## Nothing here may stop the program starting

- **No file is a first run**, and says nothing.
- **A corrupt file is reported once and replaced by the defaults.** A file
  manager that refuses to start over its own settings is worse than one that
  forgets where you were.
- **A file from another version** is the same case: unknown fields are
  rejected loudly rather than half-loaded, and the defaults start up.
- **A missing pane entry is defaulted, not fatal.** A file written when there
  were three panes must not cost you the two you have.
- **An unknown sort key falls back** instead of failing.
- **Failing to save is reported and otherwise ignored.** Refusing to work on,
  or to quit, because a settings file could not be written is a worse bargain
  than starting up in the wrong directory next time.

## Why serde, toml and toml_edit

The first dependency in this project that is not tiny, and the reliability
requirement is what settles it: the program rewrites this file whenever
something in it changes, and a hand-rolled parser is a liability on exactly
that path
([reliability.md](reliability.md)). The speed requirement is not in tension —
the file is read once at startup, and compile time is a developer cost
([performance.md](performance.md)).

**Two TOML crates, and each earns its place.** `toml` reads the file into
`Settings` and serializes what the app owns; `toml_edit` applies that to the
document without destroying the rest. One crate could not do both: a
serializer that reproduced the user's comments would have to model them, and
that is what a document editor *is*.

**Sort keys are written by name through an explicit table**, not by deriving
serde on the enum. Renaming a variant would otherwise silently change the file
format under everybody's existing settings. The round-trip test iterates that
table, so a new sort key nobody added to it fails the tests rather than being
written out as a default.
