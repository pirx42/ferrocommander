# Folder sizes — `Alt+Shift+Enter`

**Status:** In Progress — approved 2026-08-29; phase 0 done, phases 1–4 to go
**Branch:** `claude/next-phase-plan-design-lah4v5`

A directory row says `<DIR>` because nobody has counted it. `Alt+Shift+Enter`
counts it: the folder under the cursor, or every folder marked, scanned
recursively, and the answer put where a size belongs — in the size column.

## 1. What was decided, and by whom

Four questions were put to the owner; all four took the recommendation.

| Question | Answer |
|---|---|
| Where does the size show up? | **In the size column, per folder** — Total Commander's own `Alt+Shift+Enter`. |
| How does the scan behave? | **On a worker, one folder at a time**, each appearing as its scan finishes. `Escape` stops the rest. |
| What survives a re-read? | **Nothing.** `Ctrl+R`, the watcher and a finished job put the folders back to `<DIR>`. |
| What about marked *files*? | **Only folders are scanned**; files already know their size, and the accumulated total is over everything marked. On a file under the cursor with nothing marked, the key does nothing. |

The first answer is worth more than it looks, and that is why it was
recommended. `Entry::size` is what `Listing::selection_summary` sums for the
status line, and what `SortKey::Size` compares. So putting the answer there
buys two features that would otherwise each be work:

- **The accumulated total is the status line's**, already rendered, already
  paired with "n of m". Nothing new to draw.
- **Sorting by size orders folders properly**, so "which of these is the big
  one" — the question that always follows — is a keystroke rather than
  reading a column.

Five more decisions had no reason to bother the owner with:

- **"Measured" is a fact about the listing, not about the file.** A size of
  zero is a real answer for an empty folder, so "not measured" cannot be
  spelled as `size == 0`. It becomes a `Vec<bool>` beside `Listing::selected`
  — the pattern already there — rather than an `Option<u64>` on `Entry`,
  which every backend would then have to fill and which would carry the
  annotation across a layer that has no business knowing about it.
- **One arrived size splices one row.** `refresh_marks` already exists for
  "what a row *says* changed, not which rows there are", and its whole
  argument is that rebuilding fifty thousand rows to change one is 69 ms
  against 3 µs ([performance.md](../performance.md)). A size arriving is the
  same case, one row at a time.
- **Inside an archive it works.** `ArchiveFs` answers `read_dir` like any
  backend, so the scan needs no special case and gets one for free.
- **In a branch view it does nothing**, because a branch view has no
  directory rows at all ([listing.md](../listing.md)).
- **`Escape` gains a third job**, after stopping a branch walk and before
  clearing a filter. The order is what a person means: the newest thing they
  started is the thing they want stopped.

## 2. What this costs the existing code

Checked against the code rather than guessed
(skill [65](../skills/65-verify-or-ask-never-assume.md)):

| | State |
|---|---|
| `Alt+Shift+Enter` | **Free.** `Return` is bound plain (`Activate`) and with Ctrl (`InsertName`); nothing holds Alt+Shift. |
| A cancellable recursive walk | **Exists twice** — `search::spawn` and `branch::walk`. See below. |
| Streaming results to the main loop | **Exists.** `search::Results` is a receiver the shell already awaits without naming the channel crate. |
| Splicing one changed row | **Exists**, in `refresh_marks`, with the measurement that justifies it. |
| A total over what is marked | **Exists.** `selection_summary` sums `entry.size`; the status line already shows it. |
| Sorting by a folder's size | **Exists.** `SortKey::Size` compares `entry.size`; directories sort within their own group either way. |
| Forgetting the sizes on a re-read | **Free.** `Listing::reload` replaces the entries from `read_dir`, so a measured size cannot survive it by accident. |

**What has to be built** is small: the summing walk, a `measured` flag on the
listing with the one method that sets it, the key, and the arrival loop.

### The third walk

This would be the **third** breadth-first `read_dir` loop in `tc-core`, after
`search` and `branch`. Two was a considered decision — the
[branch-view plan](archive/2026-08-29-branch-view.md) recorded why they stayed
separate — and three is not the same question. Two similar loops are a
coincidence; three is a shape.

**The plan is to write it, then decide with all three on screen**, and to
record the decision either way. What makes this one different from both: it
needs no per-entry output at all, only a running total, so it is the smallest
of the three and the one most likely to fit under a shared skeleton.
Reusing `branch::walk` directly and summing its result is *not* the answer —
it allocates an entry per file to produce one number, which for a folder of a
million files is a lot of memory for a `u64`.

## 3. The awkward corners, each of which gets a test

- **An empty folder measures zero**, and zero is not `<DIR>`. This is the
  case that forces the `measured` flag and the one a wrong design silently
  gets wrong.
- **A folder that cannot be fully read** reports what it could, like every
  other walk here. A size that silently omits an unreadable subtree is a
  number nobody should trust — so what it reports and what it says about
  being partial is a decision this plan must make and test, not gloss.
- **The cursor moves while the scan runs.** Sizes arrive by *name*, the same
  rule marks already follow, so a row that has moved under a re-sort still
  gets its own answer.
