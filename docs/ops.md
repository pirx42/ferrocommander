# Ops — the file-operation engine

← Parent: [CLAUDE.md](CLAUDE.md)

`tc-core::ops` is what F5, F6, F7 and F8 actually do. It knows nothing about
threads or widgets: conflicts are answered through a `ConflictResolver`,
progress goes to a `ProgressSink`, and stopping is a `CancelToken`. The queue
supplies channel-backed versions of all three; every test supplies a `Vec` and
a script.

## Jobs

| Job | Key | What it does |
|---|---|---|
| `Copy { sources, destination }` | F5 | the sources land where `destination` says |
| `Move { sources, destination }` | F6 | the same, and each source is removed once it has arrived |
| `Delete { paths, mode }` | F8 / Del | `DeleteMode::Trash` or `Permanent` |
| `CreateDir { path }` | F7 | one directory |

A `Destination` is either `Into(dir)` — every source keeps its own name — or
`Exact(path)`, where one source lands at one path.

**There is no `Rename` job.** A rename is a move whose destination is exact,
and duplicating a file is a copy whose destination is exact. F5 and F6 open
the same dialog and what the user typed is what decides, so a separate job
would have been a second name for a shape that already existed. The parsing
rule is in [keymap.md](keymap.md).

## Packing is the same job with one step replaced

`Job::Pack` uses the same scan, the same progress events, the same cancel and
the same failure list as a copy. Only "write these bytes at the destination"
is different, and that is a `Packer` — three methods, one per format, in
[archives.md](archives.md). No file name is read for its meaning here: the
job arrives with its `Format` already decided, chosen from the archive's name
by whoever asked for the pack. The claim that "nothing about a format reaches
`ops`" used to be a claim the code did not keep — `Job::Pack` looked the
extension up itself and failed the job over an answer it could have refused
before starting.

The packer is handed a reader rather than a path, so counting the bytes and
stopping on a cancel stay here and exist once instead of once per format.

The bytes go to a temporary name beside the archive and are renamed into place
at the end, and a pack stops at the first refusal instead of carrying on: a
copy that skips a file leaves a tree missing one, which the failure list
explains, but an archive that skipped one is a single file somebody will keep.

## Scan, then execute

A job is first walked into a `Plan`: a list of `Item`s — one per top-level
source — each holding its `Task`s, its file count and its byte total.

Scanning costs one directory walk and buys three things: an honest total for
the progress bar (a bar that has to guess its total is a bar that lies), a
first cheap place for a cancel to take effect, and an execution loop that
never has to ask "what is this path" again.

Tasks are ordered so that they can simply be run in sequence: `MakeDir` before
the contents that go into it, and removals children-before-parents, so
`remove_dir` always meets an empty directory.

**A move tries the rename before it scans anything.** A rename moves a whole
tree in one syscall, so walking that tree first would be the entire cost of an
operation that is otherwise instant — 57 ms against 6 µs on 20 000 files.
Only what the rename declines is scanned: an occupied destination, where the
files inside have to meet each other one at a time, or a `CrossDevice` error,
which means "possible, just not in one step". A test pins it by asserting
that such a move performs no directory walk at all; see
[performance.md](performance.md).

## One place counts a byte, and one place notices a cancel

A copy streams through a `Metered` reader and so does a pack. It is the only
thing that emits `Progress::Advanced` and the only thing that looks at the
cancel token mid-file, so **how fast the bar moves and how soon a stop is
noticed are one decision** rather than one per caller.

They used to be two: the copy loop checked the token itself and signalled a
cancel by returning `false`, while the pack path used the reader and signalled
by erroring. Nothing was wrong with either, and that is the point — a change
to how bytes are counted would have had to be made twice, and only one of the
two had a test that the deltas add up to what the scan promised. Both do now.

The loop itself stays explicit rather than becoming `io::copy`, because the
buffer size is a decision with a reason attached: it is also how coarse a
cancel is (`COPY_BUFFER_BYTES`, 64 KiB), and `io::copy` picks its own.

A cancel and a disk failure both arrive at the caller as an error, and **only
the token tells them apart** — the message the reader carries is for a
backtrace, not for that decision. A cancel is then not a failure: an
interrupted copy reports none, because nothing went wrong and a name in front
of the user needs an action attached to it.

## Progress

`Progress::Scanned { files, bytes }` comes first and is what everything else
is measured against. Then, per path, `Started` → `Advanced` → `Finished`, or
`Failed`.

**`Advanced` carries a delta, not a running total.** No consumer has to know
the event ordering to add it up, and a dropped event costs a little accuracy
rather than a bar that jumps backwards. Σ `Advanced` equals `Scanned.bytes` on
any run that finishes — a skipped file still reports its bytes, so the bar
reaches the total the scan promised instead of stopping at 87%.

