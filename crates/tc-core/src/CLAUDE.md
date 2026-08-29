# tc-core/src/

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
| `command.rs` | running a command line, and opening an editor |
| `config.rs` | the settings file, read once and written as it changes |
| `watch.rs` | noticing what another program did to a directory |
| `glob.rs` | `*` and `?`, for the filter, the marks and the search |

## Rules that hold here

- **No GTK, ever.** The dependency is not in the manifest, which is the only
  way that stays true.
- **Threads and channels live here, not in the shell.** A long read or a job
  runs on a worker; the shell awaits a receiver whose type is aliased here so
  it never names the channel crate.
- **Constants live in a `constants` module per subsystem**, not inline at the
  use site.
