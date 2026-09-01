# Phase 1 Implementation Plan — Walking Skeleton

Status: Implemented — substance extracted to [vfs.md](../../vfs.md),
[listing.md](../../listing.md), [ui-shell.md](../../ui-shell.md),
[keymap.md](../../keymap.md); open points to
[future-improvements.md](../../future-improvements.md).
Commits 525b96b (0) / dfd4cbc (A) / 2c8bc2d (B) / 57a7660 (C) / c09aaf0 (D) /
64780ac (E).

*2026-08-28 — implements phase 1 of
[2026-08-28-tc-clone-design.md](2026-08-28-tc-clone-design.md).*

> Process: [good-development-practices.md](../../good-development-practices.md),
> skill triggers in the root [CLAUDE.md](../../../CLAUDE.md). Every sub-phase
> below is one commit series, green before it lands
> (skills [11](../../skills/11-multi-phase-commits.md),
> [25](../../skills/25-green-suite-before-commit.md)).

## 1. Scope

**One sentence:** a runnable `fc-app` window with two panes that list real
directories from a `fc-core` local VFS, navigable with Tab / arrows / Enter /
Backspace — nothing else.

### In scope

- Cargo workspace `fc-core` + `fc-app`, toolchain pinned, green gate runnable.
- `fc-core::vfs` — `VirtualFs` trait limited to **reading a directory**, plus
  the `LocalFs` implementation.
- `fc-core::listing` — directory model: entries, sort order, cursor position,
  parent entry, hidden-file flag. Pure, headless-tested.
- `fc-app` — one `ApplicationWindow`, two pane widgets (`gtk::ColumnView`)
  with columns name / ext / size / date, a path label per pane, active-pane
  highlight.
- Keyboard: Tab (switch pane), Up/Down/Home/End (cursor), Enter (descend into
  directory), Backspace (ascend), Ctrl+Q (quit).

### Explicitly out of scope (later phases of the design doc)

File operations F5–F8, job queue, conflict handling (phase 2) · sorting UI,
selection, Ctrl+S filter, drive bar, config persistence (phase 3) · viewer
(4) · search & multi-rename (5) · archives (6). Also out: the function-key
bar, mouse interaction, icons, and any writing to the filesystem — phase 1
is strictly read-only.

**Read-only is a hard invariant of this phase.** `fc-core` phase 1 contains
no code path that creates, renames or deletes anything; that keeps the first
runnable build harmless to test against a real home directory.

## 2. Coverage pre-check (skill [43](../../skills/43-coverage-before-implementation.md))

Not applicable — greenfield, there is no existing behavior to characterize.
The substitute contract: every sub-phase A–D lands its own behavioral tests
in the same commit (skills [23](../../skills/23-tests-accompany-commits.md),
[26](../../skills/26-behavioral-tests.md)), and `fc-core` sub-phases A/B must be
fully testable without a display server.

## 3. Environment gate (blocking, before phase 0)

