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

## Why serde and toml

The first dependency in this project that is not tiny, and the reliability
requirement is what settles it: the program rewrites this file whenever
something in it changes, and a hand-rolled parser is a liability on exactly
that path
([reliability.md](reliability.md)). The speed requirement is not in tension —
the file is read once at startup, and compile time is a developer cost
([performance.md](performance.md)).

**Sort keys are written by name through an explicit table**, not by deriving
serde on the enum. Renaming a variant would otherwise silently change the file
format under everybody's existing settings. The round-trip test iterates that
table, so a new sort key nobody added to it fails the tests rather than being
written out as a default.
