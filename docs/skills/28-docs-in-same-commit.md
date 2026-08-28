# Skill: Docs in the Same Commit as the Code Change

**When.** A feature / refactor / fix changes the **documented reality**
(architecture, APIs, configuration, deploy steps, save format,
JSON schema).

**Rule.** Update the docs in the **same commit** (or in the one directly
following, if the commit would otherwise get too big).

**Why.** Asynchronous documentation never goes stale on its own — it goes
stale because nobody maintains it afterwards. Staying in the same commit
forces the author to look at the docs. "Docs PR later" is in practice
"docs PR never".

**How.**
1. Before `git add`: `grep` for doc references to the changed API /
   constant / file.
2. If there are hits: adjust the docs along with the code.
3. Note it briefly in the commit body ("docs in `docs/heat-system.md`
   updated to match").
4. For large refactors: docs step as its own phase
   (conventional commit `docs(scope): ...`), but directly after the
   code commit, not "sometime later".

**Example.**
```
Commit: feat(damping): inverter as count-model with retention 0.9

Changed: src/propertyBag.ts (DEFAULT_INVERTER_RETENTION, Phase C count-based)
Updated: docs/property-system.md (Section "Inverter Damping"),
         docs/plans/2026-05-24-item-consolidation-audit.md (Section 0),
         docs/items-evaluation.md (banner: inverterMod section outdated)

Old XOR/boolean model removed entirely. New count-model documented
inline; outdated content marked.
```

**Anti-patterns.**
- Merging the feature, docs update as a sub-task "into the backlog".
- Committing a docs update that references the code state from last
  week.
- Marking a plan file "implemented" without extracting its substance
  (see [10-plan-lifecycle.md](10-plan-lifecycle.md)).

**Related.**
- [29-one-topic-per-doc.md](29-one-topic-per-doc.md)
- [30-document-the-why.md](30-document-the-why.md)
- [10-plan-lifecycle.md](10-plan-lifecycle.md)
