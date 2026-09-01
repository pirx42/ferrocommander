# Architecture and redundancy review

**Status:** Implemented and archived — all nine items, plus the audit phase.
Built on `claude/architecture-and-redundancy-fixes` from `5c601ef` and
fast-forwarded onto `main` on 2026-08-29; the per-item commits are in § 7,
and `git log --grep "architecture-and-redundancy-review"` finds the rest.
**Scope:** the whole of `crates/`, 14 139 lines of code and 9 730 of tests
**Follows:** the phase 7 audit
([2026-08-29-phase7-refactoring-audit.md](2026-08-29-phase7-refactoring-audit.md)),
and looks for what it did not catch.

> Skill [47](../../skills/47-architecture-audit-with-subagents.md) asks for
> parallel subagents on a codebase this size. This session is configured
> without them, so the review was done serially — which means the code was
> read rather than sampled, and every finding below was checked against it.
> One candidate finding did not survive that check and is recorded in § 4
> rather than quietly dropped.

## 1. What is sound

Listed first, and specifically, so that nobody "fixes" it:

- **The central invariant holds.** The only `std::fs` anywhere in `fc-app` is
  inside a `#[cfg(test)]` module. The UI genuinely does not touch the
  filesystem.
- **No panic paths in shipped code**, with one deliberate exception: seven
  `expect`s in `pane.rs`, all GTK downcasts of widgets the same file just
  built, each carrying the invariant it asserts. That is skill
  [19](../../skills/19-no-defensive-programming.md) applied correctly, not an
  oversight.
- **No duplicated helper functions** between the crates, and exactly one
  unused constant in 149 + fc-core's own (§ 3.7).
- **The keymap's three parallel lists agree.** `Action`, `BINDINGS` and
  `ACTION_NAMES` are consistent and two tests hold them so.
- **`ops/mod.rs` is large but cohesive.** Twenty-four methods on `Run`, and
  every one of them needs the same five fields. Splitting it would mean
  threading that context through a new seam for no gain. Left alone
  deliberately, as the phase 7 audit also decided.
- **`ui.rs` must stay one file**, whatever its 2 749 lines suggest. Cargo
  builds one binary per `tests/*.rs` and runs them in parallel; the
  one-app-at-a-time mutex is a `static` and therefore per **process**.
  Splitting it across files would silently let sixteen GTK apps race for X
  servers again — the exact failure the mutex exists to prevent. If it is
  ever split, it must be into `tests/ui/*.rs` submodules of one binary.

## 2. Architecture

### 2.1 `listing` names a concrete backend — the one claim the crate makes

`crates/fc-core/src/listing/mod.rs` opens with `use crate::archive::ArchiveFs;`
so that `Listing::spawn_enter` can open an archive on the worker thread. That
is the directory model — the thing the `VirtualFs` trait exists to keep
backend-agnostic — importing one backend by name. Every other module is clean:
`archive` depends only on `vfs`, and `viewer`, `search` and `config` likewise.

It is the most serious finding here because of what it costs later rather than
what it costs now: the second a backend arrives that is not an archive, this
becomes a second `spawn_*` on `Listing`, and the model starts carrying a menu
of backends.

**Proposal.** Keep the threading in `listing` and move the *choice* out:

```rust
pub fn spawn_from(
    open: impl FnOnce() -> Result<(Arc<dyn VirtualFs>, VfsPath), VfsError> + Send + 'static,
) -> Loading
```

`spawn_load` and `spawn_load_nearest` become two-line callers of it, and
`spawn_enter` moves to `archive` (or to `fc-app`'s `navigation`, which already
decides that a file *is* an archive). `listing` then imports nothing but `vfs`
and `glob`, and the claim in
[crates/CLAUDE.md](../../../crates/CLAUDE.md) is true without a footnote.

*Cost:* ~1 h. No behaviour change; the three existing `spawn_*` tests cover it.

### 2.2 `ops` knows more about archives than its own docs admit

[ops.md](../../ops.md) says *"Nothing about an archive format reaches this
module."* `ops/mod.rs` imports `archive::{self, Packer}` and
`archive::constants::PACKING_SUFFIX`, and `Run::pack` calls
`archive::format_for` to turn a file name into a `Format`. The dependency is
one-directional and small, but the sentence is stronger than the code.

**Proposal — pick one, do not leave it as it is:**

- **(a)** `Job::Pack { sources, archive, format }`. The shell resolves the name
  (it already knows `format_for` from `navigation`), and `ops` keeps only the
  `packer(format, sink)` factory. The doc claim becomes true. The refusal for
  an unknown extension moves from a job failure to a dialog before submitting,
  which is arguably where a user wants it. ~1 h, moves one end-to-end test.
