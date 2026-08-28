# Skill: Aufwand-Schaetzung kalibrieren

**Wann.** Sobald ein Plan-Dokument geschrieben wird, das eine Aufwand-Schaetzung
enthaelt (`Aufwand: M`, `~2 Tage`, `~4 Stunden`, etc.).

**Regel.** Nach Anwendung der Korrektur-Faktoren wird die Schaetzung in den
Plan-Doc geschrieben. Begruendung des Faktors NICHT im Plan-Text duplizieren —
einfach den korrigierten Wert benennen.

**Korrektur-Faktoren** (empirisch aus 23 archivierten Plaenen, Stand 2026-06-02):

| Plan-Typ | Faktor | Beispiel |
|---|---|---|
| **Refactor mit klarer Architektur + Test-Net** | × **0.10** | "2 Tage" Schaetzung → in 2 h gemacht. Wenn die Architektur klar ist und Tests existieren, ist die mechanische Umsetzung sehr schnell. |
| **Feature mit User-Iteration / Balancing** | × **0.25** | Neue Mechanik + UI + Spieltest. Iterations-Zyklen brauchen echte Wall-Clock-Zeit. |
| **Multi-Phasen-Plan mit Spieltest-Schleifen (>5 Tage)** | × **0.5** | Diese sind die ehrlichsten Schaetzungen, weil sie tatsaechlich Tage Wall-Clock dauern. |
| **Plan mit Revert-Risiko (neue Mechanik, unsichere User-Spec)** | × **1.0** | Keinen Faktor anwenden. Reverts/Reworks sind die einzige Quelle echter Unterschaetzung. |

**Default bei Unsicherheit:** × **0.15** (zwischen Median 0.08 und Geomean 0.21,
konservativ in Richtung Geomean wegen besserer Outlier-Robustheit).

**How.**

1. **Schaetze wie bisher** in Stunden/Tagen, basierend auf Phasen, Komplexitaet, Risiken.
2. **Klassifiziere den Plan-Typ** (Refactor / Feature / Multi-Phasen / Revert-Risiko).
3. **Multipliziere mit Faktor.** Beispiel: "1-2 Tage Refactor" × 0.10 = **2-4 Stunden**.
4. **Schreibe den korrigierten Wert** in den Plan-Doc unter `## Aufwand`.
5. Bei Multi-Phasen-Plaenen: pro Phase einzeln korrigieren, dann summieren.

**Faustregel.** "1 Tag Refactor-Schaetzung = 1 Stunde Wall-Clock-Implementierung"
passt fuer 90 % der Refactor-Plaene. Wenn die Schaetzung sich danach laecherlich
klein anfuehlt — das ist normal, Plan-Schaetzungen sind systematisch um Faktor
5-10x ueberschaetzt.

**Warum.** Empirische Analyse von 23 Plan-Implementations-Paaren in
`docs/plans/archive/` zeigt:

- **Median(ratio) = 0.079**, Geomean = 0.12. Plaene brauchen ~8 % bis 12 %
  der ursprueglichen Schaetzung.
- **Refactor-Plaene** werden am drastischsten ueberschaetzt (Geomean 0.094) —
  wenn Architektur klar ist und Tests stehen, ist die mechanische Umsetzung
  in Minuten gemacht. **Beispiel:** 2026-06-02-propertybag-modularisation
  (Schaetzung 13.5 h "~2 Tage", real 30 min, Faktor 0.037).
- **Feature-Plaene** sind realistischer (Geomean 0.21) — User-Iteration +
  Balancing brauchen echte Wall-Clock.
- Nur **1 von 23 Plaenen** wurde unterschaetzt: 2026-05-26-laser-charge-
  extraction (8h Schaetzung, real 17.9h, Faktor 2.24). Grund: Phase-Reverts.

**Anti-Pattern.**

- "2 Tage" ohne Faktor in den Plan schreiben → User-Erwartung falsch
  kalibriert, Plan klingt teurer als er ist.
- Nach erfolgreicher Umsetzung den Faktor erhoehen ("ich war wohl optimistisch") —
  die Daten sagen das Gegenteil, die Tendenz zur Ueberschaetzung ist robust.
- Den Faktor blind anwenden ohne Plan-Typ-Klassifikation — ein Revert-Risiko-
  Plan (neue unsichere Mechanik) braucht KEINE Korrektur.

**Datenpunkte (heute, 2026-06-02):**

| Plan | Geschaetzt | Real | Faktor |
|---|---|---|---|
| 2026-06-02-item-applyToBag-hooks | 3-4 Tage (~28h) | 16 min | 0.009 |
| 2026-06-02-propertybag-modularisation | ~2 Tage (~13.5h) | 30 min | 0.037 |
| 2026-06-02-inverter-owner-scope | ~3-4 h | 1 h | 0.25 (Feature-Bug-Fix) |

**Cross-Refs.**

- [09-scope-vor-impl.md](09-scope-vor-impl.md) — Scope vor Implementierung.
- [10-plan-lifecycle.md](10-plan-lifecycle.md) — Plan-Lifecycle (wo die Schaetzung lebt).
