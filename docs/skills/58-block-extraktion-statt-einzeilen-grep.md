# Skill: Katalog-/Config-Audits — Block-Extraktion statt Einzeilen-Grep

**Wann.** Immer wenn eine Aussage ueber ALLE Eintraege einer
Katalog-/Config-Datei getroffen wird ("welche Items haben Property X?",
"welche Modifier haben keinen cost-Faktor?") — besonders als Grundlage
fuer Design-Entscheidungen oder Klassifikationen.

**Regel.** Nie per Einzeilen-Regex (`grep '{ kind: "x", ... }'`) auditieren:
Eintraege koennen MEHRZEILIG sein (Kommentare, verschachtelte Objekte) und
fallen dann still aus dem Ergebnis. Stattdessen **Block-Extraktion**: den
Text vom Entry-Anker (`kind: "x"`) bis zum NAECHSTEN Anker schneiden und im
Block suchen — oder gleich die Struktur laden (JSON parsen, TS-Modul
importieren) statt Text zu matchen. Zusaetzlich beachten: Wirkungen leben
oft NICHT nur im Katalog (Kontext-Effekte in `propertyBag/reader.ts`,
`itemHooks/`, `energyProfiles.ts`) — ein Katalog-Audit allein ist kein
Wirkungs-Audit.

**Warum.** Vorfall 2026-07-11 (User msg 14802): die Q3b-Restliste des
XP-Level-Plans wurde per Einzeilen-Grep bestimmt — `inverterMod` (mehrzeiliger
Entry mit `cost: 1.5`) und die dynamischen Booster (resonator/steadfast/
laststand in reader.ts) fehlten. Ergebnis: falsche 12er-Liste an den User,
Korrektur noetig. Der Fehler ist systematisch, nicht zufaellig: Einzeilen-
Regex + mehrzeilige Entries = stille False-Negatives.

**How.**
1. Anker-basiert schneiden: `src.find('kind: "x"')` bis `src.find('kind: "', pos+1)`.
2. Oder Struktur laden: `npx tsx -e 'import { ITEM_CATALOG } from "./src/itemCatalog"; ...'`.
3. Wirkungs-Audits: Katalog + `propertyBag/reader.ts` (Kontext-Bloecke) +
   `itemHooks/` + `energyProfiles.ts` ALLE vier pruefen.
4. Stichprobe gegenpruefen: 2-3 bekannte Eintraege muessen im Ergebnis
   auftauchen, sonst ist die Extraktion kaputt.

**Anti-Pattern.**
- `grep '{ kind: "x",.*cost'` ueber eine Datei mit mehrzeiligen Entries.
- Klassifikations-Tabellen an den User schicken, ohne die Extraktions-
  methode gegen bekannte Faelle validiert zu haben.

**Verwandt.**
- [39-memory-verifizieren.md](39-memory-verifizieren.md) (Stand pruefen
  bevor gehandelt wird — gleiche Grundhaltung).
- [52-erhaltungs-invarianten-testen.md](52-erhaltungs-invarianten-testen.md)
  (Wachter durch Gegenprobe verifizieren).
