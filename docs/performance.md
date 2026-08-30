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
| Branch view (`Ctrl+B`) of a 20 000-file tree | — | **31 ms** |
| Folder size (`Alt+Shift+Enter`) of the same tree | — | **20 ms** |
| — the same 20 000 files in *one* directory, for comparison | — | 32 ms |

**Walking 200 directories costs no more than reading one.** The branch view
of a tree of 20 000 files takes 31 ms; a plain listing of 20 000 files in a
single directory takes 32 ms. That is the useful shape: the cost is *per
entry* — one `stat` each, which the table above already names as the
unavoidable floor — and the directory boundaries between them are free. So
`Ctrl+B` costs about what looking at the same number of files costs anyway,
and the reason it is on a worker thread with a cancel is the tree that holds
a million of them, not the one that holds twenty thousand.

**Counting a folder is cheaper than listing it**, which is not obvious and is
the reason the scan needed measuring rather than assuming. The same walk over
the same 20 000-file tree costs 20 ms for a size and 31 ms for a branch view:
a size keeps a running `u64` where the branch view allocates an `Entry` and a
relative-path `String` per file. That is also the argument against building
the size out of `branch::walk` — it would pay the 31 ms *and* the memory to
produce one number.

Reproduced with `cargo run --release -p tc-core --example bench_branch -- <dir>`.

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

## Type-ahead, and what a keystroke costs

A letter no binding claims searches the rows ([listing.md](listing.md)), which
is a case-insensitive substring match over **every visible row, on the main
loop, on every keystroke**. At 50 000 entries that is the shape this document
warns about, and it had no number until this was measured. Release build, best
of twenty, four runs, no filesystem — a `Listing` built straight from entries,
so the figure is the comparison rather than the disk.

| | 50 000 entries |
|---|---|
| A hit a few rows down — the ordinary case | **0.2 µs** |
| A miss: the whole view walked and wrapped | **1.2 ms** |
| The same miss with one non-ASCII name among the 50 000 | 1.2 ms |
| The quick filter, the same needle, for comparison | 1.1 ms |

**The ordinary case is free and the worst case is affordable.** Type-ahead
stops at the first match, so a letter that finds something costs a few
comparisons rather than fifty thousand — three orders of magnitude between the
two rows, and the top one is what happens when the search is working. The
1.2 ms is what a letter matching *nothing* costs, and at a fast typist's ten
keystrokes a second that is about 1% of the time, on a directory the document
calls large.

**The filter costs the same, which is the point.** Both run
`contains_ignoring_case` over the same entries, and the ~0.1 ms between them is
the filter rebuilding the view where type-ahead returns an index. "Does this
name match what was typed" having one answer in this program is a claim about
correctness in [listing.md](listing.md); it turns out to be a claim about cost
too.

**One non-ASCII name does not slow the other 49 999 down.** The fast path is
chosen per comparison, not per directory, so an awkward name pays the full
character rule and its neighbours do not. That was the design; this is the
check.

**Where it would stop being fine.** The cost is linear in the visible rows, so
a directory ten times larger makes a fruitless keystroke 12 ms — felt. The fix
if anybody ever meets that is to search from the cursor outwards and stop at
the first match rather than filtering, which is already what it does; what
would have to change is the *miss*, and the only honest way to make a miss
cheaper is to not scan on every letter. Nothing about that is worth building
today, and this row is what would say when it is.

Reproduced with
`cargo run --release -p tc-core --example bench_type_ahead`.

## Archives

Release build, 10 000 entries plus an 8 MB member of pseudo-English (deflate
gets about 2.4× on it — a repeating block compresses 400× and would make every
compressed number here a fiction), best of three.

| | zip stored | zip deflated | tar | tar.gz |
|---|---|---|---|---|
| Open it | 45 ms | 50 ms | 47 ms | **28 ms** |
| List one directory inside | 22 µs | 24 µs | 23 µs | 23 µs |
| A 64 KB window at offset 0 | **3.8 µs** | 64 µs | **4.0 µs** | 1.4 ms |
| The same window at offset 8 MB | **3.7 µs** | 7.6 ms | **4.1 µs** | 8.7 ms |
| Stream the whole 8 MB member | 1.0 ms | 7.5 ms | 0.83 ms | 8.8 ms |

The `.tar.gz` opens fastest only because its container is a tenth of the size
and the index pass is dominated by reading; it pays for that on every window,
because a gzip stream has to be decompressed from the beginning to reach one.

**Opening a zip is buffered, and at 8 KiB rather than 64.** The parse seeks
constantly — the end-of-directory record, then every entry's local header —
and each seek throws a buffer away. Unbuffered, 10 000 entries take **72 ms**;
buffered at the default 8 KiB, **45 ms**; buffered at the 64 KiB the rest of
the layer reads in, **155 ms**, because each seek then discards eight times as
much. Pinned by a read *count* rather than a time: opening 200 entries is
allowed five container reads per two entries, which sits between the 403 it
takes buffered and the 603 it takes without.

**Browsing an open archive reads nothing at all.** The index is complete when
the archive opens, so a listing inside it is a map lookup — which is what makes
walking around in one instant. Pinned by a test asserting the container is not
touched.

**A window of a stored entry is a window of the file.** Reading near the end of
an 8 MB stored member costs one read; the same window of a compressed one costs
milliseconds, because neither a deflate stream nor a gzip stream has a seek and
both have to be decoded from the start. That is the honest cost of the format —
a cache would move it, not remove it — and it is why the fast path exists for
the common case of an already-compressed payload sitting in a zip or a plain
tar. Pinned by a test asserting the stored window takes exactly one container
read.

**Opening a tar is a full scan and always will be.** A tar carries no index, so
every header has to be read to know what is in it, and a `.tar.gz` has to be
decompressed entirely to read them. There is no faster version of that
question; there is only a version that says so.

## Merging the two copy loops changed nothing measurable

Recorded because the prime directive asks for a number, and the honest number
here is "no signal". Copying 100 MiB as 400 files of 256 KiB, best of five,
before and after the copy path moved onto the shared metered reader:

| | Runs |
|---|---|
| Before | 174 ms, 158 ms |
| After | 151 ms, 205 ms, 225 ms |

The ranges overlap and the spread within one version is wider than the gap
between them — this box's I/O is shared and noisy. The first pair looked like
a 13% win and was not; three runs were enough to say so.

Which is also what the code predicts: the same number of reads and writes of
the same size, plus one call per turn that inlines away. **The property, not
the timing, is what the suite holds:** the progress deltas add up to what the
scan promised, on both the copy path and the pack path.

## What is deliberately still slow

**The listing loads whole directories.** No pagination, no incremental
display: `read_dir` returns everything before the pane draws. At 50 000
entries that is ~80 ms, which is acceptable; at a million it would not be.
The fix is streaming the model into the view, which needs the pane to render
rows it does not yet have.
*From:* [listing.md](listing.md).

**A branch view is re-walked on the main thread after a job**, and on
`Ctrl+R`. The walk that `Ctrl+B` itself starts is on a worker with a cancel;
the re-walk that replaces `Listing::load_nearest` is not, because the re-read
it replaces is not either — same exposure, on a list of the same size
(31 ms for 20 000 files). Both move to a worker together, or neither does.
*From:* [listing.md](listing.md).

**One `stat` per entry is unavoidable** as long as the pane shows size and
date, which is the whole point of the columns. That is what the **67 ms** of
"reading the directory" in the table above is: one `stat` each, and it is the
part no amount of work on this side removes.

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
