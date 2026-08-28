# Skill: Keine Redundanzen / kein doppelter Code

**Wann.** Beim Schreiben oder Editieren von Code merkt man: "den Block
hab ich gerade schon getippt" oder "dieselbe Logik steht auch in
Modul Y". Auch bei Refactor- oder Review-Durchgaengen, wo zwei oder
mehr Stellen auffallen, die fast identisch sind.

**Regel.** Gleiche Logik kommt **einmal** vor. Was sich an mehreren
Stellen wiederholt, wandert in einen geteilten Helper / eine
Funktion / eine Konstante.

**Warum.** Duplikate divergieren ueber die Zeit:
- Eine Stelle wird gefixt, die andere nicht — und der Bug bleibt
  unsichtbar weiter im System.
- Der Refactor-Aufwand multipliziert sich pro Kopie.
- Die Codebase blaeht auf, ohne dass die zusaetzlichen Zeilen
  Mehrwert tragen.

Konkret aus dieser Codebase: der heat-Burst-Vergleich
`(now - fireTime) < THRESHOLD` lebte an mehreren Stellen
(heatPhysics, instanceStatRows, engine.calculateDynamicHeatProduction).
Als die Zeitbasis von Date.now() auf gameTimeMs umzog, wurde nur eine
Stelle umgestellt — die anderen kollabierten still, der Bug fiel erst
dem User auf (msg 10991). Eine geteilte Helper-Funktion
`isWithinBurstWindow(now, fireTime)` waere robuster gewesen.

**Wie anwenden.**

- **Beim Tippen.** STOP, sobald man merkt "das hab ich gerade schon
  geschrieben". Auch wenn es nur 3-5 Zeilen sind: in einen Helper
  ziehen, BEVOR die zweite Kopie committet wird.
- **Beim Refactor.** Aktiv nach Duplikat suchen — gleiches Pattern in
  2+ Files mit nur geringfuegigen Unterschieden ist ein Smell. Grep
  nach charakteristischen Konstanten oder Variablen-Namen.
- **Vergleichs-Fenster / Schwellen-Checks.** `(now - fireTime) <
  THRESHOLD`, `Math.abs(a - b) < EPSILON`, `value >= minTemp &&
  value <= maxTemp` — solche Pattern lieber EINMAL als geteilte
  Funktion, mit klarem Namen.
- **Konstanten.** Zentral in einem Konstanten-Modul (siehe
  [17-konstanten-zentralisieren.md](17-konstanten-zentralisieren.md)),
  nicht in jedem Modul neu definieren.
- **Komplexe Konditionalketten.** Wenn dieselbe `if (a && b || c)`-
  Bedingung an 3 Stellen steht: eine Predicate-Funktion
  `isReadyToFire(state)` extrahieren.

**Ausnahmen.**

- **Drei sehr aehnliche Zeilen mit unterschiedlicher Semantik** sind
  besser als eine premature Abstraktion. Beispiel: drei Render-Calls
  mit unterschiedlichen Farben sind drei Zeilen — nicht eine
  Helper-Funktion mit Color-Parameter, wenn die Calls semantisch
  verschiedene Dinge zeichnen.
- Test-Setups und Fixtures: dort ist explizite Duplikation oft
  klarer als ein "smarter" Helper, der alle Felder dynamisch
  generiert.
- Boilerplate, das vom Framework verlangt wird (z.B. drei React-
  Component-Wrapper, alle aehnlich) — solange jeder eine eigene
  klare Rolle hat.

**Smell test.** "Wenn ich eine Stelle aendere, muss ich dann eine
zweite ebenfalls anfassen?" — Wenn ja: extrahieren. "Wuerde ein
Bug-Fix an Stelle A genauso in Stelle B passen?" — Wenn ja:
extrahieren.

**Quelle.** User-Spec msg 11002 (2026-05-31) — explizite permanente
Regel; eskaliert nach einer Session, in der eine duplizierte Time-
Vergleichs-Logik nach einem Refactor zerbrach.

Herkunfts-Notiz: die fruehere lokale Memory `feedback_no_redundancy` wurde
2026-08-07 ins Repo migriert (Plan
[2026-08-07-agent-wissen-ins-repository.md](../plans/archive/2026-08-07-agent-wissen-ins-repository.md))
— dieses Skill ist die kanonische Stelle.
