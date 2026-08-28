# Skill: Centralize constants as soon as they are thematically related

**When.** Three or more constants belong to the same subsystem
(heat sim, energy pool, cable physics, tutorial-run timing) and are
scattered across multiple files.

**Rule.** Collect them in a dedicated `<area>Constants.ts` file.

**Exception.** Feature-specific UI / timing constants
(`BEAM_FLASH_COOLDOWN_MS`, `FIELD_LOCK_SNAP_DURATION_MS`) stay in
their feature modules — they are not sim parameters and belong to the
respective logic.

**Why.** Scattered constants drift: two places hold the same concept
with slightly different values, or the docs cite a value that no longer
exists anywhere in that form. A central collection point makes
game-mechanic tuning local and simplifies doc references.

**How.**
1. `grep` for the value / concept across multiple files.
2. If 3+ places hold a related constant: new file
   `src/<area>Constants.ts` (e.g. `heatConstants.ts`,
   `energyPoolConstants.ts`).
3. Per constant: a short comment with "why this value" (game balance,
   physics constant, measured, ...).
4. Switch callers over to the import, remove inline values.
5. Docs (e.g. `docs/heat-system.md`) reference the file instead of
   individual values.

**Example — Chimera live.**
`src/heatConstants.ts` contains:
- `HEATMAP_DIFFUSION_RATE`
- `BORDER_COOLING_EXTRA`
- `ITEM_HEAT_DURATION_MS`
- `CABLE_HEAT_FULL_FLOW`
- `ZONE_HEAT_DAMAGE_THRESHOLD`

Before: spread across `heatPhysics.ts`, `useHeatSimulation.ts`,
`thermoInjectionTick.ts` and `extremeTemperatureTick.ts`.
`docs/heat-system.md` references them as game-mechanic parameters — only
meaningful with a single location.

**Anti-patterns.**
- Constant re-declared in every caller to "save the import".
- Throwing all game constants into one 2000-line `constants.ts` with no
  subsystem separation.

**Related.**
- [16-no-magic-values.md](16-no-magic-values.md)
- [13-prefer-existing-files.md](13-prefer-existing-files.md)
