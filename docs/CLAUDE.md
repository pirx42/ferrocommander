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
  columns, active-pane marking, the build stamp in the title, the GTK
  version floor, and the renderer the Windows build has to be told to use.
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
- [clipboard.md](clipboard.md) — `Ctrl+C`/`X`/`V`: why it is the system
  clipboard and never a buffer of our own, what the three formats carry, and
  what the encoding has to get right.
- [config.md](config.md) — the settings file: where it lives, why it is
  written as it changes rather than on exit, why the write is atomic, and why
  nothing about it may stop the program starting.
- [packaging.md](packaging.md) — the Ubuntu `.deb` and the Windows zip: what
  each holds, how their version ties to the title bar, why the runtime
  dependencies of both are derived rather than listed, and why the logic is in
  scripts rather than in the workflow.
- [windows.md](windows.md) — building, running, packaging and testing on
  Windows: the MSYS2 toolchain, the two failures that announce themselves as
  something else, the renderer the app has to be told to use, and what the
  suite can and cannot do there.
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
  [10](skills/10-plan-lifecycle.md)).

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
  | [favourite directories](plans/archive/2026-08-29-favourite-directories.md) | Total Commander's `Ctrl+D` hotlist: the `[[favourites]]` setting, the window that edits its own list, and what the probes were worth |
  | [branch view](plans/archive/2026-08-29-branch-view.md) | Total Commander's `Ctrl+B`: the walk, the listing that is not a mode, the two keys that read a row's name as something to write, and three tests that were wrong before they were right |
  | [folder sizes](plans/archive/2026-08-29-folder-sizes.md) | Total Commander's `Alt+Shift+Enter`: the scan, the size that goes where the status total and the sort already look — and the watcher bug that made the feature not work at all |
  | [an Ubuntu package](plans/archive/2026-08-29-ubuntu-package.md) | the first CI this repository has had: a `.deb` built and published on every commit to `main`, why the logic is in a script rather than the workflow, and the two bugs writing the README found |
  | [type-ahead and the clipboard](plans/archive/2026-08-30-type-ahead-and-clipboard.md) | letters that search the rows, the `→` that had to replace them in the command line first, and `Ctrl+X`/`C`/`V` through the *system* clipboard — plus the second test this repository has caught proving nothing |
  | [field report, round one](plans/archive/2026-08-29-field-report-round-one.md) | twelve findings from the first person to *use* build 132 rather than test it: ten fixed, two withdrawn, one that did not reproduce and is recorded as not reproducing — plus the three bugs the work found in itself |
  | [documentation drift and the hijacked keys](plans/archive/2026-08-30-documentation-drift-and-the-hijacked-keys.md) | what reading every document against the code found: thirty-six findings, three of them the code being wrong — including a regression that sent `Ctrl+V` in the command line to the file copier — plus the action-name table nobody could find, now generated, and the type-ahead measurement that was owed |
  | [page keys in the pane](plans/archive/2026-08-30-page-keys-in-the-pane.md) | `Page Up`/`Page Down` bound as ordinary cursor keys — the type-ahead bug that found the hole in `adopt_selection`'s contract, a design whose stated reason had expired, and a scroll reproduced by measuring the widget rather than guessing at it |
  | [adoption without remembering](plans/archive/2026-08-30-adoption-without-remembering.md) | `adopt_selection` became a signal instead of a call somebody has to know to make — no live bug behind it, only a rule that had been kept perfectly and failed anyway twice in a week; plus the test that stopped covering its subject without going red |
  | [space counts a folder](plans/archive/2026-08-30-space-counts-a-folder.md) | Total Commander's `Space`, which counts a folder as it marks it, so the status line's marked-bytes total stops reading `0 B` — plus two benchmarks that flattered themselves, and the stall the second one hid |
  | [macOS groundwork](plans/archive/2026-08-30-macos-groundwork.md) | every part of the macOS port a Linux box can do and check — the gate's aarch64 cross-check, the platform branches with their rules fixture-tested here, `cmd` made expressible and the layer shipped dormant as data — plus the trash suite that would have emptied fixtures into a real Mac's bin |

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
