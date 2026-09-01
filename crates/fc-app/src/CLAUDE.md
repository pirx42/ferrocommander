# fc-app/src/

← Parent: [../../CLAUDE.md](../../CLAUDE.md) ·
Topic doc: [docs/ui-shell.md](../../../docs/ui-shell.md)

The GTK4 shell. Thin on purpose: it draws, it listens, and it asks `fc-core`
for every answer. **Nothing here touches the filesystem directly.**

| File | |
|---|---|
| `main.rs` | the window: what is built at startup, and how a keystroke reaches an action |
| `shell.rs` | what the program knows while it runs, and the machinery every action shares — the settings, the job queue, the listings in flight, the watches |
| `actions.rs` | what each key *does*: one function per action, and the table that maps one to the other |
| `keymap.rs` | which key is which action, the defaults, and the `[keys]` overrides |
| `pane.rs` | one pane: the widget, its listing, its backend, and the archives it walked into |
| [dialogs/](dialogs/) | the modal windows — one shell, six windows with state of their own |
| `navigation.rs` | where a navigation keystroke leads — pure, over a `Listing` |
| `jobs.rs` | what a file-operation keystroke asks for — pure, over a `Listing` and the typed text |
| `command_line.rs` | the entry at the bottom, and what a typed line means |
| `progress.rs` | the throughput meter: events folded into a fraction, a caption and a path |
| `format.rs` | how a byte count, a time left and a failure are written for a person |
| `row.rs` | one row of a pane, as GTK needs it |
| `constants.rs` | every string, size and delay the shell uses |

## Rules that hold here

- **The borrow is dropped before anything that can call back.** A dialog
  always can, so an action reads what it needs out of the shell, drops the
  `RefCell` borrow, and only then opens one.
- **The shell's own state stays private.** `Shell`'s fields are
  crate-visible only where something outside `shell.rs` actually reads them;
  the queue, the settings path and the pending-write flag are its own
  business, and the window is reached through `Shell::window()` because
  `None` — the window is gone — is the only thing anyone has to handle.
- **Decisions live in the pure modules.** `navigation.rs` and `jobs.rs` are
  functions over a `Listing` and a string, so what a key means is testable
  without a display server; the rest of this crate is wiring.
- **A key does nothing until the keymap says so.** There is one table, the
  user can override it, and a binding that reaches no action is a compile
  error.
- **Every string is in `constants.rs`**, including the ones the end-to-end
  tests look for — which spell them out again rather than importing them,
  because a test that reads its expectations out of the code can only agree
  with it.