- **(b)** Soften the doc to what is true: *ops knows that formats exist and
  where the naming rule lives; it does not know what any of them are.* ~5 min.

(a) is the more correct variant (skill
[41](../../skills/41-more-correct-variant.md)); (b) is honest and free. What must
not survive is a doc that overstates a boundary — that is how the next person
puts something else behind it.

### 2.3 `PaneView` is a 55-method object, and its field list has already bitten

`pane.rs` is 1 370 lines with ~55 public methods across seven unrelated
concerns: marks, cursor, filter, navigation and backends, inline rename,
watching, and rendering.

Size is not the finding. **`exchange_with` is.** It swaps the pane's contents
by naming six fields by hand, and the phase 7 audit found a real bug there
because two of them — `fs` and `entered` — had been forgotten when archives
arrived. The shape guarantees it happens again: every field added to `PaneView`
is a silent decision about whether `Ctrl+U` should carry it.

**Proposal.** Group the swappable state into one struct:

```rust
struct Contents {
    fs: Arc<dyn VirtualFs>,
    entered: Vec<Entered>,
    listing: Listing,
    sort: Sort,
    show_hidden: bool,
    remembered_marks: Vec<String>,
}
```

`exchange_with` becomes one `mem::swap` plus the filter text, which is a widget
and genuinely separate. A new field is then carried across by construction, and
the type says which state belongs to *what the pane is showing* rather than to
the pane itself.

*Cost:* ~2 h, mechanical, no behaviour change. The `ctrl_u_carries_the_archive_across_with_the_listing`
test already pins it.

### 2.4 `Listing` is a 43-method object, a third of it selection

Fourteen of `Listing`'s 43 public methods are selection: `is_selected`,
`set_selected`, `toggle_selected`, `select_all`, `clear_selection`,
`invert_selection`, `invert_selection_files`, `select_same_extension`,
`selected_names`, `set_selected_names`, `select_range`, `select_matching`,
`selected_paths`, `selection_summary`.

**Proposal — deliberately *not* now.** A `Selection` type over the `Vec<bool>`
would take `Listing` to ~29 methods, but most of those operations need the
*view* (the visible rows after hidden-file filtering and the quick filter) and
some need `split_name`, so the extracted type would take the view as a
parameter on nearly every call. That is a seam with a cost and no defect
behind it.

Recorded as a **trigger** instead: if the selection API grows past ~16 methods,
or if a second thing ever needs to hold a selection, extract it then. Noting
the threshold is the useful half; doing it speculatively is churn.

### 2.5 Three `Shell` fields are `pub(crate)` and need not be

The phase 7 split made every `Shell` field crate-visible in one pass. Measured
against actual use outside `shell.rs`: `panes` 26, `window` 22, `active` 16,
`command_line` 9, `saved`/`drives`/`renamed`/`keymap`/`command_history` 1–2 —
and **`queue`, `config_root` and `save_queued`: zero**.

**Proposal.** Make those three private again. It costs nothing and it makes the
field list say which state is shared and which is the shell's own business.
*Cost:* ~10 min.

## 3. Redundancy

### 3.1 Two implementations of "copy bytes, count them, honour a cancel"

- `Run::stream` — a manual loop: check the token, read, write, emit
  `Advanced`; signals a cancel by returning `Ok(false)`.
- `Metered` — a `Read` adapter: check the token, read, emit `Advanced`;
  signals a cancel by returning an `io::Error` the caller re-checks against
  the token.

Same three responsibilities, two shapes, **two cancel conventions**. This is
the clearest skill [44](../../skills/44-no-redundancy.md) finding in the codebase,
and it is not cosmetic: a change to how progress is counted — coalescing events
to cut channel traffic is the obvious one, and the prime directive makes it
likely — has to be made twice, and the second one can be missed.

**Proposal.** `stream` becomes
`io::copy(&mut Metered { .. }, &mut writer)`, and the cancel test becomes the
one `pack` already uses: the copy failed *and* the token is cancelled. One
counter, one rule.

*Care:* `copy_file` relies on `Ok(false)` to trigger `discard_partial`, so the
error path must keep telling a cancel apart from a disk failure. Pinned by the
existing `CancelsMidFile` and `FailsMidRead` decorators, which is what makes
this safe to do at all. *Cost:* ~1.5 h.

### 3.2 Seven mark methods repeat the same ritual

`toggle_mark`, `extend_mark_to`, `mark_matching`, `invert_marks`, `mark_all`,
`unmark_all`, `mark_same_extension` and `restore_marks` are each
`[adopt_selection;] listing.something(); refresh_marks();`.

