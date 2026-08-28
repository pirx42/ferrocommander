# Skill: Multi-phase changes go into separate commits

**When.** A refactor / feature is non-trivial and touches multiple
subsystems or multiple logical steps.

**Rule.** Split into phases — **each phase is its own commit**, tested
and build-green on its own. No batch commits containing several
independent changes.

**Why.**
- **Revert granularity:** a bisect finds the bug more easily in
  10 small commits than in 1 big one.
- **Review readability:** small commits are reviewable; big ones get
  skimmed.
- **Discipline:** separate phases force you to think about whether the
  intermediate steps actually work.

**How.**
1. Before the refactor: sketch the phase list — what must come before what?
2. Phase N: only code for N. Tests/build/docs included. Commit.
3. Phase N+1 builds on N.
4. If phase N cannot go green on its own → the phase was cut wrong,
   re-cut it.

**Example.** Refactor "replace data model X":
- Phase 1: new types + loader + JSON migration. → Commit
- Phase 2: switch the runtime over, old types removed. → Commit
- Phase 3: bring the UI along. → Commit
Each phase green on its own.

Concretely in Chimera (from the damping generalization 2026-05-28):
```
Damping D1: constant + OR aggregation + helper                → Commit
Damping D2: phase C damped + pull out clamp                   → Commit
Damping D3: update tests                                      → Commit
Damping D4: docs + build + deploy + commit + push             → Commit
Count C1:   invertHeat:bool → invertCount:number              → Commit
Count C2:   phase C N-fold application                        → Commit
Count C3:   tests + docs + deploy                             → Commit
```

**Anti-patterns.**
- An 800-line commit "refactor: data model + runtime + UI all at once".
- Committing phase 1, phase 2 green without tests, "I'll squeeze that in
  with the push".

**Related.**
- [32-commit-per-step.md](32-commit-per-step.md)
- [25-green-suite-before-commit.md](25-green-suite-before-commit.md)
- [04-multi-phase-autonomy.md](04-multi-phase-autonomy.md)
