# Archives as directories

← Parent: [CLAUDE.md](CLAUDE.md)

An archive is a [`VirtualFs`](vfs.md). Entering one swaps the pane's backend;
the listing, the viewer, the search and the copy engine carry on without
knowing which backend they are talking to.

This is the claim the two-crate split was made for — *"archives become
browsable folders for free"* ([crates/CLAUDE.md](../crates/CLAUDE.md)) — so
this file keeps score of what it actually cost.

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
common case of an already-compressed payload — a jpeg in a zip — pages as fast
as a file. Measured figures: [performance.md](performance.md).

## Formats

| | Read | Write | Notes |
|---|---|---|---|
| `.zip` | yes | | deflate and stored |
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

## Directories that are not there

A zip need not carry an entry for a directory whose files it holds. The index
creates every ancestor of every entry, or `read_dir` on a real archive would
show nothing. A synthesised directory takes the date of the entry that implied
it; the archive's root takes the container's own date, because nothing inside
implies it.
