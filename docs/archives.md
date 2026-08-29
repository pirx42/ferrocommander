# Archives as directories

← Parent: [CLAUDE.md](CLAUDE.md)

An archive is a [`VirtualFs`](vfs.md). Entering one swaps the pane's backend;
the listing, the viewer, the search and the copy engine carry on without
knowing which backend they are talking to.

This is the claim the two-crate split was made for — *"archives become
browsable folders for free"* ([crates/CLAUDE.md](../crates/CLAUDE.md)) — so
this file keeps score of what it actually cost.

## The report card

The plan for this phase said its own measure of success would be how few lines
it took **outside** `tc-core::archive`, so here is the count. Added lines of
code, comments and blanks excluded, per step:

| | Inside `archive/` | Outside | Tests |
|---|---|---|---|
| Reading a zip as a filesystem | 464 | **16** | 444 |
| Adding tar and tar.gz | 109 | **0** | 91 |
| Walking in and out of one | 0 | 204 | 99 |
| Unpacking | 6 | 30 | 311 |
| Packing (`Alt+F5`) | 210 | 203 | 193 |

The first two rows are the claim being collected. Sixteen lines — one pair of
`VfsError` variants, a platform function for the mode an archive records, and
a `mod` line — bought a pane that lists a zip, a viewer that reads inside one,
and a search that walks it. The next two formats cost nothing outside at all.

The third row is the honest price, and it is all shell: a read has to arrive
with the backend it was read from, the pane has to remember what it walked
into, and a listing has to be able to offer a `..` row it did not earn.
Browsing an archive is free; *entering* one is not.

The fourth row is not integration at all — it is a bug the second backend
exposed. See the `Store` section below.

The fifth is a new feature rather than a cost of the abstraction: packing is
work that did not exist before, and 203 of its lines are the job, the key, the
dialog and the name it offers.

## Read-only, and why that is not a compromise

`VirtualFs` promises a `create_file` that can be called on any path in any
order, a `rename`, and a `remove_file`. A zip is a sequential stream with its
index at the end; a tar has no index at all. Honouring those promises means
rewriting the container on every call — correct, useless, and slow in exactly
the way the [prime directive](performance.md) forbids.

So every mutating call reports `VfsError::ReadOnly`, including `trash`: an
archive has no recycle bin, which the `VirtualFs` trait had already predicted
it would not. One error for the whole backend rather than a different one per
method, because whether an archive can be written to is a property of the
archive and the engine should be able to say so once.

Packing is therefore its own job with its own writer, sharing the engine's
scan, progress, cancel and failure reporting but not its per-file write.

## What holds the archive open: nothing

Opening reads the container's directory once and keeps offsets. Reading an
entry builds a decoder that **owns** its range of the container and reads
through `read_at` — no shared handle, no mutex, no borrow of an open archive.

That is not tidiness. `VirtualFs::open_read` promises a
`Box<dyn Read + Send>`, and a file operation runs on a worker thread holding
its backends across the whole job. A reader borrowing a shared archive cannot
be sent anywhere, and the alternatives — re-parsing the central directory per
file, or reading whole entries into memory — are quadratic and unbounded
respectively.

It also means the container is read *through* whatever backend holds it,
rather than as an operating-system file. An archive on any backend works,
including, when there is a reason for it, one inside another.

## Walking in and out

`Enter` on a file whose name says it is an archive opens one and shows its
root. `Enter` on `..` there, or `Backspace`, comes back out — to the backend the
archive was opened from, in the directory holding it, **with the cursor on the
archive file**. That is what `..` has always meant here: where you came from.

Three small things had to be true for that to work, and they are worth naming
because they are the whole cost of this phase outside `tc-core::archive`:

- **A read arrives with the backend it was read from.** Opening an archive is a
  full parse — a zip's central directory, or an entire `.tar.gz` — which is the
  same unbounded wait a directory read is, so it happens on the same worker
  thread. A pane that had already swapped backends would be pointing at an
  archive it could not yet show, or at one that turned out not to open.
- **The pane remembers what it entered.** A stack of `(backend, archive path)`,
  outermost first, so an archive inside an archive needs no thought. It is the
  only state entering an archive adds to the shell.
- **A listing can be given a `..` row it did not earn.** An archive's root has
  no parent *inside* the archive, and still has somewhere to go. Where the row
  leads is the pane's business; that it is there is the listing's.

## A job says which pane it writes into

A job's two backends used to be the active pane's and the other pane's,
always. That was invisibly right while there was one backend and both were the
same object, and wrong the moment a pane could hold an archive: `F7` builds its
path from the active pane, so inside an archive it would have created a
directory **on the disk**, at the path the archive calls it — `/made/` at the
root of the filesystem. Not a failure; a real directory in the wrong place.

