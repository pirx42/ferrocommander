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
constructs one.

That a `Listing` can be built with no filesystem at all is what proves
sorting, filtering and cursor movement never touch the disk.

## Rows

Row 0 is the synthetic `..` whenever the directory has a parent; the root has
no such row. `..` is not part of the loaded entries — it is prepended by the
accessors, so it cannot be sorted, filtered, or lost.

Navigation needs no special case for it: `VfsPath` normalizes lexically, so
`path_at` is `dir.child(name)` for every row and `dir.child("..")` *is* the
parent.

**A root can be given the row anyway.** `offer_parent` adds one where there is
no parent to point at, which is what the root of an [archive](archives.md)
needs: nothing above it inside the archive, and somewhere to go all the same.
Where the row leads is then the pane's business — `dir.child("..")` still
normalizes to the root, and a pane that sees that answer knows it means "out".

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

## The read happens on a worker thread

A directory read is the one thing the shell does with no bound on how long it
takes — a cold cache, a network mount, a hundred thousand files — and doing it
on the UI thread froze the window for all of it. `Listing::spawn_load` reads on
a thread and the pane takes the result on the main loop, like a job.

**The whole listing arrives at once.** Not laziness: the view is *sorted*, so an
entry read late belongs in the middle, and showing rows as they arrive would
shove everything below them down while the user is looking at it. One swap
moves the list once. The pane also keeps showing what it has until the new
listing lands, rather than blanking first — there is nothing to put there that
is more true than what is already on screen.

**A read that is overtaken is dropped.** The pane records what it asked for;
an answer for anywhere else is a navigation that has since been superseded, and
applying it would make two quick steps land in whichever order the reads
happened to finish.

**The thread is here; the choice of backend is not.** `spawn` takes a closure
and runs it on a thread of its own, and everything a pane waits for goes
through it — so a read that forgot to be a thread cannot happen. What that
closure *does* is the caller's: `spawn_load` reads a directory, and
[`archive::spawn_enter`](archives.md) opens an archive first and then reads its
root. Both are one arrival, because each is one thing the user did.

That split is deliberate. Opening an archive belongs on the worker thread —
it is the slow half — but putting it *here* would make the directory model
name a concrete backend, which is exactly what `VirtualFs` exists to prevent.
The model supplies the thread; the backend supplies what to do on it.

### A pane is about where it is *going*

Keys arrive faster than listings. Press Enter and then F7 quickly enough and
both are dispatched before the first read comes back — so a pane asked where it
*is* answers with the directory it is leaving, and F7 makes the directory in the
wrong place. That is not hypothetical; it is what happened.

So everything that acts **in** a directory asks `PaneView::target_dir`, which is
where the pane is going if a read is in flight and where it is otherwise.
Anything acting on what is **marked** keeps reading the listing, because the
marks belong to the rows the user was looking at when they made them.

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

## Selection

Marks live in a `Vec<bool>` parallel to the loaded entries, not in a set of
names. Selecting everything is then a fill, reading the selection is a scan,
and neither sorting nor filtering costs anything at all, because neither
touches the entries themselves. A set of names would allocate a string per
marked file for a question the pane asks on every redraw
([performance.md](performance.md)).

**The `..` row can never be marked.** It is not an entry — it is a navigation
control the model synthesises — so it is not in the array at all, and every
route in (set, toggle, select-all, invert, by pattern) leaves it alone by
construction rather than by five separate checks.

**Select-all, invert and select-by-pattern act on what is visible**, not on
everything loaded. What a filter or the hidden-file flag is hiding is not
something the user can see to have meant.

**Marks survive a reload by name**, exactly as the cursor does: a job that
changed one file must not silently drop the marks on the others. Names that
are gone fall out, names that are new arrive unmarked.

That is `Listing::reload`, and it is what `Ctrl+R` and the directory watcher
both go through — see [watching.md](watching.md). Navigation and the re-read
after a job use `Listing::load` instead and start with nothing marked, which is
right: those are a different directory, or a directory whose marks the job just
spent.

Pattern selection uses `tc-core::glob` — `*` and `?`, case-insensitive, which
is what Total Commander accepts and what a person types. It lives outside
`listing` because the [search](search.md) uses the same matcher.

### When the selection should become its own type — and why it has not

Fourteen of `Listing`'s public methods are selection, which is a third of the
object. Extracting a `Selection` over the `Vec<bool>` was considered by the
2026-08-29 architecture review and **deliberately declined**: most of these
operations need the *view* — the visible rows after the hidden-file flag and
the quick filter — and some need `split_name`, so the extracted type would
take the view as a parameter on nearly every call. That is a seam with a cost
and no defect behind it.

The threshold is written down instead, because it is the half that is useful
later: **extract it if the selection API grows past about sixteen methods, or
the first time a second thing has to hold a selection.** Until one of those
happens, splitting it is churn.

### Files-only is the default, directories the opt-in

`invert_selection_files` flips the visible **files**; `invert_selection` flips
the directories too. Total Commander's split, and the useful default: a person
inverting a selection is nearly always thinking about files, and having every
directory in the pane join in is a surprise that costs a second keystroke to
undo. `select_same_extension` follows the same rule — a directory called
`photos.backup` is not one of "the `.backup` files" — and a cursor row with no
extension picks out the other extension-less files, which is that rule applied
honestly rather than a special case.

### A selection is restorable only by name

`selected_names` / `set_selected_names` are how a selection is put away and
brought back, because names are the only form of it that survives anything:
every finished job builds a fresh listing and every sort reorders the one that
is there, so an index restored later points at a different file than the one it
was taken from. Restoring **replaces** what is marked rather than merging into
it, and a name that is no longer here is simply not marked — the usual case,
since the selection being restored is the one the last operation consumed.

`selected_names` reports in **display order**, so a sort legitimately reorders
it; what survives a round trip is the set, not the sequence.

## The quick filter

`set_filter` narrows the visible rows to names containing a string, ignoring
case. It is applied in `rebuild` beside the hidden-file rule, so narrowing
costs one pass over the loaded entries and never re-reads the directory — and
it composes with sorting, hidden files and the cursor rule because it is the
same code that already handled those.

The match has an ASCII fast path like the sort comparison, for the same
reason: at 50 000 entries it runs 50 000 times between one keystroke and the
next ([performance.md](performance.md)).

The cursor follows its entry while that entry is still visible and clamps when
it is not. Marks are untouched — narrowing the view is not a change of intent
— but `select_all` afterwards takes only what is left, which is what makes the
two safe together.
