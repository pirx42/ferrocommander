# Skill: Letzter Plan-Schritt = Refaktorierungs-Audit mit Korrektur

**Wann.** Jeder Mehr-Phasen-Plan in `docs/plans/`. Sobald die
Implementierungs-Phasen (Feature/Refactor/Fix + Tests + Doku) durch sind,
folgt als **letzte Plan-Phase** ein Refaktorierungs-Audit über die
**gesamte im Plan umgesetzte Implementierung**.

**Regel.** Kein Plan gilt als abgeschlossen, bevor eine eigene Schluss-Phase
die kumulierte Implementierung des Plans gegen **Architektur** und
**Redundanz** geprüft und die gefundenen Verbesserungen **umgesetzt** hat.
Diese Phase wird beim Planschreiben bereits als letzte Phase mitgeplant.

**Warum.** Über mehrere Implementierungs-Phasen hinweg entstehen lokal
sinnvolle, in der Summe aber suboptimale Strukturen: duplizierte Logik
zwischen Phasen, inline-Unions/Konstanten die zentralisiert gehören (vgl.
[44](44-keine-redundanzen.md)), Sonderfälle die ein gemeinsames Muster
verdecken, Helfer die erst nach Phase C als extrahierbar erkennbar werden.
Wer das pro Einzel-Phase nicht sieht, sieht es im Gesamt-Blick. Der Audit
am Ende ist der Moment, in dem das Gesamtbild der Plan-Implementierung
vorliegt — dort lässt sich Konsolidierung am billigsten und am sichersten
(Tests sind grün, Verhalten verifiziert) durchführen.

**Wie anwenden.**

- **Beim Planschreiben.** Die letzte Phase heisst „Refaktorierungs-Audit +
  Korrektur" und steht explizit in der Phasen-Liste und in `meta`/Status.
- **Scope.** Geprüft wird, was **dieser Plan** angefasst hat (alle in den
  Phasen geänderten/neuen Dateien), nicht die ganze Codebasis — dafür gibt
  es [47](47-architektur-audit-mit-subagents.md).
- **Audit-Achsen** (mindestens):
  - **Redundanz/DRY:** gleiche Logik in 2+ der Plan-Commits → in geteilten
    Helper/Typ/Konstante ziehen ([44](44-keine-redundanzen.md),
    [16](16-keine-magic-values.md)/[17](17-konstanten-zentralisieren.md)).
  - **Architektur:** Sonderfälle, die ein gemeinsames Muster verdecken;
    Handler-Map statt verteilter Switches (21 (`21-handler-map-pattern.md`, nur Chimera));
    pure Reducer aus Hooks (14 (`14-pure-reducer.md`, nur Chimera)); zu grosse Funktionen
    (15 (`15-grosse-hooks-extrahieren.md`, nur Chimera)).
  - **Konsistenz:** Naming, Datei-Verortung (Layer), Schnittstellen-Form
    über die neuen Teile hinweg einheitlich.
  - **Tote Reste:** Bw-Compat-Shims, ungenutzte Exporte, Scaffolding das
    nach der Konsolidierung wegfällt ([20](20-keine-bw-compat-shims.md)).
- **Korrektur umsetzen, nicht nur notieren.** Gefundene Punkte werden in
  derselben Phase behoben (eigener/eigene Commit(s) `refactor(<area>): ...`),
  Suite bleibt grün ([25](25-gruene-suite-vor-commit.md)). Was bewusst
  vertagt wird, wandert als benannter Follow-up in den Plan/Memory — nicht
  stillschweigend liegen lassen.
- **Verifikation.** Nach dem Audit-Refactor: tsc + Tests + ggf. Build/Deploy
  wie bei jeder Phase. Verhaltensgleichheit ist Pflicht (reiner Refactor).

**Anti-Pattern.**
- Plan auf „Umgesetzt" setzen, sobald das letzte Feature läuft — ohne den
  Gesamt-Blick auf Architektur/Redundanz.
- Audit-Findings nur als „TODO später" notieren statt sie in der Phase zu
  beheben.
- Den Audit auf die ganze Codebasis ausweiten (das ist [47](47-architektur-audit-mit-subagents.md));
  hier geht es um die **Plan-eigene** Implementierung.

**Quelle.** User-Spec 2026-06-05 (msg 11409): „der letzte Schritt eines Plans
soll ab sofort ein Refaktorierungs-Audit mit Korrekturumsetzung sein, das die
gesamte umgesetzte Implementierung hinsichtlich Architektur/Redundanz prüft
und verbessert."

**Verwandt.**
- [10-plan-lifecycle.md](10-plan-lifecycle.md) — Phasen-Gerüst + Plan-Ende-Ritual.
- [40-vier-phasen-zyklus.md](40-vier-phasen-zyklus.md) — Phase 4 (Refactor) im Mehr-Tages-Zyklus.
- [44-keine-redundanzen.md](44-keine-redundanzen.md), [41-korrektere-variante.md](41-korrektere-variante.md).
- [47-architektur-audit-mit-subagents.md](47-architektur-audit-mit-subagents.md) — Codebasis-weiter Audit (anderer Scope).

Memory: `feedback_plan_refactor_audit` (Auto-Memory unter `/home/pirx/.claude/...`, ausserhalb des Repos).
