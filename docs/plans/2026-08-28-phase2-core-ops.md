# Phase 2 Implementation Plan — Core File Operations

Status: In Progress — sub-phases 0, A, B, C, D, E done

*2026-08-28 — implements phase 2 of
[2026-08-28-tc-clone-design.md](2026-08-28-tc-clone-design.md).*

> Process: [good-development-practices.md](../good-development-practices.md),
> skill triggers in the root [CLAUDE.md](../../CLAUDE.md). Every sub-phase
> below is one commit series, green before it lands
> (skills [11](../skills/11-multi-phase-commits.md),
> [25](../skills/25-green-suite-before-commit.md)).

## 1. Scope

**One sentence:** the panes stop being read-only — F5 copies, F6 moves or
renames, F7 creates a directory and F8/Del deletes, executed by a background
job engine in `tc-core` that reports progress, asks about conflicts, and can
be cancelled without leaving half-written files behind.

### In scope

- **`tc-core::vfs`** — the mutating half of `VirtualFs`: `create_dir`,
  `remove_dir`, `remove_file`, `rename`, `open_read`, `create_file`,
  `set_modified`, `trash`. Implemented by `LocalFs`, with the error variants
  the operations actually produce.
- **`tc-core::ops`** — the operation engine: `Job` (`Copy`, `Move`, `Delete`,
  `CreateDir`), a scan pass that turns sources into a flat task list with
  honest totals, an execute pass that streams bytes through the VFS, a
  per-file error log, conflict resolution, cancellation with rollback of the
  file in flight.
- **`tc-core::ops::queue`** — `JobQueue`: jobs run one at a time on a worker
  thread, progress and conflict requests flow out over channels, the answer
  flows back in. Headless-testable with a scripted resolver — no GTK, no
  main loop.
- **`tc-app`** — F5 / F6 / F7 / F8 / Del / Shift+Del / Shift+F8 bindings, the
  target dialog (F5/F6), the name dialog (F7), the delete confirmation, the
  progress window with a cancel button, and the conflict dialog
  (overwrite / skip / rename / abort, each with *apply to all*).
- Both panes refresh when a job finishes; a pane whose own directory was
  deleted falls back to the nearest surviving ancestor instead of showing an
  error.

### Explicitly out of scope (later phases of the design doc)

Multi-selection with Insert/Space and Num +/− (phase 3) — every operation in
this phase acts on the **cursor row**, though the engine's API already takes
a list of sources so phase 3 changes the call site and nothing else · sorting
UI, Ctrl+S filter, drive bar, config persistence (phase 3) · F3 viewer, F4
editor (phase 4) · search, multi-rename (phase 5) · archives, pack/unpack
(phase 6).

Also out: the function-key bar (see section 6), mouse and drag-and-drop,
reordering or pausing a queued job (the queue runs FIFO and only the running
job can be cancelled), TC's Shift+F5 same-directory duplicate, and Alt+F5
pack.

**The read-only invariant of phase 1 ends here.** From sub-phase A on,
`tc-core` can destroy data. Two consequences are treated as requirements, not
as good intentions: every destructive path is exercised against a tempdir and
never against a real home directory, and delete goes to the freedesktop trash
unless the user explicitly asked for a permanent one.

## 2. Coverage pre-check (skill [43](../skills/43-coverage-before-implementation.md))

Phase 2 is almost entirely additive — new modules, new trait methods — so the
pre-check is short. What matters is the handful of *existing* contracts the
phase relies on or changes.

