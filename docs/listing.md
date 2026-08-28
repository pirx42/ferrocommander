# Listing — the directory model behind a pane

← Parent: [CLAUDE.md](CLAUDE.md)

`tc-core::listing` turns what a [VFS](vfs.md) returned into what a pane shows:
ordered rows, a hidden-file filter, a `..` row, and a cursor.

## It holds no filesystem

`Listing` is a plain value. The `VirtualFs` is passed in only to `load` and
`reload` — the two operations that actually read.

The design sketch had the listing own its backend. It does not, because the
*pane* owns the backend: stepping into an archive swaps the `VirtualFs` while
the pane lives on. Keeping the handle out of the model also means a `Listing`
can be built straight from a `Vec<Entry>` — which is how nearly every test
constructs one, and how streaming search results will feed a pane in phase 5.

That a `Listing` can be built with no filesystem at all is what proves
sorting, filtering and cursor movement never touch the disk.

## Rows

Row 0 is the synthetic `..` whenever the directory has a parent; the root has
no such row. `..` is not part of the loaded entries — it is prepended by the
accessors, so it cannot be sorted, filtered, or lost.

Navigation needs no special case for it: `VfsPath` normalizes lexically, so
`path_at` is `dir.child(name)` for every row and `dir.child("..")` *is* the
parent.

## Ordering

`Sort` is a `SortKey` (`Name`, `Ext`, `Size`, `Modified`) plus a `SortOrder`.

- **Directories sort above files, always.** The grouping is compared first and
  is never reversed: flipping the direction moves the newest file to the top,
  it does not bury the directories at the bottom. `..` stays first regardless.
- **Every key ends in a name tiebreak**, so the order is total over a
  directory's entries and `Descending` is the exact reverse of `Ascending`
  *within each group* — an invariant the tests assert directly.
- **Names compare case-insensitively with a case-sensitive tiebreak**, so
  `a.txt` and `A.txt` have a stable order rather than comparing equal. The
  comparison walks lowercased characters lazily instead of allocating two
  `String`s per comparison — it runs O(n log n) times per directory.
- **A leading dot does not start an extension.** `.gitignore` sorts as a name
  with no extension, matching both the Unix convention and Total Commander.

## Hidden entries

The VFS reports `Entry.hidden`; the listing decides whether to show it. The
raw entries stay loaded either way, so toggling costs no filesystem access —
the view is just rebuilt over the same data.

Showing hidden entries adds *exactly* the hidden ones (a count invariant the
tests pin), and toggling twice restores the identical view.

## The cursor

The cursor is an index into the visible rows and is always valid: `set_cursor`
clamps, and `move_cursor_by` stops at either end rather than wrapping.

After any rebuild — a sort change, a filter toggle, a reload — **the cursor
follows its entry by name** when that entry is still visible. A file being
re-sorted or a directory being refreshed must not slip out from under the
user. When the entry is gone, the cursor clamps back into range.

`focus_entry(name)` puts the cursor on a named row — how a pane returns the
cursor to the directory it just stepped out of. It does nothing when that
name is not visible, since the cursor cannot sit on a row that is not there.

An empty listing has no current row: `current()` and `current_path()` return
`None` rather than a placeholder.

## Loading after a job

`Listing::load_nearest` reads a directory, or the nearest ancestor that can
still be read.

A pane needs it after a file operation finishes: the job may have moved or
deleted the very directory the pane was standing in. Showing an error where a
listing belongs would strand the user somewhere they cannot navigate out of,
so the pane lands on the nearest surviving ancestor instead. It always returns
a listing — the root is the last stop, and a root that cannot be read yields
an empty one rather than no pane at all.

## Sorting is the expensive part

Ordering a directory runs the name comparison O(n log n) times — about
780 000 times for 50 000 entries — which makes it, not the disk, the
bottleneck in opening a large directory. It takes an ASCII path where that is
provably equivalent to comparing characters, which is what brought sorting
50 000 entries down from 153 ms to 22 ms. See
[performance.md](performance.md).