So each job now says it. `F5`, `F6` and `Alt+F5` write into the other pane,
which is what two panes are for; `F7`, `F8`, `Shift+F6` and a rename write in
the pane the user is looking at. The end-to-end test presses `F7` inside an
archive and asserts that nothing was created anywhere — in it, beside it, or in
the other pane.

**Everything that moves a pane moves its backend with it.** The drive bar
leaves the archive rather than sending it to `/mnt/backup`, a path it has never
heard of; `Ctrl+←`/`Ctrl+→` carries the backend and the archive stack across,
not just the path; and `Ctrl+U` swaps them along with the listings, because
swapping only the listings leaves both panes reading the wrong filesystem. Each
of the three is a rule written when there was one backend, and each has a test
that fails without the fix.

`Alt+F5` writes the archive on the other pane's backend, so a bare name typed
into its field resolves against the other pane's directory: the prefill and the
destination have to be the same place, or a name typed over the prefill lands
somewhere the prefill never mentioned.

## Where an archive is, as far as everything else is concerned

Two different paths mean two different things, and keeping them apart is what
stops a job addressing the wrong filesystem:

- **What a job uses** is the path in the backend's own coordinates — `/deeper`
  inside the archive. An operation is handed that backend, so that is the only
  path it can mean.
- **What a person reads** is the two composed: `…/bundle.zip/deeper`. That is
  what the path bar shows and what the settings file records, because `/` in a
  settings file would send the next start to the filesystem root.

Restoring such a path needs no special case. Reading a directory inside an
archive off the local filesystem fails, and the pane already walks up to the
nearest ancestor that reads — which is the directory holding the archive. So a
restart lands beside the archive rather than inside it, and the settings are
rewritten to say so.

## Unpacking is the copy engine, and nothing else

F5 out of an archive is `ops::run` reading one backend and writing another,
which is what it has taken two backends for since phase 2. The progress bar,
the conflict question, the failure list and the cancel are the ones they always
were, and the totals come from the archive's own recorded sizes — the
uncompressed ones — so the bar means what it always meant.

**Two backends are not one filesystem, and the engine now knows it.** Every
backend reports a `Store`, and two backends reporting different stores share
nothing but the shape of a path. That settles two rules that were quietly
wrong the moment a second backend existed:

- **A move's fast path is a single `rename`, and it is skipped across stores.**
  Running it would hand the *source's* path to the *target* backend —
  `/packed.txt` inside an archive naming a file at the root of the disk. The
  paths look alike and address different files, so the shortcut has to be
  skipped rather than merely allowed to fail. Pinned by a test that counts
  renames: zero across stores, one within.
- **"Copying onto itself" is a question that does not arise across stores.**
  `/notes.txt` in an archive and `/notes.txt` at the destination are two
  different files, and applying the rule anyway would refuse an ordinary
  unpack into the root of somewhere.

A **move** out of an archive therefore copies and then cannot delete: every
entry comes out, and the archive reports `ReadOnly` for the half it will not
do. Nothing is lost and the reason is on screen, which is the honest outcome
for an operation half of which is impossible.

## Packing — `Alt+F5`

The extension picks the format, by the same rule that decides whether Enter
walks into a file. One rule, so a name this writes is a name that opens again;
a name that decides nothing — `.rar` — is refused rather than answered with a
guess.

The **scan, the progress, the cancel and the failure list are the engine's**.
A pack is `ops::run` with exactly one step replaced: "write these bytes at the
destination" becomes "append this entry", behind a `Packer` trait with three
methods. Nothing about a format reaches `ops`, and nothing about jobs reaches
the packer.

The packer is handed a **reader**, not a path. That reader is the engine's: it
counts the bytes for the progress bar and refuses to go on after a cancel, so
the counting and the cancelling exist once rather than once per format.

**An archive that exists is an archive that finished.** The bytes go to a
temporary name beside the final one and are renamed into place at the end — a
rename within one directory, so there is no moment where the name exists
holding half an archive. A cancelled or failed pack removes the temporary and
leaves nothing at all: not an empty archive, and not a half-written one under
a name somebody will later open.

For the same reason a pack **stops at the first refusal** rather than carrying
on. A copy that skips one file leaves a tree missing a file, and the failure
list explains it; an archive that skipped one is a single file somebody will
keep, and the explanation is long gone by the time it matters.

A file keeps its date and its mode. On Windows there is no Unix mode to
record, so none is written, rather than a Win32 attribute mask that would read
on Linux as a permission nobody asked for. A directory's date is read with one
extra `stat` per directory: it is not on the scan's task, because no copy can
restore a directory's date — and an archive can hold one.

Zip counts its dates from 1980 and stores no zone, so a date is written as the
UTC calendar date the reader gives back, and anything before 1980 is written
as no date rather than as a wrong one.

## What does not work inside an archive, and says so

- **The directory watcher is off.** There is no operating-system path to watch,
  and registering an inotify watch on one that happens to look like a real path
  would be worse than not watching. `Ctrl+R` still re-reads — from the index,
  so it will not notice the archive being replaced underneath.