| Existing item | Coverage today | Verdict |
|---|---|---|
| `LocalFs::read_dir` / `stat` | 15 integration tests (`tests/local_fs.rs`), invariant-style over tempdir fixtures | covered — phase 2 only adds methods beside them |
| `VfsError::from(io::Error)` | table test over the three modelled kinds + the `Io` fallback | covered; the table **grows** in sub-phase A with the new variants |
| `Listing::reload` | two tests: cursor keeps its entry when the entry survives, clamps when it is gone | covered — this is the path every finished job calls |
| `Listing::load` on an unreadable directory | `PaneView::new` handles the error, no test | **reachable gap** — phase 2 makes it common (a pane standing in a directory a job just deleted) |
| `keymap::action_for` | 4 tests, one of which asserts that `F5` and `Delete` are unbound | covered, and **must change** in sub-phase D — see below |
| `PaneView::refresh` / `navigate_to` | no automated coverage (known gap, [future-improvements.md](../future-improvements.md)) | unchanged by this plan; the new dialog wiring inherits the same gap |

**Announced test change (skill [24](../skills/24-no-silent-test-changes.md)).**
`an_unbound_key_triggers_nothing` currently pins `Key::F5` and `Key::Delete`
as unbound. Sub-phase D binds both, so that test has to lose those two keys.
This is recorded here rather than being quietly edited during the
implementation: the assertion is not wrong today, it is scheduled to become
wrong. The test keeps `Key::Escape` and `Key::a`, which stay unbound, so the
"unbound keys do nothing" contract survives with a smaller witness set.

**Phase 0 therefore has exactly one job:** characterize what a pane does when
the directory under it disappears, *before* the engine can make that happen.
Details in sub-phase 0.

## 3. Environment gate (blocking, before sub-phase 0)

Phase 1 was implemented on a box with rustc **1.98.0** and GTK **4.22.4**.
The container this plan was written in has neither:

```
$ cargo check -p tc-core
error: rustc 1.94.1 is not supported by the following package:
  tc-core@0.1.0 requires rustc 1.98
$ pkg-config --modversion gtk4
Package gtk4 was not found in the pkg-config search path
```

So **the green gate cannot run here**, and neither can a single line of this
plan be implemented here. Before sub-phase 0:

```bash
rustup update stable                        # ≥ 1.98, the workspace MSRV
sudo apt install -y build-essential pkg-config libgtk-4-dev
```

Verification: `cargo check --workspace` succeeds and
`pkg-config --modversion gtk4` prints ≥ 4.12 (the floor the cursor-scrolling
API forced in phase 1).

The three facts that *were* verifiable here, and that the plan below depends
on, were checked rather than assumed
(skill [65](../skills/65-verify-or-ask-never-assume.md)):

- `io::ErrorKind::CrossesDevices`, `AlreadyExists`, `DirectoryNotEmpty` and
  `IsADirectory` all compile on stable — sub-phase A's error mapping needs
  no nightly and no string matching.
- `trash` is at **5.2.6**, MSRV 1.85, and its default features pull in
  `chrono` for trash-*listing*, which this project does not do.
- `async-channel` is at **2.5.0**.

## 4. Sub-phases

Each sub-phase is independently green (`cargo fmt --all -- --check`,
`cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace`, `cargo build --release`, plus the Windows
cross-target check on `tc-core`) and ends in a conventional commit
(skill [31](../skills/31-conventional-commit.md)).

### 0 — Coverage pre-check: a pane whose directory vanishes

*Commit:* `test(listing): pre-impl characterization of loading a lost directory`

Written **before** sub-phase A, against today's behavior, so the change in
sub-phase D is provable rather than asserted.

- Characterize `Listing::load` against a directory that was removed after the
  path was taken: it returns `Err(NotFound)` and there is no fallback of any
  kind. That is the current contract, and the test says so.
- Characterize the second way a job can invalidate a pane's directory — the
  name survives but is no longer a directory — as `Err(NotADirectory)`, so
  the failure modes are told apart before code starts branching on them.
- Characterize the half sub-phase D will lean on: walking up from a destroyed
  path reaches something loadable, the root at the very latest.

*Exit criterion:* the tests are green on unmodified phase-1 code. Nothing else
in this sub-phase — no production change.

**Deviation from this sketch, implemented deliberately.** The second test was
sketched as an *unreadable* directory (`#[cfg(unix)]`, mode `0o000`). The
implementation environment runs as root, and root bypasses mode bits — the
test would assert `PermissionDenied` and get a successful listing. Replacing
it with the directory-replaced-by-a-file case keeps the point (two distinct
failure modes, told apart before the code branches) and covers something a
file operation can actually *do*, which a permission change is not.

