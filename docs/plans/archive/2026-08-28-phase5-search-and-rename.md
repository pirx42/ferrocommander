# Phase 5 — search (Alt+F7) and the multi-rename tool (Ctrl+M)

**Status:** Implemented
**Design:** [2026-08-28-tc-clone-design.md](2026-08-28-tc-clone-design.md) § 6, phase 5.

## 1. Why

Two tools a hard-core Total Commander user reaches for daily, and the two the
design doc names for this phase:

- **Search (Alt+F7)** — by name pattern and by file content, **streaming**
  results that can feed a pane.
- **Multi-rename (Ctrl+M)** — pattern-based renaming with counters, name and
  extension placeholders, search-and-replace, a **live preview**, and undo.

## 2. What shapes each

**Search: it must stream, and it must stop.** A search over a home directory
finds the first hit in milliseconds and the last one in minutes. Waiting for
the whole walk before showing anything makes the fast case feel like the slow
one, and a search that cannot be cancelled makes the slow one unbearable. Both
are the speed requirement, not polish ([performance.md](../../performance.md)).

**Multi-rename: it must be right, and it must be visible before it runs.**
Renaming a hundred files by a rule nobody could check first is exactly the
operation this project's reliability requirement exists for
([reliability.md](../../reliability.md)). So the rule engine is **pure** — rules
times names to a preview — table-tested, and the preview is the same function
the rename uses. A preview that is computed differently from the thing it
previews is worse than none.

## 3. Phase 0 — coverage pre-check (skill 43)

| Needed | Already there |
|---|---|
| walking a tree | `ops::plan` walks one, but for copying; a search wants its own, streaming |
| name matching | `glob::matches` — `*` and `?`, case-insensitive |
| reading file bytes | `VirtualFs::read_at`, added in phase 4 for the viewer |
| a worker thread and a channel to the main loop | `command::spawn`, `Listing::spawn_load`, `ops::JobQueue` |
| cancelling something long-running | `ops::CancelToken` |
| renaming | `Job::Move` with `Destination::Exact` |
| a list to pick from | `dialogs::choose_one` |
| splitting a name from its extension | `listing::split_name` |

Genuinely new: the streaming walk, the content matcher, and the rename rules.

## 4. Sub-phases

### A. `fc-core::search` — the walk

`search::spawn(fs, roots, criteria, cancel) -> Receiver<Found>`: a thread walks
and sends each hit as it is found.

- **Name matching** reuses `glob`.
- **Content matching** reads each candidate in windows through `read_at` and
  looks for the needle, so a search does not hold a file — the same reason the
  viewer does not.
- **A directory it cannot read is skipped, not fatal.** Half a home directory
  is unreadable on any real machine, and a search that stops at the first
  `EACCES` is a search nobody can use.
- **Cancellation is checked between entries**, so stopping is immediate rather
  than at the end of the tree.

Tests: every hit and no others, for name and for content; a needle spanning a
window boundary is still found; an unreadable directory does not stop the walk;
cancelling stops it; the results are the same whatever order the walk takes.

### B. The search dialog

`Alt+F7` asks for a pattern and an optional content string, then fills a list
as results arrive. Enter on a result sends the active pane to the file's
directory and puts the cursor on it.

### C. `fc-core::rename` — the rule engine

Pure: `rename::preview(rules, names) -> Vec<Renamed>`. A template with
placeholders, plus an optional search-and-replace:

| Placeholder | |
|---|---|
| `[N]` | the name without extension |
| `[E]` | the extension |
| `[C]` | a counter, starting where the user says |

Tests are table-driven, as the design doc asks — rules × names × expected — and
the invariants get their own: a preview never produces two identical names, an
empty result falls back to the original rather than to nothing, and previewing
twice gives the same answer.

### D. The multi-rename dialog and the rename itself

`Ctrl+M` on the marked files: template, counter start, replace, and a live
preview table that updates as the rules are typed. Applying submits one move
per file through the existing queue, so conflicts are asked about the way they
always are.

**Undo** is the previous names, kept after applying, and one more batch of
moves to put them back.

### E. Docs and the audit

`docs/search.md`, `docs/multi-rename.md`, the keymap, and skill 49.

## 5. Risks

- **Content search over a large tree is slow by nature.** Streaming and
  cancellation are the answer; a index is not in scope and would be a lie about
  freshness.
- **The preview must be the rename.** If they ever diverge the tool becomes
  dangerous. One function, used twice.
- **Undo after something else moved the files** — checked at apply time by the
  same conflict machinery, not by trusting the recorded names.

## 6. Effort

Factor 0.25 per skill 45. The largest phase since 2.

| Sub-phase | Corrected |
|---|---|
| A. The walk | ~1.5 h |
| B. The search dialog | ~1 h |
| C. The rule engine | ~1 h |
| D. The rename dialog | ~1.5 h |
| E. Docs + audit | ~45 min |
| **Total** | **~5.75 h** |

## 7. Outcome

Implemented across four commits (`6a11bd9`, `cd078c8`, and the multi-rename
commit that closed it). What the plan did not foresee:

- **`PATTERN_DEFAULT` was the wrong default to reuse.** The selection dialogs
  prefill `*.`, which as a search pattern matches nothing; the search field
  needed its own `*`. The first version of the feature found no files at all,
  and the end-to-end test is what said so.
- **The walk-order tests could not bite until the walk was forced.** The search
  uses a stack rather than recursion, so "an unreadable directory does not stop
  it" only means something if the unreadable directory is not the last one
  visited. The fixtures name their directories so the order is known, and a
  `CancelsMidDirectory` decorator cancels from inside `read_dir` rather than
  hoping the timing lands.
- **The batch-collision refusal needed a test that could see it.** Both the
  preview and the job queue stop two files from landing on one name, so the
  file survives either way; the assertion had to be that no conflict dialog
  ever opened.

Subsystem docs: [search.md](../../search.md),
[multi-rename.md](../../multi-rename.md), plus the two new rows in
[keymap.md](../../keymap.md) and the undo gap in
[future-improvements.md](../../future-improvements.md).
