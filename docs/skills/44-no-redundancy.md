# Skill: No redundancy / no duplicated code

**When.** While writing or editing code you notice: "I just typed this
block already" or "the same logic also lives in module Y". Also during
refactor or review passes where two or more places stand out as nearly
identical.

**Rule.** The same logic appears **once**. Whatever repeats across
multiple places moves into a shared helper / function / constant.

**Why.** Duplicates diverge over time:
- One place gets fixed, the other does not — and the bug silently
  stays in the system.
- The refactor effort multiplies per copy.
- The codebase bloats without the additional lines carrying
  value.

Concretely from this codebase: the heat-burst comparison
`(now - fireTime) < THRESHOLD` lived in several places
(heatPhysics, instanceStatRows, engine.calculateDynamicHeatProduction).
When the time base moved from Date.now() to gameTimeMs, only one
place was migrated — the others silently collapsed, and the bug was only
noticed by the user (msg 10991). A shared helper function
`isWithinBurstWindow(now, fireTime)` would have been more robust.

**How.**

- **While typing.** STOP the moment you notice "I just wrote this
  already". Even if it is only 3-5 lines: pull it into a helper
  BEFORE the second copy is committed.
- **During a refactor.** Actively look for duplicates — the same pattern in
  2+ files with only minor differences is a smell. Grep
  for characteristic constants or variable names.
- **Comparison windows / threshold checks.** `(now - fireTime) <
  THRESHOLD`, `Math.abs(a - b) < EPSILON`, `value >= minTemp &&
  value <= maxTemp` — patterns like these belong in ONE shared
  function with a clear name.
- **Constants.** Centralized in one constants module (see
  [17-centralize-constants.md](17-centralize-constants.md)),
  not redefined in every module.
- **Complex conditional chains.** If the same `if (a && b || c)`
  condition appears in 3 places: extract a predicate function
  `isReadyToFire(state)`.

**Exceptions.**

- **Three very similar lines with different semantics** are
  better than a premature abstraction. Example: three render calls
  with different colors are three lines — not one
  helper function with a color parameter, if the calls draw semantically
  different things.
- Test setups and fixtures: there, explicit duplication is often
  clearer than a "smart" helper that generates all fields
  dynamically.
- Boilerplate demanded by the framework (e.g. three React
  component wrappers, all similar) — as long as each has its own
  clear role.

**Smell test.** "If I change one place, do I then have to touch a
second one as well?" — If yes: extract. "Would a
bug fix at place A apply just the same at place B?" — If yes:
extract.

**Source.** User spec msg 11002 (2026-05-31) — explicit permanent
rule; escalated after a session in which duplicated time-comparison
logic broke after a refactor.

Provenance note: the earlier local memory `feedback_no_redundancy` was
migrated into the repo on 2026-08-07 (plan
`2026-08-07-agent-wissen-ins-repository.md`, in the Chimera project)
— this skill is the canonical place.