### A — `tc-core::vfs`: the mutating surface

*Commit:* `feat(vfs): mutating VirtualFs surface with local backend`

```rust
pub trait VirtualFs: Send + Sync {
    // phase 1
    fn read_dir(&self, path: &VfsPath) -> Result<Vec<Entry>, VfsError>;
    fn stat(&self, path: &VfsPath) -> Result<Entry, VfsError>;
    // phase 2
    fn create_dir(&self, path: &VfsPath) -> Result<(), VfsError>;
    fn remove_dir(&self, path: &VfsPath) -> Result<(), VfsError>;   // must be empty
    fn remove_file(&self, path: &VfsPath) -> Result<(), VfsError>;
    fn rename(&self, from: &VfsPath, to: &VfsPath) -> Result<(), VfsError>;
    fn open_read(&self, path: &VfsPath) -> Result<Box<dyn Read + Send>, VfsError>;
    fn create_file(&self, path: &VfsPath) -> Result<Box<dyn Write + Send>, VfsError>;
    fn set_modified(&self, path: &VfsPath, time: SystemTime) -> Result<(), VfsError>;
    fn trash(&self, path: &VfsPath) -> Result<(), VfsError>;
}
```

- New `VfsError` variants, each with a caller that produces it in this
  phase — none added speculatively: `AlreadyExists`, `NotEmpty`,
  `IsADirectory`, `CrossDevice`. The `From<io::Error>` table grows by four
  rows and its test grows with it.
- **`remove_dir` refuses a non-empty directory on purpose.** Recursion is the
  engine's job, not the backend's: only the engine can report per-file
  progress, log a per-file error and keep going, and honour a cancel between
  two entries. A backend-side recursive delete would be a second, silent
  implementation of the same walk with none of that
  (skill [44](../skills/44-no-redundancy.md)).
- **`rename` is same-backend only** and surfaces `CrossDevice` rather than
  papering over it. The copy+delete degradation is a decision with a progress
  bar and a rollback attached, which makes it engine policy.
- **Streams, not `copy_file`.** One `Read`/`Write` pair means the copy loop
  exists once and works local→local today and local→archive in phase 6
  without a second code path. `+ Send` because the loop runs on a worker
  thread.
- **`Send + Sync` on the trait** is what lets a job hold its backend across
  threads. Phase 6's `ArchiveFs` will need interior mutability to satisfy
  `Sync`; noted in [vfs.md](../vfs.md) so it is a known constraint rather than
  a surprise.

  *Deviation, implemented deliberately:* the sketch also swapped `PaneView`'s
  `Box<dyn VirtualFs>` for an `Arc` in this commit. Deferred to sub-phase D,
  where a job first shares the handle. Until then an `Arc` has exactly one
  owner — a refcount paid for nothing, and the "API ahead of its caller"
  pattern that phase 1's audit had to correct four times. The bound itself is
  not affected: it is a bound, not a method, so it adds no untested code.
- `trash` lives on the trait, implemented by `LocalFs` via the `trash` crate
  as `trash = { version = "5", default-features = false, features = ["coinit_apartmentthreaded"] }`
  — default features are dropped because they pull `chrono` in for listing
  and restoring trash contents, which this project never does; the Windows
  COM apartment feature is kept because dropping it would silently change
  Windows behavior. No `Unsupported` variant yet: it belongs to phase 6,
  where an archive backend will be the first thing that returns it.

  *Correction found while implementing:* the trash error mapping does **not**
  belong in `local.rs`, and `CouldNotAccess` is not the variant a missing path
  produces — the freedesktop backend reports its wrapped `io::Error` instead,
  which was established by running the crate rather than by reading its
  variant list. Since that wrapping is target-dependent (Windows reports Win32
  codes and the variant does not even exist there), the mapping lives in
  `platform.rs` as a sixth platform function, which keeps `local.rs` free of
  `cfg` branches as [vfs.md](../vfs.md) promises. The crate's
  `Error::source()` is no way around the split: for the filesystem variant it
  returns the io error's own source, which is `None`.

