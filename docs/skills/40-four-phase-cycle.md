# Skill: Four-phase hardening cycle (feature → tests → docs → refactor)

**When.** Multi-day work spanning 1+ calendar week. The mini-commits from
parts A/B have accumulated; technical debt is building up.

**Rule.** A cycle of **4 phases** with defined durations, executed
in this order. Afterwards the next cycle begins with phase 1.

| Phase | Focus | Duration |
|---:|---|---|
| 1 | Feature addition | **1–3 days** |
| 2 | Test-coverage catch-up | **~½ day** |
| 3 | Docs consistency check | **~2 hours** |
| 4 | Architecture / performance / redundancy + refactor | **1 day** (rarely 2) |

**Full cycle:** ~3–5 days of work spread over ~1 calendar week.
Hardening cadence: ~7–10 days between phase-4 days.

**Phase 1 — Feature addition.**
- Thematically coherent feature package.
- Tests + docs flow in with each commit (baseline).
- **Boundary signal:** after 3 consecutive days with a feature-dominant
  profile (>40% `feat:` commits), moving to phase 2 is **mandatory**.
- Exit: package functionally done OR 3-day limit reached.

**Phase 2 — Test-coverage catch-up.**
- Check the coverage report, backfill edge cases, secure bugs that were
  only shallowly repaired in phase 1 with regression tests.
- No new feature code.
- Exit: coverage for paths introduced in phase 1 is close to gap-free.

**Phase 3 — Docs consistency check.**
- Check all subsystems changed in phase 1 against `docs/*.md`.
- Cross-references, stale passages, CLAUDE.md currency.
- No code — docs only.
- Exit: no discrepancies between documented and actual
  behavior.

**Phase 4 — Architecture / performance / redundancy + refactor.**
- **Morning:** analysis — hotspots (`analysis/complexity-analysis.md`),
  performance measurements, architecture scan. List the findings.
- **Afternoon:** implementation — `refactor:` commits, one finding per
  commit, `tsc + tests + build` green after each.
- Exit: all findings implemented or explicitly deferred to "later"
  (`docs/future-improvements.md`).

**Gating between phases.**

| Transition | Gate |
|---|---|
| Phase 1 → 2 | Per-commit suite green; package done or 3-day limit. |
| Phase 2 → 3 | No new coverage gaps from phase 1. |
| Phase 3 → 4 | Docs spot check shows no discrepancy. |
| Phase 4 → 1 | Findings implemented or booked. Build + tests green. |

**Why.**
- Mini-commits alone build up technical debt that becomes expensive
  to pay down later.
- A dedicated phase 4 makes hardening visible (instead of forgetting it).
- Phases 2/3 force you not to let test and docs debt keep running.
- Chimera history shows: hardening days deliver 20–30 `refactor:` commits
  in one burst (Apr 11, Apr 19, Apr 26).

**Triggers for a hardening day (phase 4).**
1. **Several feature bursts in a row** without a refactor day (>5
   feature-dominant days in a row).
2. **Size signal:** a file exceeds ~500 LOC or has become unclear.
3. **New feature on a shaky foundation:** the next extension needs a
   refactor first.
4. **Docs drifting visibly:** while getting into the code you notice the
   docs are several steps behind.

**What is NOT part of the cycle.**
- **No bugfix sprints.** Bugs are fixed in phase 1 or 4.
- **No release phase.** Deployment runs continuously (see
  `35-reproducible-deploy.md` (Chimera only)).
- **No kickoff/retro meetings.** The cycle is a working rhythm,
  not process ceremony.

**Anti-patterns.**
- Skipping phase 4 because "one more feature still fits" — debt
  builds up.
- Mixing phases — test catch-up **and** a new feature on the same day
  fragments both.
- A cycle < 4 days (phases 2–4 become overhead) or > 2 weeks without
  phase 4 (debt too high).

**Related.**
- [09-scope-before-implementation.md](09-scope-before-implementation.md)
- [11-multi-phase-commits.md](11-multi-phase-commits.md)
- [27-coverage-gap-triage.md](27-coverage-gap-triage.md)
- [28-docs-in-same-commit.md](28-docs-in-same-commit.md)
- `15-grosse-hooks-extrahieren.md` (Chimera only)
- In full: [good-development-practices.md → Part C](../good-development-practices.md)
