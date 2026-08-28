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
  columns, active-pane marking, and the GTK version floor.
- [keymap.md](keymap.md) — what each key does, how bindings are looked up,
  and what happens when a directory cannot be entered.
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
  keyboard navigation).
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
