# Design: Total Commander Clone for Linux

**Status:** Implemented — all seven phases, closed by the phase 7 audit

*2026-08-28 — name: "FerroCommander"*

> Process: this plan follows [good-development-practices.md](../../good-development-practices.md)
> and the [skills](../../skills/CLAUDE.md) — see section 7 for how the
> conventions map onto the phases.

## 1. Goals & Scope (v1)

A keyboard-first, dual-pane orthodox file manager for Linux, built in **Rust + GTK4**.

### In scope

- **Dual-pane browsing** — two independent panes, active/inactive state, path bar,
  drive/mount selector, hidden-file toggle, sortable columns (name, ext, size, date,
  attributes).
- **Core operations** — F5 copy, F6 move/rename, F7 mkdir, F8/Del delete (to trash by
  default, Shift+Del permanent), F3 view, F4 external edit, Tab to switch panes,
  Insert/Space selection, Num +/− pattern select.
- **Ctrl+S quick filter** — type-ahead narrowing of the current file list.
- **Background file operations** — queued jobs with progress window, pause/cancel,
  conflict resolution (overwrite / skip / rename / apply-to-all).
- **Search (Alt+F7)** — by name pattern and file content, streaming results that can
  feed a pane or a jump list.
- **Multi-rename tool (Ctrl+M)** — pattern-based renaming with counters, name/ext
  placeholders, search & replace, live preview, undo.
- **Archives as folders** — enter zip/tar/tar.gz/tar.zst like directories; copy in/out
  = pack/unpack (zip + tar family in v1, 7z later).
- **Simple internal viewer (F3)** — text with encoding detection + hex mode; F4
  launches a configurable external editor.

### Non-functional requirement: speed (owner spec, 2026-08-28)

The app must be fast, and where a decision trades speed against a prettier
surface, a larger feature set or a tidier abstraction, speed wins — the
audience is hard-core Total Commander users. This outranks the scope list
above: a feature that cannot be made fast is a feature that waits.
Details and the measured baseline: [performance.md](../../performance.md).

### Non-functional requirement: reliable operations (owner spec, 2026-08-28)

The operations must be very reliable — rather more tests than too few. The
failure that matters is not a crash but a job that reports success over a file
it destroyed. Details: [reliability.md](../../reliability.md).

### Explicitly out of v1

Tabs, FTP/SFTP, plugins, directory sync, internal editor, thumbnails view,
custom columns.

## 2. Architecture

Cargo workspace with two layers:

```
tc-core (lib crate, no GTK dependency)
├── vfs        — VirtualFs trait: list, stat, open/read/write, rename, mkdir, remove
│   ├── local  — std::fs/rustix implementation
│   └── archive— zip/tar backends behind the same trait (read + write-on-close)
├── ops        — file-operation engine: Job (Copy/Move/Delete/Pack…), JobQueue,
│                worker threads, progress events + conflict prompts over channels
├── search     — walker + name-glob + content matcher, streaming results
├── rename     — multi-rename rule engine (pure functions: rules × names → preview)
└── listing    — directory model: entries, sort orders, filter (Ctrl+S), selection

tc-app (bin crate, GTK4 via gtk4-rs)
├── main window: two PaneViews (GtkColumnView), command bar, function-key bar
├── dialogs: progress/queue, conflict, search, multi-rename, options
├── viewer window (F3): text/hex
└── glue: keymap, config (TOML in ~/.config), job events → UI via async channel
```

**Key boundary:** the UI never touches the filesystem directly — everything goes
through `tc-core`. That makes archive-as-folder free at the UI level (a pane just
shows a `VirtualFs`), lets copy *between* local and archive fall out of one code
path, and makes the whole engine testable with plain `cargo test` on tempdirs.

**Concurrency model:** the UI thread owns GTK. Each job runs on a worker thread;
progress/conflict events flow through an `async-channel` polled on the GLib main
loop. Conflicts block the worker until the UI answers (or an apply-to-all policy
is set).

## 3. Data Flow Examples

- **Copy (F5):** active pane's selection + target pane's dir → `ops::Job::Copy` →
  queue → worker walks sources via source `VirtualFs`, writes via target
  `VirtualFs` → progress events update the progress dialog → completion event
  triggers target-pane refresh.
- **Ctrl+S:** keystrokes edit a filter string on the pane's `listing`; the
  ColumnView shows the filtered model; Esc clears; Enter jumps to first match.
- **Enter on foo.zip:** the pane swaps its `VirtualFs` from local to
  `ArchiveFs(foo.zip)` with path `/`; Backspace at archive root returns to the
  local filesystem.

## 4. Error Handling & Safety

- All operations report per-file errors into the job log; a failed file never
  silently aborts the batch (TC behavior: skip/retry/abort prompt).
