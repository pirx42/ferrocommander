# Phase 3 Implementation Plan — Polish Browsing

Status: In Progress — sub-phases 0, A, B done

*2026-08-28 — implements phase 3 of
[2026-08-28-tc-clone-design.md](2026-08-28-tc-clone-design.md).*

> Process: [good-development-practices.md](../good-development-practices.md),
> skill triggers in the root [CLAUDE.md](../../CLAUDE.md). Every sub-phase
> below is one commit series, green before it lands
> (skills [11](../skills/11-multi-phase-commits.md),
> [25](../skills/25-green-suite-before-commit.md)).
>
> Both standing requirements apply throughout and are called out where they
> decide something: [performance.md](../performance.md) (speed wins) and
> [reliability.md](../reliability.md) (rather more tests than too few).

## 1. Scope

**One sentence:** the panes stop being a demonstration and become usable for
real work — many files at once, found quickly, ordered the way you want, with
the program remembering all of it next time.

### In scope

- **Selection** — Insert, Space, `Num +` / `Num −` by wildcard, `Num *` to
  invert, `Ctrl+A` for all. Every file operation acts on the selection, and
  falls back to the cursor row when nothing is selected. This is what phase
  2's `sources: Vec<VfsPath>` was shaped for.
- **Quick filter (`Ctrl+S`)** — type-ahead narrowing of the current pane,
  `Esc` to clear.
- **Sorting from the keyboard and the header** — `Ctrl+F3`…`Ctrl+F6` for
  name / ext / size / date, clicking a column header, and a visible marker
  for which column is active and in which direction.
- **Hidden files** — `Ctrl+H`.
- **Attributes column, and copies that keep their permissions** — an
  executable that arrives without its `+x` is a broken copy, which makes this
  a reliability item rather than a cosmetic one.
- **Config persistence** — last directories, sort order per pane, hidden-file
  flag, window geometry. Written atomically.
- **Drive / mount bar** — one button per mount point, switching the active
  pane.
- Two gaps homed here by earlier phases: a vanished entry must no longer fail
  the whole listing, and permission bits must survive a copy
  ([future-improvements.md](../future-improvements.md)).

### Explicitly out of scope

F3 viewer and F4 editor (phase 4) · search and multi-rename (phase 5) ·
archives (phase 6) · tabs, custom columns and thumbnails (out of v1
altogether) · directory sizes on Space, which is TC behaviour but needs a
recursive scan per keystroke and belongs with the search walker in phase 5 ·
a configurable keymap, which the one-table design already makes cheap but
which nothing yet asks for.

## 2. Coverage pre-check (skill [43](../skills/43-coverage-before-implementation.md))

| Existing item | Coverage today | Verdict |
|---|---|---|
| `Listing::rebuild` (sort + hidden + cursor restore) | 25 tests in `tests/listing.rs` | covered — selection and the filter both hook in here, so this is the contract not to break |
| `Listing::reload` | cursor keeps its entry, clamps when gone | covered, and it is what selection-survives-a-reload will be modelled on |
| `jobs::operable` | 2 tests: cursor entry, and `..` yields nothing | covered, and its **meaning changes** in sub-phase B |
| `keymap::action_for` | table + unbound-key witnesses | covered, and the witness set **changes again** — see below |
| `Sort::compare` | table over every key × order, plus the ASCII-equivalence and total-order tests | covered |
| `LocalFs::read_dir` | 15 tests | covered; the vanished-entry fix needs a seam it does not have — see sub-phase A |
| `Entry` construction | built by hand in ~6 test modules | **every one breaks** when the attributes field lands (sub-phase E), mechanically |

**Announced test changes (skill [24](../skills/24-no-silent-test-changes.md)).**
Two, both scheduled rather than discovered:

1. `an_unbound_key_triggers_nothing` currently witnesses with `Escape`, `a`,
   `F9` and `Insert`. Sub-phase B binds `Insert`, sub-phase C binds `Escape`,
   and `a` becomes bound under Ctrl. `F9` and `F12` survive as witnesses.
