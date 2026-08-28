# Skill: Push multi-phase plans through autonomously

**When.** The plan has been **approved** by the owner and contains multiple
phases / steps with no open decision points.

**Rule.** Work through all phases in one go. Interrupt **only** at
genuine decisions (tradeoff, schema break, new requirement).
After each phase: commit to the **topic branch of the plan** (+ topic push
as backup — freely allowed), give a short heads-up, keep going. Merging into
dev happens only at acceptance (skill
`64-branch-workflow-dev-topic-main.md` (Chimera only)).

**Why.** Checking back after every sub-step produces
waiting time and makes it hard to hold the plan in your head as a whole. If
the plan was agreed beforehand, working through it is the default.

**How.**
1. Keep the plan file (or TaskList) at hand.
2. Implement phase N → tests/docs/build green → commit to the topic branch (+ topic push) → one-sentence update.
3. Continue directly with phase N+1, without a new approval loop.
4. Only stop at: a choice question, a spec gap, hard conflicts with
   other parts of the plan.

**Example.**
```
Plan approved: phases 1-5, each with its own test suite + commit.
→ Phase 1 done, push abc1234. Continuing with 2.
→ Phase 2 done, push def5678. Continuing with 3.
... up to 5.
→ Phase 5 done, push ghi9012. Plan complete.
```

**Anti-patterns.**
- "Phase 1 done — should I start phase 2?" when the plan
  contained phases 1+2+3+4+5 and there are no new findings.
- Making one bulk commit at the end instead of one per phase.

**Related.**
- [32-commit-per-step.md](32-commit-per-step.md)
- [10-plan-lifecycle.md](10-plan-lifecycle.md)
- [05-updates-during-long-tasks.md](05-updates-during-long-tasks.md)
