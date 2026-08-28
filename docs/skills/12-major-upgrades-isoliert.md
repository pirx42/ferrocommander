# Skill: Major-Dependency-Upgrades isoliert halten

**Wann.** Eine Dependency springt eine **Major-Version** (TS 5→6, React 18→19,
Vite 4→5, ...).

**Regel.** Jeder Major-Bump ist sein **eigener Task** mit eigenem
Regressions-Fenster. **Nicht** im Rahmen eines anderen Tasks oder im
`npm update`-Sweep durchschieben.

**Warum.** Major-Bumps bringen Breaking Changes, die Debugging-Zeit
kosten. Mit anderem Task vermischt wird der Fehler unauffindbar — du
weisst nicht, ob es deine Aenderung oder der Bump war.

**How.**
1. Major-Bump als eigenen Plan/Commit ansetzen.
2. Vorher: `tsc --noEmit && npm test && npm run build` — Baseline gruen.
3. Bump in einer eigenen Iteration, danach das volle Set wieder gruen
   fahren (oft mit Breaking-Change-Fixes als Folge-Commits in derselben
   Iteration).
4. Erst danach normale Feature-/Fix-Arbeit weiter.

**Beispiel.**
```
Tag 1: Feature X (kein Bump).
Tag 2 vormittags: TypeScript 5.4 → 5.6 — eigener Plan/Commit, ggf.
  Fix-Folgecommits fuer neue strict-Regeln.
Tag 2 nachmittags: Feature Y.
```

**Anti-Pattern.**
- Im selben Commit „fix bug + bump typescript".
- `npm update` mit allen drin, dann „komisch, irgendwas crasht".

**Verwandt.**
- [11-mehrphasen-commits.md](11-mehrphasen-commits.md)
- [09-scope-vor-impl.md](09-scope-vor-impl.md)
