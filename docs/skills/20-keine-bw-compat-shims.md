# Skill: Keine Backward-Compatibility-Shims beim Refactoring

**Wann.** Du ersetzt eine interne API, eine Funktion, einen Type, eine
Datei.

**Regel.** Entferne das Alte **vollstaendig**:
- Kein `_unused` leftover.
- Kein `// removed XXX`-Kommentar.
- Kein Re-Export der alten API „fuer den Fall, dass".
- Keine renamed `_oldVar`-Parameter.

**Warum.** Dead Code ist schlimmer als kein Code: er wirkt wie lebender,
schickt Reader in falsche Richtungen, und beim naechsten Audit-Pass wird
gefragt „wo wird das benutzt?".

**How.**
- Nach dem Refactor: `grep` nach der alten Identifier-Range, alle
  Treffer entfernen.
- TypeScript-Compiler hilft: ungenutzte Imports / Exports werden meist
  geflagged.
- Wenn das Alte wirklich noch benoetigt wird → entweder gar nicht
  refactoren oder Migrations-Pfad explizit machen (siehe Ausnahme).

**Ausnahme — oeffentliche APIs / persistierte Daten.**
- Persistente Daten (Save-Format, DB-Schema) brauchen Migrationen mit
  Versions-Bump.
- Oeffentliche API mit externen Consumern braucht Deprecation-Pfade.
- **Interner Code: keine Ausnahme.**

**Beispiel.**
```ts
// NEIN — Re-Export-Shim fuer interne Module
// @deprecated use newAdvance
export { newAdvance as oldAdvance };

// NEIN — _unused-Parameter, weil „vielleicht spaeter"
function tick(_state, dt) { return doStuff(dt); }

// NEIN — Marker-Kommentar
// removed: applyOldHeat (replaced by applyHeat)
function applyHeat() { ... }

// JA — sauber entfernt, neues File, alte Aufrufer auf neuen Namen umgestellt
```

**Anti-Pattern.**
- Datei `heatPhysics.ts` ersetzt durch `heatPhysicsV2.ts`, alte bleibt
  „falls".
- Funktion umbenannt + alte Variante als Wrapper, der nur die neue ruft.

**Verwandt.**
- [13-bestehende-dateien-bevorzugen.md](13-bestehende-dateien-bevorzugen.md)
- [11-mehrphasen-commits.md](11-mehrphasen-commits.md)
