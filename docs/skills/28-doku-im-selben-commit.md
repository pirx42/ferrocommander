# Skill: Doku im gleichen Commit wie die Code-Aenderung

**Wann.** Ein Feature / Refactor / Fix aendert die **dokumentierte
Realitaet** (Architektur, APIs, Konfiguration, Deploy-Schritte,
Save-Format, JSON-Schema).

**Regel.** Doku im **selben Commit** updaten (oder im direkt folgenden,
wenn der Commit sonst zu gross wird).

**Warum.** Asynchrone Doku veraltert nie von selbst — sie veraltert, weil
niemand sie ihren Weg hinterher pflegt. Im selben Commit zu bleiben
zwingt den Autor, die Doku zu sehen. „Doku-PR spaeter" ist in der Praxis
„Doku-PR nie".

**How.**
1. Vor `git add`: `grep` nach Doku-Referenzen auf veraenderte API /
   Konstante / Datei.
2. Wenn Treffer: Doku mit-anpassen.
3. Im Commit-Body kurz vermerken („Doku in `docs/heat-system.md`
   nachgezogen").
4. Bei grossen Refactors: Doku-Schritt als eigene Phase
   (Conventional-Commit `docs(scope): ...`), aber direkt nach dem
   Code-Commit, nicht „spaeter mal".

**Beispiel.**
```
Commit: feat(damping): inverter as count-model with retention 0.9

Changed: src/propertyBag.ts (DEFAULT_INVERTER_RETENTION, Phase C count-based)
Updated: docs/property-system.md (Section "Inverter Damping"),
         docs/plans/2026-05-24-item-consolidation-audit.md (Section 0),
         docs/items-evaluation.md (banner: inverterMod section outdated)

Old XOR/boolean model removed entirely. New count-model documented
inline; outdated content marked.
```

**Anti-Pattern.**
- Feature mergen, Doku-Update als Sub-Task „in den Backlog".
- Doku-Update committen, das auf Code-Stand aus letzter Woche
  referenziert.
- Plan-Datei „umgesetzt" markieren, Substanz aber nicht extrahieren
  (siehe [10-plan-lifecycle.md](10-plan-lifecycle.md)).

**Verwandt.**
- [29-ein-topic-pro-doku.md](29-ein-topic-pro-doku.md)
- [30-warum-dokumentieren.md](30-warum-dokumentieren.md)
- [10-plan-lifecycle.md](10-plan-lifecycle.md)
