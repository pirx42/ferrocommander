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
- **Failing to save does not stop the program closing.** Refusing to quit
  because a settings file could not be written is a worse bargain than
  starting up in the wrong directory next time.

## Why serde and toml

The first dependency in this project that is not tiny, and the reliability
requirement is what settles it: the program rewrites this file on every exit,
and a hand-rolled parser is a liability on exactly that path
([reliability.md](reliability.md)). The speed requirement is not in tension —
the file is read once at startup, and compile time is a developer cost
([performance.md](performance.md)).

**Sort keys are written by name through an explicit table**, not by deriving
serde on the enum. Renaming a variant would otherwise silently change the file
format under everybody's existing settings. The round-trip test iterates that
table, so a new sort key nobody added to it fails the tests rather than being
written out as a default.
