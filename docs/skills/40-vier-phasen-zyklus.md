# Skill: Vier-Phasen-Hardening-Zyklus (Feature → Tests → Doku → Refactor)

**Wann.** Multi-Tage-Arbeit ueber 1+ Kalenderwoche. Die Mini-Commits aus
Teil A/B haben sich akkumuliert; technische Schuld baut sich auf.

**Regel.** Zyklus von **4 Phasen** mit definierten Dauern, durchlaufen
in dieser Reihenfolge. Danach beginnt der naechste Zyklus mit Phase 1.

| Phase | Fokus | Dauer |
|---:|---|---|
| 1 | Feature-Addition | **1–3 Tage** |
| 2 | Test-Coverage-Nachzug | **~½ Tag** |
| 3 | Doku-Konsistenz-Pruefung | **~2 Stunden** |
| 4 | Architektur / Performance / Redundanz + Refactor | **1 Tag** (selten 2) |

**Vollzyklus:** ~3–5 Tage Arbeit verteilt auf ~1 Kalenderwoche.
Hardening-Kadenz: ~7–10 Tage zwischen Phase-4-Tagen.

**Phase 1 — Feature-Addition.**
- Thematisch zusammenhaengendes Feature-Paket.
- Tests + Doku fliessen pro Commit mit (Baseline).
- **Grenzsignal:** nach 3 Tagen in Folge mit feature-dominantem
  Profil (>40% `feat:`-Commits) **zwingend** zu Phase 2.
- Exit: Paket fachlich fertig ODER 3-Tage-Limit erreicht.

**Phase 2 — Test-Coverage-Nachzug.**
- Coverage-Report pruefen, Edge-Cases nachziehen, in Phase 1 nur flach
  reparierte Bugs mit Regression-Tests sichern.
- Kein neuer Feature-Code.
- Exit: Coverage fuer in Phase 1 eingefuehrte Pfade lueckenschluss-nah.

**Phase 3 — Doku-Konsistenz-Pruefung.**
- Alle in Phase 1 veraenderten Subsysteme gegen `docs/*.md` pruefen.
- Querverweise, veraltete Passagen, CLAUDE.md-Aktualitaet.
- Kein Code — nur Doku.
- Exit: keine Diskrepanzen zwischen dokumentiertem und tatsaechlichem
  Verhalten.

**Phase 4 — Architektur / Performance / Redundanz + Refactor.**
- **Vormittags:** Analyse — Hotspots (`analysis/complexity-analysis.md`),
  Performance-Messungen, Architektur-Scan. Findings listen.
- **Nachmittags:** Umsetzung — `refactor:`-Commits, ein Befund pro
  Commit, `tsc + tests + build` gruen nach jedem.
- Exit: alle Befunde umgesetzt oder explizit auf „spaeter" zurueckgestellt
  (`docs/future-improvements.md`).

**Gating zwischen Phasen.**

| Uebergang | Gate |
|---|---|
| Phase 1 → 2 | Per-Commit-Suite gruen; Paket fertig oder 3-Tage-Limit. |
| Phase 2 → 3 | Keine neuen Coverage-Luecken aus Phase 1. |
| Phase 3 → 4 | Doku-Spot-Check ergibt keine Diskrepanz. |
| Phase 4 → 1 | Findings umgesetzt oder verbucht. Build + Tests gruen. |

**Warum.**
- Mini-Commits allein bauen technische Schuld auf, die spaeter teuer
  wird abzubauen.
- Dedizierte Phase 4 macht Hardening sichtbar (statt es zu vergessen).
- Phase 2/3 zwingen, Test- und Doku-Schuld nicht weiter laufen zu lassen.
- Chimera-Historie zeigt: Hardening-Tage bringen 20–30 `refactor:`-Commits
  in einem Schub (11.04., 19.04., 26.04.).

**Trigger fuer einen Hardening-Tag (Phase 4).**
1. **Mehrere Feature-Bursts hintereinander** ohne Refactor-Tag (>5
   feature-dominante Tage in Folge).
2. **Groessen-Signal:** Datei ueber ~500 LOC oder unklar geworden.
3. **Neu-Feature auf wackliger Basis:** naechste Erweiterung braucht
   Refactor zuerst.
4. **Doku driftet sichtbar:** beim Einarbeiten merkt man, dass Doku
   mehrere Schritte zurueck liegt.

**Was NICHT Teil des Zyklus ist.**
- **Keine Bugfix-Sprints.** Bugs werden in Phase 1 oder 4 behoben.
- **Keine Release-Phase.** Deploy laeuft durchgaengig (siehe
  `35-reproducible-deploy.md` (nur Chimera)).
- **Keine Kickoff-/Retro-Meetings.** Der Zyklus ist Arbeitsrhythmus,
  nicht Prozess-Zeremonie.

**Anti-Pattern.**
- Phase 4 ueberspringen, weil „noch ein Feature passt rein" — Schuld
  baut sich auf.
- Phasen vermischen — Test-Nachzug **und** neues Feature im selben Tag
  fragmentiert beides.
- Zyklus < 4 Tage (Phasen 2–4 werden Overhead) oder > 2 Wochen ohne
  Phase 4 (Schuld zu hoch).

**Verwandt.**
- [09-scope-vor-impl.md](09-scope-vor-impl.md)
- [11-mehrphasen-commits.md](11-mehrphasen-commits.md)
- [27-coverage-luecken-triage.md](27-coverage-luecken-triage.md)
- [28-doku-im-selben-commit.md](28-doku-im-selben-commit.md)
- `15-grosse-hooks-extrahieren.md` (nur Chimera)
- Vollstaendig: [good-development-practices.md → Teil C](../good-development-practices.md)
