# Skill: Mutation probe instead of coverage percent

**When.**
- A feature/subsystem is considered "well tested" because coverage is green.
- A bug shows up in code that was covered according to the report
  ("how can that be, there was coverage on it?").
- During the housekeeping pass (42 (`42-housekeeping-coverage-doku-drift.md`, Chimera only)),
  as the fourth round.
- Before any statement of the form "this is guarded".

**Rule.** **Coverage measures EXECUTION, not ASSERTION.** A line counts as
"covered" as soon as any test runs through it — completely independent of
whether anything is ever asserted about its result afterwards. A feature can
thus have **100 % coverage and be 0 % tested**.

The only reliable test of a safeguard is the **mutation probe**:
switch off the effect and see whether the suite screams.

```
Break it → full suite → stays green?  ⇒  NO guard. Write a test.
                        something fails?  ⇒  A guard exists.
```

**Why.** Chimera, 2026-07-14: the per-tick XP award (`energyStep`) was
COMPLETELY untested. With the award hard-disabled (`if (false)`, i.e.: no
item ever gets XP again), **8482 tests ran green**. Not a single guard —
after months of coverage checks and coverage improvements.

The reason is structural, not an oversight: the XP lines run in EVERY test
that ticks the sim (`energyStep` hangs off `stepChassisFrame`). So they are
perfectly "covered" — and by definition NEVER show up in a gap triage
([27](27-coverage-gap-triage.md)), because that only lists lines that are
NOT covered. The housekeeping pass was **blind by design** against exactly
this class. Coverage cannot find this error; only the mutation can.

**How (steps).**

1. **List observable outputs**, not lines. Per subsystem: which state
   writes are the product of this code? (Chimera sim core: `xp`, `charge`,
   `heat`, `itemHP`, shield charge, fire timestamps, siphon net,
   destruction heat bomb, …)
2. **One mutation per output** that deletes exactly this effect — not
   "change some random line":
   - Neutralize the assignment (`x = alt + delta` → `x = alt`)
   - Cut off the function body (`{` → `{ if (true) return;`)
   - No-op the setter (`map.set(id, v)` → `void v;`)
3. **Run the full suite** per mutation (not just the supposedly responsible
   file — the guard may live anywhere).
4. **Tally:** every mutation that runs green is a hole. Add a test
   ([26](26-behavioral-tests.md) /
   [52](52-test-conservation-invariants.md)).
5. **Tree clean again:** ALWAYS roll back mutations (backup before the
   patch, `finally` restore, then check `git diff`). A forgotten
   `if (false)` in the sim core is a disaster.

**Ready-made tool: `npm run mutation-sweep`** (`scripts/mutation-sweep.sh`) —
covers the central state writes of the sim core. Exit 1 if a mutation
survives. New mutation = one `run_mutation` line.

**The most important rule: GREEN BASELINE FIRST.** Before the first
mutation is applied, the suite must be green **unmutated**. Otherwise every
mutation yields exit != 0 and is booked as "GUARDED" — the sweep then
happily reports "everything guarded" and has in truth **measured nothing**.