The repetition is the small half. The real half is that **`refresh_marks()` is
an unenforced obligation**: an eighth mark operation that forgets it does not
fail, it just silently stops repainting — a bug this project has already had
once, when marks stopped repainting because a mutated-in-place object never
re-bound.

**Proposal.**

```rust
fn marking(&mut self, change: impl FnOnce(&mut Listing)) {
    self.adopt_selection();
    change(&mut self.listing);
    self.refresh_marks();
}
```

Each method becomes one line, and forgetting the repaint becomes impossible
rather than merely unlikely. *Cost:* ~45 min.

### 3.3 `adopt_selection` is called eleven times at three layers

Once unconditionally at the top of `dispatch`, four more times in `actions.rs`
handlers, and six times inside `PaneView` methods that are only ever reached
*through* `dispatch`. It is idempotent and side-effect-free — it reads the
widget's selection index into the model — and no main-loop turn runs between
those calls, so **every one after the first in a dispatch is a no-op**.

Harmless today; the cost is that nobody can tell which call is load-bearing, so
nobody dares delete any, and a new action does not know whether to add one.

**Proposal.** Keep exactly the one in `dispatch`, delete the other ten, and
write the contract where it belongs — on `PaneView`: *the widget's selection is
adopted once per dispatched action, before the action runs.* The end-to-end
suite covers the paths that matter (`Page Up/Down` then an operation), so the
deletion is testable rather than hopeful. *Cost:* ~30 min including the
verification that each removal is genuinely covered.

### 3.4 The same test doubles exist twice

`tests/common/` already holds `Rooted`, `snapshot` and the `delegate_vfs!`
macro, and yet:

| In `ops.rs` | In `archive.rs` | Same thing |
|---|---|---|
| `NoConflictsExpected` | `Refuse` | a resolver that panics on any conflict |
| `Scripted` | `Asked` | a resolver that answers and counts |
| `Counting` (wraps a backend, counts reads and walks) | `Counting` (a memory backend counting `read_at`) | *different* things sharing a name |

**Proposal.** Move the two resolvers into `tests/common/`, and rename the
archive one to `CountingBytes` (or the ops one to `CountingWalks`) so two
different fakes stop sharing a name. *Cost:* ~30 min.

### 3.5 `progress.rs` is two modules

It holds `Meter` — the throughput arithmetic, unit-tested — and also
`human_bytes`, `human_duration` and `failure_lines`, which are rendering and
are imported by `jobs.rs` and `row.rs`. Two of the three callers do not want a
meter.

**Proposal.** Split the formatters into `format.rs`. Nothing changes but the
import path and the ability to say what each file is for. Low value, low cost —
do it while touching one of them, not on its own. *Cost:* ~20 min.

### 3.6 The `window.upgrade()` ritual

Nine action handlers open with
`let Some(window) = state.window.upgrade() else { return; };`.

**Proposal.** `Shell::window()` returning `Option<gtk::ApplicationWindow>`.
Saves nothing structural; makes nine handlers start with what they are about.
*Cost:* ~15 min. Fold into § 2.5.

### 3.7 One unused constant

`ZIP_LOCAL_HEADER_BYTES` in `archive/constants.rs`, kept "to explain what the
parser already knows" — which is a comment's job, not a constant's.

**Proposal.** Delete it; move the sentence into the comment on `data_start` if
it is worth keeping. *Cost:* ~5 min.

## 4. The finding that did not survive checking

A first pass reported that `Action::InvertMarksIncludingFolders` and
`Action::SortBy` were bound to keys but missing from `ACTION_NAMES` — meaning
two keys a user could see working and could not rebind.

**It was wrong.** Both are present; `rustfmt` had wrapped their tuples across
lines and the single-line grep did not see them. This is exactly what skill
[58](../../skills/58-block-extraction-over-single-line-grep.md) is for, and it is
recorded here rather than deleted because a review that only lists its hits is
not a review anybody can calibrate.

**What is real** is the residual gap the check exposed: an `Action` in
*neither* table would compile, would be matched in `dispatch`, and would be
unreachable and unnameable — and both existing tests would pass, because one
walks `BINDINGS` and the other walks `ACTION_NAMES`, and neither walks the
enum.

**Proposal.** `Action::ALL`, the pattern `row.rs` already uses for
`Column::ALL`, plus one test asserting every variant is both named and bound.
*Cost:* ~30 min. This is the highest value-per-minute item in the document.

## 5. What this review did not cover

- **Performance.** Nothing here was re-measured; the figures in
  [performance.md](../../performance.md) stand from phase 6. § 3.1's merge touches
  the copy loop, so it wants the copy benchmark re-run, not assumed.
