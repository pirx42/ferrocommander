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

## Archives

Release build, a 10 000-entry zip with an 8 MB member, best of five.

| | Stored | Deflated |
|---|---|---|
| Open the archive (10 000 entries) | **49 ms** | 50 ms |
| List one directory inside it | **24 µs** | 26 µs |
| A 64 KB window at offset 0 | **5.7 µs** | 22 µs |
| A 64 KB window at offset 8 MB | **5.4 µs** | 1.4 ms |
| Stream the whole 8 MB member | 1.6 ms | 1.5 ms |

**Opening is buffered, and at 8 KiB rather than 64.** The parse seeks
constantly — the end-of-directory record, then every entry's local header —
and each seek throws a buffer away. Unbuffered, 10 000 entries take **63 ms**;
buffered at the default 8 KiB, **49 ms**; buffered at the 64 KiB the rest of
the layer reads in, **152 ms**, because each seek then discards eight times as
much. Pinned by a read *count* rather than a time: opening 200 entries is
allowed five container reads per two entries, which sits between the 403 it
takes buffered and the 603 it takes without.

**Browsing an open archive reads nothing at all.** The index is complete when
the archive opens, so a listing inside it is a map lookup — which is what makes
walking around in one instant. Pinned by a test asserting the container is not
touched.

**A window of a stored entry is a window of the file.** Reading near the end of
an 8 MB stored member costs one read; the same window of a compressed member
costs 1.4 ms, because a deflate stream has no seek and has to be decoded from
the start. That is the honest cost of the format — a cache would move it, not
remove it — and it is why the fast path exists for the common case of an
already-compressed payload sitting in a zip. Pinned by a test asserting the
stored window takes exactly one container read.

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

## Marking a row does not rebuild the pane

`gio::ListStore` holds one `PaneEntry` per visible row, and the first version
of `refresh` emptied it and refilled it for **every** change — including a mark
toggle. Measured on 50 000 entries, release build:

| | |
|---|---|
| Empty the store and refill it | **70 ms** |
| Splice the one row that changed | **9.6 µs** |
| Splice all 50 000 (Ctrl+A, Num *) | 60 ms |

So `Space` in a large directory cost 70 ms — a felt stall for one row changing
colour — and worse, emptying the store drops the scroll adjustment to zero
before `sync_cursor` puts it back, so the view moved under the user.

Now the marking commands go through `refresh_marks`, which finds the rows whose
mark or rename flag actually moved and splices only the span between the first
and the last. A single toggle is one row. `Ctrl+A` still replaces everything,
because everything genuinely changed — but as one splice rather than an empty
followed by a refill, so the scroll position survives.

**Replaced, not mutated.** A `ListView` rebinds a cell when its item is a
different object; mutating a row behind the model's back leaves the screen
saying what it used to, and the mark would silently never repaint. No test in
this repository could see that — the end-to-end suite asserts on the
filesystem, not on pixels — so it was checked by screenshotting the running
app before and after a `Space`. What `differs` decides has a unit test; the
repaint itself does not, and this paragraph is the record of how it was
verified instead.

**The rename editor still rebuilds.** A spliced row does not end up with the
keyboard focus the way a rebuilt one does, and a rename field that opens
without focus is no field at all. Renaming happens once in a while and can
afford 70 ms; marking cannot.
