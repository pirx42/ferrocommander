# Skill: Konstanten zentralisieren, sobald thematisch verwandt

**Wann.** Drei oder mehr Konstanten gehoeren zum gleichen Subsystem
(Heat-Sim, Energy-Pool, Cable-Physics, Tutorial-Run-Timing) und wandern
ueber mehrere Dateien.

**Regel.** Sammle sie in einer dedizierten `<bereich>Constants.ts`-Datei.

**Ausnahme.** Feature-spezifische UI- / Timing-Konstanten
(`BEAM_FLASH_COOLDOWN_MS`, `FIELD_LOCK_SNAP_DURATION_MS`) bleiben in
ihren Feature-Modulen — sie sind keine Sim-Parameter und gehoeren zur
jeweiligen Logik.

**Warum.** Versprenkelte Konstanten driften: zwei Stellen halten dasselbe
Konzept mit minimal unterschiedlichen Werten, oder die Doku zitiert einen
Wert, der so nirgends mehr existiert. Ein zentraler Sammelpunkt macht
Game-Mechanik-Tuning lokal und vereinfacht Doku-Verweise.

**How.**
1. `grep` nach dem Wert / Konzept in mehreren Dateien.
2. Falls 3+ Stellen mit verwandter Konstante: neue Datei
   `src/<bereich>Constants.ts` (z.B. `heatConstants.ts`,
   `energyPoolConstants.ts`).
3. Pro Konstante: kurzer Kommentar mit „Warum dieser Wert" (Game-Balance,
   Physik-Konstante, gemessen, ...).
4. Aufrufer auf import umstellen, Inline-Werte entfernen.
5. Doku (z.B. `docs/heat-system.md`) referenziert die Datei statt
   einzelner Werte.

**Beispiel — Chimera live.**
`src/heatConstants.ts` enthaelt:
- `HEATMAP_DIFFUSION_RATE`
- `BORDER_COOLING_EXTRA`
- `ITEM_HEAT_DURATION_MS`
- `CABLE_HEAT_FULL_FLOW`
- `ZONE_HEAT_DAMAGE_THRESHOLD`

Vorher: verteilt in `heatPhysics.ts`, `useHeatSimulation.ts`,
`thermoInjectionTick.ts` und `extremeTemperatureTick.ts`.
`docs/heat-system.md` referenziert sie als Spielmechanik-Parameter — nur
sinnvoll mit einem Standort.

**Anti-Pattern.**
- Konstante in jedem Caller redeklariert, weil „den Import einsparen".
- Alle Game-Konstanten in einer 2000-Zeilen-`constants.ts` zusammenwerfen,
  ohne Subsystem-Trennung.

**Verwandt.**
- [16-keine-magic-values.md](16-keine-magic-values.md)
- [13-bestehende-dateien-bevorzugen.md](13-bestehende-dateien-bevorzugen.md)