*Tests* (tempdir fixtures, headless):
- Round trip per method: `create_dir` then `read_dir` shows it; `create_file`
  + write then `open_read` returns the same bytes; `rename` moves the name
  and conserves the byte sum of the directory; `remove_file` /`remove_dir`
  reduce the entry count by exactly one and leave the siblings' name set
  untouched.
- Error mapping table: missing → `NotFound`, existing target for `create_dir`
  → `AlreadyExists`, non-empty for `remove_dir` → `NotEmpty`,
  `remove_file` on a directory → `IsADirectory`.
- `set_modified` survives a `stat` round trip within filesystem timestamp
  resolution.
- Trash gets its **own integration test binary** (`tests/trash.rs`) holding a
  single `#[cfg(unix)]` test, because it has to point `XDG_DATA_HOME` at a
  tempdir and a process-global env var must not race parallel tests. It
  asserts the entry left the source directory *and* arrived under the
  redirected trash dir. If the crate turns out not to honour
  `XDG_DATA_HOME`, the test degrades to the source side only and the missing
  half goes to [future-improvements.md](../future-improvements.md) rather
  than being faked. *Verified: the crate honours it, so the test asserts both
  halves.* The trash **error mapping** needs no redirection — it fails while
  resolving the path — so it is tested alongside the other write calls
  instead of costing a second single-test binary.

*Two gaps this sub-phase opens, both recorded rather than hidden:* a copy
carries no permission bits (`Entry` has no mode, so `+x` is lost), and
`set_modified` cannot stamp a directory (no platform hands out a writable
handle to one). Files keep their date, which is what the date column shows.

*Docs:* [vfs.md](../vfs.md) — the full trait, the new error variants, why
`remove_dir` is non-recursive, why `rename` reports `CrossDevice`, the
`Send + Sync` constraint on future backends, and trash as a backend
capability.

### B — `tc-core::ops`: the synchronous engine

*Commit:* `feat(ops): copy, move, delete and mkdir with conflict and cancel handling`

The whole engine, still called straight from the test — no threads yet, so
its behavior is pinned before concurrency can obscure it.

- `Job` — `Copy { sources: Vec<VfsPath>, target_dir: VfsPath }`,
  `Move { .. }`, `Delete { paths: Vec<VfsPath>, mode: DeleteMode }`,
  `CreateDir { path: VfsPath }`. `DeleteMode` is `Trash | Permanent`.
- **Scan, then execute.** The scan walks the sources into a flat `Vec<Task>`
  plus `total_bytes` / `total_files`. A progress bar that has to guess its
  total is a progress bar that lies, and the scan is also the first place a
  cancel can take effect for free.
- `Progress` events: `Scanned { files, bytes }`, `FileStarted { path }`,
  `Advanced { bytes }`, `FileFinished`, `Failed { path, error }`,
  `Finished { outcome }`. `Advanced` carries a delta, not a running total, so
  no consumer has to know the ordering to add it up.
- **Per-file errors never abort the batch.** A failure is logged as `Failed`
  and the walk continues, which is what the design doc's error-handling
  section asks for.
- `Conflict` is answered by a `ConflictResolver` trait. Sub-phase C
  implements it over a channel; every test here implements it as a scripted
  list, so conflict behavior is tested without a thread in sight.
  *Apply-to-all* is a wrapper resolver, so the policy lives in one pure place
  instead of in the dialog.

  *Deviation, implemented deliberately:* the sketch's `Resolution::Rename(String)`
  became `Resolution::KeepBoth`, with the engine inventing `name (2).ext`
  rather than the user typing a name. A typed name cannot be applied to all —
  a hundred collisions would need a hundred names — so the parameterless
  version is what makes *apply to all* mean anything on that answer, and it
  keeps the "target count +1, original untouched" invariant testable without
  a UI. A user-typed name is a later refinement.

  *Also added:* a `Job::Rename` variant. F6 edited down to a bare name is a
  distinct intent and the UI has to know which question to ask, even though it
  executes exactly like a move.
