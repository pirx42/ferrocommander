# Skill: Pausen explizit ansagen

**Wann.** Arbeit hat einen Halte-Punkt:
- Warte auf User-Entscheidung
- Alle Tasks abgeschlossen, nichts in der Pipeline
- Sub-Agent laeuft im Background, Hauptthread idle
- Du beendest die aktive Bearbeitung

**Regel.** Den Halte-Punkt **explizit** melden. Nie stillschweigend
verstummen.

**Warum.** Aus Sicht des Auftraggebers ist „kein neuer Output" identisch
mit „crashed", „vergessen" oder „wartet auf mich". Er muss selbst
nachfragen, um den Unterschied zu sehen. Eine Zeile loest die
Mehrdeutigkeit.

**How.**
- Beim Pausieren: ein Satz, warum + worauf wartet wird.
- Beim Idle: „alle Tasks durch — bereit fuer naechste Runde".
- Bei Background-Sub-Agent: „Sub-Agent X laeuft, ich melde mich wenn er
  fertig ist; in der Zwischenzeit idle".

**Beispiel.**
```
Warte auf Entscheidung zu Phase 3 (Option A vs B).
Bis dahin halte ich Phasen 4-5 zurueck.

Alle 5 geplanten Phasen committed + gepusht.
Bereit fuer naechste Aufgabe.

Sub-Agent „Coverage-Audit" laeuft im Background (ETA ~10 min).
Hauptthread idle bis Ergebnis da ist.
```

**Anti-Pattern.**
- Phase 2 fertig, kein Output, User wundert sich nach 20 min „lebst du noch?".
- „fertig" ohne zu sagen womit.

**Verwandt.**
- [05-updates-bei-langen-tasks.md](05-updates-bei-langen-tasks.md)
- [04-multi-phasen-autonomie.md](04-multi-phasen-autonomie.md)