**A failed path never aborts the batch.** It becomes a `Failed` event and an
entry in `Report::failures`, and the job carries on. A batch that stops at the
first unreadable file is worse than useless on a big tree, and six failures
should cost one summary rather than six dialogs.

**That holds inside the scan too, and it is easy to get wrong.** A path the
scan cannot turn into a task — an unreadable subdirectory, a symlink it
cannot recreate — is collected into `Item::failures` and the walk carries on.
The first version returned it as an error for the whole source instead, so one
symlink anywhere inside a tree discarded every task already collected and
copied nothing at all. The unit tests missed it because they copied such a
link on its own; a real run against a real tree showed an empty target
directory. An item that collected scan failures also counts as incomplete, so
a move will not delete a source it could not fully copy.

## Conflicts

A conflict is raised when something is already at the destination. The answer
is one of:

| Resolution | Effect |
|---|---|
| `Overwrite` | replace what is there |
| `Skip` | leave both sides alone |
| `KeepBoth` | keep the existing one, put the new one beside it as `name (2).ext` |
| `Abort` | stop the whole job |

**Two directories are a merge, not a conflict.** Copying `photos` onto an
existing `photos` means the files inside meet each other. Asking about the
directory itself would be a question the user cannot act on, since neither
overwriting nor skipping the whole tree is what they meant.

**`KeepBoth` invents the name; the user does not type one.** That is what lets
*apply to all* mean something here: one answer can serve a hundred collisions
and each still lands somewhere different. The counter goes before the
extension — `notes (2).txt` — so the file still opens in the same program.

**Redirecting a directory carries its contents along.** When `KeepBoth`
resolves a directory collision, the destinations the plan computed for
everything inside it are stale. The engine records the redirect as a path
prefix and applies it to every later task, however deep.

**`ApplyToAll` is a wrapper, not a flag.** It remembers an answer marked
*apply to all* and gives it for everything after. The policy is then one pure,
testable thing, and the dialog only has to report what the user ticked.

## Cancelling, and what it does not undo

The cancel token is checked between tasks and once per turn of the copy loop,
so `COPY_BUFFER_BYTES` is also how coarse a cancel is.

**Rollback covers the file in flight, and only it.** The half-written
destination is removed, so the target never holds a truncated file that looks
whole. Files that already finished stay: undoing a completed 4 GB copy because
the fifth file was cancelled would be the more surprising behavior, and it is
what Total Commander does too.

**Not when overwriting.** If the destination existed and was being replaced,
the original was already gone the moment the file was truncated. Deleting the
remains would leave the user with neither copy instead of one damaged one, so
the partial file stays. The invariant is therefore about destinations that did
not exist before: each holds either a complete copy or nothing.

**A move only deletes what actually arrived.** If any file in an item was
skipped or failed, the source tree is left alone. The copy is the entire
reason the original is expendable, and a skip is not an arrival.

## Refusals

**A backend that cannot be written to is refused first**, before the scan,
with one `ReadOnly` per destination the job named rather than one per file
inside it. A delete asks the source backend; everything else asks the target.
A move *out of* a read-only backend is the exception that proves the rule: its
copy half is worth doing, so it runs and the delete half is reported per entry.


Two destinations are refused before anything touches the disk, because both
destroy data rather than merely failing:

- **The destination is the source.** Copying a file onto itself opens the
  destination for writing while the source handle is still open, so the copy
  reads back an empty file and the job reports success over what it just
  emptied. Not contrived: both panes can be showing one directory, and then
  the prefilled target *is* the source.
- **The destination is inside the source.** The walk would keep finding what
  it had just written.

"Inside" compares whole path components, so `/x/treeish` is not inside
`/x/tree` however alike the strings look.

**Neither question arises between two stores.** `/notes.txt` in an archive and
`/notes.txt` at the destination are two different files that happen to be
spelled the same, and refusing on that resemblance would refuse an ordinary
unpack into the root of somewhere. Both refusals are skipped when the two
backends report different [`Store`](vfs.md)s.

## Symlinks

Deleting and copying treat them differently, and both are deliberate:

- **Delete removes the link, never what it points at.** Descending into a
  symlinked directory would reach outside the tree the user selected and
  destroy someone else's files. This is correctness, not a limitation.
- **Copy follows a link to a *file*** — reading it gives the file's bytes —
  **and refuses a link to a directory.** Following it loops forever on a cycle
  (`/usr/bin/X11 -> .` is a real example), and `VirtualFs` has no `symlink`
  call to recreate it with. The refusal is reported as a per-path failure, so
  it is visible rather than silent.

## The queue

`ops::queue` runs jobs off the caller's thread: one worker thread takes them
in submission order, and everything needed while a job runs hangs off the
`JobHandle` that `submit` returns — a progress receiver, a conflict receiver,
the cancel token, and a report channel that yields exactly one `Report`.

