# Skill: Clarify ambiguous tasks with named options

**When.** Before the first line of code of a task that has unspoken assumptions
or several plausible implementations (skip penalty: 1 / all / none?
Confirm dialog yes/no? Which sectors affected?).

**Rule.** Ask the question **with named options (A/B/C)** and short
tradeoffs — not as an open-ended question. Wait for the answer, do not start
speculatively.

**Why.** Every second of clarification before the work saves hours of rework.
Options are easier to answer than "how exactly should this be?".

**How.**
- Identify 2–4 plausible variants.
- Per variant: one sentence "what happens" + one sentence "tradeoff".
- Optionally mark one as the recommendation ("(Recommended)").
- Stop after the question. No proposed implementation while you
  are waiting.

**Example.**
```
Skip button — three options:
  A) Skip without penalty (free). Tempts click-through.
  B) Skip with 1 random penalty. Consistent with the fail model.   (Recommended)
  C) Skip with all penalties. Harsh, could be frustrating.
Please choose.
```

**Anti-patterns.**
- "Should I add Skip? How exactly?" — open-ended question without options.
- Implementing option B right away and then asking "is this okay?".

**Related.**
- [02-answer-meta-questions-directly.md](02-answer-meta-questions-directly.md)
- [09-scope-before-implementation.md](09-scope-before-implementation.md)
