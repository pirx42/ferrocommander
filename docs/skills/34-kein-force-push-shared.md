# Skill: Kein `--force`-Push auf Shared Branches

**Wann.** Du ueberlegst `git push --force` oder `git push --force-with-lease`
auf einen Branch, der von mehreren genutzt wird.

**Regel.** `git push --force` auf `main` oder andere Shared Branches ist
**verboten** — auch wenn der eigene Branch „nur noch schoener" waere.
Memory `feedback_git_autonomy` schliesst force-push explizit aus dem
Auto-Pass aus.

**Eigene Feature-Branches:** OK, wenn niemand mitarbeitet.

**Warum.** Force-Push zerstoert Arbeit anderer — Commits anderer
verschwinden ohne Vorwarnung, lokale Clones haengen in unreachable-Refs,
PR-Review-Kontext geht verloren. Einmal gemacht, dauerhaft schmerzhaft.

**How.**
1. Wenn du dachtest, du brauchst force-push: ueberleg, **warum**.
   - Commit unsauber → besserer Weg: neuer Commit (z.B. „revert" oder
     fix-up), kein force.
   - Pushed Secret → siehe [38-secrets-nicht-ins-repo.md](38-secrets-nicht-ins-repo.md)
     und Secret rotieren; force-push macht das Secret nicht weg (history
     bleibt in clones).
   - Rebase auf main → wenn Branch shared ist: keinen force-push, normalen
     Merge nutzen.
2. Wenn wirklich gewollt: **explizite User-Zustimmung** einholen mit
   klarer Begruendung.

**Anti-Pattern.**
- „Ich force-push schnell, war eh nur ich" — und dann war es nicht „nur
  ich".
- `git push --force-with-lease` als „sicherer" Force → ist sicherer,
  aber loescht trotzdem Commits, wenn der Lease passt.
- Force-Push als Default fuer rebase-after-pull.

**Verwandt.**
- [03-bestaetigung-destruktiv.md](03-bestaetigung-destruktiv.md)
- [33-hooks-nicht-skip.md](33-hooks-nicht-skip.md)
- Memory `feedback_git_autonomy`.
