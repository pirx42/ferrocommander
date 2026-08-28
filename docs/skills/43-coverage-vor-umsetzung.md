# Skill: Coverage-Pre-Check vor der Umsetzung

**Wann.** Sobald ein Plan inhaltlich festzeht (Phasen, Files, Funktionen
sind benannt) und BEVOR die erste Code-Aenderung erfolgt. Phase 0 des
Plans, vor Phase A.

**Regel.** Pro Datei/Funktion, die der Plan modifizieren wird:
1. Vorhandene Test-Coverage feststellen — gibt es Verhaltens-Tests fuer
   die heutigen Vertraege der Funktion?
2. Luecken pro Funktion triagieren wie in
   [27-coverage-luecken-triage.md](27-coverage-luecken-triage.md)
   (erreichbar / defensiv / dead).
3. **Erreichbare Pfade ohne Tests bekommen Charakterisierungs-Tests JETZT**
   — Tests gegen das aktuelle Verhalten, BEVOR die Implementierung
   startet. Stil: [26-verhaltens-tests.md](26-verhaltens-tests.md).
4. Diese Tests werden **vor Phase A committed**, gruen, ggf. als
   eigene Plan-Phase 0 dokumentiert.

**Warum.** Tests, die NACH einer Aenderung geschrieben werden, sind
unbewusst an der neuen Implementierung orientiert — sie pruefen, was die
geaenderte Funktion jetzt tut, nicht, was sie tun *soll*. Charakterisierungs-
Tests vor der Umsetzung halten das alte Verhalten fest und decken
Regressions auf, sobald die neue Logik vom alten Vertrag abweicht. Sie
verwandeln ein „Refactor mit Vertrauen" in ein „Refactor mit Netz".

Zweiter Effekt: das Schreiben dieser Tests zwingt zum genauen Lesen der
Funktion — Vertrags-Luecken, ueberflueschsige Branches und stille
Voraussetzungen fallen sofort auf und koennen in den Plan eingearbeitet
werden, bevor sie zur Bug-Quelle werden.

**How (Phase 0 im Plan).**
1. Plan-Phase **0 — Coverage-Pre-Check** einfuegen. Listet pro Funktion:
   `funcName` — abgedeckte Pfade / fehlende Pfade / „defensive"-OK.
2. Coverage-Lauf gegen die betroffenen Dateien:
   `npm run coverage` (Alias fuer `vitest run --coverage --exclude='**/*.slow.test.ts'`,
   seit QW5/2026-06-03). Ergebnis-JSON `coverage/coverage-summary.json`
   oder text-Output filtern. Fuer einzelne Files:
   `npx vitest run --coverage src/__tests__/<file>.test.ts`.
3. Verhaltens-Tests fuer die erreichbaren Luecken schreiben — pro Funktion
   ein eigener Test-Commit, klassifiziert als `test(<area>): pre-impl
   characterization` (typografisch eindeutig vom spaeteren
   feature/refactor-Commit getrennt).
4. Nach Phase 0 ist der Coverage-Vertrag definiert: **alle erreichbaren
   Pfade sind grun**. Phase A beginnt erst dann.

**Wann _nicht_ noetig.**
- Reine Bug-Fix-Patches ohne Refactor — der Regression-Test (Skill 23)
  ersetzt den Pre-Check.
- Funktion ist bereits gut abgedeckt (> 80 % Branch-Coverage UND
  Verhaltens-Tests sichtbar) — Pre-Check ist Doku im Plan, kein
  neuer Test.
- Reine Doku-Aenderung oder triviale Style-Aenderung.

**Anti-Pattern.**
- Plan-Phase 0 weglassen, „weil ich die Funktion schon kenne". Im Kopf
  bleibt das Vertrauen nicht hochaufloesend — der Test schon.
- Charakterisierungs-Tests im selben Commit wie der Refactor anlegen —
  dann ist nicht mehr nachweisbar, welches Verhalten alt und welches
  neu war.
- „Quick-and-Dirty" als Argument fuer das Auslassen — siehe
  [41-korrektere-variante.md](41-korrektere-variante.md), das ist genau
  der Fall, in dem die korrektere Variante die Tests vorzieht.

**Verwandt.**
- [09-scope-vor-impl.md](09-scope-vor-impl.md) — Scope steht **vor** dem
  Pre-Check fest; Coverage-Pre-Check setzt auf den abgegrenzten Scope auf.
- [10-plan-lifecycle.md](10-plan-lifecycle.md) — Plan-Phase 0 ist
  obligatorischer Bestandteil eines Plan-Dokuments.
- [23-tests-pro-commit.md](23-tests-pro-commit.md),
  [26-verhaltens-tests.md](26-verhaltens-tests.md),
  [27-coverage-luecken-triage.md](27-coverage-luecken-triage.md).
- `42-housekeeping-coverage-doku-drift.md` (nur Chimera)
  — Daily-Housekeeping nimmt die ALT-Coverage-Luecken auf, Skill 43 nimmt
  die im aktuellen Plan beruehrten.