2. `operable` returning a single path becomes `operable` returning the
   selection, so both its tests are rewritten around the new contract. The
   old behaviour — cursor row when nothing is selected — remains one of the
   cases.

**Phase 0 therefore characterises one thing:** that `rebuild` restores the
cursor onto the *named* entry across a sort change, a hidden toggle and a
reload. Selection remapping runs in the same place and must not disturb it,
and today's guarantee is spread across three tests that each check one
trigger. One test that pins all three together goes in before any of it moves.

## 3. Sub-phases

Each is independently green (`cargo fmt --all -- --check`,
`cargo clippy --workspace --all-targets -- -D warnings`, the Windows
cross-target check, `cargo test --workspace`, `cargo build --release`) and
ends in a conventional commit.

### 0 — Coverage pre-check

*Commit:* `test(listing): pin the cursor contract the selection has to share`

One test asserting the cursor lands on the same named entry after a sort
change, a hidden toggle and a reload — the invariant sub-phase A's selection
remapping shares a code path with.

### A — `tc-core`: selection, and a wildcard matcher

*Commit:* `feat(listing): selection with wildcard select and deselect`

- `Listing` gains `selected: Vec<bool>`, parallel to `entries` rather than a
  set of names. Selecting everything becomes a fill, iterating the selection
  a scan, and sorting or filtering costs nothing at all because neither
  touches `entries`. A `HashSet<String>` would allocate a string per selected
  file for a question asked on every keystroke
  ([performance.md](../performance.md)).
- API: `toggle(index)`, `set_selected(index, bool)`, `select_all`,
  `clear_selection`, `invert_selection`, `select_matching(pattern, bool)`,
  `selected_paths()`, `selection_summary()` (count and byte total for the
  status line).
- **`..` is never selectable.** It is a navigation control, and every
  operation would have to special-case it afterwards otherwise — the same
  rule `jobs::operable` already applies.
- **Selection survives a reload by name**, exactly as the cursor does: a job
  that changed one file must not silently drop the marks on the others.
  Vanished names fall out; new names arrive unselected.
- New `tc-core::glob` — `*` and `?` only, table-tested. Phase 5's search needs
  the same matcher, so it is written where both can reach it rather than
  twice ([44](../skills/44-no-redundancy.md)).
- **The vanished-entry gap closes here.** `read_dir` currently fails the whole
  listing if an entry disappears between being enumerated and being stat'ed.
  The collect loop becomes a small function over `(name, Result<Metadata>)`
  pairs that drops `NotFound` and keeps everything else, which is both the
  fix and the seam that makes it testable at all.

*Tests:* selection count conservation across sort, filter and hidden toggles;
`invert` twice is the identity; `select_all` then `clear` is empty; a reload
keeps the marks on surviving names and drops the rest; `..` cannot be
selected by any route including `select_all` and `invert`; glob table
(`*.txt`, `a?c`, `*`, no wildcard, empty pattern, a name containing `*`).

### B — `tc-app`: selection keys, and operations that use it

*Commit:* `feat(app): select files and operate on the selection`

- Bindings: `Insert` (toggle and step down — the one that gets held down),
  `Space` (toggle in place), `Num +` / `Num −` (a dialog asking for a
  pattern), `Num *` (invert), `Ctrl+A` (all).
- Selected rows are marked in the pane. A status line under each pane shows
  `n of m selected, x of y` — the number a TC user checks before pressing F5.
- `jobs::operable` becomes `jobs::sources`: the selection, or the cursor row
  when nothing is selected. One pure function, and the only change the
  operations need — phase 2 shaped `Job` around a `Vec<VfsPath>` for this.
- The delete confirmation and the F5/F6 dialogs say how many entries are at
  stake rather than naming one.

