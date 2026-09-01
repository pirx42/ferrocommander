# Watching — noticing what another program did

← Parent: [CLAUDE.md](CLAUDE.md)

A pane shows what it read. Something else adding or deleting a file used to be
invisible until you navigated away and back — there was not even a key to ask.
Now there are two ways in: the pane watches its own directory, and `Ctrl+R`
re-reads on demand.

## A re-read is not something you can see happening

Whether the change came from `Ctrl+R` or from the watcher, the rows are
brought up to date underneath whatever the user is looking at: the view does
not move, the marks survive by name, and the cursor stays on the row it was
on. Until 2026-09-01 it did move — a rebuild cost the list its scroll anchor,
so a file appearing in the directory yanked the pane onto its cursor row. The
mechanism, and why putting the offset back afterwards could not fix it, is in
[performance.md](performance.md).

## Both go through the same re-read

`PaneView::reread` uses `Listing::reload`, which is the one that keeps the
**marks by name** ([listing.md](listing.md)). A refresh that silently dropped a
selection somebody spent a minute building would be worse than not refreshing at
all. The cursor follows the same way — by name, not by row number, because a
file appearing above it shifts every index below.

**The scroll offset is put back** around it, in pixels rather than rows: a
change elsewhere in the directory must not move what is being looked at. Rows
above the viewport coming and going will shift the view, which is the rarer
case and the one where "the same position" has no better answer.

A directory that has gone falls back to the nearest ancestor that can still be
read, the same as after a job.

## Reading a directory is not a change to it

`inotify` reports an **access** to any child of the watched directory —
another program listing a subdirectory, or this one doing it on its own
behalf. The watcher discards `EventKind::Access` for that reason: a pane that
re-read itself because something read the tree below it would be doing work
nobody asked for, over and over.

That was not a precaution. The folder-size scan ([keymap.md](keymap.md)) reads
every subdirectory of the pane it is counting, which nudged the watch, which
re-read the pane, which threw the counted sizes away — reliably, every time,
so the feature did not work at all until this filter existed. It was found by
building the feature, not by reading the watcher.

An **error** from the watcher still counts as a change. A watcher that has
lost track of a directory is precisely when a re-read is worth doing.

## The watcher is coalesced, not streamed

`fc-core::watch` holds one `notify` watcher per pane — inotify on Linux,
`ReadDirectoryChangesW` on Windows, behind one interface, which is the shape
every platform difference in this project takes. It is in `fc-core` because
watching a directory is a filesystem concern and the UI does not reach past its
own layer.

**One nudge per quiet period**, and the nudge says nothing about what changed.
An unpacking archive fires an event per file, and re-reading a fifty-thousand
entry directory per event would make the program unusable exactly when it is
busiest ([performance.md](performance.md)). The pane re-reads everything
anyway, so which file moved is not information it can use. The timer restarts
on every event, so a directory under continuous change is re-read when it
settles rather than never.

## Dropping a watcher is not free

`notify`'s watcher joins its worker thread when it is dropped, and that worker
sits in a poll with a timeout — so replacing a pane's watch on the UI thread
stalled **every navigation** by up to a fifth of a second, measured. The old
watch now goes onto a thread of its own to be dropped. Nothing waits for it: an
inotify registration outliving its pane by a few milliseconds costs nothing.

Starting one is cheap by comparison — under 300 µs for a start-and-drop pair.

## What it deliberately does not do

- **It does not watch a subtree.** A pane shows one directory; waking it for
  changes it is not displaying would be work for nothing.
- **It does not re-read under an open rename.** Rebuilding the rows would take
  the editor away mid-word.
- **It does not fail.** A directory that cannot be watched — a network mount, a
  filesystem the platform does not cover, a permission that is not there — is a
  pane that does not refresh itself, not a pane that fails to open. `Ctrl+R`
  still works, which is most of why that key exists.
- **A watch is replaced on navigation**, and a nudge that crosses a navigation
  is dropped: it is about a directory nobody is looking at any more.
- **It does not watch inside an archive.** There is no operating-system path
  there to watch, and registering one anyway would either fail or watch a real
  directory that happens to have the same name. `Ctrl+R` still re-reads, from
  the archive's index — so it will not notice the archive itself being
  replaced ([archives.md](archives.md)).

## What is not covered by a test

**The scroll offset.** The end-to-end harness drives the app through X and
asserts on the filesystem; there is no way for it to read where a pane is
scrolled to. The marks, the cursor and the re-read itself all have tests — and
each has a probe that breaks them — but the scroll restore is verified by hand.
Said here rather than left to look like coverage it is not.
