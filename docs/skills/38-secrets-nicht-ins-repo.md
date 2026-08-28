# Skill: Keine Secrets ins Repo + explizites git-add

**Wann.** Vor jedem `git add` / `git commit`.

**Regel — Secrets.**
- `.env`, `credentials.json`, API-Keys, Service-Account-JSONs gehoeren
  **nicht in Commits**.
- Pre-commit-Hook und `.gitignore` enforcen das.
- Im Zweifel: einmal **zuviel** fragen statt zuwenig.

**Regel — Specific files statt `-A`.**
- `git add <explizite-datei>` statt `git add -A` oder `git add .`.
- Letzteres nimmt ungewollte temporaere Files, Builds, Secrets mit.

**Warum.**
- Einmal gepushte Secrets sind **kompromittiert**, egal wie schnell man
  den Commit zurueckzieht. Rollen und Keys muessen dann rotiert werden.
- Force-push macht das Secret **nicht weg** — es bleibt in jedem Clone
  und in GitHub's Object-Store (auch nach „commit weggeloescht").
- Explizites Staging ist ein zusaetzlicher Gedanken-Schritt, der Fehler
  faengt.

**How.**
1. `git status` lesen, **bewusst** entscheiden was rein soll.
2. `git add src/foo.ts src/bar.ts docs/baz.md` — namentlich.
3. `git diff --cached` lesen vor dem Commit.
4. Wenn ein Secret aus Versehen drin landet:
   - **Vor Push:** `git restore --staged <file>` + lokal saeubern.
   - **Nach Push:** Secret als kompromittiert betrachten, **rotieren**.

**Beispiel.**
```bash
# JA — bewusst
git status
git add src/propertyBag.ts src/__tests__/inverter.test.ts docs/property-system.md
git diff --cached
git commit -m "..."

# NEIN — nimmt alles, auch .env / build-artifacts
git add -A
git commit -am "..."
```

**Anti-Pattern.**
- `git add .` aus Bequemlichkeit — und beim naechsten Commit ist das
  `.env` mit drin.
- `git add -A` in einem Repo mit untrackten **Artefakten**: 2026-07-29
  landeten so ~80 RL-Zwischenmodelle (binaer + JSONL, 7,5k Zeilen) in
  einem 5-Dateien-Doku-Commit — vor dem Push per `git reset --soft` +
  `git restore --staged models/` neu gebaut. Gilt nicht nur fuer Secrets:
  alles Untrackte ist bis zum Beweis des Gegenteils NICHT Commit-Material.
- `git mv` nach einem Edit: `git mv` staged den Datei-INHALT zum
  Zeitpunkt des Moves — davor/danach gemachte Aenderungen bleiben
  unstaged zurueck (2× passiert, 2026-07-28). Nach jedem `git mv`:
  `git status` + `git diff` auf den neuen Pfad pruefen.
- Secret im Commit-Body „dokumentiert" (z.B. „API-Key war XYZ, jetzt
  rotiert") — Body bleibt im Repo.
- Secret in einer JSON-Fixture „nur fuers Beispiel".

**Verwandt.**
- [25-gruene-suite-vor-commit.md](25-gruene-suite-vor-commit.md)
- [33-hooks-nicht-skip.md](33-hooks-nicht-skip.md)
- [03-bestaetigung-destruktiv.md](03-bestaetigung-destruktiv.md)