*Tests:* the keymap table grows; `sources` is unit-tested for empty
selection, one selected, many selected, and a selection that includes the
cursor; the delete prompt for many; end-to-end UI cases for Insert-then-F5
and `Ctrl+A`-then-F8.

### C — `tc-core` + `tc-app`: the quick filter

*Commit:* `feat(listing): quick filter narrowing the visible rows`

- `Listing::set_filter(&str)` folded into `rebuild` beside the hidden-file
  rule, so filtering allocates nothing and re-reads nothing.
- Matching is case-insensitive substring, reusing the ASCII fast path from
  the sort comparison rather than lowercasing per keystroke.
- `Ctrl+S` opens an inline filter field in the pane; typing narrows;
  `Esc` clears and closes; `Enter` keeps the narrowed view and returns focus
  to the rows.
- The cursor stays on its entry while it is still visible, and clamps when it
  is not — the same rule as the hidden toggle, and the reason phase 0 pins it.

*Tests:* filtered count plus excluded count equals total; clearing restores
exactly the previous view; the filter composes with hidden files and with
sort; the cursor never lands on a filtered-out row; UI case for
`Ctrl+S`-narrow-`Esc`.

### D — `tc-app`: sorting and hidden files from the keyboard

*Commit:* `feat(app): sort from the header or the keyboard, and toggle hidden files`

- `Ctrl+F3`…`Ctrl+F6` set the sort key; pressing the same key again flips the
  direction. Clicking a header does the same.
- The active column shows its direction in the header.
- `Ctrl+H` toggles hidden files.
- Both are already in the model, so this sub-phase is wiring and a marker.

*Tests:* the keymap rows; the pure "which sort does this key mean, given the
current one" function, including the flip-on-repeat rule; a UI case that
sorts by size and checks the top row changed.

### E — `tc-core`: attributes, and copies that keep them

*Commit:* `feat(vfs): entry attributes, preserved across a copy`

Reliability, not decoration: an executable that arrives without its `+x` is a
broken copy, and today every copy does that
([reliability.md](../reliability.md)).

- `Entry` gains `attributes: Attributes` — a small, `Copy` value, not a
  string, so the column renders it and the copy engine restores it from the
  same source.
- Platform split in `vfs/platform.rs` as always: Unix mode bits, Windows file
  attributes. The rendered form (`rwxr-xr-x`, `RHSA`) is the platform's job.
- `VirtualFs::set_attributes`, called by the copy engine after
  `set_modified`, and reported as a per-path failure when it fails rather
  than sinking the file.
- An `Attr` column in the pane.

*Tests:* an executable copied through the engine is still executable
(`#[cfg(unix)]`); attributes survive a `stat` round trip; a copy where
`set_attributes` fails reports it and keeps the file; the rendering table.

### F — `tc-core`: config persistence

*Commit:* `feat(config): remember directories, sort order and geometry`

- `tc-core::config` — a `Settings` value plus `load`/`save`. In `tc-core`
  because the UI never touches a filesystem directly, and the same rule
  applies to its own settings file.
- `serde` + `toml`. A hand-rolled parser would save a dependency and cost
  reliability, which is the wrong way round for a file the program rewrites
  on every exit ([reliability.md](../reliability.md)).
- **Written atomically**: to a temporary name in the same directory, then
  renamed over the target. A config truncated by a crash mid-write is a
  program that starts up wrong, and `VirtualFs` already has both calls.
- Location from `vfs/platform.rs`: `$XDG_CONFIG_HOME` or `~/.config` on
  Unix, `%APPDATA%` on Windows.
- **A missing or unreadable config is not an error.** It is a first run, and
  the defaults apply. A corrupt one is reported once and replaced — never a
  refusal to start.

*Tests:* round trip through save and load; defaults when the file is absent;
defaults plus a warning when it is malformed; the temporary file is gone
afterwards; an interrupted save leaves the previous config intact.

### G — `tc-app`: the drive and mount bar

*Commit:* `feat(app): drive bar for switching the active pane between mounts`