- `Cancel` is an `AtomicBool` token, checked between tasks and between copy
  chunks. **Rollback on cancel is limited to the file in flight**: the
  partially written destination is removed, so the target never holds a
  truncated file. Already-finished files stay — Total Commander behaves the
  same way, and undoing a completed 4 GB copy because the fifth file was
  cancelled would be the more surprising choice.
- `Move` tries `rename` first and falls back to copy+delete on
  `CrossDevice`. The fallback reuses the copy path exactly; nothing about
  copying is written twice.
- Constants in `ops::constants`: copy buffer size, the rename-suffix pattern
  for `Resolution::Rename` defaults.

*Tests* — conservation invariants first
(skill [52](../skills/52-test-conservation-invariants.md)), across a fixture
tree of nested directories, mixed sizes, an empty file, an empty directory
and a unicode name:
- **Copy conserves everything:** recursive file count and byte sum at the
  target equal those at the source, per-relative-path contents match, and the
  source tree's name set and byte sum are *unchanged*.
- **Move conserves the union:** (files at source + files at target) is the
  same before and after; afterwards the source side is empty and the target
  side holds exactly what the source held.
- **A same-device move never reads a byte.** A counting decorator around
  `VirtualFs` asserts `open_read` was called zero times — this is the
  invariant that catches a `rename` silently degrading into a copy, which no
  value assertion would notice because the result looks identical.
- **The cross-device fallback conserves the same things**, driven by calling
  the fallback path directly, since a single-device test box cannot produce a
  real `CrossDevice`.
- **Delete touches nothing but its targets:** the parent's entry count drops
  by exactly the number of top-level paths deleted and the siblings' name set
  is untouched, for a file, an empty directory and a deep tree alike.
- **Cancel leaves no truncated file:** for every source file, the target
  holds either a byte-identical copy or nothing at all — never a partial one
  — and the source byte sum is unchanged.
- **Conflict table:** (`Overwrite`, `Skip`, `Rename`, `Abort`) ×
  (existing file, existing directory) → `Skip` leaves the target bytes
  untouched, `Overwrite` makes them equal the source, `Rename` conserves both
  (target count +1, original untouched), `Abort` stops with the remaining
  sources untouched.
- **Apply-to-all is one question:** with N conflicting files and the wrapper
  in place, exactly one `resolve` call reaches the inner resolver.
- **Progress adds up:** Σ `Advanced.bytes` == `Scanned.bytes` on a run with
  no skips — the event stream is checked as an invariant, not as a
  hand-written sequence.
- **Symlinks**, whose two operations differ on purpose: delete removes the
  link and never what it points at (descending would destroy files outside
  the selected tree), while copy follows a link to a file and refuses a link
  to a directory (following loops forever on a cycle, and `VirtualFs` has no
  `symlink` call to recreate one).
- **Mutation probes** (skill [59](../skills/59-mutation-probe-over-coverage-percent.md)),
  run and recorded: rollback removed, rename fast path removed, skip tracking
  removed, mtime stamping removed — each must turn its own test red.

  *The first probe earned its keep immediately.* With the rollback deleted
  outright, all 21 tests stayed green: the cancel was landing on a file that
  had already been read completely, so nothing was ever truncated. The
  decorator now cancels only after a read that filled the whole buffer, which
  is the only moment a destination is genuinely half written. Every probe
  fails its test now.

*Deviation on rollback, implemented deliberately:* the sketch rolled back the
file in flight unconditionally. It does not roll back an **overwrite** — the
original was already gone the moment the destination was truncated, so
deleting the remains would leave the user with neither copy instead of one
damaged one. The invariant is therefore about destinations that did not exist
before, which is what the test asserts.

*Gap opened and recorded:* `Move` assumes a single store. The rename fast path
and the `CrossDevice` fallback both address one backend, which holds while the
UI has one; phase 6 is where a cross-store move has to be told apart.

