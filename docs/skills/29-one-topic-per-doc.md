# Skill: One Topic per Doc File + Clear Linking

**When.** You are creating new documentation or considering splitting /
merging an existing file.

**Rule.**
- The `docs/` folder is organized by **topics** (architecture, testing,
  deploy, one file per major subsystem).
- `CLAUDE.md` (root) as an index with a short description + link per file.
- Subdir CLAUDE.md links children + parent.

**File granularity (rule of thumb).**
- Subsystem docs: **200–1500 lines** is healthy.
- **< 100 lines:** check whether the section would be better placed in a
  larger file.
- **> 2000 lines:** check whether a standalone sub-topic can be
  extracted (example: `tasks.md` NPC subsystem →
  `npcs.md`).

**Linking convention (binding since 2026-05-05).**
- Internal refs to markdown files: relative `.md` paths with anchor:
  `[runs.md → Loot-Pools](../runs.md#loot-pools-boss-runjson)`.
- Plan refs as markdown links, not as code spans:
  `[plan-name.md](plans/...)` instead of `` `plans/...` ``.
- Active plans: `docs/plans/...md`. Archived:
  `docs/plans/archive/...md`.
- Cross-refs are **always updated along** when a plan is archived.

**Language (Chimera-specific).**
- German. The user base is German-speaking, new docs are consistently
  German. Existing mixed-language files are Germanized on the next
  substantial edit — no dedicated pass needed.

**H1 style (Chimera-specific).**
- `# Chimera — <Topic>` or `# <Topic>` (without "Chimera —") accepted.

**Test-coverage section.** If present: always `## Test-Coverage`
(not "Tests" or "Test-Abdeckung"). Subsystem docs without a
test section get at least a pointer at the end
("Tests in `src/__tests__/<file>.test.ts`").

**Why.** Nobody reads a 3000-line monster doc. Small, topic-focused
files get read and maintained.

**Anti-patterns.**
- `notes.md` with "everything that comes up".
- A 50-line `X.md` and a 50-line `Y.md` that are thematically
  identical.
- Leaving a plan in the active path after it was marked "implemented"
  without moving its substance to `docs/`.

**Related.**
- [28-docs-in-same-commit.md](28-docs-in-same-commit.md)
- [10-plan-lifecycle.md](10-plan-lifecycle.md)
- [13-prefer-existing-files.md](13-prefer-existing-files.md)
