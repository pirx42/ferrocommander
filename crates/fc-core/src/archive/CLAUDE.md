# archive/

← Parent: [../CLAUDE.md](../CLAUDE.md) ·
Topic doc: [docs/archives.md](../../../../docs/archives.md)

Reading archives as filesystems, and writing new ones. The module the
`VirtualFs` abstraction was designed for: browsing a zip cost sixteen lines
outside this directory, and the two formats after the first cost none.

| File | |
|---|---|
| `mod.rs` | `Format`, `format_for`, `spawn_enter`, and `ArchiveFs` — the read-only `VirtualFs` |
| `index.rs` | what an archive contains, read once at open: paths, offsets, and the sanitising that makes an escaping name impossible |
| `reader.rs` | `Container` and `Region` — bytes out of a container that is itself only a `VfsPath`; `Wrapper` for a gzipped one |
| `entry.rs` | one entry's decoded bytes, with its checksum verified |
| `zip.rs` | the central directory, parsed into an `Index` |
| `tar.rs` | the headers, scanned into an `Index` |
| `date.rs` | zip dates, both ways — kept together because the two must be exact inverses |
| `pack.rs` | `Packer`: appending entries to a new archive, one format each |
| `constants.rs` | names, buffer sizes, and the calendar constants |

## Rules that hold here

- **The dependency points this way.** This module knows `listing` (it hands
  `spawn` a closure that opens an archive and reads its root); `listing` does
  not know this one. A directory model that named a backend would be the one
  thing `VirtualFs` exists to prevent.
- **The read side is a `VirtualFs` and nothing else.** If browsing an archive
  ever needs a special case in the pane, the listing, the viewer or `ops`, the
  abstraction did not hold and the honest thing is to write that down rather
  than spread the case around.
- **The write side is not one**, and cannot be: a zip is a stream with its
  index at the end. `pack.rs` is a sequential appender the operation engine
  drives.
- **Nothing holds an archive open.** Every reader owns its range of the
  container and reads through `read_at`, because `open_read` promises a
  `Send` reader and a job holds its backends across a worker thread.
- **An entry name is somebody else's input.** It is resolved once, at index
  time, by `VfsPath::new` — so the archive cannot express a path outside
  itself.
