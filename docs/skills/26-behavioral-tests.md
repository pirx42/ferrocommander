# Skill: Behavioral Tests Over Structural Tests

**When.** You are designing a new test.

**Rule.** Tests check **observability** — what the code does, not how it
is structured internally. No test that has to assert a file path or an
internal class hierarchy in order to survive.

**Why.** Structural tests break on every harmless-looking refactor.
Behavioral tests survive refactors and protect real invariants.
A test that checks "file X exists at path Y" says nothing about
correctness — it only says the file is in place.

**How.**
- **The test asks:** "If I provide this input and trigger this action,
  do I get this output / this visible behavior?"
- **NOT:** "Does this caller internally access this method with this
  signature?"
- Prefer checking API calls at public interfaces
  (reducer output, render output, observed side effects) rather than at
  internal helpers.
- Use mocks sparingly — if the test can be well formulated without mocks,
  it's usually a better test too.

**Example.**
```ts
// YES — behavior
it("advanceToNextSection sets sectionIndex +1 and sectionStartMs", () => {
  const state = { sectionIndex: 0, sectionStartMs: 0 };
  const next = advanceToNextSection(state, runDef, 5000);
  expect(next.sectionIndex).toBe(1);
  expect(next.sectionStartMs).toBe(5000);
});

// NO — structure
it("advanceToNextSection calls helper nextIndex()", () => {
  const spy = jest.spyOn(internals, "nextIndex");
  advanceToNextSection(...);
  expect(spy).toHaveBeenCalled();
});

// NO — file layout
it("section reducer lives in src/run/sectionReducer.ts", () => {
  expect(fs.existsSync("src/run/sectionReducer.ts")).toBe(true);
});
```

**Special case: authored content (boss/run JSONs).** Behavior here means
not just "function with constructed input", but **real content loaded +
ticked + checked against invariants**: load every boss, tick for 90 s,
verify (no self-cook, emitter fires + survives, shields charge) — plus a
parse-completeness guard (every event parses, nothing silently rejected).
Synthetic unit tests let real content bugs through (gameTimeMs heat,
stripped xp, energyEater `zones`→`zone`); these suites caught them.
Details: [good-development-practices.md](../good-development-practices.md) §4.7
(`bossContentSimInvariants` / `bossEventsParse`).

**Anti-patterns.**
- Tests that go red on every `git mv`.
- A spy/mock cube that has more code than the function itself.
- Snapshot tests for internal data structures that never reach the UI.

**Related.**
- `14-pure-reducer.md` (Chimera only)
- [23-tests-accompany-commits.md](23-tests-accompany-commits.md)
- [27-coverage-gap-triage.md](27-coverage-gap-triage.md)
