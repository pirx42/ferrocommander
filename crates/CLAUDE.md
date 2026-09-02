# crates/

← Parent: [../CLAUDE.md](../CLAUDE.md)

The Cargo workspace members. The split into two crates is the project's
central boundary: **the UI never touches the filesystem directly.**

**`fc` is FerroCommander.** Both crates carried a Total Commander prefix
until 2026-09-01, from a working title — a prefix naming the program this
one imitates rather than the one being written. `scripts/check-naming.py`
keeps the new one honest, and still allows `TC` in prose, which is how the
documents name Total Commander itself
([the plan](../docs/plans/archive/2026-09-01-drop-the-tc-prefix.md)).

| Crate | Kind | Contents |
|---|---|---|
| [fc-core](fc-core/src/CLAUDE.md) | lib | VFS, listing model, file operations, archives, search, multi-rename. **No GTK dependency** — headless-testable with `cargo test`. |
| [fc-app](fc-app/src/CLAUDE.md) | bin | GTK4 shell: main window with two panes, dialogs, viewer, keymap, config. |
| fc-shellmenu | lib | Explorer's own context menu over COM, for the Windows build. Empty on every other platform; its own crate so it can be type-checked from Linux without GTK for Windows, and tested on the Windows CI job without the application. |

## Why the split

Everything the UI does goes through a `VirtualFs` from `fc-core`. That buys
three things at once: archives become browsable folders for free (a pane just
holds a different `VirtualFs`), copying between local and archive falls out of
a single code path, and the entire engine is testable without a display
server. Details:
[docs/plans/2026-08-28-tc-clone-design.md](../docs/plans/archive/2026-08-28-tc-clone-design.md).

**The first claim has now been collected and counted.** Browsing a zip cost
sixteen lines outside `fc-core::archive`; the two formats after it cost none;
entering and leaving one cost 204, all of them in the shell. The full report
card is in [docs/archives.md](../docs/archives.md).

## Conventions

- Third-party versions and shared package metadata live in the root
  `Cargo.toml` under `[workspace.dependencies]` / `[workspace.package]`;
  member manifests reference them with `workspace = true`.
- Per-crate constants live in a `constants` module, not inline at the use
  site (skills [16](../docs/skills/16-no-magic-values.md) /
  [17](../docs/skills/17-centralize-constants.md)).
- The toolchain is pinned in `rust-toolchain.toml` so the green gate means
  the same thing everywhere.
