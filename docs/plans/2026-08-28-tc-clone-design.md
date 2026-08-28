# Design: Total Commander Clone for Linux

Status: In Progress — phases 1, 2 and 3 implemented

*2026-08-28 — working name: "Ferrocommander" (pick whatever you like)*

> Process: this plan follows [good-development-practices.md](../good-development-practices.md)
> and the [skills](../skills/CLAUDE.md) — see section 7 for how the
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
Details and the measured baseline: [performance.md](../performance.md).

### Non-functional requirement: reliable operations (owner spec, 2026-08-28)

The operations must be very reliable — rather more tests than too few. The
failure that matters is not a crash but a job that reports success over a file
it destroyed. Details: [reliability.md](../reliability.md).

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
[43](../skills/43-coverage-before-implementation.md)) is not applicable —
this is a greenfield project with no existing code to characterize. Instead,
every phase ships its tests alongside the code (skill
[23](../skills/23-tests-accompany-commits.md)).

1. **Walking skeleton** — workspace, local VFS, one window with two panes listing
   directories, Tab/cursor navigation. ✅ **Implemented** (commits
   525b96b…64780ac); plan archived at
   [archive/2026-08-28-phase1-walking-skeleton.md](archive/2026-08-28-phase1-walking-skeleton.md).
   Subsystem docs: [vfs.md](../vfs.md), [listing.md](../listing.md),
   [ui-shell.md](../ui-shell.md), [keymap.md](../keymap.md).
2. **Core ops** — F5/F6/F7/F8 synchronous first, then the job queue + progress +
   conflicts (background operations). Conservation-invariant tests for
   copy/move/delete (byte sums, file counts, rollback on cancel — skill
   [52](../skills/52-test-conservation-invariants.md)). ✅ **Implemented**
   (commits 2ac3018…6d37cd2); plan archived at
   [archive/2026-08-28-phase2-core-ops.md](archive/2026-08-28-phase2-core-ops.md).
   Subsystem docs: [ops.md](../ops.md), plus the write half of
   [vfs.md](../vfs.md) and the operations half of [keymap.md](../keymap.md)
   and [ui-shell.md](../ui-shell.md).
3. **Polish browsing** — sorting, selection commands, Ctrl+S filter, drive/mount
   bar, hidden files, config persistence. *(~1–2 days)*
   ✅ **Implemented** (commits e3c44b0…e8b735f); plan archived at
   [archive/2026-08-28-phase3-polish-browsing.md](archive/2026-08-28-phase3-polish-browsing.md).
   Subsystem docs: [config.md](../config.md), plus the selection and filter
   halves of [listing.md](../listing.md) and [keymap.md](../keymap.md).
4. **Viewer** — F3 text/hex viewer, F4 external editor hook. *(~1 day)*
5. **Search & multi-rename** — Alt+F7 dialog + streaming results; Ctrl+M tool
   with table-driven rename-engine tests. *(~1–2 days)*
6. **Archives** — archive VFS read, then pack/unpack via the same copy pipeline;
   pack→unpack roundtrip invariant tests. *(~2 days)*
7. **Refactoring audit** — final phase per skill
   [49](../skills/49-final-phase-refactoring-audit.md): audit the whole
   implementation for architecture drift and redundancy
   ([44](../skills/44-no-redundancy.md)), apply corrections. The plan counts
   as "Implemented" only after this phase. *(~1 day)*

Each phase leaves a usable, testable program. Effort estimates are already
calibrated per skill [45](../skills/45-calibrate-effort-estimates.md)
(feature factor applied to raw intuition).

## 7. Process conventions

How the imported practices apply to this plan:

- **One phase = one commit series, merged as a unit** — each phase lands as
  its own conventional commits (skills [11](../skills/11-multi-phase-commits.md),
  [31](../skills/31-conventional-commit.md),
  [32](../skills/32-commit-per-step.md)).
- **Green gate before every commit** — `cargo fmt --check`, `clippy -D warnings`,
  `cargo test --workspace`, release build
  (skill [25](../skills/25-green-suite-before-commit.md)).
- **Tests accompany every feature commit** ([23](../skills/23-tests-accompany-commits.md)),
  written as behavioral tests ([26](../skills/26-behavioral-tests.md));
  engine code (ops/vfs/archive) additionally gets conservation-invariant
  tests and mutation probes
  ([52](../skills/52-test-conservation-invariants.md),
  [59](../skills/59-mutation-probe-over-coverage-percent.md)).
- **Docs move in the same commit as the code** ([28](../skills/28-docs-in-same-commit.md));
  subsystem docs (one topic per file, [29](../skills/29-one-topic-per-doc.md))
  grow under `docs/` as the subsystems appear, with design "why"s recorded
  ([30](../skills/30-document-the-why.md)).
- **Constants** — no magic values; per-subsystem constants modules
  ([16](../skills/16-no-magic-values.md), [17](../skills/17-centralize-constants.md)).
- **Plan lifecycle** — this document keeps its Status header current
  (Draft → In progress → Implemented → Archived) and moves to
  `docs/plans/archive/` after substance has been extracted into subsystem
  docs (skill [10](../skills/10-plan-lifecycle.md)).
