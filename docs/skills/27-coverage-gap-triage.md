# Skill: Coverage Gap Triage — Reachable / Defensive / Not-Yet-Activated

**When.** The coverage report shows uncovered branches. You are thinking
about how to close them.

**Rule — three cases.**

**(a) Reachable via API call** → **write a test**.

**(b) Defensive guard against cases that structural invariants exclude**
(clamped index ranges, closed discriminated unions, etc.) →
**either remove it** or **document it explicitly in the commit body**
("defensive guards, unreachable via current API calls").

**(c) Hook code for a not-yet-built feature**
(e.g. diode modifier with `zeroDemand: true` — no item sets the flag,
but resolveTree paths exist) → via `vi.mock(... importOriginal())` insert
a **synthetic item def** that activates the paths; this turns the hook
code into testable spec documentation without deleting it.

**Why.** 100 % branch coverage for dead code produces either nested mock
setups (which distort the test picture) or pushes toward deleting
protective code (which is then missing after an unobserved refactor).
Clear separation:
- reachable path → test,
- dead/defensive path → decision in the commit (delete or
  document), no test theater.

**How.**
1. For each uncovered branch: is it reachable via the public API?
2. If yes → test (case a).
3. If no, but excluded by a clear invariant → delete it,
   or keep it with a note in the commit body (case b).
4. If the path represents a planned hook for a future feature
   → insert a synthetic trigger via `vi.mock(... importOriginal())`
   (case c). That is spec documentation, not test theater.

**Example — Chimera live.**
`heatPhysics` coverage 91.78 % → 97.26 % via targeted tests.
The last 4 uncovered branches:
- `paintZone` grid OOB
- heater event-kind filter
- cable `totalHeat<=0`

All are defensive guards that duplicate structural API invariants
(`worldRectToCellRange` already clamps, etc.) — named explicitly in the
commit body, not "covered" via synthetic mocks.

**Anti-patterns.**
- Adding a synthetic mock just to reach a defensive guard branch — the
  branch is structurally unreachable; the mock lies.
- Deleting a defensive guard because "coverage 100 %", and after
  refactor X it blows up.
- Accepting 90% coverage as "good enough" without knowing which 10 %
  are missing.

**Related.**
- [19-no-defensive-programming.md](19-no-defensive-programming.md)
- [23-tests-accompany-commits.md](23-tests-accompany-commits.md)
- [26-behavioral-tests.md](26-behavioral-tests.md)
