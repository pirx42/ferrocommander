# Skill: Tests accompany features and fixes — per commit

**When.** You are writing a feature commit or a bug-fix commit.

**Rule.**
- **Feature commit:** brings tests for the new logic with it.
- **Bug-fix commit:** brings a regression test with it that fails
  **without the fix**.

**Why.**
- Without tests you only know "works right now", not "stays correct".
- Bug regression tests **document** what the reported problem was —
  the test name + the assertion replace a long comment.
- Tests belong to the same logical step — committing them separately
  fragments the history.

**How.**
- Bug-fix order: write the regression test first → it fails red →
  fix → test green → commit.
- Feature: at least one test per new path (happy path + 1 edge case).
- Test in the same commit as the code. Not "I'll commit the test right
  after".

**Example — bug-fix pattern.**
```ts
// FIRST: regression test that FAILS without the fix
it("section transition with sticky-fulfilled-condition does not reset heldMs", () => {
  const evaluator = makeEvaluator(...);
  const r1 = evaluator.tick(state1);
  expect(r1.status).toBe("fulfilled");
  const r2 = evaluator.tick(state2WithBriefMismatch);
  expect(r2.status).toBe("fulfilled"); // bug: was "ongoing"
});

// THEN: fix + commit both together
```

**Example — feature pattern.**
```ts
// New effect: zoneCompression. In the same commit:
it("zoneCompression reduces zone bbox by config factor", () => { ... });
it("zoneCompression with factor=1 is no-op", () => { ... });
it("zoneCompression aggregated with other ambient offsets", () => { ... });
```

**Anti-patterns.**
- "commit-now-test-later" — tests often stay undone, doc debt
  accumulates.
- Writing a test so generic that it would pass even without the bug fix
  — no real protection.

**Related.**
- [24-no-silent-test-changes.md](24-no-silent-test-changes.md)
- [25-green-suite-before-commit.md](25-green-suite-before-commit.md)
- [27-coverage-gap-triage.md](27-coverage-gap-triage.md)
