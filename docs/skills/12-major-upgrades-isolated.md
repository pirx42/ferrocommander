# Skill: Keep major dependency upgrades isolated

**When.** A dependency jumps a **major version** (TS 5→6, React 18→19,
Vite 4→5, ...).

**Rule.** Every major bump is its **own task** with its own regression
window. Do **not** push it through as part of another task or in an
`npm update` sweep.

**Why.** Major bumps bring breaking changes that cost debugging time.
Mixed in with another task, the bug becomes untraceable — you don't know
whether it was your change or the bump.

**How.**
1. Set up the major bump as its own plan/commit.
2. Beforehand: `tsc --noEmit && npm test && npm run build` — baseline green.
3. Do the bump in its own iteration, then get the full set green again
   (often with breaking-change fixes as follow-up commits in the same
   iteration).
4. Only then continue with normal feature/fix work.

**Example.**
```
Day 1: feature X (no bump).
Day 2 morning: TypeScript 5.4 → 5.6 — own plan/commit, possibly
  fix follow-up commits for new strict rules.
Day 2 afternoon: feature Y.
```

**Anti-patterns.**
- "fix bug + bump typescript" in the same commit.
- `npm update` with everything in it, then "weird, something is crashing".

**Related.**
- [11-multi-phase-commits.md](11-multi-phase-commits.md)
- [09-scope-before-implementation.md](09-scope-before-implementation.md)
