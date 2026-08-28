# Skill: Test conservation/physics invariants (instead of value asserts)

**When.** You are writing tests for solver/sim/aggregation code (energy,
heat, pressure, flows) — or you are wondering why a bug slipped through
despite a large test suite.

**Rule.** For code that must satisfy a **physical/structural invariant**,
test the **invariant** across many/random/combined inputs — NOT just concrete
output values of individual cases. Value asserts (`cableFlow === 15`) codify
exactly the wrong value as "correct" when there is a bug.

**Why.** Characterizing value tests are often written by observing the
current behavior and pinning it down. If the implementation leaks, the test
says "15 is expected" — and protects the bug instead of finding it. An
invariant (`delivered ≤ produced`) is true independent of the concrete value
and fails on ANY topology that violates it.

**War story (2026-06-24).** A generator with an internal consumer
(`generator(sensor)`) exported its GROSS production to the cable AND fed the
sensor internally → the system delivered 20 out of 15 production (5 E/s out
of thin air). ~5200 existing tests did NOT catch it: they asserted concrete
values (`demand=20`, `cableFlow=15`), and the leak looked functionally
healthy (shield charged, sensor ran) — even the boss invariants ("shields
charge") stayed green, BECAUSE the bug produced "working" behavior. It only
became visible when a new display SUMMED production vs. consumption. The fix
test is an invariant: "every source exports ≤ effProduction − effDemand",
across 38 topologies (producers × sinks × charge states + chains).
Cross-checked: red on the buggy code, green with the fix.

**War story II — the same invariant, reversed sign (2026-07-12).**
The mirror case: not "energy out of thin air", but legitimate flow that is
SILENTLY blocked. A siphon-hit non-source node (ventilator) became a
temporary source, but `applyStorageBottlenecks` hard-set the export capacity
of EVERY non-source node to `0` → its surplus was treated as "out of thin
air" and the cable outflow was zeroed; a cabled capacitor NEVER charged (a
shield did — it pulls via its own demand). ~6800 tests did not catch it: the
new contribution was correct in the reader fold AND in `buildHydraulicNode`
(injection), but the third stage (`applyStorageBottlenecks`) did not know
about it — no test verified end-to-end that a producer's surplus actually
REACHES a cabled consumer/storage. Lessons: (1) conservation holds in both
directions — `delivered ≤ produced` AND `connected surplus arrives`; (2) a
new energy contribution must be threaded through ALL solver stages
(reader → node build → bottleneck → charge update) and secured with a
system-level test (cabled storage charges from the producer), not just per
function. Cross-checked: fix reverted → capacitor charge 0.

**How.**
- **State the conservation explicitly:** Σ output ≤ Σ input (+ storage
  discharge). Per node: outflow ≤ net capacity. Both directions: connected
  surplus must actually REACH the consumer/storage (not silently drain away).
- **Thread through all pipeline stages:** a new contribution
  (source/sink/delta) must arrive in EVERY stage (reader → node build →
  bottleneck → charge), not just the first — a system-level test covers the
  gap that per-function tests leave open.
- **Property-/table-driven:** generate many combinations (components ×
  wiring × initial states) and assert the invariant in EVERY one — not one
  hand-picked value.
- **Check across multiple ticks** (transients + steady state).
- **Measure at the right point:** not at a display aggregate metric that
  mixes pool draw + self-discharge (e.g. `totalActualConsumption` counts the
  whole internalDrain when a ChargeSink is full). Use the real flow/charge
  quantity (cable flow, charge delta).
- **Verify the guard:** briefly revert the fix → the test MUST turn red.
  An invariant test that does not catch the known bug is worthless.

**Related principle — also applies outside of tests.** Verify the
INVARIANT, not the surface pattern. Example content migration (46 JSONs,
`productionMultiplier` in torso zones): the first regex pass on
`"zone":"torso"` also hit item `location` objects + events. Caught via a
STRUCTURAL check (parse the JSON, assert that only objects with `maxHp` =
real zone defs were changed) → revert → retry with a `maxHp` anchor + count
verification. Not "should be fine", but structurally prove that only the
intended set was touched.

**Anti-patterns.**
- `expect(result).toBe(<observed value>)` for solver output without checking
  whether the value is physically correct.
- Testing the invariant only on the known bug case instead of across the
  combinatorics.
- Content migration via blind search-and-replace without a structural
  cross-check.

**Related.**
- [26-behavioral-tests.md](26-behavioral-tests.md) — behavior over structure (invariants are the strongest form of it).
- `48-design-invarianten-respektieren.md` (Chimera only) — respect design invariants.
- [23-tests-accompany-commits.md](23-tests-accompany-commits.md) — a bug fix brings a regression test.
