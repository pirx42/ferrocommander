# Skill: Generated file instead of hardcoded list/value (against drift)

**When.** You are facing a list or a value that can be **derived from a
source of truth** — existing files in the build output, `package.json`,
installed versions, an existing constants source — and you are about to
maintain it by hand in a **second** place. Or: a bug turns out to be "the
hardcoded copy is outdated/incomplete".

**Rule.** Derivable data is **generated at build time**, not duplicated.
A script (`scripts/generate-*.ts`) reads the source and writes a
**checked-in** file under `src/generated/`; `npm run build` calls it as the
first step (always fresh). Consumers import the generated file.

**Why.** Every hand-maintained copy drifts away from its source — and the
drift is invisible until someone notices the difference:
- A new file/version is added, the copy is not updated.
- The copy tries things that (no longer) exist → errors/404 noise.

Concretely from this codebase (2026-06-30, one user report each):
- **Run list:** `NewRunScreen` fetched through a fixed `ALL_RUN_IDS` list →
  404 blind probes for runs stripped from the public build. Fix: build
  manifest `runs/index.json` (`generate-run-manifest.ts`) → only what exists
  gets loaded.
- **Credits license list:** hardcoded, 6/10 versions outdated, Playwright +
  fonts missing. Fix: `generate-licenses.ts` reads `package.json` ×
  `node_modules` → versions + license texts stay automatically correct.

**How.**

1. **Identify the source.** What can the list/value be derived from?
   (Directory listing, `package.json`, another constant.)
2. **Pure core logic into `src/`** (testable without fs), fs/CLI wrapper
   into the script (`scripts/`). Pattern: `src/run/runManifest.ts` (pure) +
   `scripts/generate-run-manifest.ts`.
3. **Hook into the build:** `npm run build` calls the script (before
   `tsc`/`vite`, depending on the dependency). Manually via its own
   `generate:*` npm script.
4. **Keep it checked in** (so `vite dev` runs without a build) — pattern
   `propertyNormFactors.ts`. The build regenerates it (verify idempotence:
   `git diff` empty after the build).
5. **Fallback in the consumer**, in case the generated artifact can be
   missing (dev without build / old deploy) → fall back gracefully to the
   old behavior.

**Scope / exceptions.**
- **Deliberately static values** stay hand-maintained if the user wants it
  that way (e.g. the README test-count badge, user request 2026-06-23) —
  that is a decision, not a drift bug.
- Generating only pays off if the source is really the truth and changes.
  A 3-line list that never changes does not need a generator.

**Follow-up trap.** Whatever is generated/stripped must not be **hard
referenced** by the app elsewhere (see the run list above). Public build
variant: see [good-development-practices.md](../good-development-practices.md) 7.4.

**Follow-up trap 2 — regenerating is a LAST step (2026-08-07, msg 16753).**
Regenerating generated content docs (`docs:deployment`, `docs:loot`) in the
middle of a multi-phase content overhaul produces a checked-in intermediate
state that looks "current" — after later phases (here: deception rollback +
bolt fixes AFTER the phase-D regeneration) it is silently wrong. Rule: all
`docs:*` generators run once at the very END, after the last content
change; the owner's finding was that deployment-content.md still listed
removed pools as deployed. Second finding in the same pass: the generator
replace must be verified against the source after the run (does `git diff`
show the expected change?) — the docs:loot marker replace never matched
because of regex special characters and let the block silently go stale.

**Follow-up trap 3 — the generator sucks up machine-local artifacts
(housekeeping finding 2026-08-21).** Whoever reads a DIRECTORY also picks up
what does not belong there. `generate-deployment-doc` counted a gitignored
optimizer snapshot (`static/runs/*.best-bot.json`, written by
`npm run optimize`) as a run — on the next regeneration an artifact from
YOUR OWN machine would have landed in a committed doc. The same file was
offered by the run picker as a playable run. Two consequences for this
skill:

- ALWAYS read the diff of a regenerated doc before committing it. A
  generator output is not automatically reproducible — it is only as
  reproducible as its input directory is clean.
- Filter out non-content at the SOURCE and share the predicate with all
  consumers (here `runManifest.isRunFile`, used by the manifest AND the doc
  generator) — not a separate filter copy in every generator.

**Related.**
- [16-no-magic-values.md](16-no-magic-values.md) / [17-centralize-constants.md](17-centralize-constants.md)
- [44-no-redundancy.md](44-no-redundancy.md) (DRY — generating is DRY for data)
- `35-reproducible-deploy.md` (Chimera only) (the generator is part of the build process)

**Source.** Learnings from week 2026-06-23…30: three times a hardcoded
list/value cost a user report (run 404s, outdated credits versions, missing
license texts). Good-dev-practices 3.3c.
