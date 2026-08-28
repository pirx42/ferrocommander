# Skill: Edit existing files, don't create new ones lightly

**When.** You want to add a function / constant / component and are
wondering: its own file, or extend an existing one?

**Rule.** If the change fits into an existing file, go there.
**New files only for new conceptual units** — not because "it's tidier
that way".

**Why.** File proliferation makes navigation harder. The first question
should be: "Where is the natural place?", not "Where do I create a new
file?".

**How.**
1. `grep` / `Glob` for a related function / constant / component.
2. If a fitting file exists (e.g. `heatPhysics.ts` for everything around
   heat diffusion) → add it there.
3. Only create a new file when:
   - **New concept** that thematically does not belong in an existing
     file (e.g. a new subsystem loader, a new handler type).
   - The existing file would exceed the rule-of-thumb limit
     (2000 LOC, see [29-one-topic-per-doc.md](29-one-topic-per-doc.md))
     through the addition.
4. For test files: one per subsystem, not one per function.

**Example.**
- New constant `BORDER_COOLING_EXTRA` → into `heatConstants.ts` (exists).
  Don't create `borderCoolingConstants.ts`.
- New handler `STEAM_PUFF_HANDLER` → its own file
  `src/run/effectHandlers/steamPuff.ts` (by convention: one file per
  variant, see `21-handler-map-pattern.md` (Chimera only)).

**Anti-patterns.**
- "I'll put this in its own file so the other file doesn't get longer" —
  and suddenly everything becomes its own file.
- 80 mini-files with one 5-line function each.

**Related.**
- [17-centralize-constants.md](17-centralize-constants.md)
- [29-one-topic-per-doc.md](29-one-topic-per-doc.md)
- `21-handler-map-pattern.md` (Chimera only)
