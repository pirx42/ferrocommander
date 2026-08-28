# Skill: Plan file names + lifecycle (Draft → Archived)

**When.** As soon as a task has multi-phase character and a
plan document in `docs/plans/` makes sense.

**Rule — file name.** The plan file MUST start with `YYYY-MM-DD-`
(memory `feedback_plan_filenames`). Example: `2026-05-24-item-consolidation-audit.md`.

**Rule — topic branch (since 2026-07-18, owner msg 15495).** Every plan
starts with its own topic branch ON dev
(`git checkout dev && git checkout -b topic/<plan-slug>`); during the
plan, commits go only there (topic pushes are free). After acceptance/approval:
`git checkout dev && git merge --no-ff topic/<plan-slug>` — the merge is
part of completing the plan. Details: skill
`64-branch-workflow-dev-topic-main.md` (Chimera only).

**Rule — status header.** First or second line after the H1:
```
Status: Draft | In Progress | Implemented | Deferred | Archived
```
For "Implemented" + "Archived", additionally the implementation commit hashes
(`commits abc123/def456`) or a note `(commits see git log --grep ...)`.

**Rule — calibrate effort estimates.** If the plan contains an effort
estimate (`~2 days`, `effort: M`, `~4 hours`): before writing it
into the plan doc, apply the factor from [45-calibrate-effort-estimates.md](45-calibrate-effort-estimates.md).
Empirically: refactor plans are overestimated by a factor of 10x,
feature plans by 4x. Default ×0.15.

**Rule — phase 0 coverage pre-check.** After scope/phase planning
and BEFORE phase A:
1. For each file/function the plan modifies: review existing test
   coverage, classify gaps via [27-coverage-gap-triage.md](27-coverage-gap-triage.md)
   (reachable / defensive / dead).
2. Reachable paths without tests get characterization tests **before**
   phase A that pin down the *current* behavior — style:
   [26-behavioral-tests.md](26-behavioral-tests.md).
3. Phase 0 is documented as its own plan phase; its commit carries
   `test(<area>): pre-impl characterization` and is cleanly separated from
   the refactor commit.

Details + rationale in [43-coverage-before-implementation.md](43-coverage-before-implementation.md).

**Rule — final phase = refactoring audit.** The concluding phase
of every plan is a refactoring audit + correction across the entire
plan implementation (architecture/redundancy). It is planned in as the
last phase when writing the plan and actually carried out before the plan
counts as "Implemented". Details: [49-final-phase-refactoring-audit.md](49-final-phase-refactoring-audit.md).

**Rule — end-of-plan ritual.** As soon as the implementation (incl. audit phase)
is complete:

1. **(a) Extract substance.** Does the plan contain permanently useful
   conceptual material (tables, rationales, API schemas)? → move it into the
   appropriate `docs/` entry (`architecture.md`, `property-system.md`,
   `items.md`, ...). Plan status becomes
   "Implemented + substance extracted to `<file>`".

2. **(b) Archive.** If the plan was essentially a
   step-by-step guide whose traces live in the code:
   `git mv docs/plans/<file>.md docs/plans/archive/<file>.md`
   with an updated status header.
   **Never** archive before (a) has been cross-checked — otherwise you
   get a two-pass operation with a git gap.

3. **(c) Delete.** Only if the plan contained pure implementation to-dos
   with no lasting value (rare).

**Rule — cross-refs.** Active plans as `docs/plans/...md`, archived ones as
`docs/plans/archive/...md`. When archiving, simultaneously repoint cross-refs
in code/docs to the new path.

**Rule — audit.** Every 4 weeks or after big sprints, three questions per
plan: (i) is the status header still correct? (ii) does substance live in the
plan that would belong in `docs/`? (iii) should the plan be archived?

**Why.** Plan documents otherwise drift between "active" and "long dead".
The date in the name sorts chronologically + shows age at a glance.
Status header + archiving separate the active plan backlog from historical
material.

**Anti-patterns.**
- `plans/item-audit.md` without a date → in 6 months it is unclear whether it is current.
- Marking a plan "Implemented" but leaving the substance in the plan file → docs
  drift, because readers do not look for it there.
- Archiving before the substance has been extracted to `docs/`.

**Related.**
- [09-scope-before-implementation.md](09-scope-before-implementation.md)
- [28-docs-in-same-commit.md](28-docs-in-same-commit.md)
- [43-coverage-before-implementation.md](43-coverage-before-implementation.md) — the phase-0 operation.
- [49-final-phase-refactoring-audit.md](49-final-phase-refactoring-audit.md) — the final-phase operation.
