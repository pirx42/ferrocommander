# Skill: Push carries ALL unpushed commits along

**When.** Before every `git push` — especially before "harmless" test/docs
pushes.

**Rule.** `git push` ALWAYS pushes all local commits before HEAD — not
just the last one. Before pushing, check `git log @{u}..HEAD --oneline`: if
there are commits there without push approval, do NOT push (or get
approval). The approval rule applies to the entire commit stack, not to the
most recent commit alone.

**Approval rule since the branch workflow (2026-07-18, msg 15497):**
- **Topic branches:** push freely (no CI/deploy effect).
- **dev:** push ONLY after approval (acceptance merge or approved small
  stuff) — the earlier tests/docs-are-free exception is thereby superseded.
- **main:** exclusively a release merge from dev on explicit instruction —
  deploys immediately to the testers.

**Why.** A docs-fix push would otherwise carry unapproved production
commits along — the approval rule would be silently bypassed. (Memory
`feedback_push_carries_unpushed_commits`.)

**How.**
```bash
git log @{u}..HEAD --oneline   # empty = nothing unpushed; otherwise check the stack
```
- Only test/docs commits in the stack → push directly.
- Production commit included, approval missing → ask or wait.
- With approved multi-phase plans: phase pushes are covered
  ([04-multi-phase-autonomy.md](04-multi-phase-autonomy.md)).

**Anti-patterns.**
- "It's just the README fix" — and three unapproved feature commits ride
  along.

**Related.**
- [34-no-force-push-shared.md](34-no-force-push-shared.md)
- [32-commit-per-step.md](32-commit-per-step.md)
