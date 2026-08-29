# Reliability — the standing requirement

← Parent: [CLAUDE.md](CLAUDE.md)

**Owner spec, 2026-08-28.** The operations must be very reliable: **rather
more tests than too few**. A file manager is trusted with the only copy of
things, and the failure that matters is not a crash — it is a job that
reports success over a file it destroyed.

This sits beside [performance.md](performance.md). Where the two disagree,
they do not: an operation that is fast and wrong is worthless. Speed decides
between two *correct* designs.

## What "reliable" means here

- **No silent loss.** Every operation either does what it said or reports why
  it did not. A `Report` with no failures must mean exactly that.
- **Never destroy the only copy.** A move deletes the source only once the
  copy has arrived; a skip is not an arrival. A rollback removes a
  half-written destination, but never one that was overwriting something, as
  that would leave the user with neither.
- **A failure costs its own path and nothing else.** One unreadable
  subdirectory must not cost the other ten, and one symlink must not abort a
  tree.
- **The awkward corner gets a test, not the benefit of the doubt.** Copying a
  file onto itself, a directory into its own subtree, the same source twice,
  a read that fails halfway, a trash that refuses — each is unlikely right up
  until it costs someone their files.
- **Every invariant is checked against its own bug.** Disable the effect and
  the test must go red; a test that does not catch the thing it exists for is
  decoration (skill [59](skills/59-mutation-probe-over-coverage-percent.md)).
- **Assert conservation, not values.** File counts, byte sums, per-path
  contents, what the source still holds. An engine that writes the right file
  and quietly damages a sibling passes a value assertion and fails these
  (skill [52](skills/52-test-conservation-invariants.md)).

## What this has already caught

Each of these was found by writing the test, not by reading the code:

| Found | How |
|---|---|
| A tree containing one symlink copied **nothing at all** — the scan returned the refusal for the whole source | running the app against a real tree |
| Copying a file onto itself **emptied it** and reported success | a sweep of the awkward corners |
| The cancel rollback could be deleted outright with all tests green — the cancel was landing on an already-finished file | a mutation probe |
| A conflict dialog with no focused button: answerable only with the mouse | the end-to-end UI suite |
| Every copy silently dropped the original's permissions, so an executable arrived unrunnable | taking the requirement seriously enough to give `Entry` a mode |
| Sorting a pane and then copying one file put the order back to name | writing the end-to-end test for sorting |
| A move out of an archive would have handed the archive's `/packed.txt` to the local filesystem, where it names a file at the root of the disk | asking what a second backend does to a rule written for one |

The second one is the one to remember. Both panes open at the same directory,
so F5 on a file with the prefilled target accepted was enough — no editing, no
unusual input. It is now refused in the engine and covered twice: in the ops
suite and end to end through the real binary.

## Where the tests live

| Suite | What it pins |
|---|---|
| `crates/tc-core/tests/ops.rs` | the engine: conservation, conflicts, cancel, the awkward corners |
| `crates/tc-core/tests/queue.rs` | the background queue: ordering, the conflict round trip, cancelling from outside |
| `crates/tc-core/tests/local_fs.rs` | the filesystem backend, per call |
| `crates/tc-core/tests/trash.rs` | that a trashed file is still recoverable |
| `crates/tc-core/tests/archive.rs` | archives: escaping names, checksums, what refuses to be written, and the pack→unpack roundtrip ([archives.md](archives.md)) |
| `crates/tc-core/tests/rename.rs` | the multi-rename rules, as a table, plus what they refuse ([multi-rename.md](multi-rename.md)) |
| `crates/tc-core/tests/search.rs` | the walk: every hit and no others, refusals skipped, cancellation ([search.md](search.md)) |
| `crates/tc-app/tests/ui.rs` | the real binary, driven by real key presses ([ui-shell.md](ui-shell.md)) |

**The doubles every suite shares live in `crates/tc-core/tests/common/`** —
the tree builder, the snapshot comparison, the `delegate_vfs!` macro, and the
two conflict resolvers: `NoConflictsExpected`, which fails the test if the
engine asks anything at all, and `Scripted`, which answers in order and
records what it was asked. Both existed twice under two names until the
architecture review counted them. A double that only one suite needs stays in
that suite, and no two of them share a name.

Failure injection is done with decorators around `VirtualFs` — one that
counts reads and directory walks, one that reports every rename as crossing a
filesystem, one that cancels partway through a file, one that fails a read
partway through, one that refuses to trash anything. That is how a
single-device test box exercises a cross-device move, and how "the disk gave
up halfway" becomes an ordinary test case.
