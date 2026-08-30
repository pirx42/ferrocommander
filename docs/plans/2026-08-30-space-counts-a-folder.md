# Space counts a folder

Status: Proposed

`Space` marks the row under the cursor. In Total Commander it does one more
thing when that row is a **folder**: it counts what the folder holds,
recursively, and puts the number in the size column. This is that.

## 1. Why it matters, and it is checkable

A pane's status line reads `n of m selected — x of y`, and the marked-bytes
half is **the number a person checks before pressing F5**. For folders it is a
lie today:

```rust
pub fn selection_summary(&self) -> Selection {
    for &position in &self.view {
        if self.selected[position] {
            summary.count += 1;
            summary.bytes += self.entries[position].size;   // 0 for a directory
        }
    }
}
```

A directory's `size` is 0 until something counts it ([vfs.md](../vfs.md) — "a
directory's inode size tells the user nothing"). So marking three folders and
reading the status line says **`3 of 12 selected — 0 B of 240 B`**, and the
one question the line exists to answer is the one it gets wrong.

That is why Total Commander counts on `Space` rather than leaving it to a
separate key: the count is not a curiosity, it is what makes the selection
total true. FerroCommander already has the separate key —
`Alt+Shift+Enter` — and the total stays wrong until somebody thinks to press
it.

## 2. Almost all of it is already built

`Alt+Shift+Enter` does this work today, and everything it needs is reusable:

| | |
|---|---|
| `tc-core::sizes::measure` | the walk, over a queue, cancel checked per directory |
| `sizes::spawn` | it on a worker, streaming one answer per folder |
| `Listing::set_measured` | the answer into the entry's own `size`, so the status total and the sort pick it up for free |
| the `+` suffix | a partial count says so ([keymap.md](../keymap.md)) |
| `Escape` | stops the rest, keeps what arrived |
| `Listing::is_measured` | whether a row already has an answer |

So this is not a new feature so much as **a second trigger for one that
exists**, and the interesting part is not the walk.

## 3. The one real problem: a second press must not cancel the first

`PaneView::measure_folders` opens with this:

```rust
// A second press re-counts, so the one already running is stopped
// first rather than left racing the new one for the same rows.
self.abandon_background();
```

That is right for `Alt+Shift+Enter`, which means *re-count everything marked*.
It is **wrong for `Space`**, which means *and this one too*: marking folder A
starts a scan, and marking folder B a second later would cancel A's before it
finished. Marking five folders in a row would leave four uncounted.

Three ways out:

- **(a) Concurrent scans.** One token per scan. `measuring` is a single
  `Option<CancelToken>` and would become a collection, and `Escape` would have
  to cancel all of them. Most code, most states to get wrong.
- **(b) A queue.** One worker, folders appended, nothing ever cancelled by a
  new request. The best behaviour and a new mechanism to build and test.
- **(c) Restart over what is left. ← recommended.** Cancel as now, and start
  again over every marked folder that is **not yet measured**. The answers
  already streamed in stay — `abandon_background`'s own comment says so, "each
  folder's number is its own and complete" — so nothing counted is lost, and
  `is_measured` already exists to do the filtering. One scan at a time, which
  is what the code already assumes.

**What (c) costs**: the folder being walked *at the moment of the next press*
restarts from scratch. Marking N folders quickly can therefore re-walk the
in-flight one up to N times. Bounded by one folder rather than by the whole
set, because everything finished is filtered out — but it is a real cost and
§ 6 measures it rather than waving at it. If the measurement says it hurts,
(b) is the answer and this plan says so in advance.

## 4. What is deliberately unchanged

- **Unmarking counts nothing.** `Space` on a marked folder takes the mark off;
  there is nothing to make true.
- **An already-counted folder is not re-counted.** The number stays until a
  re-read forgets it ([listing.md](../listing.md)), and `Alt+Shift+Enter` is
  still how you ask for a fresh count.
- **A file is untouched.** It knows its size.
- **`..` cannot be marked**, so it cannot be counted.
- **`Ctrl+A` counts nothing.** Marking everything in a directory of a thousand
  folders would start a thousand walks from one keystroke, which is the
  opposite of what the key is for.

## 5. Questions to answer before starting

1. **Do `Insert` and `Shift+↓` count too, or only `Space`?** They mark as
   `Space` does, and the argument in § 1 applies to them equally — the total
   is just as wrong. Against: `Insert` is the key you *hold* to sweep a run of
   rows, and holding it down a list of folders is exactly the burst § 3 makes
   expensive. My understanding is that Total Commander counts on `Space` only,
   but I have no way to check that here and will not assert it — this needs
   somebody who can look.
2. **Does the count follow the cursor or the mark?** `Space` marks and counts
   the same row, so the two agree. They stop agreeing if (1) says the marking
   keys count too and somebody marks a folder while a scan is running.
   Recommendation: always the row that was just marked, never the cursor.
3. **Should a partial answer be retried?** A folder whose count was cut short
   by `Escape` shows `+` and counts as measured, so `Space` on it later would
   do nothing. Recommendation: leave it — `Alt+Shift+Enter` re-counts, and a
   key that sometimes restarts a walk and sometimes does not is harder to
   predict than one that never does.

## 6. Phases — one phase, one commit

**Phase 0 — coverage pre-check** (skill 43). What holds the scan today:
`crates/tc-core/tests/sizes.rs` pins the walk, the partial answer and the
cancel; end to end, `alt_shift_enter_counts_the_marked_folders_and_the_sort_can_see_it`
drives the real key. **Nothing pins the status-line total for a marked
folder**, which is the claim § 1 rests on — so that test comes first, asserting
the lie before the fix removes it.

**Phase 1 — `Space` counts what it marks**, with (c) from § 3, and the
documentation in the same commit (skill 28): `keymap.md`'s `Space` row and its
folder-sizes section, and `listing.md` where the measured flag is described.
Tests: a folder marked with `Space` reaches the status total; a second `Space`
on another folder does not cost the first its answer; a file is unaffected;
unmarking counts nothing.

**Phase 2 — the measurement** (the prime directive: a speed decision carries a
number). What a burst of marks costs, against the restart in § 3 — marking ten
folders as fast as the key repeats, over a tree deep enough for one walk to
outlive the next press. Into `performance.md`, and if it is bad, (b).

**Phase 3 — refactoring audit** (skill 49).

## 7. Effort

Corrected per skill 45, factor 0.10.

| Phase | Raw | Factor | Corrected |
|---|---|---|---|
| 0 the status-line test | 0.5 d | 0.10 | 0.5 h |
| 1 the feature and its docs | 1.5 d | 0.10 | 1.5 h |
| 2 the measurement | 0.5 d | 0.10 | 0.5 h |
| 3 audit | 0.5 d | 0.10 | 0.5 h |

**About three hours**, and the gate is over half of it.

## 8. Found while reading, not in scope

`listing.md` says the measured flag "is a `Vec<bool>` parallel to the entries,
exactly as `selected` is". It is a **`Vec<Option<bool>>`** — the `Option` is
"has it been counted", the `bool` inside is "was the count complete", which is
what the `+` suffix reads. The distinction is the whole of how a partial
answer is told from a whole one, so the sentence is wrong about the part that
matters. One line, and it belongs to phase 1's documentation commit since that
paragraph is being edited anyway.
