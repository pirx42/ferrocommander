# Skill: Scope vor Implementierung festzurren

**Wann.** Vor der ersten Codezeile eines neuen Tasks.

**Regel.** Halte schriftlich fest (Plan-Datei, TaskList, oder kurze
Bestaetigung an den Auftraggeber), **was zu der Aufgabe gehoert**.
Bei aufkommender Scope-Erweiterung waehrend der Umsetzung („das sollten
wir auch gleich saeubern"): **kein Ja** vor Rueckfrage.

**Warum.** Scope-Creep erzeugt grosse, schwer reviewbare Diffs,
verwaessert den Fix mit unverwandten Aenderungen und macht `git blame`
nutzlos. Ein klar abgegrenzter Scope ist das Fundament fuer einen
sauberen Commit.

**How.**
1. Vor der ersten Codezeile: Scope in einem Satz formulieren — was IST
   drin, was NICHT.
2. Beim Code-Lesen entdeckte Nebenbaustellen separat notieren (Issue,
   TaskList, future-improvements.md), nicht mit-anfassen.
3. Wenn ein Refactor sich aufdraengt, weil der Fix sonst nicht sauber
   geht: kurz nachfragen („sauberer Fix braucht einen Mini-Refactor in X,
   ok oder separat?").

**Beispiel.**
```
Task: „Fix Bug — Skip-Button zeigt falsche Anzahl Penalties".

In-Scope: SkipButton-Komponente + Penalty-Berechnung in usePenalties.
Out-of-Scope: das unuebersichtliche TaskHUD daneben (separater Task).
```

**Anti-Pattern.**
- „Wenn ich schon dabei bin"-Refactor in einem Bug-Fix-Commit.
- Zwei unabhaengige Bugs im selben Commit, „weil ich beide gerade gesehen
  habe".

**Verwandt.**
- [11-mehrphasen-commits.md](11-mehrphasen-commits.md)
- [32-commit-pro-schritt.md](32-commit-pro-schritt.md)
- [10-plan-lifecycle.md](10-plan-lifecycle.md)