This is not a theoretical risk. On 2026-07-14 the script called `npx vitest
run` **without the project excludes** from `package.json`
(`--exclude='tests/**'`). That pulled the Playwright e2e specs into the
run, which fundamentally fail under vitest ("Playwright Test did not expect
test() to be called here") — the suite was **always** red. Result: 9/9 and
19/19 "GUARDED", all worthless, including a conclusion already reported to
the user.

Two hard consequences from this:

1. **Baseline gate in the script** (it's in): unmutated run, red → abort.
2. **The sweep must run EXACTLY the suite the project considers green** —
   i.e. the same flags as the `test` script in `package.json`. Measuring a
   different suite than the one you commit against is inherently
   meaningless.

And a lesson about controls: the sweep had two "control mutations" at
places considered guarded. They did **not** catch the error — they cannot
distinguish "guarded" from "always red", both look like exit != 0. The only
control that works is the **idle run** (no mutation at all → must be
green). *If a test CANNOT fail, it is not a control.*

**Operational traps (solved in the script, do not optimize away):**

| Trap | Solution |
|---|---|
| **Suite already red without a mutation → everything reports GUARDED, the sweep measures nothing** | **Baseline gate**: unmutated run before the first mutation, red → exit 2. And the same flags as `npm test`. |
| Full suite per mutation = expensive | `--bail=1` — we only want to know WHETHER a guard exists. A guarded mutation dies at the FIRST failure (~2 min) instead of sitting out the suite. |
| Some mutations make sim tests run into their 120 s timeout (they wait for a state that never occurs) — one run drags on for hours | short `--testTimeout=8000` |
| Python's `subprocess` timeout kills `npx`, but the vitest workers keep holding the stdout pipe → the sweep hangs instead of continuing | `timeout(1)` at shell level: kills the whole **process group** |
| `pkill -f <pattern>` matches its own shell (exit 144) | Bracket trick `"[p]attern"` — or don't kill at all |
| Aborting mid-sweep leaves the mutation in the code | Restore in `trap EXIT` **and** `git diff` check at the end (exit 2 if dirty) |
| **Running the sweep in the background and working alongside it** | **Don't.** The sweep mutates real files in the working tree. While it runs, the tree is at times deliberately broken: `git status` shows foreign changes, a `git add -A` commits a MUTATION, and any parallel `tsc`/`vitest`/`lint` measures a manipulated state. On 2026-07-15 `src/heatManager.ts` sat exactly like that — the commit only failed to happen because `git status` was read before adding. Either wait, or run the sweep in a separate worktree. |

**Result of the sweep (2026-07-14, 28 mutations, corrected harness):**
**11 GUARDED, 17 SILENT.** And the distribution is the real insight:

| Layer | Result |
|---|---|
| **Sim core (physics):** energy, heat, charge, damage | **9/9 GUARDED** |
| **Pure reducers** (`src/run/`, `src/profile/` — both controls) | **GUARDED** |
| **React hook seam** (`useRunFinalize`, `useSectorTasks`, `useRunChallengeTick`, `useRunProgression`, `useCodexEncounters`, `useTimedEvents`, `useDialog`) | **17/17 SILENT** |

The physics is tight because there every missing term violates a
**conservation invariant** ([52](52-test-conservation-invariants.md)) — a
test fails on its own. The holes sit elsewhere, and at a very sharp edge:

> **The pure modules are tested. That their result ever lands in the state is tested by no one.**

Chassis unlocking is the textbook example: the trigger table has a test,
the profile mutation has a test — but that at run end anyone hooks the one
up to the other is asserted by no single test. You can delete the line and
the suite stays green. The same holds for combos, run unlocks, statistics,
run history, codex capture and the run-end `completed: true`.

**Two diseases the sweep lumps together (2026-07-15).** After the coverage
pre-check of the hook seam plan:

| File | Coverage | Mutation | Diagnosis |
|---|---|---|---|
| `hooks/useRunFinalize.ts` | **0 %** | silent | not executed at all — **coverage would have sufficed** |
| `run/runLifecycle.ts` | **90 %** | silent | executed, but nothing asserted — **only the mutation finds this** |

So: "silent" does not automatically mean "coverage failed". Only looking at
the coverage number tells you WHICH of the two diseases is present — and
thus whether a harness is missing (0 %) or an assertion (90 %).

**The measuring device measures only what you put into it.** While securing
`useRunFinalize` it turned out that `unlockComboForChallenge` (challenge →
loot combo) appeared in NO mutation — the sweep never reported the line, so
it silently counted as fine. It was unguarded. A sweep proves something
about the lines it touches and **nothing** about the others. When creating
a mutation group, therefore enumerate the effect sites of a file, don't
just take the most prominent one. (Extra trap: the two combo lines differ
only in indentation — one is a SUBSTRING of the other, `sed`/`replace` hits
the wrong one. Two-line anchor.)

**"But there is a test on the field" says nothing about the PATH.**
`useDialog.closedAtMs` had a test — for the seen-once path, where the entry
is created as closed already at enqueue time. The dismiss path
(`markCurrentClosed`), which the unread dot hangs off, ran in none. The
mutation there survived even though the field looked "tested".

**Remember — two probes, not one:**
1. **Bookkeeping** instead of physics (XP, levels, unlocks, statistics,
   currency): its failure violates no invariant, so the suite stays silent.
2. **Seam** instead of core: not just "does the rule work?", but "is the
   rule even hooked up?". A green-tested reducer that nobody calls is dead
   capital — and coverage shows it as covered.

**Relation to coverage.** Coverage remains useful — but ONLY as a negative
signal: "nothing runs through here at all" is a hard statement. The
converse is invalid. So:

| Signal | Meaning |
|---|---|
| Coverage 0 % | Real gap. Reliable. |
| Coverage 100 % | **Says nothing** about safeguarding. |
| Mutation survives | Real hole. Reliable. |
| Mutation dies | A guard exists. Reliable. |

**Anti-patterns.**
- "The file has 90 % coverage, the feature is guarded" — non sequitur.
- Running only the file's own test suite (`vitest run <file>`) — the guard
  could live elsewhere; a green partial run proves nothing.
- Not rolling back the mutation.
- Choosing the mutation so that it produces a type error/crash: then the
  suite fails for the wrong reason and you have learned nothing. The
  mutation must **compile and run** — only the EFFECT is missing.

**Related.**
- [52-test-conservation-invariants.md](52-test-conservation-invariants.md) —
  "verify the guard via fix revert" is the same idea, there for the
  INDIVIDUAL new fix; this skill runs it as a sweep over the existing code.
- [27-coverage-gap-triage.md](27-coverage-gap-triage.md) — the gap triage;
  structurally blind to covered-but-unasserted.
- `42-housekeeping-coverage-doku-drift.md` (Chimera only)
  — the mutation pass is the fourth round there.
- [26-behavioral-tests.md](26-behavioral-tests.md) — what fills the holes
  found.
