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
- [search.md](search.md) — Alt+F7: why the walk streams and stops, and what
  choosing a result does.
- [archives.md](archives.md) — archives as directories: why the backend is
  read-only, what holds an archive open, and why an entry cannot name a path
  outside it.
- [multi-rename.md](multi-rename.md) — Ctrl+M: why the preview is the
  rename, what the rules are, and how one batch of undo works.
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

- [plans/](plans/archive/) — plan documents. A plan is written before the work,
  keeps its Status header current, and moves to `plans/archive/` with an
  outcome section when it is done (skill
  [10](skills/10-plan-lifecycle.md)). **All of them are archived**: v1 is
  implemented.

  | Plan | What it built |
  |---|---|
  | [tc-clone-design](plans/archive/2026-08-28-tc-clone-design.md) | v1 itself: scope, the tc-core/tc-app split, the phases, and what the whole thing cost |
  | [phase 1 — walking skeleton](plans/archive/2026-08-28-phase1-walking-skeleton.md) | the workspace, a read-only VFS, the listing model, two panes, keyboard navigation |
  | [phase 2 — core ops](plans/archive/2026-08-28-phase2-core-ops.md) | the mutating VFS, the copy/move/delete/mkdir engine, the background queue, progress and conflicts |
  | [phase 3 — polish browsing](plans/archive/2026-08-28-phase3-polish-browsing.md) | selection, the quick filter, sorting, attributes, the settings file, the drive bar |
  | [phase 3b — keys and keymap](plans/archive/2026-08-28-phase3b-keys-and-keymap.md) | the rest of Total Commander's marking keys, the two-pane commands, and a configurable keymap |
  | [phase 3c — command line](plans/archive/2026-08-28-phase3c-command-line.md) | the entry at the bottom and `Ctrl+↓` for its history |
  | [phase 4 — viewer](plans/archive/2026-08-28-phase4-viewer.md) | `F3`, `read_at`, and the `F4` editor hook |
  | [phase 5 — search and rename](plans/archive/2026-08-28-phase5-search-and-rename.md) | `Alt+F7` with streaming results, and the `Ctrl+M` tool |
  | [phase 6 — archives](plans/archive/2026-08-28-phase6-archives.md) | archives as directories, unpacking through the copy engine, `Alt+F5` to pack |
  | [phase 7 — the audit](plans/archive/2026-08-29-phase7-refactoring-audit.md) | the closing sweep: rules that drifted, files whose names stopped describing them, the documentation pass |
  | [architecture and redundancy review](plans/archive/2026-08-29-architecture-and-redundancy-review.md) | what a whole-codebase read found after phase 7: nine findings, all implemented, plus what it checked and found sound, the one candidate finding that did not survive checking, and a § 7 report card on where the work deviated from the proposal |

  No plan is currently active.

  3b and 3c were not foreseen. Each exists because a key in the phase before
  it could not be built without them.

- [skills/CLAUDE.md](skills/CLAUDE.md) — focused working-rule files
  (trigger table in the root CLAUDE.md).

Code directories carry their own `CLAUDE.md` beside the code:
[crates/CLAUDE.md](../crates/CLAUDE.md) →
[tc-core/src/](../crates/tc-core/src/CLAUDE.md) →
[vfs/](../crates/tc-core/src/vfs/CLAUDE.md),
[listing/](../crates/tc-core/src/listing/CLAUDE.md),
[ops/](../crates/tc-core/src/ops/CLAUDE.md),
[archive/](../crates/tc-core/src/archive/CLAUDE.md); and
[tc-app/src/](../crates/tc-app/src/CLAUDE.md).

## Conventions

- Everything in this repository is written in English — docs, code,
  comments, commit messages.
- Plan documents go to `docs/plans/` with a `YYYY-MM-DD-` prefix and a
  Status header, and move to `docs/plans/archive/` when done (skill
  [10](skills/10-plan-lifecycle.md)).
- New doc file: one topic per file, add it to this index (skill
  [29](skills/29-one-topic-per-doc.md)).
- After moving or renaming anything: `python3 scripts/check-links.py`. Every
  relative link in the repository, checked in a second. The phase 7 audit
  found eleven broken at once, all of them made by archiving a plan.
