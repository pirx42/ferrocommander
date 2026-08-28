# Skill: Conventional-Commit-Format

**Wann.** Du formulierst eine Commit-Subject-Line.

**Regel.** Subject-Line beginnt mit `feat|fix|refactor|docs|test|chore|perf`
gefolgt von optionalem Scope und Doppelpunkt. Konsistent im gesamten Repo.

**Format.**
```
<type>(<scope>): <kurze beschreibung>

<optional body>

<optional footer>
```

**Typen.**
- `feat` — neues Feature / neuer User-sichtbarer Code-Pfad.
- `fix` — Bug-Fix.
- `refactor` — interne Umstrukturierung ohne Verhaltensaenderung.
- `docs` — nur Dokumentation.
- `test` — nur Tests (Nachzug / Coverage-Schliessung).
- `chore` — Build/CI/Deps/Tooling.
- `perf` — Performance-Optimierung (Verhalten gleich, schneller).

**Scope.** Subsystem / Modul / Datei (`tasks`, `runs/pruefstand`, `hud`,
`heatPhysics`). Optional, aber empfohlen.

**Body.** Optional bei trivialen Commits, **empfohlen ab 10+ Zeilen Diff**.

**Footer.** `Co-Authored-By: ...`, `Closes #...`, etc.

**Warum.** Macht Changelogs, Filter und bisect trivial. Tools wie
semantic-release hebeln darauf. `git log --grep="^feat:"` zeigt alle
Features einer Periode.

**Beispiele.**
```
feat(tasks): Section-SuccessCondition ersetzt per-Task holdMs
fix(runs/pruefstand): kein Start-Clause mehr bei t=0 erfuellt
refactor(hud): Schaltplan-Darstellung fuer Sektor-Condition
docs: dev-analysis.md — Vorgehen fuer Commit-Statistik
perf(propertyBag): ResolveCache via WeakMap, ~40% schneller
test(heatPhysics): Coverage 91.78% → 97.26% (paintZone, heater, cable)
chore: bump vite 4.5 → 4.5.1 (security advisory)
```

**Anti-Pattern.**
- „WIP", „update", „fix stuff" — gibt keinen Hinweis.
- `feat` fuer Bug-Fix, `fix` fuer neues Feature — vergiftet die Filter.
- Subject-Line > 72 Zeichen — bricht in vielen Tools.

**Verwandt.**
- [32-commit-pro-schritt.md](32-commit-pro-schritt.md)
- [33-hooks-nicht-skip.md](33-hooks-nicht-skip.md)