**Satisfied 2026-08-28.** The box had none of it; installed since:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh   # rustup + stable
sudo apt install -y build-essential pkg-config libgtk-4-dev       # linker + GTK4 headers
```

Resulting versions: rustc/cargo **1.98.0**, GTK **4.22.4**.

`build-essential` is not optional garnish: rustc links through the `cc`
driver, so without it every `cargo test` and `cargo build` fails with
``linker `cc` not found`` while `fmt` and `clippy` still pass — a green-looking
gate that never linked anything.

Verification: `cargo --version && pkg-config --modversion gtk4` both succeed.

## 4. Sub-phases

Each sub-phase is independently green (`cargo fmt --all -- --check`,
`cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace`, `cargo build --release`) and ends in a
conventional commit (skill [31](../../skills/31-conventional-commit.md)).

**Both platforms are build targets** (owner spec, 2026-08-28). Since a Linux
build never compiles the `cfg(windows)` half, the gate carries a fifth
command:

```bash
cargo clippy -p fc-core --all-targets --target x86_64-pc-windows-gnu -- -D warnings
```

`cargo check`-style verification needs no Windows linker, so this runs on the
Linux box. It is not ceremony — it caught a Windows-only build break on its
first run in sub-phase A.

Scoped to `fc-core` since sub-phase C: cross-checking `fc-app` would require
GTK's `-sys` build scripts to find a mingw libgtk-4 via pkg-config, which this
box has no way to provide. The boundary is acceptable because every
platform-divergent line lives in `fc-core`; `fc-app` has no `cfg` branches.
A real Windows or mingw toolchain is what would verify the GTK build.

### 0 — Workspace bootstrap

*Commit:* `chore(workspace): initialize cargo workspace fc-core + fc-app`

- Root `Cargo.toml` with `[workspace]`, `resolver = "2"`, shared
  `[workspace.package]` (edition, rust-version, license) and
  `[workspace.dependencies]` so crate versions are declared once
  (skill [17](../../skills/17-centralize-constants.md) applied to dependencies).
- `rust-toolchain.toml` pinning the stable channel + `rustfmt`, `clippy`
  components, so the green gate means the same thing on every machine.
- `crates/fc-core/` (lib) and `crates/fc-app/` (bin) with placeholder
  modules; `.gitignore` for `target/`.
- One smoke test per crate so `cargo test --workspace` is meaningful from
  commit one.
- Docs: root `CLAUDE.md` build commands verified against the real layout;
  `crates/CLAUDE.md` with the `← Parent` link describing the two-layer split.

*Exit criterion:* the four gate commands pass on an empty workspace.

**Done.** The smoke tests are deliberately wired across the crate boundary —
`fc-app` renders its banner from `fc_core::version()` — so they fail if the
workspace dependency edge breaks, rather than asserting a constant against
itself.

### A — `fc-core::vfs`: trait + `LocalFs` (read side)

*Commit:* `feat(vfs): read-only VirtualFs trait with local filesystem backend`

- `vfs::types` — `Entry { name, kind, size, modified }`, `EntryKind
  { Dir, File, Symlink }`, `VfsError` (`NotFound`, `PermissionDenied`,
  `NotADirectory`, `Io`) mapped from `std::io::ErrorKind`.
- `VirtualFs` trait, phase-1 surface only:
  `fn read_dir(&self, path: &VfsPath) -> Result<Vec<Entry>, VfsError>` and
  `fn stat(&self, path: &VfsPath) -> Result<Entry, VfsError>`.
- `VfsPath` — an owned, always-absolute, `/`-normalized path type; the
  boundary type that later lets `ArchiveFs` reuse the same addressing without
  a `PathBuf` leaking archive-internal paths into `std::fs` calls.
- `LocalFs` — `std::fs` implementation. Symlinks are reported as
  `EntryKind::Symlink(SymlinkTarget)` where the target is `Dir`, `File` or
  `Broken`, so the UI can show them without following broken links.
  `Entry::is_dir()` answers "can I descend into this", true for directories
  and symlinks to directories alike.
- `Entry.hidden` is set by the *backend*. This changed when both platforms
  became targets: on Windows, hidden is `FILE_ATTRIBUTE_HIDDEN`, which the
  listing layer cannot derive from the name, so the flag must come from the
  filesystem rather than from a dot-prefix rule in `listing`.
- `vfs/platform.rs` holds every Linux/Windows difference behind three
  functions (`to_std_path`, `is_hidden`, `root_entries`), so adding a
  platform touches one file instead of scattering `cfg` blocks.

**Decision — why the trait is not the full design-doc trait yet.**
The design doc lists `open/read/write, rename, mkdir, remove` on
`VirtualFs`. Phase 1 adds none of them: unimplemented trait methods are dead
code with no test and no caller, and `todo!()` stubs are exactly the kind of
scaffolding skill [20](../../skills/20-no-backward-compat-shims.md) tells us to
avoid. The mutating half of the trait arrives in phase 2 together with its
implementation and its conservation-invariant tests
([52](../../skills/52-test-conservation-invariants.md)).

*Tests* (tempdir fixtures, headless):
- `read_dir` returns exactly the files created — count and name set
  (conservation-style assertion, not a hand-written expected vector).
- Nested dirs, empty dir, dir with hidden (dot) files — hidden entries are
  *returned* by the VFS; filtering is `listing`'s job, not the VFS's.
- Sizes match what was written (byte-sum invariant over the fixture).
- Error mapping: missing path → `NotFound`; a file passed where a directory
  is expected → `NotADirectory`.
- Symlink to a file, symlink to a directory, broken symlink.
- Unicode and space-containing names survive the round trip.

*Docs:* [docs/vfs.md](../../vfs.md) — the trait contract, the `VfsPath`
invariant, the platform table, and the "why" of the read-only phase-1 surface
(skills [29](../../skills/29-one-topic-per-doc.md),
[30](../../skills/30-document-the-why.md)).

**Done.** 28 tests green on Linux; the Windows branch clippy-clean via the
cross-target check.

*Decision — the Windows VFS root is the drive list.* Windows has no single
filesystem root, so `/` must mean something. Making it the list of drives
keeps `VfsPath` uniform across platforms and hands the design doc's drive
selector to the UI for free: a pane at `/` on Windows shows `C:`, `D:`, … as
directories, exactly as Total Commander does.

*Follow-up (deliberately deferred, not forgotten).* If an entry disappears
between `read_dir` enumerating it and `stat` reading it, the whole listing
fails with `NotFound` rather than omitting the vanished entry. Skipping it
would need an injection seam to be testable at all, so the version with no
untested code shipped; phase 3's refresh logic is where this belongs.

### B — `fc-core::listing`: directory model

*Commit:* `feat(listing): sorted directory model with cursor and parent entry`

- `Listing { dir, entries, cursor, sort, show_hidden }` built by
  `Listing::load(fs, dir)` or `Listing::new(dir, entries)`.

  *Deviation from this sketch, implemented deliberately:* the listing does
  **not** own the `VirtualFs`. The pane does — stepping into an archive swaps
  the backend while the pane lives on — and the fs is passed only to `load`
  and `reload`. Keeping the handle out of the model makes a `Listing`
  constructible from a bare `Vec<Entry>`, which is how nearly every test
  builds one and how phase 5's streaming search results will feed a pane. It
  is also the proof that sorting, filtering and cursor movement never touch
  the disk: there is nothing there to touch it with.
- `SortKey { Name, Ext, Size, Modified }` × `SortOrder { Asc, Desc }`;
  directories always sort before files (Total Commander behavior), the
  synthetic `..` entry always sorts first regardless of key/order.
- Name comparison is case-insensitive with a case-sensitive tiebreak, so
  `a.txt`/`A.txt` have a stable total order.
- `show_hidden` filters dot-entries out of the *view*, not out of the loaded
  data — toggling it must not re-hit the filesystem.
- Cursor API: `move_by(delta)`, `move_to_first/last`, `entry_at_cursor()`;
  the cursor clamps to the visible range and never goes out of bounds after a
  filter toggle or a reload (that clamp is the invariant the tests pin).
- Constants (`listing::constants`): the `..` display name, the parent-entry
  sentinel — no string literals scattered through the module
  (skills [16](../../skills/16-no-magic-values.md)/[17](../../skills/17-centralize-constants.md)).

*Tests* (table-driven where the input is a rule set, skill 26):
- Sort table: for each `SortKey` × `SortOrder`, a fixture of mixed
  dirs/files/extensions/sizes/timestamps → expected name order.
- Dirs-before-files holds under every key and both orders.
- `..` is first under every key and both orders; it is absent at the
  filesystem root.
- Hidden toggle: entry count with hidden on == count with hidden off + number
  of dot-entries (conservation invariant), and toggling twice returns the
  identical view.
- Cursor clamping: cursor on the last entry, then hide hidden files → cursor
  still points at an existing, visible entry.
- Reload of a changed directory keeps the cursor on the same *name* when that
  name still exists, falls back to a clamped index when it does not.

*Docs:* [docs/listing.md](../../listing.md) — sort rules, the `..` and
hidden-file semantics, cursor behavior.

**Done.** 50 tests green across the workspace; Windows branch clippy-clean.

*Decision — `Descending` reverses only within a group.* Directories are
grouped ahead of files before the direction is applied, so flipping the order
moves the newest file to the top rather than burying the directories at the
bottom. Every key ends in a name tiebreak, which makes the order total and
turns "descending is the exact reverse of ascending within each group" into an
invariant the tests assert directly rather than a claim in a comment.

### C — `fc-app`: window with two panes

*Commit:* `feat(app): dual-pane main window listing directories`

- `main.rs` — `gtk::Application`, one `ApplicationWindow`, `gtk::Paned`
  splitting two `PaneView`s 50/50.
- `PaneView` — path label + `gtk::ColumnView` over a
  `gtk::gio::ListStore` of a `PaneEntry` GObject wrapping a `fc-core`
  `Entry`; columns name / ext / size / modified with a shared factory.
- `app::constants` — window default size, pane split ratio, column widths,
  size/date display formats. No literal numbers in widget code.
- Active-pane state: exactly one pane is active; a CSS class marks it, and
  the inactive pane keeps its cursor row visible but dimmed.
- Both panes start at `$HOME` (config persistence is phase 3, so the start
  directory is a constant here, not a setting).

**Decision — `ColumnView` over `TreeView`.** `gtk::TreeView` is deprecated in
GTK4 and its model does not map cleanly onto a swappable `VirtualFs`;
`ColumnView` + `ListStore` gives the virtualized scrolling the pane needs for
large directories and is what the archive-as-folder phase will reuse
unchanged.

*Tests:* the GTK layer stays thin by construction — the only logic in
`fc-app` is formatting, so `size/date` formatting and the `Entry` →
`PaneEntry` mapping are unit-tested headlessly; window construction is
covered by a manual smoke run, per the design doc's testing section.

*Docs:* [docs/ui-shell.md](../../ui-shell.md) — widget tree, row rendering,
columns, active-pane marking, the GTK version floor.

**Done.** 62 tests green; the binary runs and holds a window open.

*Decision — build against the baseline GTK 4.0 API.* No `v4_x` feature is
enabled on the `gtk4` bindings. The shell needs nothing newer, and a higher
floor would exclude distributions and Windows builds shipping an older
libgtk-4. It bites immediately and usefully:
`CssProvider::load_from_string` requires GTK 4.12 and is simply not there, so
the baseline `load_from_data` is used instead.

*Decision — the active pane is marked on its path bar, not by dimming the
other pane.* In a dual-pane manager the inactive side must stay fully
readable: at that moment its entire job is to show where a copy would land.

*Phase-0 scaffolding retired as planned.* `fc-app`'s banner function and its
test existed to prove the workspace dependency edge before there was a UI.
The real window replaces them, and `row.rs` now carries the crate's tests.

### D — Keyboard navigation

*Commit:* `feat(app): keyboard navigation across panes and directories`

- `app::keymap` — a single table mapping (key, modifiers) → `Action`, so the
  key bindings live in one place and phase 3's configurable keymap has an
  obvious seam (skill [53](../../skills/53-generate-instead-of-duplicating.md):
  the GTK controller is generated from the table, not written per key).
- Actions: `SwitchPane`, `CursorUp/Down/First/Last`, `Activate` (Enter),
  `GoParent` (Backspace), `Quit`.
- `Activate` on a directory or on `..` reloads the pane's `Listing`; on a
  file it does nothing in phase 1 (F3/F4 are phase 4).
- Failure to enter a directory (permission denied) shows an inline error in
  the path bar and leaves the pane where it was — the pane never ends up in
  an unreadable state.

*Tests:* the keymap table and the action-dispatch function are pure and
tested headlessly (key + modifier → expected `Action`, including "unbound key
yields no action"); the reload-on-activate path is tested through
`fc-core::listing` against a tempdir.

*Docs:* [docs/keymap.md](../../keymap.md) — the binding table, the capture
phase, and the failed-navigation behavior.

**Done.** 74 tests green; the binary runs clean with no GTK warnings.

*Decision — the key controller sits in the capture phase.* The window must see
Tab and the arrow keys before the `ColumnView` applies its own focus and
selection handling, which would otherwise fight the pane cursor.

*Decision — unlisted modifiers are masked out; listed ones must match.* GTK
reports Caps Lock, Num Lock and held mouse buttons alongside real modifiers,
so a binding that compared them raw would leave a user with Caps Lock on
unable to navigate. Conversely `Ctrl+↓` does not fall through to plain `↓`,
because phase 3 will bind it to something else.

*Verification gap.* The keymap table and the navigation targets are unit
tested, and the window runs warning-free, but the path from a physical
keypress through the GTK controller to the pane has no automated coverage —
this session has no way to synthesize keystrokes. It needs a human at the
keyboard.

### E — Refactoring audit + correction (skill [49](../../skills/49-final-phase-refactoring-audit.md))

*Commit:* `refactor(core,app): audit corrections for the walking skeleton`

**Done.** Audit over everything sub-phases 0–D added, corrections implemented
in the same phase. 71 tests green (74 minus three deleted alongside the code
they covered); behavior unchanged, verified by re-running the binary.

**Dead scaffolding.** `fc_core::version()` lost its only caller when
sub-phase C replaced the phase-0 banner with the real window. Removed, with
its test. This is precisely the failure mode skill 49 names: nothing is
visible per phase, only in the overall view.

**API ahead of its caller.** Four items were reachable from outside without
anything outside calling them, and each was narrowed to what actually uses
it:

| Item | Only caller | Correction |
|---|---|---|
| `SortOrder::flipped` | its own test | removed — phase 3's column headers will add it back with a real caller |
| `Listing::set_show_hidden` | `toggle_hidden` | made private |
| `PaneView::sync_cursor` | `PaneView` itself | made private |
| `listing::extension` | `listing::sort` | un-exported, `pub(super)` |

The same rule that kept the mutating half off `VirtualFs` in sub-phase A and
removed `PaneView::listing()`/`fs()` in C, applied in hindsight.

**Magic values.** Column titles, widths and cell alignment were inline
literals in `pane.rs`, and the pane switch was a hardcoded `1 - active`.
Titles, widths and alignment moved into `constants.rs` alongside
`WINDOW_WIDTH`; switching now cycles modulo `PANE_COUNT`, so a third pane
would need no new switching logic. The `Column` enum keeps the *mapping*, so
adding a column is still one variant.

**Redundancy.** A third copy of "a backend works through a trait object" in
`navigation.rs` — `fc-core` already asserts it twice. Removed.

**Architecture: clean.** No `gtk`/`glib`/`gdk` reference exists anywhere in
`fc-core`, and no `std::fs` or `std::path` in `fc-app` production code. The
one `std::fs` in `fc-app` is inside test fixtures, which need it because
`fc-core` has no write API until phase 2.

**Accepted without change.** Entry-building helpers are duplicated across four
test modules in two crates. Sharing them needs a `test-support` feature on
`fc-core`, and the fixtures are small and shaped to each test's needs — the
cure costs more than the disease today. Revisit when phase 2's operation
tests need the same shapes.

## 5. Effort

Calibrated per skill [45](../../skills/45-calibrate-effort-estimates.md)
(feature plan, factor ×0.25):

| Sub-phase | Calibrated |
|---|---|
| 0 — workspace bootstrap | ~0.5 h |
| A — vfs + LocalFs | ~1 h |
| B — listing model | ~1 h |
| C — dual-pane window | ~1.5 h |
| D — keyboard navigation | ~1 h |
| E — refactoring audit | ~0.5 h |
| **Total** | **~5.5 h** |

Consistent with the design doc's "~1 day" for phase 1. Excludes the
environment gate (section 3), which is owner-side setup.

## 6. Risks & open points

- **Toolchain absent** (section 3) — blocks everything; resolve first.
- **GTK4 version skew.** The `gtk4` crate requires a minimum libgtk-4
  version; pin the crate feature to the version actually installed rather
  than the newest, otherwise the release build fails on the dev box while
  passing locally.
- ~~**Windows dev box.**~~ **Resolved 2026-08-28 (owner spec): Windows and
  Linux are both supported targets.** Consequences already absorbed in
  sub-phase A — `Entry.hidden` comes from the backend, `vfs/platform.rs`
  isolates the divergence, the VFS root is the drive list on Windows, and the
  gate cross-compiles the Windows branch. Still unverified on real hardware:
  nothing here has been *run* on Windows. Since sub-phase C the cross-check
  covers `fc-core` only — GTK's `-sys` crates cannot be cross-compiled without
  a mingw libgtk-4 — so **the Windows GTK build is entirely unverified** and
  needs a Windows or mingw toolchain to confirm.
- **Branch workflow.** Skill [10](../../skills/10-plan-lifecycle.md) prescribes a
  topic branch on `dev`, but skill 64 is marked Chimera-only and this repo has
  only `main`. Decide before sub-phase 0 whether to adopt `dev` + topic
  branches here; until then the phases land on `main`.
- **Large directories.** No pagination in phase 1; `read_dir` loads
  everything. Acceptable for the skeleton — revisit if a real directory makes
  the pane visibly stall.

## 7. Definition of done

- `cargo run -p fc-app` opens a window with two working panes.
- Tab, arrows, Enter, Backspace behave as specified in sub-phase D.
- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -D warnings`,
  `cargo test --workspace`, `cargo build --release` all green.
- `docs/vfs.md`, `docs/listing.md`, `docs/ui-shell.md`, `docs/keymap.md`
  exist and are linked from [docs/CLAUDE.md](../../CLAUDE.md).
- The Windows cross-target clippy check is green alongside the Linux gate.
- Sub-phase E is done, and this document's Status becomes `Implemented`
  with the commit hashes, per skill [10](../../skills/10-plan-lifecycle.md).
