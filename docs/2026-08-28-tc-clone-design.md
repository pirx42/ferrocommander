# Design: Total Commander Clone for Linux

*Status: draft for review — 2026-08-28*
*(working name: "Ferrocommander" — pick whatever you like)*

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

## 6. Milestones

1. **Walking skeleton** — workspace, local VFS, one window with two panes listing
   directories, Tab/cursor navigation.
2. **Core ops** — F5/F6/F7/F8 synchronous first, then the job queue + progress +
   conflicts (background operations).
3. **Polish browsing** — sorting, selection commands, Ctrl+S filter, drive/mount
   bar, hidden files, config persistence.
4. **Viewer** — F3 text/hex viewer, F4 external editor hook.
5. **Search & multi-rename** — Alt+F7 dialog + streaming results; Ctrl+M tool.
6. **Archives** — archive VFS read, then pack/unpack via the same copy pipeline.

Each milestone leaves a usable, testable program.
