# Skill: Memory + Audit-Reports verifizieren, bevor du handelst

**Wann.**
- Du willst aus einem Memory-Eintrag heraus eine Aktion machen
  („Funktion X liegt in Y, dort fixen").
- Ein Sub-Agent / externer Audit hat einen Befund geliefert
  („Datei Z hat kein Caching", „Funktion W hat keinen Early-Break").

**Regel.** **Vor der Umsetzung** den Befund / Memory-Eintrag gegen den
**aktuellen Code** pruefen:
- gezielter `grep` nach dem genannten Identifier,
- 30-Zeilen-Read auf die genannte Stelle,
- `ls` auf den genannten Pfad.

**Stimmt der Befund nicht**: entweder den Auftraggeber informieren oder
das tatsaechliche Optimierungspotenzial neu identifizieren — **nicht
blind „nachreichen"**.

**Warum.**
- Erinnerungen koennen veralten. „Das war mal so" reicht nicht.
- Analyse-Reports sind hilfreich aber nicht autoritativ. Ein falscher
  Befund umgesetzt kostet Zeit und kann funktionierenden Code
  verschlimm-bessern.
- Beispiel aus Chimera 2026-04-27: Performance-Analyse nannte
  „pressureSolver hat keinen Early-Break" — ein Blick in die Datei
  zeigte, dass `if (maxDelta < SOLVER_EPSILON) break;` **bereits**
  drinstand. Echter Hebel war epsilon-Kalibrierung (1e-6 → 1e-4); das
  wurde im Commit-Body offen so dokumentiert, statt einen nicht
  existierenden Bug zu fixen.

**How.**
1. Memory / Audit zitiert einen Pfad / Funktion / Variable.
2. `grep -n` oder `Read` (max 30–50 Zeilen) auf die Stelle.
3. Was steht **tatsaechlich** da?
   - Stimmt mit Memory ueberein → handeln.
   - Stimmt nicht → Memory aktualisieren ODER ablehnen, dem Auftraggeber
     mitteilen, was wirklich der Fall ist.
4. Bei systematischer Drift: Memory loeschen / korrigieren.

**Beispiel.**
```
Memory: "useHeatSimulation lebt in src/run/useHeatSimulation.ts"
→ ls src/run/useHeatSimulation.ts
   → File existiert. OK, handeln.

Memory: "Funktion X nimmt Argument Y mit Default Z"
→ grep -n "function X" src/...
   → Default ist W, nicht Z. Memory veraltert.
   → Memory korrigieren oder loeschen, bevor naechste Handlung.
```

**Anti-Pattern.**
- Blind auf „Sub-Agent sagt X" handeln, X-Stelle gar nicht aufgemacht.
- Memory aus 6 Monaten als „das stimmt schon" akzeptieren — die
  betroffene Datei wurde wahrscheinlich umstrukturiert.
- Falschen Befund implementieren, weil „der Bericht sah ueberzeugend
  aus".

**Verwandt.**
- [09-scope-vor-impl.md](09-scope-vor-impl.md) (Verifikation als Teil
  der Scope-Klaerung).
- Projektwissen lebt seit 2026-08-07 direkt im Repo (Skills / GDP /
  [docs/backlog.md](../backlog.md); Plan
  [2026-08-07-agent-wissen-ins-repository.md](../plans/archive/2026-08-07-agent-wissen-ins-repository.md)).
  Die lokale Agent-Memory haelt nur noch Maschinen-Lokales — auch fuer
  deren Eintraege gilt diese Verifikations-Regel unveraendert.