- Delete defaults to freedesktop trash (via the `trash` crate); permanent delete
  requires Shift+Del plus confirmation.
- Moves across devices degrade to copy+delete, with rollback of partially copied
  files on cancel.
- Config and state (window geometry, last dirs, sort orders) are saved atomically.

## 5. Testing

- `tc-core` is fully unit/integration tested headless: tempdir fixtures, archive
  round-trips, conflict scenarios, rename-engine table tests.
- UI: kept thin enough that manual testing plus a few `gtk::test` smoke tests
  suffice for v1.

## 6. Phases

Phase 0 (coverage pre-check per skill
[43](../../skills/43-coverage-before-implementation.md)) is not applicable —
this is a greenfield project with no existing code to characterize. Instead,
every phase ships its tests alongside the code (skill
[23](../../skills/23-tests-accompany-commits.md)).

1. **Walking skeleton** — workspace, local VFS, one window with two panes listing
   directories, Tab/cursor navigation. ✅ **Implemented** (commits
   525b96b…64780ac); plan archived at
   [2026-08-28-phase1-walking-skeleton.md](2026-08-28-phase1-walking-skeleton.md).
   Subsystem docs: [vfs.md](../../vfs.md), [listing.md](../../listing.md),
   [ui-shell.md](../../ui-shell.md), [keymap.md](../../keymap.md).
2. **Core ops** — F5/F6/F7/F8 synchronous first, then the job queue + progress +
   conflicts (background operations). Conservation-invariant tests for
   copy/move/delete (byte sums, file counts, rollback on cancel — skill
   [52](../../skills/52-test-conservation-invariants.md)). ✅ **Implemented**
   (commits 2ac3018…6d37cd2); plan archived at
   [2026-08-28-phase2-core-ops.md](2026-08-28-phase2-core-ops.md).
   Subsystem docs: [ops.md](../../ops.md), plus the write half of
   [vfs.md](../../vfs.md) and the operations half of [keymap.md](../../keymap.md)
   and [ui-shell.md](../../ui-shell.md).
3. **Polish browsing** — sorting, selection commands, Ctrl+S filter, drive/mount
   bar, hidden files, config persistence. *(~1–2 days)*
   ✅ **Implemented** (commits e3c44b0…e8b735f); plan archived at
   [2026-08-28-phase3-polish-browsing.md](2026-08-28-phase3-polish-browsing.md).
   Subsystem docs: [config.md](../../config.md), plus the selection and filter
   halves of [listing.md](../../listing.md) and [keymap.md](../../keymap.md).
3b. **Keys and keymap** — the rest of Total Commander's selection keys
   (`Shift`+cursor, `Ctrl+Num −`, `Alt+Num ±`, `Num /`, `Shift+Num *`), the
   two-pane commands (`Ctrl+←/→`, `Ctrl+U`), and a configurable keymap laid
   over the defaults. Inserted here rather than deferred, because it finishes
   phase 3's territory. ✅ **Implemented**; plan archived at
   [2026-08-28-phase3b-keys-and-keymap.md](2026-08-28-phase3b-keys-and-keymap.md).
   Subsystem docs: the marking half of [keymap.md](../../keymap.md), the
   selection primitives in [listing.md](../../listing.md), and the `[keys]` table
   in [config.md](../../config.md).
3c. **Command line** — an entry at the bottom running `$SHELL -c` in the
   active pane's directory, with `Ctrl+↓` / `Alt+F8` for its history and
   `Ctrl+Enter` to insert the name under the cursor. Not in the original
   scope; added because `Ctrl+↓` could not be built without it. ✅
   **Implemented**; plan archived at
   [2026-08-28-phase3c-command-line.md](2026-08-28-phase3c-command-line.md),
   subsystem doc [command-line.md](../../command-line.md).
4. **Viewer** — F3 text/hex viewer, F4 external editor hook. ✅
   **Implemented**; plan archived at
   [2026-08-28-phase4-viewer.md](2026-08-28-phase4-viewer.md),
   subsystem doc [viewer.md](../../viewer.md).
5. **Search & multi-rename** — Alt+F7 dialog + streaming results; Ctrl+M tool
   with table-driven rename-engine tests. ✅ **Implemented**; plan archived at
   [2026-08-28-phase5-search-and-rename.md](2026-08-28-phase5-search-and-rename.md),
   subsystem docs [search.md](../../search.md) and
   [multi-rename.md](../../multi-rename.md).
6. **Archives** — archive VFS read, then pack/unpack via the same copy pipeline;
   pack→unpack roundtrip invariant tests. ✅ **Implemented**; plan archived at
   [2026-08-28-phase6-archives.md](2026-08-28-phase6-archives.md),
   subsystem doc [archives.md](../../archives.md) — which also carries the report
   card this design's central claim earned.
