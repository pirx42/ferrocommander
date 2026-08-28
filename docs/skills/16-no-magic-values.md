# Skill: No magic values — numbers/strings as named constants

**When.** You are writing a number or a string with semantic meaning
into the code: threshold, timeout, tile size, URL path, error code,
feature-flag name.

**Rule.** As a **named constant** (uppercase + suffix), not inline.
Thresholds, timeouts, units — everything named.

**Why.** Named constants are documentation.
`const TASK_START_DELAY_MS = 5000` explains itself; a bare
`5000` in a timer call does not. Later someone searches "where is this
set?" — the name is the search aid.

**How.**
- Naming: `THING_PURPOSE_UNIT` (`HEATMAP_DIFFUSION_RATE`,
  `TASK_START_DELAY_MS`, `MAX_RETRY_ATTEMPTS`).
- Centralize per thematic subsystem in `<area>Constants.ts`
  (see [17-centralize-constants.md](17-centralize-constants.md)).
- Inline constants are OK when they are truly used in only one place AND
  don't repeat — but when in doubt: name them.

**Example.**
```ts
// YES
const SECTOR_TRANSITION_DEBOUNCE_MS = 1500;
setTimeout(transition, SECTOR_TRANSITION_DEBOUNCE_MS);

// NO
setTimeout(transition, 1500); // why 1500?

// YES
if (heat > ZONE_HEAT_DAMAGE_THRESHOLD) { ... }

// NO
if (heat > 0.8) { ... } // 0.8 of what?
```

**Anti-patterns.**
- Several places with the same value (`5000`, `5000`, `5000`) — drift
  guaranteed.
- Constants with cryptic names (`X = 0.8`) — almost as bad as inline.

**Related.**
- [17-centralize-constants.md](17-centralize-constants.md)
- [18-comments-explain-why.md](18-comments-explain-why.md)
