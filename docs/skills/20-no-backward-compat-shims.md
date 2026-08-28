# Skill: No backward-compatibility shims when refactoring

**When.** You are replacing an internal API, a function, a type, a
file.

**Rule.** Remove the old thing **completely**:
- No `_unused` leftover.
- No `// removed XXX` comment.
- No re-export of the old API "just in case".
- No renamed `_oldVar` parameters.

**Why.** Dead code is worse than no code: it looks alive, sends readers
in wrong directions, and at the next audit pass someone asks "where is
this used?".

**How.**
- After the refactor: `grep` for the old identifier range, remove all
  hits.
- The TypeScript compiler helps: unused imports / exports are usually
  flagged.
- If the old thing is genuinely still needed → either don't refactor at
  all or make the migration path explicit (see exception).

**Exception — public APIs / persisted data.**
- Persistent data (save format, DB schema) needs migrations with a
  version bump.
- A public API with external consumers needs deprecation paths.
- **Internal code: no exception.**

**Example.**
```ts
// NO — re-export shim for internal modules
// @deprecated use newAdvance
export { newAdvance as oldAdvance };

// NO — _unused parameter, because "maybe later"
function tick(_state, dt) { return doStuff(dt); }

// NO — marker comment
// removed: applyOldHeat (replaced by applyHeat)
function applyHeat() { ... }

// YES — cleanly removed, new file, old callers switched to the new name
```

**Anti-patterns.**
- File `heatPhysics.ts` replaced by `heatPhysicsV2.ts`, the old one stays
  "just in case".
- Function renamed + old variant kept as a wrapper that only calls the
  new one.

**Related.**
- [13-prefer-existing-files.md](13-prefer-existing-files.md)
- [11-multi-phase-commits.md](11-multi-phase-commits.md)
