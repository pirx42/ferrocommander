# Skill: One Logical Step = One Commit (Immediately, Not at the End)

**When.** You are working through a multi-step task (plan with phases,
refactor spanning multiple modules, bug round with multiple bugs).

**Rule.** **Commit EVERY completed individual step immediately** —
don't collect them for the end. One commit = one conceptual change. Two
independent fixes = two commits, even if they were noticed in the same
context.

**Why.**
- **Revert granularity:** a bisect finds the error more easily in
  10 small commits than in 1 big one.
- **Code-review readability:** small commits are reviewable.
- **Clean bisect.** With a collective commit you are stuck bisecting
  200 lines.
- **Push discipline:** [04-multi-phase-autonomy.md](04-multi-phase-autonomy.md)
  pushes after every phase — that requires a separate commit per phase.
- Memory `feedback_commit_per_step`: explicit user instruction.

**How.**
1. After each finished sub-step: `tsc + tests + build` green.
2. **Commit + push immediately.** Not "I'll bundle it with the next one".
3. Start the next step.

**Example — damping plan 2026-05-28.**
```
Damping D1: constant + OR aggregation + helper                 → commit
Damping D2: Phase C damped + pull out clamp                    → commit
Damping D3: update tests                                       → commit
Damping D4: docs + build + deploy + commit + push              → commit
Count C1:   invertHeat:bool → invertCount:number               → commit
Count C2:   Phase C N-fold application                         → commit
Count C3:   tests + docs + deploy                              → commit
```

Every commit was green and reviewable on its own — not one
"big-damping-refactor" collective commit.

**Anti-patterns.**
- Working through 7 steps, then one huge `feat: damping +
  count + tests + docs` commit at the end.
- Sneaking a "quick fix on the side" into an unrelated commit.
- Multiple bugs in one bug-round commit that ends up being called
  "4 bugs".

**Related.**
- [11-multi-phase-commits.md](11-multi-phase-commits.md) (phase planning).
- [04-multi-phase-autonomy.md](04-multi-phase-autonomy.md)
- [31-conventional-commit.md](31-conventional-commit.md)
- Memory `feedback_commit_per_step`.