*Docs:* new [ops.md](../ops.md) — the job model, the scan/execute split, the
event vocabulary, the conflict protocol, and what cancel does and does not
undo.

### C — `tc-core::ops::queue`: the background queue

*Commit:* `feat(ops): background job queue with progress and conflict channels`

- `JobQueue::spawn(job, fs_source, fs_target) -> JobHandle`. Jobs run FIFO on
  one worker thread; `JobHandle` carries the progress receiver, the conflict
  channel pair and the cancel token.
- `async-channel` in **`tc-core`**, not in `tc-app`: it is a plain channel,
  not a UI dependency, and putting it here is what lets the UI `await` the
  event stream on the GLib main loop without `tc-core` knowing GLib exists.
- The conflict round trip is a request with a bounded(1) reply channel. The
  worker blocks on the reply.
- **A dropped reply channel means `Abort`.** If the UI dies, or a dialog is
  dismissed without answering, the worker must not sit on a channel forever
  holding a half-copied tree. Choosing `Abort` rather than `Skip` for that
  case is deliberate: silence is not consent to overwrite anything, and
  aborting is the outcome the user can always recover from by starting again.

*Tests* (headless, no GTK, no main loop):
- A job driven end to end through the queue produces the same tree as the
  same job driven synchronously in sub-phase B — the concurrency wrapper
  changes timing, not results.
- Progress events arrive in order and their byte deltas still sum to the
  scanned total.
- A conflict is answered over the channel and the job proceeds; the same
  conflict with the reply channel *dropped* ends the job as `Abort` and
  leaves the target untouched.
- Cancelling a running job stops it and the cancel invariant from sub-phase B
  still holds when the cancel arrives from another thread.
- Two queued jobs run in submission order and neither observes the other's
  events.

*Docs:* [ops.md](../ops.md) — the threading model, the channel shapes, and
the dropped-reply rule.

### D — `tc-app`: keys, dialogs, and refresh

*Commit:* `feat(app): file operations on F5, F6, F7 and F8`

- Keymap grows by six bindings, in the one table where bindings live:

  | Key | Action |
  |---|---|
  | `F5` | Copy the cursor entry to the other pane's directory |
  | `F6` | Move or rename the cursor entry |
  | `F7` | Create a directory |
  | `F8`, `Delete` | Delete to trash |
  | `Shift+F8`, `Shift+Delete` | Delete permanently |

- Dialogs, all modal, all built from the same small helper so three dialogs
  do not become three layouts
  (skill [44](../skills/44-no-redundancy.md)):
  - **Target dialog** (F5/F6) — an entry prefilled with the *other* pane's
    directory. F6 with the path edited down to a bare name is a rename; the
    engine needs no separate rename job, because a move whose target is the
    source's own directory is exactly that.
  - **Name dialog** (F7).
  - **Delete confirmation** — names the entry and says *trash* or
    *permanently*, because those are not the same question. The permanent one
    is the only dialog whose default button is the cancelling one.
- The dialog decisions — what F6's target string means, which confirmation
  text applies, what a job's sources are given a cursor row — are pure
  functions in `tc-app`, unit tested headlessly, exactly as `navigation.rs`
  is. The widgets stay assembly.
- **Both panes reload when a job finishes**, because a copy changes the
  target pane and a move changes both. A pane whose own directory no longer
  exists falls back to the nearest surviving ancestor via a new
  `Listing::load_nearest` in `tc-core` (the root always succeeds), which is
  the sub-phase 0 characterization being deliberately superseded — the test
  written there is updated in this commit, with the old contract quoted in
  the message.

*Tests:* the keymap table gains its new rows (and loses `F5`/`Delete` from
the unbound-key witness set, as announced in section 2); the pure dialog
decisions and `Listing::load_nearest` are unit tested.

