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

**The grep in step 1 is necessary and not sufficient**, and the
2026-08-30 documentation review is the evidence. It found thirty-six
drifted claims, and the greppable ones were the minority. Three kinds
get past a search for what you changed:

- **A sentence about a rule you changed, in a file you did not touch.**
  Binding `Ctrl+C` in `keymap.rs` made a sentence in `command-line.md`
  false. Nothing in that file changed, and nobody was reading it.
- **A feature described as future work.** "Phase D puts the reason in
  the path bar" names no identifier at all, so no grep finds it — and
  it stays grammatical and plausible forever. Four of these were found
  at once, across three files.
- **A number.** Test counts, timings, how many of a thing there are.
  Every one of them in that review was stale, including one in a
  sentence arguing that a dated measurement stays honest.

So step 1 has two more parts, both cheap:

5. `grep` for the *behaviour* as well as the identifier — the rule you
   changed, in the words a document would use for it, across all of
   `docs/`. That is what a search for the changed symbol cannot reach.
6. If the change adds or removes an entry in a **table or a list** —
   a binding, a module, a trait method, a suite — consider generating
   the document's copy of it instead of editing it (skill
   [53](53-generate-instead-of-duplicating.md)). A table checked by a
   test cannot drift; a table you remembered to update this time can.

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
