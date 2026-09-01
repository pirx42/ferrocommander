# Space counts a folder

Status: Implemented

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

A directory's `size` is 0 until something counts it ([vfs.md](../../vfs.md) — "a
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
| `fc-core::sizes::measure` | the walk, over a queue, cancel checked per directory |
| `sizes::spawn` | it on a worker, streaming one answer per folder |
| `Listing::set_measured` | the answer into the entry's own `size`, so the status total and the sort pick it up for free |
| the `+` suffix | a partial count says so ([keymap.md](../../keymap.md)) |
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
  re-read forgets it ([listing.md](../../listing.md)), and `Alt+Shift+Enter` is
  still how you ask for a fresh count.
- **A file is untouched.** It knows its size.
- **`..` cannot be marked**, so it cannot be counted.
- **`Ctrl+A` counts nothing.** Marking everything in a directory of a thousand
  folders would start a thousand walks from one keystroke, which is the
  opposite of what the key is for.

## 5. Questions to answer before starting

1. **Do `Insert` and `Shift+↓` count too, or only `Space`?**
   **Answered by the owner: only `Space`.** `Insert` and `Shift+↓` mark and
   nothing more, which is Total Commander's split and also removes the worst
   case in § 3 — the key you *hold* is no longer the key that starts walks.
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
`crates/fc-core/tests/sizes.rs` pins the walk, the partial answer and the
cancel; end to end,
`alt_shift_enter_counts_the_marked_folders_and_the_sort_can_see_it` drives the
real key.

**The pre-check corrected the plan.** § 1 argues from the status line, and the
first draft of this phase proposed a test pinning that total. There can be no
such test: the status line is a GTK label, and the end-to-end suite sees window
titles and the filesystem and nothing else
([future-improvements.md](../../future-improvements.md)). The count is observable
only through the one thing that reacts to it — **sorting by size** — which is
exactly how the `Alt+Shift+Enter` test above does it, and how phase 1's must.
So this phase produces no commit of its own; it produced a correction.

**Phase 1 — `Space` counts what it marks**, with (c) from § 3, and the
documentation in the same commit (skill 28): `keymap.md`'s `Space` row and its
folder-sizes section, and `listing.md` where the measured flag is described.
Tests, all observed through the sort: two folders marked with `Space` are both
counted, which is the accumulation § 3 is about; `Insert` marks without
counting, which is the owner's split.

**Done — and the probes found that two of the three tests could not fail.**
Removing the counting from `Space` turns the first test red, as it should. But
*restricting the restart to only the newest folder* — the exact bug (c) exists
to avoid — left it green, because the fixture's folders are 4 KB and 1 byte and
the first scan finishes long before the second keystroke lands. The
accumulation is never contended, so the end-to-end test passes either way. The
same went for the guard that stops an *unmarking* press counting: no
end-to-end test can see a scan that should not have started.

So the rule moved where it can be checked: `owed_from` is a pure function over
the counting list and the listing, and
`a_counted_folder_is_no_longer_owed_and_an_uncounted_one_still_is` pins it
headlessly. Both probes bite there — returning everything, and returning only
the newest. The end-to-end pair keeps the user-visible outcome; the unit test
keeps the mechanism.

That is the second time in two days a test has passed for a reason other than
the one it was written for, and both times a probe was the only thing that
said so.

**Phase 2 — the measurement. Done**, and it says (c) stands: the queue in (b)
is not built.

Ten folders, the first holding 20 000 files. Counting all ten once costs
**16 ms**; the modelled worst case, where every press re-walks everything
unfinished, costs **156 ms** — a ratio of 9.8×, which turns out to be the
wrong number to look at.

The large folder counts in **16 ms on its own**, about 0.8 µs a file. So the
worst case assumes something a person cannot provoke: at thirty presses a
second a folder would need forty thousand files before a keystroke could
outrun the walk. And the wasted work is on the scan thread, not the main
loop — nothing stalls, which is what the prime directive is actually about.

Recorded in [performance.md](../../performance.md) with the shape that would
change the answer: a directory of folders each large enough to outlast a
keystroke.

**Phase 3 — refactoring audit** (skill 49). **Done**, and it found a stall.

`owed` asked the listing which marked folders already carried a size, one
`index_of` per name — a forward scan of the view, on the main loop, on every
`Space`. **18.7 ms for 100 marked folders in a directory of 50 000, 179 ms for
1000.** An answer now takes its own folder off the list as it arrives, which is
O(names) and reads no rows; `Listing::index_of` went back to private, since
nothing outside needed it after all.

**The first benchmark said 0.02 ms.** It drew its names from the front of the
listing, where a forward scan finds them immediately. Moving them to the end
showed the real number — a hundred times larger. A benchmark that chooses its
own inputs chooses flattering ones unless somebody stops it, which is the same
lesson as the fixture that was too small to contend in phase 1, arriving twice
in one plan.

The unit test moved with the rule and is probed both ways. **Its wiring is
not** pinned: removing the call from `measured` leaves the end-to-end test
green, because a list that never shrinks is a superset — every folder is still
counted, merely re-walked. The absence is a performance regression, and no test
here can see one. Said rather than left looking covered.

**Left alone.** `start_folder_sizes` and `toggle_mark_and_count` are two
four-line functions that differ only in which folders they ask for and share
`count_folders`. That is the shape the plan wanted: one scan driver, two
questions.

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
