# Phase 6 — archives as directories

**Status:** Implemented
**Design:** [2026-08-28-tc-clone-design.md](2026-08-28-tc-clone-design.md) § 6, phase 6.

## 1. Why

This is the phase the whole architecture was built for. The design doc's
justification for putting a `VirtualFs` trait between the UI and the disk was
that *"archives become browsable folders for free — a pane just holds a
different `VirtualFs`"* ([crates/CLAUDE.md](../../../crates/CLAUDE.md)). Phase 6
is where that claim is either collected or exposed as wishful thinking.

Concretely, what a Total Commander user expects:

- **Enter on `foo.zip` walks into it**, and the pane looks like any other pane.
- **F5 out of it unpacks**, through the ordinary copy machinery — the same
  progress bar, the same conflict question, the same cancel.
- **`Alt+F5` packs** what is marked into a new archive.
- **F3 on a file inside an archive views it** without unpacking it first.

## 2. What shapes it

**The read side is a `VirtualFs` and nothing else.** If entering an archive
needs one line of special case in the pane, in the listing, in the viewer or in
`ops`, the abstraction did not hold and the honest thing is to say so in the
plan rather than to spread the special case around. The measure of this phase
is how little outside `tc-core::archive` has to change.

**The write side cannot be.** `VirtualFs` promises `rename`, `remove_file`, and
a `create_file` that can be called on any path in any order. A zip is a
sequential stream with an index at the end; a tar is a sequential stream with
no index at all. Pretending otherwise — by rewriting the whole archive on every
call — would be a `VirtualFs` implementation that is technically correct and
useless, and it would be slow in exactly the way the prime directive forbids
([performance.md](../../performance.md)). So **the archive backend is read-only**,
and packing is its own job with its own writer, reusing the engine's scan,
progress, cancel and failure reporting rather than its per-file write step.

**A corrupt archive is ordinary input.** Every entry name in a zip is written
by whoever made the zip, including `../../etc/passwd` and absolute paths. An
unpack that trusts them writes outside the destination — the "zip slip"
failure, and precisely the kind of thing [reliability.md](../../reliability.md)
exists for: a job that reports success over a file it destroyed. Name
sanitising is therefore in the backend, tested, and not a nicety.

## 3. Phase 0 — coverage pre-check (skill 43)

| Needed | Already there |
|---|---|
| a pane over an arbitrary backend | `PaneView` already holds `Arc<dyn VirtualFs>`; `Listing::spawn_load` takes one |
| copying between two different backends | `ops::run(job, source_fs, target_fs, …)` already takes them separately |
| reading a window out of a file | `VirtualFs::read_at`, and the viewer on top of it |
| walking a tree with progress and cancel | `ops::plan` + `ops::run` |
| a job the queue can carry | `ops::Job`, `JobQueue::submit` |
| reporting a backend that will not do something | `VfsError` — needs one new variant |

Genuinely new: the archive index, the entry readers, the pack writer, and the
"leaving an archive" step in the pane.

## 4. Formats, and the dependencies they cost

| Format | Read | Write | Crate |
|---|---|---|---|
| `.zip` | yes | yes | `zip`, `default-features = false`, feature `deflate` |
| `.tar` | yes | yes | `tar` |
| `.tar.gz`, `.tgz` | yes | yes | `tar` + `flate2` |
| `.7z`, `.rar` | no | no | — |

`deflate` alone, rather than zip's default feature set: bzip2, zstd and lzma
each add a compressor this project has no evidence anybody needs, and two of
them want a C toolchain — which the Windows target does not have in this
repository's build story ([vfs.md](../../vfs.md)). An entry compressed by a method
that is not enabled is reported as unsupported, by name, rather than silently
producing wrong bytes.

`.rar` is out because no freely licensed extractor exists; `.7z` because its
decoder is large and its demand here is unproven. Both are named in
[future-improvements.md](../../future-improvements.md) rather than left as a
silent gap.

## 5. Sub-phases

### A. `tc-core::archive` — the index and the read backend, zip first

`Archive::open(fs, path) -> Result<Archive, VfsError>` reads the container's
directory once and builds a path → entry map. `ArchiveFs` wraps it and
implements `VirtualFs`:

- `read_dir`, `stat`, `open_read`, `read_at` — real.
- everything mutating — `VfsError::ReadOnly`, a new variant.
- `trash` — `ReadOnly` too. The trait doc already anticipated this: *"only the
  backend knows whether its storage has such a thing. Phase 6's archives will
  not."*

**Directories are synthesised.** A zip need not contain an entry for a
directory whose files it holds, so the index adds every ancestor of every entry
— otherwise `read_dir` on a real archive shows nothing.

**Names are sanitised at index time**, once, so no later caller can forget:
absolute paths are made relative, `..` components are dropped, and an entry
that sanitises to nothing is refused. The archive then cannot express a path
outside itself, which is a stronger statement than "the unpacker checks".

Tests: an archive made by `zip(1)` lists what is in it; a nested directory
appears even when the archive has no entry for it; `read_at` returns the same
bytes as `open_read` for every window; every mutating call reports `ReadOnly`;
a zip-slip archive lists sanitised names and nothing above the root; a
truncated archive reports an error rather than panicking.

### B. tar and tar.gz

The same `Archive`, a second reader. Two properties are different and both are
worth writing down:

- **A tar has no index.** Opening one is a full scan of its headers, and
  opening a `.tar.gz` decompresses the whole stream to do it. Measured, and
  recorded in [performance.md](../../performance.md) as deliberately slow.