- `platform::mount_points()` — the drive list on Windows, the real mounts on
  Unix, filtered to the ones a person would navigate to.
- A button row above the panes; clicking one sends the *active* pane there.

*Tests:* the mount list is parsed from a fixture rather than from the running
system, so the test does not depend on the box it runs on; the filter rules
(pseudo-filesystems excluded) are a table.

### H — Refactoring audit + correction (skill [49](../skills/49-final-phase-refactoring-audit.md))

*Commit:* `refactor(core,app): audit corrections for the browsing phase`

Same axes as phase 2's, which found real things: redundancy across the
sub-phases, `pub` items with no caller outside their own tests, magic values
that drifted into widget code, and the architecture invariants (no GTK in
`tc-core`, no `std::fs` in `tc-app`, no `cfg` outside `vfs/platform.rs`).
Plus, new this phase: every optimisation claim in
[performance.md](../performance.md) still measured, and every mutation probe
listed in [reliability.md](../reliability.md) still red when reverted.

## 4. Effort

Calibrated per skill [45](../skills/45-calibrate-effort-estimates.md) (feature
plan, factor ×0.25):

| Sub-phase | Calibrated |
|---|---|
| 0 — coverage pre-check | ~0.25 h |
| A — selection model + glob | ~2 h |
| B — selection keys and operations | ~2 h |
| C — quick filter | ~1.5 h |
| D — sorting and hidden files | ~1 h |
| E — attributes and permissions | ~2 h |
| F — config persistence | ~2 h |
| G — drive bar | ~1 h |
| H — refactoring audit | ~1 h |
| **Total** | **~12.75 h** |

**This is larger than the design doc's "~1–2 days" for phase 3**, and the
estimate is not being bent to fit it. Two items are the reason: attributes
with permission-preserving copies, which arrived here as a reliability gap
rather than as browsing polish, and config persistence, which is a small
feature with a large correctness surface. Both are worth their cost; the
design doc's line was written before either had a home.

## 5. Risks & open points

- **`Entry` gains a field, and every hand-built fixture breaks.** Mechanical,
  but it touches six test modules across both crates. This is the moment the
  shared-fixture question deferred twice becomes cheap to answer, and
  sub-phase H is where it gets asked again.
- **Selection and the filter share `rebuild` with the cursor rule.** Three
  behaviours in one function is where phase 3's bugs will be, which is why
  phase 0 pins the existing contract first and why every new test asserts
  composition rather than each rule alone.
- **`serde` + `toml` is the first dependency that is not tiny.** Compile time
  is a developer cost, not an app cost, and the config is read once at
  startup — the speed requirement is not in tension here. Worth stating
  because the requirement is new and this is the first decision it could have
  been read as blocking.
- **The mount list is the least portable thing in the project so far.** Unix
  has no single answer, and the filter for "mounts a person cares about" is a
  judgement call. Testing against a fixture rather than the live system keeps
  the judgement reviewable.
- **Directory sizes on Space are deliberately absent.** TC computes them, and
  it means a recursive scan per keystroke — exactly the kind of thing the
  speed requirement rules out until it can be done off the UI thread. It
  belongs with phase 5's walker.

## 6. Definition of done

- Insert, Space, `Num +`/`−`/`*` and `Ctrl+A` select, the status line shows
  what is selected, and F5–F8 act on the selection.
- `Ctrl+S` narrows the pane as you type; `Esc` clears it.
- `Ctrl+F3`…`Ctrl+F6` and the column headers sort; `Ctrl+H` toggles hidden
  files.
- A copied executable is still executable, and the Attr column shows why.
- Closing and reopening the app returns both panes to where they were, with
  the same sort order and window size.
- The drive bar switches the active pane.
- The full gate is green, the UI suite covers the new bindings, and every
  claim added to [performance.md](../performance.md) carries a measurement.
- Sub-phase H is done, and this document's Status becomes `Implemented` with
  the commit hashes, per skill [10](../skills/10-plan-lifecycle.md).