- **The command line refuses**, and so does `F4`. A path inside an archive is
  not somewhere a process can run, and it is not a path an editor can be handed
  either: running either against whatever that path means on the real
  filesystem is how something meant for an archive acts on a home directory
  instead — or creates a file on the disk when the editor saves. `F3` reads
  through the backend and works.
- **Everything that writes is refused with `ReadOnly`**: `F7`, `F8`,
  `Shift+F6`, and the target side of `F5`, `F6` or `Alt+F5`. Refused **once**,
  before the scan, rather than once per file — a copy of a large tree into an
  archive would otherwise be a failure list with a thousand identical lines,
  which is the same as no failure list at all. The exception is a move *out
  of* an archive, whose copy half works and whose delete half is reported per
  entry.

## An archive cannot express a path outside itself

Every entry name in an archive was written by whoever made the archive,
`../../etc/passwd` included. An unpacker that trusts those names writes outside
the destination — the "zip slip" failure, and precisely the kind of thing
[reliability.md](reliability.md) exists for.

Names are therefore resolved **once, at index time**, by handing them to
`VfsPath::new`, which is total by construction: a leading separator, `.`, and
`..` beyond the root all collapse away. There is no such thing as a `VfsPath`
outside the root, so there is no such thing as an archive entry outside the
archive. A name that resolves to the root itself — `../..` — is filed nowhere
rather than under some fallback.

Doing it in the index rather than in the unpacker is the point: it is a
property of the type, not a check every caller has to remember. The one thing
`VfsPath` does not know about is the backslash, which archives written on
Windows separate with, so that is replaced first.

The end-to-end test asserts the escaping name lands *inside*; it cannot tell
apart the two mechanisms that make that true, and the `VfsPath` half is pinned
by `VfsPath`'s own tests.

## Bytes are checked, not trusted

A zip records a CRC32 per entry, and every reader here verifies it at end of
stream. A copy that reports success over a file it got half right is the
failure this project's reliability requirement is about, and the check costs
one pass over bytes already in cache.

An entry compressed by a method this build cannot decode — bzip2, zstd, lzma —
stays in the listing and fails when read, naming the method. Hiding it would be
a listing of something other than the archive. Encrypted entries fail the same
way: reading one needs a password, and there is nowhere to ask for it that
would not also have to hold it.

## What is deliberately slow

`read_at` on a **compressed** entry decodes from the entry's start, so paging
a viewer into the middle of a large compressed member is O(offset). That is
the honest cost of a format with no seek; a cache would move the cost, not
remove it. A **stored** entry is read straight out of the container, so the
common case of an already-compressed payload — a jpeg in a zip, anything in a
plain tar — pages as fast as a file. Measured figures:
[performance.md](performance.md).

**Opening a tar is a full scan**, because a tar has no index; opening a
`.tar.gz` decompresses the whole stream to read the headers. There is no faster
version of that question. And because a gzip stream has no seek either, *every*
entry in a `.tar.gz` is reached by decompressing everything before it — the one
place where the wrapper, not the entry, is what costs.

## Formats

| | Read | Write | Notes |
|---|---|---|---|
| `.zip` | yes | yes | reads deflate and stored, writes deflate |
| `.tar` | yes | yes | read in place, like a zip |
| `.tar.gz`, `.tgz` | yes | yes | one gzip stream around a tar — see below |
| `.7z`, `.rar` | no | no | see [future-improvements.md](future-improvements.md) |

The `zip` crate is a **parser** here and nothing else: it is depended on with
its default features off, so no compressor comes with it, and the entry bytes
are decoded through `flate2` over the raw range the central directory points
at. What it is for is the part nobody should hand-roll — the end-of-directory
record, zip64, the two date encodings, and where each entry's bytes begin.

Dates are converted by hand rather than by adding a date library for six
fields. A zip stores local wall-clock time with no zone, so it is read as UTC;
being an hour out in a column is a smaller lie than showing no date. It also
stores seconds halved, so odd seconds do not exist in the format and a reader
that invented the missing one would be claiming to know something the file does
not say.

## What a tar holds that a listing cannot show

A tar records symlinks, hard links, devices and fifos. None of them is listed.
There is nowhere in a pane to say what a symlink inside an archive points at,
and their recorded size is zero — so listing one would unpack it as an empty
file, which is a copy that quietly got it wrong and exactly what
[reliability.md](reliability.md) is about. Named in
[future-improvements.md](future-improvements.md) rather than left as a
surprise.

A tar also records no checksum of an entry's contents, only of its header, so
nothing there can be verified the way a zip's CRC is.

## Directories that are not there

A zip need not carry an entry for a directory whose files it holds. The index
creates every ancestor of every entry, or `read_dir` on a real archive would
show nothing. A synthesised directory takes the date of the entry that implied
it; the archive's root takes the container's own date, because nothing inside
implies it.