- **A gz member has no cheap seek.** `read_at` decompresses from the member's
  start. A one-entry cache makes paging through the viewer bearable; past a cap
  it re-reads rather than holding the file in memory, because a file manager
  that runs out of memory on a large archive is worse than a slow one.

### C. Entering and leaving an archive

Enter on a file whose name matches a known extension puts the pane on an
`ArchiveFs` at its root. `..` at that root puts the pane back in the containing
directory of the archive file, with the cursor on it — the way `..` always
lands on where you came from ([listing.md](../../listing.md)).

The pane therefore remembers what it entered from. That is the one piece of
state this phase adds to the UI, and the plan says so plainly so that the audit
can check nothing else crept in.

### D. Unpacking

Should already work: F5 in an archive pane runs `ops::run` with the archive as
`source_fs` and the other pane's backend as `target_fs`. What needs deciding
and testing rather than assuming:

- A **move** out of an archive must not fall back to copy + delete, because the
  delete cannot happen. `ops` already treats move as single-store; a
  cross-backend move is refused with an error that says why.
- Progress totals come from the archive's own sizes, which are the uncompressed
  ones — so the bar means the same thing it always did.

### E. Packing — `Alt+F5`

`Job::Pack { sources, archive, format }`, executed by `ops` using the same scan
for the file list, the same progress events, the same cancel, and the same
failure list — with the per-file write replaced by an append to a `Packer`.
Nothing about the archive's format reaches `ops`: it sees a trait with
`add_dir`, `add_file` and `finish`.

The archive is written to a temporary name and renamed into place at `finish`,
so an interrupted pack leaves no half-archive that looks like an archive.

### F. The roundtrip invariants (skill 52)

The tests that matter here are conservation, not value asserts:

- **pack → unpack is the identity**: the same file count, the same relative
  paths, the same total bytes, the same content per file, for a tree with
  nested directories, an empty directory, an empty file, a file with a
  non-ASCII name, and a file larger than one compression window.
- **cancel leaves no archive**, only the temporary that is removed.
- **unpack of an archive containing `../` escapes nothing** — asserted by there
  being nothing at all outside the destination afterwards, not by the names
  looking right.
- **an entry whose compression method is not built in** fails that entry and
  reports it, without touching the others.

### G. Docs and the audit

`docs/archives.md`, plus the paragraphs this phase changes in
[vfs.md](../../vfs.md), [ops.md](../../ops.md), [keymap.md](../../keymap.md),
[performance.md](../../performance.md) and
[future-improvements.md](../../future-improvements.md); then skill
[49](../../skills/49-final-phase-refactoring-audit.md) — which for this phase has
a specific question to answer: **how many lines outside `tc-core::archive` did
it take?** That number is the design's report card and belongs in the doc.

## 6. Risks

- **The abstraction may not hold.** If entering an archive needs special cases
  scattered through the shell, the fix is to name them in `archives.md` rather
  than hide them. Better a documented seam than a silent one.
- **`.tar.gz` is slow to open, by nature.** Streaming the listing would hide it
  and lie about completeness; the plan measures it and says so instead.
- **Writing into an existing archive is not in scope.** Adding one file to a
  50 GB zip by rewriting it is not a feature. Packing creates; it does not
  amend.

## 7. Effort

Factor 0.25 per skill [45](../../skills/45-calibrate-effort-estimates.md). The
largest phase in the project.

| Sub-phase | Corrected |
|---|---|
| A. The zip read backend | ~2 h |
| B. tar and tar.gz | ~1.5 h |
| C. Entering and leaving | ~1 h |
| D. Unpacking | ~1 h |
| E. Packing | ~2 h |
| F. Roundtrip invariants | ~1.5 h |
| G. Docs + audit | ~1 h |
| **Total** | **~10 h** |

## 8. Outcome

Implemented across five commits (`b60c401`…`760ef1a`). The plan's own question
— how many lines outside `tc-core::archive` — is answered in
[archives.md](../../archives.md); the short version is 16 for browsing a zip,
0 for the two formats after it, and 204 for walking in and out, which is all
shell.

What the plan did not foresee:

- **The `zip` crate is better as a parser than as a reader.** `open_read`
  promises a `Send` reader and a job holds its backends across a worker
  thread, so a reader borrowing an open archive cannot exist. Decoding the raw
  range with `flate2` instead is what made a streaming, owned reader possible
  at all — and made the crate's compression features unnecessary.
- **Buffering the *index* pass is worth 30% of the opening time, and only at
  8 KiB.** The parse seeks constantly, and a 64 KiB buffer is three times
  slower than none. Measured rather than guessed, and pinned by a read count
  rather than a time.
- **Sub-phase A's zip-slip test could not tell its two mechanisms apart.**
  `VfsPath::new` already collapses `..`, so the archive layer's own filtering
  was redundant; the sanitiser is now one call to `VfsPath::new` plus the
  backslash, and the doc says which half the test bites on.
- **A second backend exposed a live bug in the engine.** A move's fast path is
  one `rename`, and `ops` handed the *source's* path to the *target* backend —
  which for an archive means `/packed.txt` naming a file at the root of the
  disk. That is what `Store` is for, and it was not in the plan.
- **Restoring a pane exposed another.** A pane opened with a plain load and
  showed an error where a listing belongs; it now falls back to the nearest
  readable ancestor, and the settings are rewritten to say where it really is.
- **Sub-phase G found the repository not following its own rule** that every
  directory with its own semantics carries a `CLAUDE.md`. Four were missing
  under `crates/tc-core/src/`; they are there now.

Effort was close to the estimate: roughly a full day against the corrected
~10 h, with sub-phase A the largest by some way.
