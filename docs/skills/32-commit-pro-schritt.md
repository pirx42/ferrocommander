# Skill: Ein logischer Schritt = ein Commit (sofort, nicht am Ende)

**Wann.** Du arbeitest einen mehrschrittigen Task ab (Plan mit Phasen,
Refactor mit mehreren Modulen, Bug-Round mit mehreren Bugs).

**Regel.** **JEDEN abgearbeiteten Einzelschritt direkt committen** —
nicht am Ende sammeln. Ein Commit = eine gedankliche Aenderung. Zwei
unabhaengige Fixes = zwei Commits, auch wenn sie im selben Kontext
aufgefallen sind.

**Warum.**
- **Revert-Granularitaet:** ein bisect findet den Fehler leichter in
  10 kleinen Commits als in 1 grossen.
- **Code-Review-Lesbarkeit:** kleine Commits sind reviewbar.
- **Klarer Bisect.** Bei Sammel-Commit haengst du in der Bisection 200
  Zeilen ab.
- **Push-Disziplin:** [04-multi-phasen-autonomie.md](04-multi-phasen-autonomie.md)
  pusht nach jeder Phase — das braucht einen separaten Commit pro Phase.
- Memory `feedback_commit_per_step`: explizite User-Anweisung.

**How.**
1. Nach jedem fertigen Sub-Schritt: `tsc + tests + build` gruen.
2. **Sofort commit + push.** Nicht „pack ich noch zum naechsten".
3. Naechsten Schritt anfangen.

**Beispiel — Damping-Plan 2026-05-28.**
```
Damping D1: Konstante + OR-Aggregation + Helper                → Commit
Damping D2: Phase C damped + Clamp ausziehen                   → Commit
Damping D3: Tests aktualisieren                                → Commit
Damping D4: Doku + Build + Deploy + Commit + Push              → Commit
Count C1:   invertHeat:bool → invertCount:number               → Commit
Count C2:   Phase C N-fache Anwendung                          → Commit
Count C3:   Tests + Doku + Deploy                              → Commit
```

Jeder Commit war fuer sich gruen und reviewbar — nicht ein
„big-damping-refactor"-Sammel-Commit.

**Anti-Pattern.**
- 7 Schritte abarbeiten, am Ende einen riesigen `feat: damping +
  count + tests + doku` Commit.
- „Quick fix dazu" in einem unverwandten Commit mit-rein.
- Mehrere Bugs in einem Bug-Round-Commit, der dann „4 bugs" heisst.

**Verwandt.**
- [11-mehrphasen-commits.md](11-mehrphasen-commits.md) (Phasen-Planung).
- [04-multi-phasen-autonomie.md](04-multi-phasen-autonomie.md)
- [31-conventional-commit.md](31-conventional-commit.md)
- Memory `feedback_commit_per_step`.
