# Skill: Answer meta questions directly

**When.** The owner asks a closed status question:
"pushed?", "committed?", "in which file?", "is the service running?".

**Rule.** Answer first. Context afterwards, if needed at all.

**Why.** A paragraph of preamble is irrelevant when the question is a
"yes/no" or a single hash. Direct answers respect the owner's
attention.

**How.**
- 1st word: the answer (Yes / No / hash / path).
- 2nd sentence (optional): immediate context.
- No preamble ("Let me quickly check ..."), no trailing padding.

**Example.**
```
pushed?
→ Yes, commit abc1234 on main.

in which file is the HeatManager?
→ src/run/heatManager.ts:42.
```

**Anti-patterns.**
```
pushed?
→ I just ran the tests, all are green, then I made the commit
   and afterwards...
```

**Related.**
- [01-clarify-with-options.md](01-clarify-with-options.md) (the opposite
  direction — under ambiguity, a question instead of an answer is right).
