# Skill: Last plan step = refactoring audit with correction

**When.** Every multi-phase plan in `docs/plans/`. Once the
implementation phases (feature/refactor/fix + tests + docs) are through,
the **final plan phase** is a refactoring audit over the
**entire implementation delivered by the plan**.

**Rule.** No plan counts as finished before a dedicated closing phase
has checked the plan's cumulative implementation against **architecture** and
**redundancy** and has **implemented** the improvements found.
This phase is already planned in as the last phase when the plan is written.

**Why.** Across several implementation phases, structures emerge that are
locally sensible but suboptimal in sum: duplicated logic
between phases, inline unions/constants that belong centralized (cf.
[44](44-no-redundancy.md)), special cases that hide a common pattern,
helpers that only become recognizable as extractable after phase C.
What you cannot see per individual phase, you see in the overall view. The audit
at the end is the moment when the full picture of the plan implementation
is available — that is where consolidation is cheapest and safest
(tests are green, behavior verified).

**How.**

- **When writing the plan.** The last phase is called "refactoring audit +
  correction" and appears explicitly in the phase list and in `meta`/status.
- **Scope.** What is checked is what **this plan** touched (all files
  changed/added in the phases), not the whole codebase — for that there
  is [47](47-architecture-audit-with-subagents.md).
- **Audit axes** (at minimum):
  - **Redundancy/DRY:** same logic in 2+ of the plan's commits → pull into a shared
    helper/type/constant ([44](44-no-redundancy.md),
    [16](16-no-magic-values.md)/[17](17-centralize-constants.md)).
  - **Architecture:** special cases that hide a common pattern;
    handler map instead of scattered switches (21 (`21-handler-map-pattern.md`, Chimera only));
    pure reducers out of hooks (14 (`14-pure-reducer.md`, Chimera only)); overly large functions
    (15 (`15-grosse-hooks-extrahieren.md`, Chimera only)).
  - **Consistency:** naming, file placement (layer), interface shape
    uniform across the new parts.
  - **Dead remains:** backward-compat shims, unused exports, scaffolding that
    falls away after consolidation ([20](20-no-backward-compat-shims.md)).
- **Implement the correction, don't just note it.** Points found are
  fixed in the same phase (own commit(s) `refactor(<area>): ...`),
  the suite stays green ([25](25-green-suite-before-commit.md)). What is deliberately
  deferred moves into the plan/memory as a named follow-up — not
  silently left lying around.
- **Verification.** After the audit refactor: tsc + tests + build/deploy if
  applicable, as in every phase. Behavioral equivalence is mandatory (pure refactor).

**Anti-patterns.**
- Setting the plan to "Implemented" as soon as the last feature runs — without the
  overall view of architecture/redundancy.
- Only noting audit findings as "TODO later" instead of fixing them in the phase.
- Extending the audit to the whole codebase (that is [47](47-architecture-audit-with-subagents.md));
  this is about the **plan's own** implementation.

**Source.** User spec 2026-06-05 (msg 11409): "from now on the last step of a plan
shall be a refactoring audit with correction implementation, which checks and
improves the entire delivered implementation with regard to architecture/redundancy."

**Related.**
- [10-plan-lifecycle.md](10-plan-lifecycle.md) — phase scaffold + plan-closing ritual.
- [40-four-phase-cycle.md](40-four-phase-cycle.md) — phase 4 (refactor) in the multi-day cycle.
- [44-no-redundancy.md](44-no-redundancy.md), [41-more-correct-variant.md](41-more-correct-variant.md).
- [47-architecture-audit-with-subagents.md](47-architecture-audit-with-subagents.md) — codebase-wide audit (different scope).

Memory: `feedback_plan_refactor_audit` (auto-memory under `/home/pirx/.claude/...`, outside the repo).
