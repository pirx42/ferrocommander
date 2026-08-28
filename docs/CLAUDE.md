# docs/

← Parent: [../CLAUDE.md](../CLAUDE.md)

Project documentation for the Total Commander clone. One file per topic,
cross-references instead of duplication.

## Contents

- [vfs.md](vfs.md) — the virtual filesystem layer: the `VirtualFs`
  interface, `VfsPath` semantics, entry/error model, and where Linux and
  Windows differ.
- [listing.md](listing.md) — the directory model behind a pane: ordering
  rules, the `..` row, hidden-file filtering, cursor behavior.
- [ui-shell.md](ui-shell.md) — the GTK4 window: widget tree, row rendering,
  columns, active-pane marking, the build stamp in the title, and the GTK
  version floor.
- [ops.md](ops.md) — the file-operation engine: jobs, the scan/execute
  split, progress events, conflict resolution, what a cancel does and does
  not undo.
- [keymap.md](keymap.md) — what each key does, how bindings are looked up,
  and what happens when a directory cannot be entered.
- [watching.md](watching.md) — how a pane notices what another program did:
  the directory watcher, `Ctrl+R`, and what survives a re-read.
- [viewer.md](viewer.md) — the F3 viewer: why it never reads the file, how
  paging works, and what encoding detection amounts to.
- [command-line.md](command-line.md) — the command line at the bottom: how a
  typed line is run, why `cd` is read rather than spawned, and when output is
  shown.
- [config.md](config.md) — the settings file: where it lives, why it is
  written as it changes rather than on exit, why the write is atomic, and why
  nothing about it may stop the program starting.
- [reliability.md](reliability.md) — the standing reliability requirement for
  file operations, and where the tests live.
- [performance.md](performance.md) — the standing speed requirement, the
  measured baseline, and what is deliberately still slow.
- [future-improvements.md](future-improvements.md) — gaps deliberately left
  open, each with its reason and the phase it belongs to.
- [good-development-practices.md](good-development-practices.md) —
  working rules, workflow, and the four-phase cycle (adopted from the
  Chimera project); read before starting non-trivial work.

## Subdirectories

- [plans/](plans/2026-08-28-tc-clone-design.md) — active plan documents;
  finished plans move to `plans/archive/`. Currently active:
  [2026-08-28-tc-clone-design.md](plans/2026-08-28-tc-clone-design.md)
  (v1 design: scope, architecture tc-core + tc-app, data flow, phases).
  Implemented and archived:
  [archive/2026-08-28-phase1-walking-skeleton.md](plans/archive/2026-08-28-phase1-walking-skeleton.md)
  (design phase 1: workspace, read-only VFS, listing model, dual-pane window,
  keyboard navigation) and
  [archive/2026-08-28-phase2-core-ops.md](plans/archive/2026-08-28-phase2-core-ops.md)
  (design phase 2: mutating VFS surface, the copy/move/delete/mkdir job
  engine, the background queue, progress and conflict dialogs) and
  [archive/2026-08-28-phase3-polish-browsing.md](plans/archive/2026-08-28-phase3-polish-browsing.md)
  (design phase 3: selection, quick filter, sorting, attributes, config
  persistence, drive bar) and
  [archive/2026-08-28-phase3b-keys-and-keymap.md](plans/archive/2026-08-28-phase3b-keys-and-keymap.md)
  (inserted between design phases 3 and 4: the rest of Total Commander's
  selection keys, the pane commands, and a configurable keymap).
  and
  [archive/2026-08-28-phase3c-command-line.md](plans/archive/2026-08-28-phase3c-command-line.md)
  (the command line and `Ctrl+↓` for its history — `Ctrl+↑` needs tabs, which
  stay out of v1).
  and
  [archive/2026-08-28-phase4-viewer.md](plans/archive/2026-08-28-phase4-viewer.md)
  (design phase 4: the F3 viewer and the F4 editor hook).
- [skills/CLAUDE.md](skills/CLAUDE.md) — focused working-rule files
  (trigger table in the root CLAUDE.md).

## Conventions

- Everything in this repository is written in English — docs, code,
  comments, commit messages.
- Plan documents go to `docs/plans/` with a `YYYY-MM-DD-` prefix and a
  Status header, and move to `docs/plans/archive/` when done (skill
  [10](skills/10-plan-lifecycle.md)).
- New doc file: one topic per file, add it to this index (skill
  [29](skills/29-one-topic-per-doc.md)).