*The widget wiring does **not** inherit the manual-verification gap, which is
the biggest deviation in this plan.* The implementation environment turned out
to have `Xvfb` and `xdotool`, so `scripts/smoke-keys.sh` now drives the real
binary with real X key events and checks the filesystem afterwards — Tab, the
cursor keys and Enter in passing, F5, F7 and F8 with their dialogs directly.
It earned its keep on the first run by finding a defect no test could see: the
conflict dialog opened with no focused button and could only be answered with
the mouse. What it still does not reach is listed in
[future-improvements.md](../future-improvements.md).

*Deviations, implemented deliberately:*
- **The conflict dialog moved from sub-phase E into D.** Without it, every
  colliding copy in D would have been answered by the engine's
  dropped-question rule — a silent abort. A sub-phase has to be usable on its
  own (skill [11](../skills/11-multi-phase-commits.md)), and that one would
  not have been. E keeps the progress window and the failure summary.
- **`Job::Rename` was removed and `Destination` introduced.** Building the
  dialog showed the UI needs "copy to an exact name" (duplicating a file) just
  as much as "move to an exact name", so the asymmetry between a `Copy` that
  could only target a directory and a `Rename` that could only target a path
  collapsed into one `Destination { Into, Exact }`. Only test *call sites*
  changed; no assertion did.
- **The `Box`→`Arc` migration deferred from sub-phase A landed here**, where a
  job first shares the handle.
- **Sub-phase 0's characterization needed no superseding.** `load_nearest` is
  a new function rather than a change to `load`, so the old contract is still
  true and the phase-0 tests still hold; the third of them turned into the
  specification the new function is tested against.

*Docs:* [keymap.md](../keymap.md) — the new bindings and why a rename is a
move · [ui-shell.md](../ui-shell.md) — the dialogs and the refresh rule ·
[listing.md](../listing.md) — `load_nearest`.

### E — `tc-app`: progress window and conflict dialog

*Commit:* `feat(app): progress window with cancel and conflict resolution`

- A progress window per running job: current file, a bar driven by the
  summed `Advanced` deltas against the scanned total, and a Cancel button
  wired to the cancel token.
- The conflict dialog: overwrite / skip / rename / abort, each with an
  *apply to all* checkbox that installs the wrapper resolver from sub-phase B
  — the UI supplies the checkbox, never a second copy of the policy.
- Failures collected during the job are shown once at the end rather than one
  dialog per file; a batch that hit six unreadable files must not need six
  clicks.
- Events are consumed with `glib::spawn_future_local` on the main loop, so
  the UI thread still owns GTK and no widget is touched from a worker.

*Tests:* the whole meter is pure and tested — delta sums into a fraction, the
clamp at both ends, a job with nothing to move being complete rather than
divided by zero, byte formatting including the largest-unit bound, and the
failure list's cut-off.

*Found by running it, not by a test:* copying a tree that contains a symlink
to a directory copied **nothing at all**. The scan returned the refusal as an
error for the whole source, discarding every task already collected. The unit
test missed it because it copies such a link on its own; the smoke run showed
an empty target directory. A scan failure is now collected per path and the
walk carries on, an item that collected one counts as incomplete so a move
will not delete a source it could not fully copy, and two regression tests
cover both halves. Probed: without the per-item reporting, all three symlink
tests go red.

*Also found by running it:* the failure summary opened as a window mostly full
of empty space to report a single failure. It now grows with its content and
stops at a screenful.

*Docs:* [ui-shell.md](../ui-shell.md) — the progress window, the conflict
dialog, and where the event loop is attached.

### F — Refactoring audit + correction (skill [49](../skills/49-final-phase-refactoring-audit.md))

*Commit:* `refactor(core,app): audit corrections for the core operations phase`

Audit over everything sub-phases 0–E added, corrections implemented in the
same phase. The axes that phase 1's audit found things on, and that this
phase is most likely to repeat:

- **Redundancy** — the copy loop, the conflict policy and the "which sources"
  derivation each exist once, or they get pulled together now.
- **API ahead of its caller** — every `pub` item in `ops` has a caller outside
  its own tests, or it is narrowed. Phase 1 found four of these.
