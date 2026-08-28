# Skill: Multi-Phasen-Plaene autonom durchziehen

**Wann.** Plan ist vom Auftraggeber **freigegeben** und enthaelt mehrere
Phasen / Schritte ohne offene Entscheidungspunkte.

**Regel.** Alle Phasen am Stueck durcharbeiten. Unterbrechen **nur** bei
echten Entscheidungen (Tradeoff, Schema-Bruch, neue Anforderung).
Nach jeder Phase: in den **Topic-Branch des Plans** committen (+ Topic-Push
als Backup — frei erlaubt), kurz Bescheid geben, weitermachen. Nach dev
gemergt wird erst bei der Abnahme (Skill
`64-branch-workflow-dev-topic-main.md` (nur Chimera)).

**Warum.** Sich nach jedem Sub-Schritt rueckzuversichern produziert
Wartezeit und macht es schwer, den Plan im Stueck mental zu halten. Wenn
der Plan vorher abgestimmt war, ist Wegarbeiten das Default.

**How.**
1. Plan-Datei (oder TaskList) zur Hand halten.
2. Phase N umsetzen → Tests/Doku/Build gruen → commit auf den Topic-Branch (+ Topic-Push) → ein-Satz-Update.
3. Direkt mit Phase N+1 weiter, ohne neuer Zustimmungs-Schleife.
4. Nur stoppen bei: Auswahl-Frage, Spec-Luecke, harten Konflikten zu
   anderen Plan-Teilen.

**Beispiel.**
```
Plan freigegeben: Phasen 1-5, jede mit eigener Test-Suite + Commit.
→ Phase 1 fertig, push abc1234. Weiter mit 2.
→ Phase 2 fertig, push def5678. Weiter mit 3.
... bis 5.
→ Phase 5 fertig, push ghi9012. Plan abgeschlossen.
```

**Anti-Pattern.**
- „Phase 1 fertig — soll ich Phase 2 anfangen?" wenn der Plan
  Phase 1+2+3+4+5 enthielt und es keine neuen Erkenntnisse gibt.
- Am Ende einen Sammel-Commit machen, statt pro Phase einen.

**Verwandt.**
- [32-commit-pro-schritt.md](32-commit-pro-schritt.md)
- [10-plan-lifecycle.md](10-plan-lifecycle.md)
- [05-updates-bei-langen-tasks.md](05-updates-bei-langen-tasks.md)
