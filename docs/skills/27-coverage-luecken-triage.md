# Skill: Coverage-Luecken-Triage — erreichbar / defensiv / nicht-aktiviert

**Wann.** Coverage-Report zeigt unbedeckte Branches. Du ueberlegst, wie du
sie zumachst.

**Regel — drei Faelle.**

**(a) Erreichbar ueber API-Aufruf** → **Test schreiben**.

**(b) Defensiv-Guard gegen Faelle, die strukturelle Invarianten ausschliessen**
(clamped Index-Ranges, geschlossene discriminated Unions, etc.) →
**entweder entfernen** oder **im Commit-Body explizit dokumentieren**
(„defensive Guards, unerreichbar ueber aktuelle API-Aufrufe").

**(c) Hook-Code fuer noch-nicht-eingebautes Feature**
(z.B. Diode-Modifier mit `zeroDemand: true` — kein Item setzt das Flag,
aber resolveTree-Pfade existieren) → via `vi.mock(... importOriginal())`
einen **synthetischen Item-Def** einfuegen, der die Pfade aktiviert; das
macht aus dem Hook-Code testbare Spec-Doku, ohne ihn zu loeschen.

**Warum.** 100 % Branch-Coverage fuer dead code erzeugt entweder
verschachtelte Mock-Setups (die das Test-Bild verzerren) oder treibt zum
Streichen von Schutz-Code (der dann nach einem unbeobachteten Refactor
fehlt). Klare Trennung:
- erreichbarer Pfad → Test,
- dead/defensive Pfad → Entscheidung im Commit (loeschen oder
  dokumentieren), kein Test-Theater.

**How.**
1. Pro unbedeckter Branch: ist sie ueber die oeffentliche API aufrufbar?
2. Wenn ja → Test (Fall a).
3. Wenn nein, aber durch klares Invariant ausgeschlossen → loeschen
   oder mit Commit-Body-Vermerk drinlassen (Fall b).
4. Wenn der Pfad einen geplanten Hook fuer kuenftiges Feature darstellt
   → mit `vi.mock(... importOriginal())` einen synthetischen Trigger
   einfuegen (Fall c). Das ist Spec-Doku, nicht Test-Theater.

**Beispiel — Chimera live.**
`heatPhysics`-Coverage 91.78 % → 97.26 % per gezielten Tests.
Die letzten 4 unbedeckten Branches:
- `paintZone`-Grid-OOB
- heater event-kind-filter
- cable `totalHeat<=0`

Alle sind defensive Guards, die strukturelle API-Invarianten doppeln
(`worldRectToCellRange` clamped schon, etc.) — explizit im Commit-Body
genannt, nicht ueber synthetische Mocks „abgedeckt".

**Anti-Pattern.**
- Synthetischen Mock einbauen, nur um eine defensive Guard-Branch zu
  erreichen — die Branch ist strukturell unerreichbar; der Mock luegt.
- Defensive Guard loeschen, weil „Coverage 100 %", und nach Refactor X
  knallt es.
- 90% Coverage als „gut genug" akzeptieren, ohne zu wissen, welche 10 %
  fehlen.

**Verwandt.**
- [19-keine-defensive-prog.md](19-keine-defensive-prog.md)
- [23-tests-pro-commit.md](23-tests-pro-commit.md)
- [26-verhaltens-tests.md](26-verhaltens-tests.md)
