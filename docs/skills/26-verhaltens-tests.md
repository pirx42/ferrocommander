# Skill: Verhaltens-Tests vor Struktur-Tests

**Wann.** Du entwirfst gerade einen neuen Test.

**Regel.** Tests pruefen **Beobachtbarkeit** — was der Code tut, nicht
wie er intern aufgebaut ist. Kein Test, der einen Dateipfad oder eine
interne Klassen-Hierarchie asserten muss, um zu ueberleben.

**Warum.** Struktur-Tests brechen bei jedem harmlos aussehenden Refactor.
Verhaltens-Tests ueberleben Refactors und schuetzen echte Invarianten.
Ein Test, der „Datei X existiert in Pfad Y" prueft, sagt nichts ueber
Korrektheit — er sagt nur, dass die Datei am Ort liegt.

**How.**
- **Test fragt:** „Wenn ich diesen Input gebe und diese Aktion ausloese,
  bekomme ich diesen Output / dieses sichtbare Verhalten?"
- **NICHT:** „Greift dieser Caller intern auf diese Methode mit dieser
  Signatur zu?"
- API-Aufrufe lieber an oeffentlichen Schnittstellen pruefen
  (Reducer-Output, Render-Output, Side-Effect-Beobachtung) als an
  internen Helpers.
- Mocks sparsam — wenn der Test ohne Mocks gut formuliert werden kann,
  ist er meist auch besser.

**Beispiel.**
```ts
// JA — Verhalten
it("advanceToNextSection setzt sectionIndex +1 und sectionStartMs", () => {
  const state = { sectionIndex: 0, sectionStartMs: 0 };
  const next = advanceToNextSection(state, runDef, 5000);
  expect(next.sectionIndex).toBe(1);
  expect(next.sectionStartMs).toBe(5000);
});

// NEIN — Struktur
it("advanceToNextSection ruft helper nextIndex() auf", () => {
  const spy = jest.spyOn(internals, "nextIndex");
  advanceToNextSection(...);
  expect(spy).toHaveBeenCalled();
});

// NEIN — Datei-Layout
it("section reducer wohnt in src/run/sectionReducer.ts", () => {
  expect(fs.existsSync("src/run/sectionReducer.ts")).toBe(true);
});
```

**Sonderfall: authored Content (Boss-/Run-JSONs).** Verhalten heißt hier nicht
nur „Funktion mit konstruiertem Input", sondern **echter Content geladen + getickt
+ gegen Invarianten geprüft**: jeden Boss laden, 90 s ticken, prüfen
(kein Self-Cook, Emitter feuert + überlebt, Schilde laden) — plus ein
Parse-Vollständigkeits-Guard (jedes Event parst, kein still-rejected). Synthetik-
Unit-Tests ließen reale Content-Bugs durch (gameTimeMs-Hitze, gestrippte xp,
energyEater `zones`→`zone`); diese Suiten fingen sie. Details:
[good-development-practices.md](../good-development-practices.md) §4.7
(`bossContentSimInvariants` / `bossEventsParse`).

**Anti-Pattern.**
- Tests die bei jedem `git mv` rot werden.
- Spy-/Mock-Wuerfel, der mehr Code als die Funktion selbst hat.
- Snapshot-Tests fuer interne Datenstrukturen, die nie an die UI gehen.

**Verwandt.**
- `14-pure-reducer.md` (nur Chimera)
- [23-tests-pro-commit.md](23-tests-pro-commit.md)
- [27-coverage-luecken-triage.md](27-coverage-luecken-triage.md)
