# Skill: Bestaetigung fuer destruktive / shared-state Aktionen einholen

**Wann.** Vor einer Aktion mit **nicht-lokaler Wirkung**:
- `git push` **von Produktions-Code** (`src/`-Runtime, Server, Build-Config),
  `git push --force`, Branch loeschen
- DB-Migration ausfuehren, Schema-Bruch
- Dependency-Downgrade / Major-Bump
- Production-Service neu starten
- Nachrichten an Dritte (Slack, Mail, externe APIs mit Kosten)
- Sensitives ins Repo committen

**Regel.** Explizite Zustimmung des Auftraggebers fuer den **konkreten
Scope** einholen — auch wenn CLAUDE.md generelle Freigabe gibt. Einmalige
Zustimmung gilt nicht fuer den naechsten Fall.

**Warum.** Ein versehentliches `git push --force` zerstoert Arbeit anderer.
Ein Restart im falschen Moment wuergt User mitten im Level ab. Die Kosten
fuer Nachfragen sind klein, die Kosten fuer Fehler oft gross.

**How.**
- Aktion + Ziel + Wirkung in einem Satz benennen.
- Frage stellen, dann warten.
- Auch bei genereller Freigabe (z.B. „du darfst lokal commits machen") gilt
  der separate Schritt fuer Operationen mit Aussenwirkung.

**Beispiel.**
```
Bereit zu `git push origin main` (Commits abc1234 + def5678). OK?

Vor Restart von chimera-Service: laufen gerade noch User in Sessions?
Wenn unsicher, lieber kurz warten.
```

**Ausnahme.** Wenn der User eine dauerhafte Freigabe explizit verankert hat
(„du darfst diese Aktion ohne Rueckfrage durchziehen"):
- Lokale Git-Ops + Commits unter /home/pirx/projects ohne Rueckfrage
  (Grant `feedback_git_autonomy`, heute in
  [../workspace/agent-arbeitsregeln.md](../workspace/agent-arbeitsregeln.md)).
- **Topic-Branch-Pushes sind frei** (loesen weder CI noch Deploy aus) —
  siehe `64-branch-workflow-dev-topic-main.md` (nur Chimera).

**Historie (abgeloest):** die fruehere Ausnahme „`git push` von reinen
Test-/Doc-Aenderungen ohne Rueckfrage" (User-Spec msg 11745, 2026-06-09) gilt
seit dem Branch-Workflow 2026-07-18 (Owner msg 15495/15497) NICHT mehr —
heute braucht JEDER dev-/main-Push eine Freigabe (der Abnahme-Merge eines
Plans zaehlt als Freigabe). `git push --force` IMMER fragen.

**Anti-Pattern.**
- "Ich hab schon gepusht — passte das?".
- Ein einmaliges „push ok" beim naechsten Push wieder anwenden, ohne neue
  Zustimmung.

**Verwandt.**
- [34-kein-force-push-shared.md](34-kein-force-push-shared.md)
- `37-restart-verifizieren.md` (nur Chimera)
- `64-branch-workflow-dev-topic-main.md` (nur Chimera)
