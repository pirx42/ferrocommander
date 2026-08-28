# Skill: Vertraue Framework-Garantien — validiere nur an Systemgrenzen

**Wann.** Du ueberlegst gerade, ob du einen Null-Check, Typ-Check oder
Try/Catch einbauen sollst.

**Regel.** **Keine defensive Programmierung** fuer Szenarien, die das
Framework / der Compiler ausschliesst. Validation **nur an Aussengrenzen**:
- User-Input
- Externe APIs
- File-Parser / JSON-Loader
- Netzwerkgrenzen

**Warum.** Ueberdefensiver Code verbirgt die tatsaechliche Geschaeftslogik
unter Null-Checks und Try/Catches. Wenn der Compiler oder das Framework
garantiert, dass X nicht passiert, verschwende keine Zeile darauf.
Defensiver Code an internen Stellen signalisiert „hier passiert etwas
unerwartetes" — der Reader sucht dann nach dem Risiko, das es nicht gibt.

**How.**
- Vertraue TypeScript-Typen. Ein Param vom Typ `string` ist kein `number`.
- Vertraue React-Garantien. `useEffect` wird nicht „zufaellig" mehrfach
  gefeuert.
- Vertraue eigene Invarianten. Wenn `bag.kind === "engine"` schon einmal
  geprueft wurde, nicht in jeder Sub-Funktion neu.
- An Systemgrenzen: explizit validieren mit klaren Error-Messages
  (Schema-Parser, JSON-Loader, URL-Parser).

**Beispiele.**
```ts
// JA — Systemgrenze: JSON-Loader validiert
function parseRun(raw: unknown): RunDef {
  if (typeof raw !== "object" || raw === null) throw new Error("not an object");
  // ...
}

// NEIN — interner Code, TS garantiert string
function formatLabel(s: string): string {
  if (typeof s !== "string") throw new Error("s not string"); // ueberfluessig
  return s.toUpperCase();
}

// JA — kein Check noetig, TS-Typ ist Garantie
function formatLabel(s: string): string {
  return s.toUpperCase();
}

// NEIN — paranoide Try/Catch
try {
  return state.items.length; // wann sollte state.items null werden?
} catch { return 0; }
```

**Anti-Pattern.**
- `if (typeof obj.foo !== "string")` in einem TS-Typ `{ foo: string }`.
- `try { obj?.method?.() }` wenn `obj` aus dem eigenen Reducer kommt.
- Fallback-Werte fuer „falls X nicht da ist", obwohl X immer da ist.

**Verwandt.**
- [20-keine-bw-compat-shims.md](20-keine-bw-compat-shims.md)
- [27-coverage-luecken-triage.md](27-coverage-luecken-triage.md) (defensive
  Guards in Coverage-Audit).