```
submit(job, source_fs, target_fs) ──► worker thread
        │                                  │
        │  progress  ◄─────────────────────┤  unbounded
        │  conflicts ◄─────────────────────┤  question + its own reply channel
        │  report    ◄─────────────────────┘  exactly one, at the end
        └─ cancel ─────────────────────────►  shared flag
```

`async-channel` lives in `tc-core` rather than in the shell, because a channel
is not a UI dependency and because its two halves cover both directions: the
worker *waits* for a conflict answer with the blocking half, while the shell
awaits events on the GLib main loop with the async half — without `tc-core`
knowing GLib exists.

**Progress is unbounded.** A bounded channel would let a stalled UI throttle
the disk; unbounded means a busy caller slows down the display instead of the
copy.

**A conflict question carries its own reply channel**, and answering consumes
the question, so it cannot be answered twice.

**No answer means abort.** If the caller is gone, or a dialog is dismissed
without deciding, the worker must not sit on a channel forever holding a
half-copied tree. Abort rather than skip: silence is not consent to overwrite
anything, and aborting is the outcome a user can always recover from by
starting again. That also makes dropping the queue safe — a job parked on a
question wakes the moment its handles go.

## Testing

Assertions are conservation statements — file counts, byte sums,
per-relative-path contents, what the source still holds — rather than checks
on the one path a call names. An engine that writes the right file and quietly
loses a sibling passes the narrow kind of test and fails these
(skill [52](skills/52-test-conservation-invariants.md)).

The queue's own tests drive it entirely from a test thread, and the two that
would otherwise be timing-dependent are made deterministic by construction:
the cancel test parks the worker on a conflict question first, and job
ordering is checked by effect — the second job creates a directory inside the
first job's, so it can only succeed if the first already ran.

Three seams make the awkward cases reachable without a second device or a real
trash: a decorator that counts reads (so "a same-device move reads no bytes"
is checkable at all), one that reports every rename as `CrossDevice` (which
drives the copy+delete fallback), and one that cancels partway through a file.

Each invariant was checked against its own bug (skill
[59](skills/59-mutation-probe-over-coverage-percent.md)) — disable the effect,
confirm the test goes red:

| Mutation | Test that must fail |
|---|---|
| rollback on cancel removed | `a_cancel_never_leaves_a_truncated_file` |
| rename fast path removed | `a_move_within_one_filesystem_reads_no_bytes` |
| skip tracking removed | `a_move_that_skipped_a_file_does_not_delete_it` |
| modification time not stamped | `a_copied_file_keeps_the_original_date` |
| no answer means skip, not abort | `a_conflict_question_dropped_unanswered_aborts_instead_of_hanging` |

The first probe found a real hole rather than confirming one: the cancel test
originally stopped on a file that had already been read completely, so nothing
was ever truncated and the rollback could have been deleted outright with all
21 tests green. The decorator now cancels only after a read that filled the
whole buffer, which is the only moment a destination is genuinely half
written.

## A move across two stores

The fast path is a single `rename`, which only means anything within one
store: across two, the source's path handed to the target backend addresses a
different file that is spelled the same — `/packed.txt` inside an archive
naming a file at the root of the disk. So it is skipped, by comparing the two
backends' [`Store`](vfs.md), and the move is a copy followed by a delete.

Where the source cannot be deleted from — an [archive](archives.md) — the copy
half succeeds and the delete half reports `ReadOnly` per entry. Everything
comes out, nothing is lost, and the failure list says which half did not
happen. That is the honest outcome for an operation half of which is
impossible on that backend.

Pinned by a test that counts renames rather than by one that checks a result:
zero across stores, one within.

## The progress window says how fast and how much longer

Under the bar: how much of how much, then the rate, then the time left —
`10.0 MiB of 100.0 MiB · 1.0 MiB/s · 1:30 left`.

**The rate is measured over a few seconds, not over the whole job.** A run of
small files followed by one big one leaves a whole-job average saying something
that stopped being true minutes ago, and an estimate built on it is wrong for
the rest of the run. The window is `PROGRESS_RATE_WINDOW`; samples older than
that are dropped, except that the last two are always kept — events arrive when
they arrive, and a job that reported nothing for a while would otherwise be left
with a single sample and no span to measure over.

Each part appears only when it is worth trusting:

- **No rate for the first `PROGRESS_RATE_DELAY`.** A number computed from the
  first fifty milliseconds is noise, and one that appears and then halves reads
  as a program that does not know what it is doing.
- **A stall reports no rate at all**, rather than `0 B/s`. Zero divides into an
  infinite estimate, and it is not news anybody wants during a pause on a
  network mount.
- **No estimate once there is nothing left.** An estimate is a guess and says
  so by disappearing, rather than counting down to a zero it may not reach.

The arithmetic is in `Meter` and is tested without a window, like the rest of
`progress.rs`. Folding an event stays free of the clock — what a stream of
events adds up to is arithmetic, and how fast they arrived is the shell's
observation, passed in through `observe`.
