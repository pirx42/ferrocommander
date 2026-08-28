# Skill: Tests begleiten Features und Fixes — pro Commit

**Wann.** Du schreibst einen Feature-Commit oder einen Bug-Fix-Commit.

**Regel.**
- **Feature-Commit:** bringt Tests fuer die neue Logik mit.
- **Bug-Fix-Commit:** bringt einen Regressions-Test mit, der **ohne den
  Fix** fehlschlaegt.

**Warum.**
- Ohne Tests weisst du nur „funktioniert gerade", nicht „bleibt korrekt".
- Bug-Regressions-Tests **dokumentieren**, was das gemeldete Problem war
  — der Test-Name + die Assertion ersetzen einen langen Kommentar.
- Tests gehoeren zum gleichen logischen Schritt — getrennt zu commiten
  fragmentiert die Geschichte.

**How.**
- Bug-Fix-Reihenfolge: zuerst Regressions-Test schreiben → faellt rot →
  Fix → Test gruen → committen.
- Feature: zumindest einen Test pro neuem Pfad (Happy-Path + 1 Edge-Case).
- Test im selben Commit wie der Code. Nicht „test commit ich gleich
  nach".

**Beispiel — Bug-Fix-Pattern.**
```ts
// FIRST: regression test that FAILS without the fix
it("section transition with sticky-fulfilled-condition does not reset heldMs", () => {
  const evaluator = makeEvaluator(...);
  const r1 = evaluator.tick(state1);
  expect(r1.status).toBe("fulfilled");
  const r2 = evaluator.tick(state2WithBriefMismatch);
  expect(r2.status).toBe("fulfilled"); // bug: was "ongoing"
});

// THEN: fix + commit beide zusammen
```

**Beispiel — Feature-Pattern.**
```ts
// Neuer Effect: zoneCompression. Im selben Commit:
it("zoneCompression reduces zone bbox by config factor", () => { ... });
it("zoneCompression with factor=1 is no-op", () => { ... });
it("zoneCompression aggregated with other ambient offsets", () => { ... });
```

**Anti-Pattern.**
- „commit-now-test-later" — Tests bleiben oft liegen, Doku-Schuld
  akkumuliert.
- Test schreiben, der so generisch ist, dass er ohne den Bug-Fix auch
  gruen waere — kein echter Schutz.

**Verwandt.**
- [24-tests-nicht-stumm-aendern.md](24-tests-nicht-stumm-aendern.md)
- [25-gruene-suite-vor-commit.md](25-gruene-suite-vor-commit.md)
- [27-coverage-luecken-triage.md](27-coverage-luecken-triage.md)
