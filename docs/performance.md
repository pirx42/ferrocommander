# Performance — the standing requirement

← Parent: [CLAUDE.md](CLAUDE.md)

**Owner spec, 2026-08-28.** The app must be fast. Where a decision trades
speed against something else — a prettier surface, a larger feature set, a
tidier-looking abstraction — **speed wins**. The audience is hard-core Total
Commander users, and what they come for is a file manager that never makes
them wait.

This is not a licence to write unreadable code. It settles the cases where
the choice is genuinely between two working designs.

## What it means in practice

- **Everything between a keystroke and a redrawn pane is hot.** A directory
  of 50 000 files is not exotic, and 200 ms of it is felt.
- **Never do work whose result is thrown away.** A same-device move must not
  walk the tree it is about to move with one syscall.
- **Prefer the cheap primitive where it is provably equivalent.** Bytes
  instead of Unicode tables when the two agree on the answer; a
  directory-relative stat instead of one that re-resolves the whole path.
- **A performance claim in this repository carries a measurement.** "Faster"
  is not a review comment, and an optimisation without a number next to it is
  a guess.
- **An optimisation needs a test that pins the property, not the timing.**
  Timing tests are flaky; "this operation performs no directory walk" is not.

## Measured baseline

Release build, 50 000 files in one directory and a tree of 20 000 files, best
of five runs on the development box. The point of the table is the *shape* —
the ratios and what dominates — not the absolute numbers.

| Operation | Before | Now |
|---|---|---|
| List a directory of 50 000 entries | 222 ms | **79 ms** |
| — of which reading the directory | 78 ms | 67 ms |
| — of which sorting and building the view | 153 ms | **22 ms** |
| Move 20 000 files within one filesystem | 57 ms | **6 µs** |

### Where the wins came from

**Sorting was the bottleneck, not the disk.** Ordering a directory runs the
name comparison O(n log n) times — about 780 000 times for 50 000 entries —
and `char::to_lowercase` walks Unicode tables and yields an iterator per
character, because one character can lowercase into several. Filenames are
almost always ASCII, where lowercasing is one instruction on a byte and byte
order is character order. The comparison takes that path when both names are
ASCII and falls back to the full character rule the moment either is not, so
nothing is traded away. Pinned by a test that asserts the two orderings agree
over every pair of an awkward set, plus one that the result is still a total
order.

**Reading a directory re-resolved every path.** `symlink_metadata(full_path)`
walks the path from the root for each entry and allocates a `PathBuf` to do
it; `DirEntry::metadata` stats relative to the directory that is already
open. Same result, same lack of symlink following. The path is now built only
for entries that turn out to be symlinks.

**A same-device move scanned the tree it was about to rename.** The scan
existed to give the progress bar an honest total, which is right for a copy
and pure waste for a rename that moves the whole tree in one syscall. The
rename is attempted first now; only what it declines — an occupied
destination, or a `CrossDevice` error — is scanned and copied. Pinned by a
test asserting that such a move performs no directory walk and reads no
bytes.

### Re-measured after phase 3

Phase 3 added a selection array, a quick filter and an attributes read to the
listing path, so the numbers were taken again rather than assumed: **79 ms**
to list 50 000 entries and **6.5 µs** to move 20 000 files within one
filesystem. Marks cost nothing to carry because they are a `Vec<bool>` beside
the entries, and the filter is folded into the pass that was already
happening.

## What is deliberately still slow

**The listing loads whole directories.** No pagination, no incremental
display: `read_dir` returns everything before the pane draws. At 50 000
entries that is ~80 ms, which is acceptable; at a million it would not be.
The fix is streaming the model into the view, which needs the pane to render
rows it does not yet have.
*From:* [listing.md](listing.md).

**One `stat` per entry is unavoidable** as long as the pane shows size and
date, which is the whole point of the columns. The 12 ms floor in the table
is names only.
