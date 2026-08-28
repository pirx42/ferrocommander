# Skill: Calibrate effort estimates

**When.** Whenever a plan document is written that contains an effort estimate
(`Effort: M`, `~2 days`, `~4 hours`, etc.).

**Rule.** After applying the correction factors, the estimate is written into
the plan doc. Do NOT duplicate the rationale for the factor in the plan text —
just state the corrected value.

**Correction factors** (empirical, from 23 archived plans, as of 2026-06-02):

| Plan type | Factor | Example |
|---|---|---|
| **Refactor with clear architecture + test net** | × **0.10** | "2 days" estimate → done in 2 h. When the architecture is clear and tests exist, the mechanical implementation is very fast. |
| **Feature with user iteration / balancing** | × **0.25** | New mechanic + UI + playtest. Iteration cycles need real wall-clock time. |
| **Multi-phase plan with playtest loops (>5 days)** | × **0.5** | These are the most honest estimates, because they actually take days of wall clock. |
| **Plan with revert risk (new mechanic, uncertain user spec)** | × **1.0** | Apply no factor. Reverts/reworks are the only source of real underestimation. |

**Default under uncertainty:** × **0.15** (between median 0.08 and geomean 0.21,
conservative toward the geomean for better outlier robustness).

**How.**

1. **Estimate as usual** in hours/days, based on phases, complexity, risks.
2. **Classify the plan type** (refactor / feature / multi-phase / revert risk).
3. **Multiply by the factor.** Example: "1-2 days refactor" × 0.10 = **2-4 hours**.
4. **Write the corrected value** into the plan doc under `## Effort`.
5. For multi-phase plans: correct each phase individually, then sum.

**Rule of thumb.** "1 day of refactor estimate = 1 hour of wall-clock implementation"
fits 90% of refactor plans. If the estimate then feels ridiculously
small — that is normal, plan estimates are systematically overestimated by a
factor of 5-10x.

**Why.** Empirical analysis of 23 plan-implementation pairs in
`docs/plans/archive/` shows:

- **Median(ratio) = 0.079**, geomean = 0.12. Plans take ~8% to 12%
  of the original estimate.
- **Refactor plans** are overestimated most drastically (geomean 0.094) —
  when the architecture is clear and tests are in place, the mechanical
  implementation is done in minutes. **Example:** 2026-06-02-propertybag-modularisation
  (estimate 13.5 h "~2 days", actual 30 min, factor 0.037).
- **Feature plans** are more realistic (geomean 0.21) — user iteration +
  balancing need real wall clock.
- Only **1 of 23 plans** was underestimated: 2026-05-26-laser-charge-
  extraction (8h estimate, actual 17.9h, factor 2.24). Reason: phase reverts.

**Anti-patterns.**

- Writing "2 days" into the plan without a factor → user expectation wrongly
  calibrated, the plan sounds more expensive than it is.
- Raising the factor after a successful implementation ("I guess I was optimistic") —
  the data says the opposite, the tendency to overestimate is robust.
- Applying the factor blindly without plan-type classification — a revert-risk
  plan (new uncertain mechanic) needs NO correction.

**Data points (today, 2026-06-02):**

| Plan | Estimated | Actual | Factor |
|---|---|---|---|
| 2026-06-02-item-applyToBag-hooks | 3-4 days (~28h) | 16 min | 0.009 |
| 2026-06-02-propertybag-modularisation | ~2 days (~13.5h) | 30 min | 0.037 |
| 2026-06-02-inverter-owner-scope | ~3-4 h | 1 h | 0.25 (feature bug fix) |

**Related.**

- [09-scope-before-implementation.md](09-scope-before-implementation.md) — scope before implementation.
- [10-plan-lifecycle.md](10-plan-lifecycle.md) — plan lifecycle (where the estimate lives).
