# Skill: Verify memory + audit reports before acting

**When.**
- You want to take an action based on a memory entry
  ("function X lives in Y, fix it there").
- A sub-agent / external audit has delivered a finding
  ("file Z has no caching", "function W has no early break").

**Rule.** **Before implementing**, check the finding / memory entry against
the **current code**:
- a targeted `grep` for the named identifier,
- a 30-line Read of the named location,
- `ls` on the named path.

**If the finding is wrong**: either inform the requester or re-identify
the actual optimization potential — do **not blindly "deliver anyway"**.

**Why.**
- Memories can go stale. "It used to be like that" is not enough.
- Analysis reports are helpful but not authoritative. Implementing a wrong
  finding costs time and can make working code worse under the guise of
  improving it.
- Example from Chimera 2026-04-27: a performance analysis claimed
  "pressureSolver has no early break" — a look into the file showed
  that `if (maxDelta < SOLVER_EPSILON) break;` was **already** there.
  The real lever was epsilon calibration (1e-6 → 1e-4); that was openly
  documented in the commit body, instead of fixing a bug that did not
  exist.

**How.**
1. Memory / audit cites a path / function / variable.
2. `grep -n` or `Read` (max 30–50 lines) on the location.
3. What is **actually** there?
   - Matches the memory → act.
   - Does not match → update the memory OR decline, and tell the requester
     what is really the case.
4. On systematic drift: delete / correct the memory.

**Example.**
```
Memory: "useHeatSimulation lives in src/run/useHeatSimulation.ts"
→ ls src/run/useHeatSimulation.ts
   → File exists. OK, act.

Memory: "Function X takes argument Y with default Z"
→ grep -n "function X" src/...
   → Default is W, not Z. Memory is stale.
   → Correct or delete the memory before the next action.
```

**Anti-patterns.**
- Acting blindly on "the sub-agent says X" without even opening the
  location of X.
- Accepting a 6-month-old memory as "that's surely still right" — the
  affected file has probably been restructured.
- Implementing a wrong finding because "the report looked convincing".

**Related.**
- [09-scope-before-implementation.md](09-scope-before-implementation.md) (verification as part
  of scope clarification).
- Since 2026-08-07, project knowledge lives directly in the repo (skills / GDP /
  `docs/backlog.md`; plan `2026-08-07-agent-wissen-ins-repository.md` — both
  in the Chimera project, where this skill came from).
  The local agent memory only holds machine-local facts — this verification
  rule applies unchanged to those entries as well.
