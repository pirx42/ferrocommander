# Skill: Kommentare erklaeren das „Warum", nicht das „Was"

**Wann.** Du tippst gerade einen Kommentar in den Code.

**Regel.** Kein Kommentar, der die Code-Zeile paraphrasiert. Kommentare
**nur** fuer:
- verborgene Invarianten
- Workarounds
- Design-Entscheidungen, die dem Leser ohne Kontext nicht klar waeren
- Hinweise auf externe Issues / Bugs

**Warum.** Der Code sagt was er tut; der Kommentar soll sagen, warum.
„Increments counter" ist Laerm; „Retry-Zaehler — dient dem Jitter im
Reconnect-Flow (Issue #412)" ist Kontext.

**How.**
- Default: kein Kommentar.
- Frage dich: „Wuerde ein guter Reader diese Zeile beim ersten Lesen
  verstehen?" Wenn ja → kein Kommentar. Wenn nein → minimaler Hinweis.
- Multi-Paragraph-Docstrings vermeiden — eine Zeile reicht meist.
- Kommentare nicht als Aufgabenkommunikation („used by X", „added for Y
  flow") — das gehoert in den PR / Commit-Body und rottet sonst im Code.

**Beispiele.**
```ts
// JA — verborgene Invariante
// HM-Refs muessen vor Pass 2 gesetzt sein, sonst NaN in resolveTree.
const hm = heatManagerRef.current;

// JA — Workaround mit Begruendung
// iOS Safari: download.click() throttled die naechsten ~700ms — siehe Memory
// project_chimera_post_load_canvas_freeze.
showClipboardModal();

// NEIN — paraphrasiert
// Increment counter
counter++;

// NEIN — Aufgabenkontext, rottet
// Used by SettingsScreen after the 2026-04 refactor
function getThemeColor() { ... }
```

**Anti-Pattern.**
- Banner-Kommentare ueber jeder Funktion mit Signatur-Repetition.
- Kommentare, die genau das wiederholen, was der Variablen-Name sagt.
- „TODO"-Kommentare ohne Issue-Referenz oder Datum.

**Verwandt.**
- [30-warum-dokumentieren.md](30-warum-dokumentieren.md)
- [19-keine-defensive-prog.md](19-keine-defensive-prog.md)