- **Escape mid-scan** leaves the sizes already found in place and stops the
  rest. Unlike the branch walk, a partial answer here is not misleading:
  each folder's number is its own and complete.
- **A re-read forgets them**, including the one nudged by the watcher while
  a scan is still running.
- **Marked files are counted, not scanned**, and the status line total agrees
  with itself before and after the key.
- **A second press re-scans**, which is how a stale number is refreshed.

## 4. Phases — one phase, one commit

Docs ride in the commit that changes the behaviour
(skill [28](../skills/28-docs-in-same-commit.md)).

### Phase 0 — coverage pre-check

Skill [43](../skills/43-coverage-before-implementation.md), probes rather than
reading. What this touches is mostly code that already works, so the question
is whether that code is pinned:

- Does anything fail if `selection_summary` stops summing bytes?
- Does anything fail if `SortKey::Size` compares names instead?
- Is "a directory row shows `<DIR>`" pinned, or only observed?
- Does `refresh_marks`' splice have a test that would notice it replacing
  every row instead of the changed span?

Whatever does not bite gets a characterization test first, in its own commit.

**Done. Three of five were pinned; one gap was real and is now closed; one
cannot be pinned by anything this suite can do.**

| Probe | What was broken | Result |
|---|---|---|
| `selection_summary` sums bytes | stopped adding them | **Bit.** `selecting_everything_marks_every_visible_row`. |
| `SortKey::Size` compares sizes | compared names instead | **Bit**, three: the sort-order table, and two end-to-end tests including `ctrl_f6_sorts_by_size_and_ctrl_f6_again_reverses_it`. |
| A directory row shows `<DIR>` | rendered its byte count | **Bit**, two row tests. |
| The status line reports the marked **bytes** | reported zero | **Did not bite.** The whole suite stayed green — and this is the line the accumulated total lands on. |
| `refresh_marks` splices only the changed span | made it cover every row | **Cannot bite.** See below. |

The status-line gap is now closed by a unit test over `selection_status`,
re-probed so it bites (`1265649`). Worth noting *why* it was missing: the
existing test asserts `"0 of 2"` and `"1 of 2"` — the counts — and the bytes
sit in the same rendered string with nothing checking them.

**The splice's narrowness is a performance property with no seam.** Replacing
every row instead of the changed span is *visually identical* and only
slower, so no assertion about what is on screen can see it — and the harness
cannot inspect the store's object identity, which is what actually matters
(a `ListView` rebinds a cell when its item is a different object). It gets no
phase 0 test, and the feature is designed not to depend on it: a size that
arrives splices **its own row directly**, rather than re-deriving which rows
differ.

### Phase 1 — the size, and the listing that holds it

`tc-core`: the summing walk, cancellable, reporting per folder; the
`measured` flag beside `selected`; the one method that records an answer by
name; and `reload` forgetting them by construction. Headless tests, including
the empty-folder-measures-zero case.

**This phase carries a measurement**, because the prime directive says a
performance claim does: the scan of the 20 000-file tree the benchmark
already builds, against the 31 ms the branch view costs over the same tree.

### Phase 2 — the key and the arrivals

`Action::FolderSizes` on `Alt+Shift+Enter`; the scan started for the marks or
the cursor row; each answer spliced into its row as it lands; `Escape`
stopping the rest. The end-to-end test marks two folders and checks the
status line's total — which is the accumulated answer the request asked for.

### Phase 3 — the corners

Everything in § 3, each with its test: the unreadable subtree, the cursor
moving mid-scan, the re-read forgetting, the marked file counted rather than
scanned, the second press.

### Phase 4 — refactoring audit

Skill [49](../skills/49-final-phase-refactoring-audit.md), and **the third
walk is decided here** if phase 1 left it open. Plus the cookbook row in
`CLAUDE.md`, [listing.md](../listing.md), [keymap.md](../keymap.md), and
`scripts/check-links.py`.

## 5. What is deliberately not in this

- **Measuring every folder in the pane at once**, which is what Total
  Commander's `Alt+Shift+Enter` actually does. The request was the focus or
  the marks, and `Ctrl+A` then this key is already that.
- **Keeping sizes across a re-read**, and the "this number is old" marker
  that would have to come with it.
- **A separate "space occupied" window**, TC's `Ctrl+L`. The status line
  already shows the total.
- **Watching a measured folder** so its number stays true. That is a watch
  per subtree, which is a different feature with a different cost.

## 6. Effort

Factor 0.25 per skill [45](../skills/45-calibrate-effort-estimates.md).

| Phase | Raw | Corrected |
|---|---|---|
| 0 — coverage pre-check | ~2 h | ~30 min |
| 1 — the size and the listing | ~4 h | ~1 h |
| 2 — the key and the arrivals | ~4 h | ~1 h |
| 3 — the corners | ~4 h | ~1 h |
| 4 — audit | ~2 h | ~30 min |
| **Total** | | **~4 h** |

The estimate's soft spot is phase 2. Sizes arriving one at a time into a list
that may be **sorted by size** is the one place this feature can make the
view move under the cursor, and the branch-view plan hit the same class of
problem from the other side. If that turns out to need more than splicing a
row, it is phase 2 that grows.