- **Magic values** — dialog titles, button labels, buffer sizes and format
  strings live in `constants.rs`, not inline.
- **Architecture** — no `gtk`/`glib`/`gdk` anywhere in `tc-core`, no
  `std::fs`/`std::path` in `tc-app` production code, and the four-test-module
  `Entry` fixture duplication that phase 1 deliberately accepted is
  re-decided now that phase 2's operation tests need the same shapes.

## 5. Effort

Calibrated per skill [45](../skills/45-calibrate-effort-estimates.md)
(feature plan, factor ×0.25):

| Sub-phase | Calibrated |
|---|---|
| 0 — coverage pre-check | ~0.5 h |
| A — mutating VFS surface | ~1.5 h |
| B — synchronous ops engine | ~3 h |
| C — background job queue | ~2 h |
| D — keys, dialogs, refresh | ~2.5 h |
| E — progress window, conflicts | ~2 h |
| F — refactoring audit | ~1 h |
| **Total** | **~12.5 h** |

Consistent with the design doc's "~2–3 days" for phase 2, and with phase 1's
measured ~5.5 h for roughly half this surface.

## 6. Risks & open points

- **Toolchain, blocking** (section 3). The plan cannot be implemented in a
  container with rustc < 1.98 and no `libgtk-4-dev`; resolve first.
- **`Send + Sync` reaches into phase 6.** Making the trait thread-safe is
  free for `LocalFs` (a unit struct) and a real constraint for `ArchiveFs`,
  which holds an open archive handle and will need interior mutability. Better
  to pay that in the type system now than to discover it when the archive
  backend is half written.
- **Cross-device move is untestable on one device.** The fallback is exercised
  directly, so the *behavior* is covered while the *trigger* is not. The
  `CrossDevice` mapping itself is covered by the error-kind table. Anything
  left over goes to [future-improvements.md](../future-improvements.md).
- **The trash test depends on `XDG_DATA_HOME` being honoured** by the crate
  (sub-phase A). If it is not, half the assertion is lost; the plan says what
  happens then rather than discovering it mid-implementation.
- **The keypress path still has no automated coverage.** Phase 1's manual
  pass found three bugs that 74 green tests missed, all of them in the
  composition between GTK and the model. Phase 2 adds dialogs, a modal event
  loop and a job that mutates the filesystem underneath a pane — strictly
  more composition, in a phase where a bug can delete something. The manual
  pass is not optional here, and its checklist grows with sub-phases D and E.
- **The function-key bar is out of scope, and this is the phase that makes it
  worth having** (skill [73](../skills/73-proactively-suggest-better-approaches.md)).
  Eight labelled buttons across the bottom is perhaps an hour, it is the
  affordance that makes F5–F8 discoverable at all, and it is currently
  unassigned to any phase. Left out to keep the phase's shape, but it would
  sit naturally at the end of sub-phase D — owner's call.

## 7. Definition of done

- F5, F6, F7, F8, Del and Shift+Del work against the cursor entry, with the
  other pane as the target, and both panes show the result afterwards.
- A copy of a large tree shows honest progress, can be cancelled, and leaves
  no truncated file behind.
- A name collision asks once per file, or once in total with *apply to all*.
- Delete goes to the trash by default; permanent delete needs Shift and a
  confirmation whose default button is Cancel.
- `cargo fmt --all -- --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace`, `cargo build --release` and the Windows
  cross-target clippy check are all green.
- [ops.md](../ops.md) exists and is linked from [docs/CLAUDE.md](../CLAUDE.md);
  [vfs.md](../vfs.md), [keymap.md](../keymap.md), [ui-shell.md](../ui-shell.md)
  and [listing.md](../listing.md) reflect the new behavior.
- The manual keyboard-and-dialog pass has been run by the owner and its result
  recorded, as phase 1's was.
- Sub-phase F is done, and this document's Status becomes `Implemented` with
  the commit hashes, per skill [10](../skills/10-plan-lifecycle.md).
