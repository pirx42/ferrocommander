# Skill: Document the "Why" Behind Design Decisions

**When.** You are making an architecture decision that cannot be read
from the code: why pure reducers instead of OOP? why SQLite instead of
Postgres? why this state machine and not another? why this refactor
right now?

**Rule.** If the decision is **not readable from the code**, the
rationale belongs in the docs — at least a paragraph in the appropriate
`docs/<subsystem>.md` or in the plan document.

**What does NOT belong in the docs** (see anti-patterns below):
- API signatures, file paths — those live in the code.
- Commit history — `git log` is authoritative.
- Bug-fix recipes — the fix is in the code, the commit body has the
  context.

**Why.** Six months later, nobody remembers why X is the way it is.
Without docs, the temptation arises to optimize X away and re-live the
same old pitfalls.

**How.**
- With every larger refactor / architecture decision: a paragraph
  "Why this approach?" with a few alternatives and
  trade-offs.
- In the appropriate `docs/<subsystem>.md` or plan doc.
- Keep it brief — 3–5 sentences are often enough.

**Example.**
```markdown
## Pure reducers instead of OOP game state

Game state is a flat immutable tree, transformed via pure reducers
(`advanceToNextSection(state, ...) → state`).

Alternatives rejected:
- **OOP with mutation:** makes bot sim and RL rollouts harder
  (every rollout has to copy).
- **Redux Toolkit:** additional dependency without meaningful
  added value over `useState(s => reducer(s, ...))`.

Trade-off: many small object spreads. Not measurable in practice
(see perf pass 2026-04-27).
```

**Anti-patterns.**
- A doc paragraph that merely repeats the signature of
  `advanceToNextSection` — that's already in the code.
- "We use pure reducers." without rationale — the next refactor will
  throw them away without hesitation.

**Related.**
- [18-comments-explain-why.md](18-comments-explain-why.md) (smaller
  scale — inline comments).
- [28-docs-in-same-commit.md](28-docs-in-same-commit.md)
- [10-plan-lifecycle.md](10-plan-lifecycle.md)
