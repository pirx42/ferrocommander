# Skill: Push traegt ALLE unpushed Commits mit

**Wann.** Vor jedem `git push` — besonders vor „harmlosen" Test-/Doku-Pushes.

**Regel.** `git push` schiebt IMMER alle lokalen Commits vor HEAD — nicht
nur den letzten. Vor dem Push `git log @{u}..HEAD --oneline` pruefen: liegen
dort Commits, fuer die noch keine Push-Freigabe existiert, dann NICHT
pushen (bzw. Freigabe holen). Die Freigabe-Regel gilt fuer den gesamten
Commit-Stapel, nicht fuer den juengsten Commit allein.

**Freigabe-Regel seit Branch-Workflow (2026-07-18, msg 15497):**
- **Topic-Branches:** Push frei (kein CI/Deploy-Effekt).
- **dev:** Push NUR nach Freigabe (Abnahme-Merge oder freigegebener
  Kleinkram) — die fruehere Test/Doku-frei-Ausnahme ist damit abgeloest.
- **main:** ausschliesslich Release-Merge von dev auf explizite Ansage —
  deployt sofort zu den Testern.

**Warum.** Ein Doku-Fix-Push wuerde sonst ungenehmigte Produktiv-Commits
mitschieben — die Freigabe-Regel waere still ausgehebelt. (Memory
`feedback_push_carries_unpushed_commits`.)

**How.**
```bash
git log @{u}..HEAD --oneline   # leer = nichts unpushed; sonst Stapel pruefen
```
- Nur Test-/Doku-Commits im Stapel → direkt pushen.
- Produktiv-Commit dabei, Freigabe fehlt → fragen oder warten.
- Bei freigegebenen Mehr-Phasen-Plaenen: Phasen-Pushes sind gedeckt
  ([04-multi-phasen-autonomie.md](04-multi-phasen-autonomie.md)).

**Anti-Pattern.**
- „Ist ja nur der README-Fix" — und drei ungefragte Feature-Commits
  fahren mit.

**Verwandt.**
- [34-kein-force-push-shared.md](34-kein-force-push-shared.md)
- [32-commit-pro-schritt.md](32-commit-pro-schritt.md)
