# Skill: Mehrphasige Aenderungen in separate Commits

**Wann.** Refactor / Feature ist nicht-trivial und beruehrt mehrere
Subsysteme oder mehrere Logik-Schritte.

**Regel.** In Phasen teilen — **jede Phase ein eigener Commit**, getestet
+ build-green einzeln. Keine Batch-Commits mit mehreren unabhaengigen
Aenderungen.

**Warum.**
- **Revert-Granularitaet:** ein bisect findet den Fehler leichter in
  10 kleinen Commits als in 1 grossen.
- **Review-Lesbarkeit:** kleine Commits sind reviewbar; grosse werden
  ueberflogen.
- **Disziplin:** getrennte Phasen zwingen dich zu ueberlegen, ob die
  Zwischenschritte wirklich lauffaehig sind.

**How.**
1. Vor dem Refactor: Phasenliste skizzieren — was muss vor was?
2. Phase N: nur Code fuer N. Tests/Build/Doku mit. Commit.
3. Phase N+1 baut auf N auf.
4. Wenn Phase N nicht alleine gruen wird → Phase falsch geschnitten,
   neu schneiden.

**Beispiel.** Refactor „Data-Model X abloesen":
- Phase 1: neue Typen + Loader + JSON-Migration. → Commit
- Phase 2: Runtime umstellen, alte Typen entfernt. → Commit
- Phase 3: UI nachziehen. → Commit
Jede Phase fuer sich gruen.

Konkret Chimera (aus Damping-Generalisierung 2026-05-28):
```
Damping D1: Konstante + OR-Aggregation + Helper            → Commit
Damping D2: Phase C damped + Clamp ausziehen               → Commit
Damping D3: Tests aktualisieren                            → Commit
Damping D4: Doku + Build + Deploy + Commit + Push          → Commit
Count C1:   invertHeat:bool → invertCount:number           → Commit
Count C2:   Phase C N-fache Anwendung                      → Commit
Count C3:   Tests + Doku + Deploy                          → Commit
```

**Anti-Pattern.**
- 800-Zeilen-Commit „refactor: data-model + runtime + UI auf einmal".
- Phase 1 committen, Phase 2 ohne Test gruen, „pack ich beim Push noch
  rein".

**Verwandt.**
- [32-commit-pro-schritt.md](32-commit-pro-schritt.md)
- [25-gruene-suite-vor-commit.md](25-gruene-suite-vor-commit.md)
- [04-multi-phasen-autonomie.md](04-multi-phasen-autonomie.md)
