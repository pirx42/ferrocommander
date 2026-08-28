# Skill: Plan-Dateinamen + Lifecycle (Entwurf → Archiviert)

**Wann.** Sobald eine Aufgabe Mehr-Phasen-Charakter hat und ein
Plan-Dokument in `docs/plans/` Sinn ergibt.

**Regel — Dateiname.** Plan-Datei MUSS mit `YYYY-MM-DD-` beginnen
(Memory `feedback_plan_filenames`). Beispiel: `2026-05-24-item-consolidation-audit.md`.

**Regel — Topic-Branch (seit 2026-07-18, Owner msg 15495).** Jeder Plan
startet mit einem eigenen Topic-Branch AUF dev
(`git checkout dev && git checkout -b topic/<plan-slug>`); waehrend des
Plans wird nur dorthin committet (Topic-Pushes frei). Nach Abnahme/Freigabe:
`git checkout dev && git merge --no-ff topic/<plan-slug>` — der Merge ist
Teil des Plan-Abschlusses. Details: Skill
`64-branch-workflow-dev-topic-main.md` (nur Chimera).

**Regel — Status-Header.** Erste oder zweite Zeile nach H1:
```
Status: Entwurf | In Arbeit | Umgesetzt | Deferred | Archiviert
```
Bei „Umgesetzt" + „Archiviert" zusaetzlich Implementations-Commit-Hashes
(`commits abc123/def456`) oder Hinweis `(commits siehe git log --grep ...)`.

**Regel — Aufwand-Schaetzung kalibrieren.** Wenn der Plan eine Aufwand-
Schaetzung enthaelt (`~2 Tage`, `Aufwand: M`, `~4 Stunden`): vor dem Schreiben
in den Plan-Doc den Faktor aus [45-aufwand-schaetzung-kalibrieren.md](45-aufwand-schaetzung-kalibrieren.md)
anwenden. Empirisch: Refactor-Plaene werden um Faktor 10x ueberschaetzt,
Feature-Plaene um 4x. Default ×0.15.

**Regel — Phase 0 Coverage-Pre-Check.** Nach der Scope-/Phasen-Planung
und VOR Phase A:
1. Pro Datei/Funktion, die der Plan modifiziert: Vorhandene Test-Coverage
   sichten, Luecken via [27-coverage-luecken-triage.md](27-coverage-luecken-triage.md)
   einordnen (erreichbar / defensiv / dead).
2. Erreichbare Pfade ohne Tests bekommen **vor** Phase A Charakterisierungs-
   Tests, die das *aktuelle* Verhalten festschreiben — Stil:
   [26-verhaltens-tests.md](26-verhaltens-tests.md).
3. Phase 0 wird als eigene Plan-Phase dokumentiert; ihr Commit traegt
   `test(<area>): pre-impl characterization` und ist sauber vom
   Refactor-Commit getrennt.

Details + Begruendung in [43-coverage-vor-umsetzung.md](43-coverage-vor-umsetzung.md).

**Regel — Letzte Phase = Refaktorierungs-Audit.** Die abschliessende Phase
jedes Plans ist ein Refaktorierungs-Audit + Korrektur über die gesamte
Plan-Implementierung (Architektur/Redundanz). Wird beim Planschreiben als
letzte Phase mitgeplant und tatsaechlich umgesetzt, bevor der Plan als
„Umgesetzt" gilt. Details: [49-plan-abschluss-refaktorierungs-audit.md](49-plan-abschluss-refaktorierungs-audit.md).

**Regel — Plan-Ende-Ritual.** Sobald die Implementierung (inkl. Audit-Phase)
abgeschlossen ist:

1. **(a) Substanz extrahieren.** Enthaelt der Plan dauerhaft nuetzliches
   Konzept-Material (Tabellen, Begruendungen, API-Schemata)? → in den
   passenden `docs/`-Eintrag wandern (`architecture.md`, `property-system.md`,
   `items.md`, ...). Plan-Status auf
   „Umgesetzt + Substanz extrahiert nach `<file>`".

2. **(b) Archivieren.** Wenn der Plan im Wesentlichen eine
   Schritt-fuer-Schritt-Anleitung war, deren Spuren im Code stehen:
   `git mv docs/plans/<file>.md docs/plans/archive/<file>.md`
   mit aktualisiertem Status-Header.
   **Nie** archivieren bevor (a) gegengeprueft ist — sonst entsteht eine
   zwei-Pass-Operation mit git-Luecke.

3. **(c) Loeschen.** Nur wenn der Plan reine Implementierungs-To-Dos
   ohne dauerhaften Wert enthielt (selten).

**Regel — Cross-Refs.** Aktive Plaene als `docs/plans/...md`, archivierte als
`docs/plans/archive/...md`. Beim Archivieren gleichzeitig Cross-Refs in
Code/Doku auf den neuen Pfad umbiegen.

**Regel — Audit.** Alle 4 Wochen oder nach grossen Sprints drei Fragen pro
Plan: (i) Status-Header noch korrekt? (ii) lebt Substanz im Plan, die nach
`docs/` gehoeren wuerde? (iii) sollte der Plan archiviert werden?

**Warum.** Plan-Dokumente driften sonst zwischen „aktiv" und „lange tot".
Datum im Namen sortiert chronologisch + zeigt Alter auf einen Blick.
Status-Header + Archivierung trennen aktiven Plan-Backlog von historischem
Material.

**Anti-Pattern.**
- `plans/item-audit.md` ohne Datum → in 6 Monaten unklar, ob aktuell.
- Plan „Umgesetzt" markieren, Substanz aber im Plan-File belassen → Doku
  driftet, weil Reader sie nicht dort sucht.
- Archivieren bevor Substanz nach `docs/` extrahiert ist.

**Verwandt.**
- [09-scope-vor-impl.md](09-scope-vor-impl.md)
- [28-doku-im-selben-commit.md](28-doku-im-selben-commit.md)
- [43-coverage-vor-umsetzung.md](43-coverage-vor-umsetzung.md) — Phase-0-Operation.
- [49-plan-abschluss-refaktorierungs-audit.md](49-plan-abschluss-refaktorierungs-audit.md) — Letzte-Phase-Operation.
