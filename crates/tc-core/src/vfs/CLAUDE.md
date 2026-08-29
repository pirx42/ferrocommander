# vfs/

← Parent: [../CLAUDE.md](../CLAUDE.md) ·
Topic doc: [docs/vfs.md](../../../../docs/vfs.md)

The one interface everything that touches a filesystem goes through.

| File | |
|---|---|
| `mod.rs` | the `VirtualFs` trait, and the few functions that let a path leave this layer |
| `path.rs` | `VfsPath` — `/`-rooted and total by construction |
| `types.rs` | `Entry`, `Attributes`, `Store`, `VfsError` |
| `local.rs` | the local filesystem |
| `platform.rs` | **every** Linux/Windows difference, in one file |
| `constants.rs` | the path vocabulary |

## Rules that hold here

- **`platform` stays private.** Only `to_std_path`, `mount_points`,
  `render_attributes` and the two `unix_mode` bridges escape it, so no caller
  reinvents what a separator or a permission bit is.
- **A `VfsPath` cannot be invalid.** `..` beyond the root, `.`, and empty
  components collapse in the constructor, which is what makes an archive
  unable to name a path outside itself.
- **`VfsError` is a closed set** and carries no `io::Error`, so tests can
  compare it and the UI has a finite set of cases to draw.