7. **Refactoring audit** — final phase per skill
   [49](../../skills/49-final-phase-refactoring-audit.md): audit the whole
   implementation for architecture drift and redundancy
   ([44](../../skills/44-no-redundancy.md)), apply corrections. The plan counts
   as "Implemented" only after this phase. *(~1 day)* Plan:
   [2026-08-29-phase7-refactoring-audit.md](2026-08-29-phase7-refactoring-audit.md).

Each phase leaves a usable, testable program. Effort estimates are already
calibrated per skill [45](../../skills/45-calibrate-effort-estimates.md)
(feature factor applied to raw intuition).

## 7. Process conventions

How the imported practices apply to this plan:

- **One phase = one commit series, merged as a unit** — each phase lands as
  its own conventional commits (skills [11](../../skills/11-multi-phase-commits.md),
  [31](../../skills/31-conventional-commit.md),
  [32](../../skills/32-commit-per-step.md)).
- **Green gate before every commit** — `cargo fmt --check`, `clippy -D warnings`,
  `cargo test --workspace`, release build
  (skill [25](../../skills/25-green-suite-before-commit.md)).
- **Tests accompany every feature commit** ([23](../../skills/23-tests-accompany-commits.md)),
  written as behavioral tests ([26](../../skills/26-behavioral-tests.md));
  engine code (ops/vfs/archive) additionally gets conservation-invariant
  tests and mutation probes
  ([52](../../skills/52-test-conservation-invariants.md),
  [59](../../skills/59-mutation-probe-over-coverage-percent.md)).
- **Docs move in the same commit as the code** ([28](../../skills/28-docs-in-same-commit.md));
  subsystem docs (one topic per file, [29](../../skills/29-one-topic-per-doc.md))
  grow under `docs/` as the subsystems appear, with design "why"s recorded
  ([30](../../skills/30-document-the-why.md)).
- **Constants** — no magic values; per-subsystem constants modules
  ([16](../../skills/16-no-magic-values.md), [17](../../skills/17-centralize-constants.md)).
- **Plan lifecycle** — this document keeps its Status header current
  (Draft → In progress → Implemented → Archived) and moves to
  `docs/plans/archive/` after substance has been extracted into subsystem
  docs (skill [10](../../skills/10-plan-lifecycle.md)).

## 8. Outcome

**Implemented**, seven phases plus two inserted ones (3b keys and keymap, 3c
the command line), from `525b96b` to the phase 7 audit. 13 800 lines of code,
9 500 of tests, 513 of them, and a green gate — `fmt`, `clippy -D warnings` on
both targets, the whole suite, and a release build — before every commit.

### What the scope list got

Everything in § 1 except three things, each named where it belongs rather than
quietly dropped:

- **`.tar.zst`** is not read or written. Zstd is another compressor and another
  dependency, and nothing in the phase turned up a reason for it beyond the
  scope line. [future-improvements.md](../../future-improvements.md).
- **A job cannot be paused**, only cancelled. Pause was one word in the scope
  list; what it means for a job holding an open file across a conflict prompt
  is not one word, and nothing asked for it.
- **`Ctrl+↑`** wants tabs, which are explicitly out of v1. Noted when the
  command line shipped.

### What the architecture claim was worth

The design's central bet was that a `VirtualFs` between the UI and the disk
would make archives browsable folders for free. It was collected and counted
in phase 6: **16 lines outside `tc-core::archive`** for a pane that lists a
zip, a viewer that reads inside one and a search that walks it; **0** for the
two formats after the first. The full report card is in
[archives.md](../../archives.md).

The price it did not advertise is also counted there: 204 lines, all shell, for
*entering* and leaving one — a read has to arrive with the backend it was read
from, a pane has to remember what it walked into, and a listing has to be able
to offer a `..` row it did not earn.

### What the phases cost, against their estimates

Estimates were corrected by skill 45's factor before work started, and the
factor held: the phases with a clear shape (4, 5, 6) landed close, and the two
that were inserted mid-plan (3b, 3c) existed because a key could not be built
without them rather than because they were foreseen.

### What this way of working actually caught

Six defects that a green suite did not, each found by asking a question rather
than by reading code — a symlink that made a copy do nothing, a copy onto
itself that emptied the file, a cancel rollback that could be deleted with all
tests passing, a conflict dialog answerable only with the mouse, permissions
silently dropped from every copy, and a move that would have handed an
archive's `/packed.txt` to the local filesystem. The list, and how each was
found, is in [reliability.md](../../reliability.md).

The last one is the pattern worth keeping: **a rule written when there was one
of something is wrong when there are two**, and no existing test can see it,
because with one of something every wrong answer is also the right one. Phase 7
asked that question of every such rule and found four more.
