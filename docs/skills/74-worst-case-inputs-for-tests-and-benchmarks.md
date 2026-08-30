# Skill: A test that picks its own inputs picks flattering ones

**When.**
- Writing a test whose fixture the test itself constructs (sizes, names,
  positions, timings).
- Writing a benchmark that generates its own data.
- A probe (skill [59](59-mutation-probe-over-coverage-percent.md)) stays
  green against a mutation that should obviously have killed it.
- A test that used to pin a behaviour still passes after the behaviour's
  *trigger* moved elsewhere.

**Rule.** A test proves something about the inputs it runs on, and inputs
the test author invents are drawn from the friendly end of the distribution
unless something forces them not to be. Two failure modes, both silent:

1. **The inputs are too kind for the property to be contended.** The
   property under test never comes under load, so the assertion passes with
   the safeguard present *and absent*.
2. **The subject moved out from under the test.** The code path the test
   was written to exercise stopped being reachable the way the test reaches
   it — and the test goes on passing, now about nothing.

Both are found the same way: **probe the test, not only the code** — break
the property and watch the test; and when a trigger is rebound, re-ask which
tests reached the behaviour *through* that trigger.

**Why (both happened here, 2026-08-30, twice within one plan).**

- *Too-kind inputs, tests:* `Space` counts the folder it marks, and a second
  press restarts the scan over the folders not yet answered. The end-to-end
  test marked two folders of 4 KB and 1 byte — and the first scan finished
  long before the second keystroke landed, so the accumulation rule was
  never contended. Mutating the restart to cover *only the newest* folder —
  the exact bug the design exists to avoid — left the test green. The rule
  had to move into a pure function (`owed_from` → `forget_counted`) with a
  unit test the mutation does kill.
- *Too-kind inputs, benchmarks:* the cost of asking a 50 000-row listing
  about 100 marked folders measured as **0.02 ms** — because the generated
  folder names sat at the *front* of the listing, where a forward scan finds
  them immediately. Placing them at the end: **18.7 ms**, on the main loop,
  per keystroke. A hundredfold, hidden by input choice alone.
- *Subject moved:* three end-to-end tests pinned "after a widget-driven page,
  the model adopts the selection before acting". The day the page keys became
  bound actions, the widget stopped moving the selection on a page — and all
  three tests kept passing while no longer touching the mechanism they were
  written for. The removal of the adoption call they guarded left them green;
  only a *new* test (click, then act) could hold it.

**How.**
1. When constructing fixture data, ask: *where in this data does the
   property come under the most load?* Names at the end of a scanned list,
   sizes big enough to outlive a keystroke, collisions rather than clean
   paths. Put the data there, or say in a comment why it does not matter.
2. For a benchmark: place the generated inputs at the adversarial end
   **deliberately and say so in a comment** — a benchmark's inputs are part
   of its claim.
3. After writing the test, run the mutation from skill
   [59](59-mutation-probe-over-coverage-percent.md). If the probe stays
   green, do not first suspect the probe — suspect the inputs, then the
   route.
4. When a key, trigger, or route is rebound: grep for the tests that reach
   the old behaviour through it and re-run their probes. A green test after
   a rebinding is a *candidate* for "no longer about anything".
5. A property no affordable input can contend (a race the fixture always
   wins, a stall no test can see) moves into a pure function and gets a unit
   test — and the E2E keeps only the user-visible outcome, with the split
   named in the plan or commit.

**Anti-patterns.**
- Sequential/alphabetical generated names probed by a scan that starts at
  the front.
- Fixture files sized for test speed and then used to test a race.
- "The old test still passes" as evidence after the trigger it pressed was
  rebound.
- A benchmark whose input layout is unstated — the number it prints is then
  a property of the layout, not of the code.

**Related.**
- [59-mutation-probe-over-coverage-percent.md](59-mutation-probe-over-coverage-percent.md)
  — the probe that exposes both failure modes; this skill is about why the
  probe sometimes has to be aimed at the *test*.
- [52-test-conservation-invariants.md](52-test-conservation-invariants.md)
  — invariants resist kind inputs better than value asserts, but do not fix
  an uncontended race.
- [26-behavioral-tests.md](26-behavioral-tests.md) — what to write once the
  hole is found.
