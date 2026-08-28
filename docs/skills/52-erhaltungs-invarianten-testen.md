# Skill: Erhaltungs-/Physik-Invarianten testen (statt Wert-Asserts)

**Wann.** Du schreibst Tests für Solver-/Sim-/Aggregations-Code (Energie,
Hitze, Druck, Flüsse) — oder du fragst dich, warum ein Bug trotz großer
Test-Suite durchrutschte.

**Regel.** Für Code, der eine **physikalische/strukturelle Invariante** erfüllen
muss, teste die **Invariante** über viele/zufällige/kombinierte Inputs — NICHT
nur konkrete Output-Werte einzelner Fälle. Wert-Asserts (`cableFlow === 15`)
kodifizieren bei einem Bug genau den falschen Wert als „korrekt".

**Warum.** Charakterisierende Wert-Tests werden oft geschrieben, indem man das
aktuelle Verhalten beobachtet und festschreibt. Leakt die Implementierung, sagt
der Test „15 ist erwartet" — und schützt den Bug, statt ihn zu finden. Eine
Invariante (`geliefert ≤ produziert`) ist unabhängig vom konkreten Wert wahr und
fällt über JEDE Topologie, die sie verletzt.

**Kriegsgeschichte (2026-06-24).** Ein Generator mit internem Verbraucher
(`generator(sensor)`) exportierte seine BRUTTO-Produktion ans Kabel UND speiste
intern den Sensor → das System lieferte 20 aus 15 Produktion (5 E/s aus dem
Nichts). ~5200 bestehende Tests fingen es NICHT: sie asserten konkrete Werte
(`demand=20`, `cableFlow=15`), und das Leck sah funktional gesund aus (Schild
lud, Sensor lief) — selbst die Boss-Invarianten („Schilde laden") blieben grün,
WEIL der Bug „funktionierendes" Verhalten erzeugte. Sichtbar wurde es erst, als
eine neue Anzeige Produktion vs. Verbrauch SUMMIERTE. Der Fix-Test ist eine
Invariante: „jede Quelle exportiert ≤ effProduction − effDemand", über 38
Topologien (Produzenten × Senken × Ladestände + Ketten). Gegengeprüft: rot auf
dem Buggy-Code, grün mit Fix.

**Kriegsgeschichte II — dieselbe Invariante, umgekehrtes Vorzeichen (2026-07-12).**
Der Spiegelfall: nicht „Energie aus dem Nichts", sondern legitimer Fluss, der
STUMM geblockt wird. Ein siphon-getroffener Nicht-Source-Knoten (Ventilator) wurde
zur temporären Quelle, aber `applyStorageBottlenecks` gab JEDEM Nicht-Source-Knoten
Export-Kapazität hart `0` → sein Überschuss wurde als „aus dem Nichts" gewertet und
der Kabel-Abfluss genullt; ein gecableter Kondensator lud NIE (ein Schild schon — der
zieht über die eigene Demand). ~6800 Tests fingen es nicht: die neue Contribution war
im Reader-Fold UND in `buildHydraulicNode` (Injektion) korrekt, aber die dritte Stufe
(`applyStorageBottlenecks`) kannte sie nicht — kein Test prüfte end-to-end, dass der
Überschuss eines Produzenten einen gecableten Verbraucher/Speicher wirklich ERREICHT.
Lehre: (1) die Erhaltung gilt beidseitig — `geliefert ≤ produziert` UND `verbundener
Überschuss kommt an`; (2) ein neuer Energie-Beitrag muss durch ALLE Solver-Stufen
(Reader → Node-Build → Bottleneck → Charge-Update) gefädelt und mit einem
System-Level-Test (gecableter Speicher lädt aus dem Producer) abgesichert werden,
nicht nur je Funktion. Gegengeprüft: Fix-Revert → Kondensator-Ladung 0.

**How.**
- **Formuliere die Erhaltung explizit:** Σ Output ≤ Σ Input (+ Speicher-
  Entladung). Pro Knoten: Abfluss ≤ Netto-Kapazität. Beidseitig: verbundener
  Überschuss muss den Verbraucher/Speicher auch ERREICHEN (nicht still versanden).
- **Über alle Pipeline-Stufen fädeln:** ein neuer Beitrag (Quelle/Senke/Delta) muss
  in JEDER Stufe ankommen (Reader → Node-Build → Bottleneck → Charge), nicht nur der
  ersten — ein System-Level-Test deckt die Lücke, die Per-Funktions-Tests offenlassen.
- **Property-/tabellengetrieben:** generiere viele Kombinationen (Bauteile ×
  Verschaltung × Anfangszustände) und assertiere die Invariante in JEDER — nicht
  einen handverlesenen Wert.
- **Über mehrere Ticks** prüfen (Transienten + eingeschwungener Zustand).
- **Miss am richtigen Punkt:** nicht an einer Display-Aggregat-Kennzahl, die
  Pool-Bezug + Selbst-Entladung mischt (z.B. `totalActualConsumption` zählt bei
  vollem ChargeSink den ganzen internalDrain). Nutze die echte Fluss-/Ladungs-
  Größe (Kabel-Flow, Ladungs-Delta).
- **Verifiziere den Wächter:** Fix kurz zurücknehmen → Test MUSS rot werden.
  Ein Invarianten-Test, der den bekannten Bug nicht fängt, ist wertlos.

**Verwandtes Prinzip — gilt auch außerhalb von Tests.** Verifiziere die
INVARIANTE, nicht das Oberflächen-Muster. Beispiel Content-Migration (46 JSONs,
`productionMultiplier` in Torso-Zonen): der erste Regex-Pass auf `"zone":"torso"`
traf auch Item-`location`-Objekte + Events. Catch via STRUKTUR-Check (JSON
parsen, assertieren dass nur Objekte mit `maxHp` = echte Zonen-Defs geändert
wurden) → revert → Neuanlauf mit `maxHp`-Anker + Count-Verifikation. Nicht „passt
schon", sondern strukturell beweisen, dass nur die gewollte Menge getroffen ist.

**Anti-Pattern.**
- `expect(result).toBe(<beobachteter Wert>)` für Solver-Output ohne zu prüfen,
  ob der Wert physikalisch stimmt.
- Invariante nur am bekannten Bug-Fall testen statt über die Kombinatorik.
- Content-Migration per blindem Such-Ersetzen ohne strukturelle Gegenprobe.

**Verwandt.**
- [26-verhaltens-tests.md](26-verhaltens-tests.md) — Verhalten statt Struktur (Invarianten sind die stärkste Form davon).
- `48-design-invarianten-respektieren.md` (nur Chimera) — Design-Invarianten respektieren.
- [23-tests-pro-commit.md](23-tests-pro-commit.md) — Bug-Fix bringt Regressions-Test.
