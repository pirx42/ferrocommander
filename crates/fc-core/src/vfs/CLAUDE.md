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

- **`platform` stays private, and nothing outside `vfs/` names it.** Every
  Linux/Windows difference — paths, attributes, hidden-ness, mounts, free
  space, the config directory, trash errors — is reached through a function
  here, so no caller reinvents what a separator or a permission bit is. That
  the module has grown to fourteen such functions is the rule working, not
  leaking: each one is a difference that would otherwise be a `cfg` somewhere
  else.
- **A `VfsPath` cannot be invalid.** `..` beyond the root, `.`, and empty
  components collapse in the constructor, which is what makes an archive
  unable to name a path outside itself.
- **`VfsError` is a closed set** and carries no `io::Error`, so tests can
  compare it and the UI has a finite set of cases to draw.
