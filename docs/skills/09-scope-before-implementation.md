# Skill: Nail down scope before implementation

**When.** Before the first line of code of a new task.

**Rule.** Record in writing (plan file, TaskList, or a short
confirmation to the owner) **what belongs to the task**.
When scope expansion comes up during implementation ("we should
clean that up too while we're at it"): **no yes** before asking back.

**Why.** Scope creep produces large, hard-to-review diffs,
dilutes the fix with unrelated changes, and makes `git blame`
useless. A clearly delimited scope is the foundation for a
clean commit.

**How.**
1. Before the first line of code: formulate the scope in one sentence — what
   IS in, what is NOT.
2. Note side issues discovered while reading code separately (issue,
   TaskList, future-improvements.md); do not touch them along the way.
3. If a refactor forces itself on you because the fix cannot be done
   cleanly otherwise: ask briefly ("a clean fix needs a mini-refactor in X,
   okay or separate?").

**Example.**
```
Task: "Fix bug — skip button shows wrong number of penalties".

In scope: SkipButton component + penalty calculation in usePenalties.
Out of scope: the cluttered TaskHUD next to it (separate task).
```

**Anti-patterns.**
- "While I'm at it" refactor inside a bug-fix commit.
- Two independent bugs in the same commit, "because I happened to see
  both".

**Related.**
- [11-multi-phase-commits.md](11-multi-phase-commits.md)
- [32-commit-per-step.md](32-commit-per-step.md)
- [10-plan-lifecycle.md](10-plan-lifecycle.md)
