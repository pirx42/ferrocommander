# fc-core/src/

← Parent: [../../CLAUDE.md](../../CLAUDE.md)

The engine. No GTK, no display server, no window — everything here is testable
with a plain `cargo test`.

| Module | |
|---|---|
| [vfs/](vfs/CLAUDE.md) | the filesystem interface, `VfsPath`, and every platform difference |
| [listing/](listing/CLAUDE.md) | the model behind a pane |
| [ops/](ops/CLAUDE.md) | copy, move, delete, mkdir, pack — the job engine |
| [archive/](archive/CLAUDE.md) | archives as filesystems, and writing new ones |
| `search.rs` | the streaming, cancellable walk behind `Alt+F7` |
| `branch.rs` | the whole tree below a directory as one flat listing — `Ctrl+B` |
| `sizes.rs` | what a folder actually holds, counted — `Alt+Shift+Enter` |
| `rename.rs` | the multi-rename rules — pure, so the preview *is* the rename |
| `viewer.rs` | what `F3` shows, built on `read_at` and nothing else |
| `command.rs` | running a command line, and handing a file to the desktop |
| `compare.rs` | two files as paired diff rows, or a byte verdict — compare by content |
| `config.rs` | the settings file, read once and written as it changes |
| `watch.rs` | noticing what another program did to a directory |
| `glob.rs` | `*` and `?`, for the filter, the marks and the search |
| `clipboard.rs` | the freedesktop clipboard formats — encoding and decoding only, no I/O |

## Rules that hold here

- **No GTK, ever.** The dependency is not in the manifest, which is the only
  way that stays true.
- **Threads and channels live here, not in the shell.** A long read or a job
  runs on a worker; the shell awaits a receiver whose type is aliased here so
  it never names the channel crate.
- **Constants live in a `constants` module per subsystem**, not inline at the
  use site.
- **A recursive read is a queue, never recursion.** `search`, `branch` and
  `sizes` each walk a tree, and each does it by popping from a `Vec` of
  pending directories. A directory tree is user input, and a deep enough one
  turns recursion into a stack overflow — a crash in a file manager, over
  somebody else's directory layout. Two more rules come with the shape:
  **unreadable is skipped, not fatal**, because one directory nobody may
  enter must not cost the whole answer; and **the cancel is checked at least
  once per directory**, more often only where the per-entry work can itself
  be slow (`search` opens and reads files; the other two do not).

  The three are **not** one function, and that was decided with all three on
  screen rather than assumed. What they share is those six lines and these
  rules; what differs is everything interesting — what the queue carries, what
  each entry becomes, and what a refusal or a cancel means to the answer. A
  walker general enough for all three needs a payload type, a descend closure,
  a per-entry closure and a control-flow enum, which is more shape than the
  duplication costs. So the *reasoning* is shared, here, and each walk owns
  its own loop.
