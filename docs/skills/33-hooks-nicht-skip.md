# Skill: Hooks nicht ueberspringen, Commits nicht amenden

**Wann.** Ein Pre-commit-Hook schlaegt fehl, oder du ueberlegst, mit
`git commit --amend` einen vorigen Commit zu aendern.

**Regel.**
- Pre-commit-Hooks **nicht ueberspringen** (`--no-verify`,
  `--no-gpg-sign`).
- Nach einem Hook-Fail einen **NEUEN Commit** machen, nicht amenden —
  der fehlerhafte Commit „existierte nie".
- Wenn der Hook schlaegt: den **Fehler fixen**, nicht den Hook umgehen.

**Warum.**
- Hooks sind der letzte Schutz vor Kaputtem im Repo. Umgehen unterwandert
  Team-Vertrauen.
- Amend auf bereits-gepushten Commits **zerstoert Git-Historie** anderer
  Branches.
- Nach einem Hook-Fail wurde der Commit nicht erstellt — `--amend`
  modifiziert den **vorherigen** Commit, kann also frueheres Work
  zerstoeren.

**How.**
1. Hook schlaegt fehl → Fehlermeldung lesen, Ursache identifizieren.
2. Code fixen (oder Test, oder Doku — was immer der Hook moniert).
3. `git add <files>` neu.
4. `git commit -m "..."` neu — frischer Commit, nicht amend.
5. Niemals `--no-verify` aus Bequemlichkeit.

**Ausnahme.** Wenn der User **explizit** den Hook-Skip anfordert (selten,
mit Begruendung) — dann erlaubt, aber im Commit-Body vermerken warum.

**Beispiel — Hook-Fail-Flow.**
```
$ git commit -m "feat: new thing"
pre-commit hook: tsc --noEmit failed

# FIX:
# - File X line 42 hat Type-Fehler
# - korrigiere, dann:
$ git add src/X.ts
$ git commit -m "feat: new thing"
# Erfolgreich. Kein --amend, kein --no-verify.
```

**Anti-Pattern.**
- `git commit --no-verify -m "wip, fix later"` — und „later" kommt nie.
- `git commit --amend` nach gepushtem Commit → push --force noetig →
  fremde Branches kaputt.
- Hook-Fehler aus Bequemlichkeit ignorieren — der Fehler ist real.

**Verwandt.**
- [25-gruene-suite-vor-commit.md](25-gruene-suite-vor-commit.md)
- [34-kein-force-push-shared.md](34-kein-force-push-shared.md)
