# Phase 7 — the refactoring audit

**Status:** Planned
**Design:** [2026-08-28-tc-clone-design.md](2026-08-28-tc-clone-design.md) § 6, phase 7.
**Skill:** [49](../skills/49-final-phase-refactoring-audit.md) — the last phase
of a plan is an audit of everything the plan built. The v1 design counts as
Implemented only after this.

## 1. Why

Six phases were built one after another, each correct on its own and each
adding to files that were already there. What that produces is not bad code —
the suite is green and every invariant is pinned — but it does produce
*drift*: a `main.rs` that grew a new `start_*` function per phase, a
`dialogs.rs` that grew a window per phase, and decisions taken in phase 2 that
phase 6 quietly made wrong.

This phase is where that is looked at as a whole rather than a phase at a
time. It ships no feature.

## 2. What the audit is looking for

Four things, in this order of seriousness:

1. **Correctness that drifted.** A rule written when there was one backend, one
   pane state, one format. Phase 6 already found two of these — the cross-store
   `rename`, and a pane restoring into a directory that has gone — and found
   them by asking the question, not by reading the code. The question is worth
   asking again everywhere.
2. **Redundancy** ([44](../skills/44-no-redundancy.md)): the same logic in two
   places, or a value that is derived in one place and hardcoded in another.
3. **Files that are no longer about one thing.** Size is a symptom, not the
   problem; the problem is a file whose name no longer says what is in it.
4. **Documentation that no longer matches the code.** Every doc claim is a
   claim about the current code, and six phases is enough for some of them to
   have stopped being true.

## 3. What is already known, before starting

Measured rather than guessed, at the end of phase 6 — 13 843 lines of code and
9 469 of tests, 513 tests:

| | Lines | What is in it |
|---|---|---|
| `tc-app/src/main.rs` | 1530 | the `Shell`, the settings, `dispatch`, every `start_*` handler, the window, the watcher wiring |
| `tc-app/src/pane.rs` | 1328 | one widget, one model, the archive stack, the watch, the inline rename |
| `tc-app/src/keymap.rs` | 1220 | 505 of table and code, 715 of tests |
| `tc-app/src/dialogs.rs` | 969 | six functions and four windows |
| `tc-core/src/ops/mod.rs` | 798 | `Job`, `run`, and three executors |

`main.rs` and `dialogs.rs` are the two whose names have stopped describing
their contents. The others are large but coherent — `keymap.rs` is mostly a
table and its tests, and `ops/mod.rs` holds the executors, which belong
together.

There are **no** `TODO`, `FIXME` or `#[allow]` markers anywhere in the crates,
and no duplicated helper functions between them. That is the starting point,
not the finding.

## 4. Sub-phases

Each is one commit, and the whole suite is green before each
([25](../skills/25-green-suite-before-commit.md)). **No behaviour changes**: a
refactoring that needed a test changed is not a refactoring
([24](../skills/24-no-silent-test-changes.md)), and the 112 end-to-end tests
are the proof.

### A. The correctness sweep

Go through the rules written before the thing that broke them existed, and ask
each one what a second backend, an async listing, or a pane that is not on a
real filesystem does to it. The two known examples came from exactly this
question. Anything found gets a test **before** a fix.

### B. `main.rs` becomes a module

The `Shell` and its settings, the action handlers, and the window construction
are three things. Split so that each file's name says what is in it; `dispatch`
stays with the window, since that is where a key arrives.

### C. `dialogs.rs` becomes a module

The four windows with state — progress, viewer, search, multi-rename — are each
their own file; the shell and the small functions built on it stay in `mod.rs`.

### D. The documentation pass

Read every doc file against the code it describes rather than against its own
last version. Particular attention to the claims phase 6 changed: what a pane
holds, what `..` means, what a move does, and what the settings file records.

### E. The plan closes

The v1 design doc is marked Implemented and archived, with an outcome section:
what the seven phases actually cost against their estimates, and which of the
design's claims were collected.

## 5. Risks

- **A refactor with no behaviour change is invisible in the tests**, which is
  exactly what makes it safe to get wrong. Every step is a green suite and a
  reviewable diff, never a rewrite.
- **The temptation to improve while moving.** A move is a move. Anything worth
  changing gets its own commit, after.
- **An audit finds more than a phase can hold.** What is not fixed is written
  into [future-improvements.md](../future-improvements.md) with its reason,
  which is what that file is for.

## 6. Effort

Factor 0.25 per skill [45](../skills/45-calibrate-effort-estimates.md).

| Sub-phase | Corrected |
|---|---|
| A. The correctness sweep | ~2 h |
| B. `main.rs` | ~1 h |
| C. `dialogs.rs` | ~45 min |
| D. The documentation pass | ~1 h |
| E. Closing the plan | ~30 min |
| **Total** | **~5.25 h** |
