# Skill: Bestehende Dateien editieren, keine neuen leichtfertig erzeugen

**Wann.** Du willst eine Funktion / Konstante / Komponente hinzufuegen
und ueberlegst: eigenes File oder bestehende erweitern?

**Regel.** Wenn die Aenderung in eine bestehende Datei passt, geh dort
hinein. **Neue Dateien nur bei neuen konzeptuellen Einheiten** — nicht
weil „dann ist es uebersichtlicher".

**Warum.** Datei-Proliferation erschwert Navigation. Die erste Frage
sollte lauten: „Wo ist der natuerliche Platz?", nicht „Wo erzeuge ich
eine neue Datei?".

**How.**
1. `grep` / `Glob` nach verwandter Funktion / Konstante / Komponente.
2. Wenn ein passendes File existiert (z.B. `heatPhysics.ts` fuer alles
   um Heat-Diffusion) → dort hinzufuegen.
3. Nur dann neue Datei, wenn:
   - **Neues Konzept**, das thematisch nicht in eine bestehende Datei
     gehoert (z.B. neuer Subsystem-Loader, neuer Handler-Typ).
   - Bestehende Datei wuerde durch die Erweiterung ueber das
     Faustregel-Limit (2000 LOC, siehe [29-ein-topic-pro-doku.md](29-ein-topic-pro-doku.md))
     gehen.
4. Bei Test-Files: einen pro Subsystem, nicht einen pro Funktion.

**Beispiel.**
- Neue Konstante `BORDER_COOLING_EXTRA` → in `heatConstants.ts` (existiert).
  Nicht `borderCoolingConstants.ts` anlegen.
- Neuer Handler `STEAM_PUFF_HANDLER` → eigenes File
  `src/run/effectHandlers/steamPuff.ts` (per Konvention: ein File pro
  Variante, siehe `21-handler-map-pattern.md` (nur Chimera)).

**Anti-Pattern.**
- „Ich packe das in ein eigenes File, damit das andere File nicht laenger
  wird" — dann wird auf einmal alles ein File.
- 80 Mini-Files mit je einer 5-Zeilen-Funktion.

**Verwandt.**
- [17-konstanten-zentralisieren.md](17-konstanten-zentralisieren.md)
- [29-ein-topic-pro-doku.md](29-ein-topic-pro-doku.md)
- `21-handler-map-pattern.md` (nur Chimera)