- **Windows and macOS.** Both are in
  [future-improvements.md](../../future-improvements.md); nothing here changes
  either.
- **The GTK layer's own correctness.** Verified by the end-to-end suite and by
  running the program, not by reading — and the text a person reads is still
  outside what any test can see.

## 6. Order, and effort

Factor 0.25 per skill [45](../../skills/45-calibrate-effort-estimates.md).

| | Finding | Why this order | Corrected |
|---|---|---|---|
| 1 | § 4 `Action::ALL` + test | closes a silent class, costs half an hour | ~30 min |
| 2 | § 3.1 one copy loop | the only redundancy with a plausible future bug in it | ~1.5 h |
| 3 | § 2.3 `Contents` struct | the only finding with a *shipped* bug behind it | ~2 h |
| 4 | § 2.1 `listing` stops naming a backend | the crate's central claim | ~1 h |
| 5 | § 3.2 + § 3.3 marks and `adopt_selection` | same file, one commit | ~1.25 h |
| 6 | § 2.5 + § 3.6 `Shell` surface | trivial, do it while in the file | ~30 min |
| 7 | § 3.4 shared test doubles | tests only | ~30 min |
| 8 | § 2.2 the `ops`/archive boundary | pick (a) or (b) — a decision, not work | ~1 h or 5 min |
| 9 | § 3.5, § 3.7 | while passing | ~25 min |
| | **Total** | | **~8.5 h** |

Items 1–4 are worth doing on their own merits. 5–9 are worth doing while the
file is already open, and are not worth a day of their own.

**Nothing here is urgent.** The suite is green, no finding is a live defect,
and two of the nine are explicitly recommendations *not* to act (§ 2.4, and
the "leave it alone" half of § 1). The document exists so the next person
changing one of these files knows what was already noticed.

## 7. What was actually done

One commit per item, coverage checked before each one: for every finding the
test that pinned the current behaviour was found first and **probed** — the
effect disabled, the suite re-run — and only a probe that bit counted as
coverage (skill [59](../../skills/59-mutation-probe-over-coverage-percent.md)).
Where nothing bit, the test came first, in its own commit (`5c601ef`).

| | Finding | Commit | Deviation from the proposal |
|---|---|---|---|
| 0 | coverage pre-check | `5c601ef` | three end-to-end tests and one core test added where a probe found nothing biting |
| 1 | § 4 `Action::ALL` | `c02e3b0`, `ca604af` | `ALL` is `#[cfg(test)]`, not `pub` — see below |
| 2 | § 3.1 one copy loop | `ad25acc` | none; the benchmark was re-run and found **no measurable change** ([performance.md](../../performance.md)) |
| 3 | § 2.3 `Contents` | `1c13d3b` | none |
| 4 | § 2.1 `listing` | `0da7b3d` | none |
| 5 | §§ 3.2, 3.3 | `72d04fb` | **nine** deletions, not ten: `exchange_with`'s `other.adopt_selection()` is not the same call, and `marking` does not adopt |
| 6 | §§ 2.5, 3.6 | `83dbe0e` | `window` went private too, once the accessor left it no outside reader |
| 7 | § 3.4 test doubles | `3ad55f8` | both `Counting`s renamed (`CountingBackend`, `CountedFile`), not one |
| 8 | § 2.2 `ops`/archive | `a8ae20d` | option (a), the owner's choice: `Job::Pack` carries a `Format` |
| 9 | §§ 3.5, 3.7 | `83dbe0e` | `row.rs` was **not** a caller of `progress.rs`, as claimed — the callers are `jobs.rs` and `dialogs/mod.rs` |

**§ 2.4 was declined, as proposed**, and the threshold that would change that
answer is now written where somebody will meet it:
[listing.md](../../listing.md) § "When the selection should become its own type".

### The one mistake worth recording

Item 1 was committed with `clippy -D warnings` **failing**. The gate had been
run as a `&&` chain with the output truncated, so a red clippy hid behind a
green test run. Fixed in `ca604af`, and the cause fixed with it:
`scripts/green-gate.sh` now runs all six steps, reports each by name, and
fails loudly. Every commit after it went through that script.

### Audit phase (skill [49](../../skills/49-final-phase-refactoring-audit.md))

Re-reading the nine commits as one diff found one thing: item 9 created
`format.rs` for "how a number is written for a person" while leaving
`group_digits` — the size column's thousands separator — behind in `row.rs`,
which made the new module's own claim false. Moved, with the reason it is not
`human_bytes` written down. The row's *timestamp* stayed in `row.rs`
deliberately: it needs `glib::DateTime` for the local time zone, and
`format.rs` is glib-free.
