# Skill: Coverage pre-check before implementation

**When.** As soon as a plan is settled in substance (phases, files, functions
are named) and BEFORE the first code change happens. Phase 0 of the
plan, before phase A.

**Rule.** For each file/function the plan will modify:
1. Determine existing test coverage — are there behavioral tests for
   the function's current contracts?
2. Triage gaps per function as in
   [27-coverage-gap-triage.md](27-coverage-gap-triage.md)
   (reachable / defensive / dead).
3. **Reachable paths without tests get characterization tests NOW**
   — tests against the current behavior, BEFORE the implementation
   starts. Style: [26-behavioral-tests.md](26-behavioral-tests.md).
4. These tests are committed **before phase A**, green, and documented
   as their own plan phase 0 if applicable.

**Why.** Tests written AFTER a change are unconsciously oriented toward
the new implementation — they check what the changed function does now,
not what it *should* do. Characterization tests before the implementation
pin down the old behavior and expose regressions as soon as the new logic
deviates from the old contract. They turn a "refactor with confidence"
into a "refactor with a net".

Second effect: writing these tests forces a close reading of the
function — contract gaps, superfluous branches and silent
assumptions immediately stand out and can be worked into the plan
before they become a source of bugs.

**How (phase 0 in the plan).**
1. Insert plan phase **0 — coverage pre-check**. Per function, it lists:
   `funcName` — covered paths / missing paths / "defensive"-OK.
2. Coverage run against the affected files:
   `npm run coverage` (alias for `vitest run --coverage --exclude='**/*.slow.test.ts'`,
   since QW5/2026-06-03). Filter the result JSON `coverage/coverage-summary.json`
   or the text output. For individual files:
   `npx vitest run --coverage src/__tests__/<file>.test.ts`.
3. Write behavioral tests for the reachable gaps — one dedicated test
   commit per function, classified as `test(<area>): pre-impl
   characterization` (typographically clearly separated from the later
   feature/refactor commit).
4. After phase 0 the coverage contract is defined: **all reachable
   paths are green**. Only then does phase A begin.

**When it is _not_ needed.**
- Pure bug-fix patches without a refactor — the regression test (skill 23)
  replaces the pre-check.
- The function is already well covered (> 80% branch coverage AND
  behavioral tests visible) — the pre-check is documentation in the plan,
  not a new test.
- Pure docs change or trivial style change.

**Anti-patterns.**
- Omitting plan phase 0 "because I already know the function". Confidence
  in your head does not stay high-resolution — the test does.
- Creating characterization tests in the same commit as the refactor —
  then it is no longer provable which behavior was old and which
  was new.
- "Quick-and-dirty" as an argument for skipping — see
  [41-more-correct-variant.md](41-more-correct-variant.md), this is exactly
  the case where the more correct variant pulls the tests forward.

**Related.**
- [09-scope-before-implementation.md](09-scope-before-implementation.md) — scope is fixed **before** the
  pre-check; the coverage pre-check builds on the delimited scope.
- [10-plan-lifecycle.md](10-plan-lifecycle.md) — plan phase 0 is an
  obligatory part of a plan document.
- [23-tests-accompany-commits.md](23-tests-accompany-commits.md),
  [26-behavioral-tests.md](26-behavioral-tests.md),
  [27-coverage-gap-triage.md](27-coverage-gap-triage.md).
- `42-housekeeping-coverage-doku-drift.md` (Chimera only)
  — daily housekeeping picks up the OLD coverage gaps, skill 43 picks up
  those touched by the current plan.
