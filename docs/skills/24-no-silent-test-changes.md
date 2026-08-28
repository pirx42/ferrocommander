# Skill: Explain + ask before changing existing tests

**When.** A code change makes an existing test fail.

**Rule.** Tests are **not simply "adjusted so they go green"**.
An existing test has a reason. If a change makes it fail:

1. **Think first:** is the change really supposed to break the
   documented behavior?
2. If yes: **explain** the change and — for genuine behavioral rules —
   align with the client.
3. Only then adjust the test along with it.

**Why.** Tests are **codified requirements**. Changing a test to repair
a build often erases exactly the invariant the test protects. Memory
`feedback_test_changes`: "tests only change when requirements change,
not to silence failures".

**How.**
1. Analyze the red test: what was the protected invariant?
2. Check the hypotheses:
   - **Requirement change:** adjust the test correctly + explain it in
     the commit body ("behavior X changed, test brought along because
     ...").
   - **Bug in your code:** your code is wrong, the test was right —
     fix the code, leave the test alone.
   - **Test-setup drift:** the test relies on incidental details that
     your change re-sets — stabilize the setup minimally, no changing
     of assertion values.
3. When uncertain or facing a requirement conflict: **ask the client**
   before the test is changed.

**Example.**
```
itemCombinations.slow.test.ts: cooler(bypassMod, inverterMod)
expected heat=cooler.heatCooling=300, now needs 0.9*300=270.

Justification: the invertCount model with damping 0.9 now applies per
item, not XOR-aggregated. Test correctly brought along — the new formula
is 0.9^N * value. Commit body links to the damping plan.
```

**Anti-patterns.**
- `expect(x).toBe(270)` instead of `300` — without a comment, because
  "that makes it green".
- Deleting the test entirely because "this path is different now". Sure
  that nobody is supposed to hit it anymore?
- Across several refactor steps: the test gets "adjusted" in every phase
  until it is silent.

**Related.**
- [23-tests-accompany-commits.md](23-tests-accompany-commits.md)
- [26-behavioral-tests.md](26-behavioral-tests.md)
- Memory `feedback_test_changes`.
