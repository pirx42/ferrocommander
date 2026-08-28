# Skill: Keine Magic Values — Zahlen/Strings als benannte Konstanten

**Wann.** Du schreibst eine Zahl oder einen String mit semantischer
Bedeutung in den Code: Schwellenwert, Timeout, Tile-Groesse, URL-Pfad,
Error-Code, Feature-Flag-Name.

**Regel.** Als **benannte Konstante** (uppercase + Suffix), nicht inline.
Schwellenwerte, Timeouts, Einheiten — alles benannt.

**Warum.** Benannte Konstanten sind Dokumentation.
`const TASK_START_DELAY_MS = 5000` erklaert sich selbst; ein nacktes
`5000` in einem Timer-Aufruf nicht. Spaeter sucht jemand „wo wird das
gesetzt?" — der Name ist die Suchhilfe.

**How.**
- Naming: `THING_PURPOSE_UNIT` (`HEATMAP_DIFFUSION_RATE`,
  `TASK_START_DELAY_MS`, `MAX_RETRY_ATTEMPTS`).
- Pro thematischem Subsystem in `<bereich>Constants.ts` zentralisieren
  (siehe [17-konstanten-zentralisieren.md](17-konstanten-zentralisieren.md)).
- Inline-Konstanten OK, wenn sie wirklich nur an einer Stelle gebraucht
  werden UND sich nicht wiederholen — aber im Zweifel: benennen.

**Beispiel.**
```ts
// JA
const SECTOR_TRANSITION_DEBOUNCE_MS = 1500;
setTimeout(transition, SECTOR_TRANSITION_DEBOUNCE_MS);

// NEIN
setTimeout(transition, 1500); // why 1500?

// JA
if (heat > ZONE_HEAT_DAMAGE_THRESHOLD) { ... }

// NEIN
if (heat > 0.8) { ... } // 0.8 of what?
```

**Anti-Pattern.**
- Mehrere Stellen mit demselben Wert (`5000`, `5000`, `5000`) — Drift
  garantiert.
- Konstanten mit kryptischen Namen (`X = 0.8`) — fast so schlimm wie inline.

**Verwandt.**
- [17-konstanten-zentralisieren.md](17-konstanten-zentralisieren.md)
- [18-kommentare-warum.md](18-kommentare-warum.md)
