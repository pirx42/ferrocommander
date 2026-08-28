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

## Known gap

**Move assumes one store.** The rename fast path and the `CrossDevice`
fallback both address a single backend. Phase 2's UI has exactly one, so it
holds; phase 6 introduces a second and is where a cross-store move has to be
told apart from a same-store one.
