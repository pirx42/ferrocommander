# Skill: Ein Topic pro Doku-Datei + klare Verlinkung

**Wann.** Du legst neue Doku an oder erwaegst, eine bestehende zu
splitten / zusammenzulegen.

**Regel.**
- `docs/`-Ordner ist nach **Themen** gegliedert (Architektur, Testing,
  Deploy, ein File pro grossem Subsystem).
- `CLAUDE.md` (root) als Index mit Kurzbeschreibung + Link pro File.
- Subdir-CLAUDE.md verlinkt Children + parent.

**Datei-Granularitaet (Faustregel).**
- Subsystem-Doku: **200–1500 Zeilen** ist gesund.
- **< 100 Zeilen:** pruefen ob Sektion in groesserer Datei besser
  aufgehoben waere.
- **> 2000 Zeilen:** pruefen ob ein eigenstaendiges Sub-Topic
  herausgeloest werden kann (Beispiel: `tasks.md` NPC-Subsystem →
  `npcs.md`).

**Linking-Konvention (verbindlich seit 2026-05-05).**
- Interne Refs auf Markdown-Files: relative `.md`-Pfade mit Anker:
  `[runs.md → Loot-Pools](../runs.md#loot-pools-boss-runjson)`.
- Plan-Refs als Markdown-Link, nicht als Code-Span:
  `[plan-name.md](plans/...)` statt `` `plans/...` ``.
- Aktive Plaene: `docs/plans/...md`. Archivierte:
  `docs/plans/archive/...md`.
- Cross-Refs **immer mit-umbiegen**, wenn ein Plan archiviert wird.

**Sprache (Chimera-spezifisch).**
- Deutsch. User-Base ist deutschsprachig, neue Docs sind durchgaengig
  deutsch. Bestehende mischsprachige Files werden beim naechsten
  substantiellen Edit eingedeutscht — kein eigener Pass noetig.

**H1-Stil (Chimera-spezifisch).**
- `# Chimera — <Topic>` oder `# <Topic>` (ohne „Chimera —") akzeptiert.

**Test-Coverage-Sektion.** Wenn vorhanden: immer `## Test-Coverage`
(nicht „Tests" oder „Test-Abdeckung"). Subsystem-Docs ohne
Test-Sektion bekommen mindestens einen Pointer am Ende
(„Tests in `src/__tests__/<file>.test.ts`").

**Warum.** Eine 3000-Zeilen-Monster-Doku liest niemand. Kleine,
topic-fokussierte Dateien werden gelesen und gepflegt.

**Anti-Pattern.**
- `notes.md` mit „alles was so anfaellt".
- 50-Zeilen-`X.md` und 50-Zeilen-`Y.md`, die thematisch identisch sind.
- Plan im aktiven Pfad belassen, nachdem er „Umgesetzt" markiert
  wurde, ohne Substanz nach `docs/` zu schieben.

**Verwandt.**
- [28-doku-im-selben-commit.md](28-doku-im-selben-commit.md)
- [10-plan-lifecycle.md](10-plan-lifecycle.md)
- [13-bestehende-dateien-bevorzugen.md](13-bestehende-dateien-bevorzugen.md)
