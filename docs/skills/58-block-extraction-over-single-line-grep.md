# Skill: Catalog/config audits — block extraction instead of single-line grep

**When.** Whenever a statement is made about ALL entries of a
catalog/config file ("which items have property X?", "which modifiers have
no cost factor?") — especially as the basis for design decisions or
classifications.

**Rule.** Never audit via single-line regex (`grep '{ kind: "x", ... }'`):
entries can be MULTI-LINE (comments, nested objects) and then silently drop
out of the result. Instead use **block extraction**: cut the text from the
entry anchor (`kind: "x"`) to the NEXT anchor and search within the block —
or load the structure directly (parse the JSON, import the TS module)
instead of matching text. Also keep in mind: effects often do NOT live only
in the catalog (context effects in `propertyBag/reader.ts`, `itemHooks/`,
`energyProfiles.ts`) — a catalog audit alone is not an effects audit.

**Why.** Incident 2026-07-11 (user msg 14802): the Q3b remainder list of
the XP-level plan was determined via single-line grep — `inverterMod`
(multi-line entry with `cost: 1.5`) and the dynamic boosters
(resonator/steadfast/laststand in reader.ts) were missing. Result: a wrong
12-item list sent to the user, correction needed. The error is systematic,
not random: single-line regex + multi-line entries = silent false
negatives.

**How.**
1. Cut anchor-based: `src.find('kind: "x"')` to `src.find('kind: "', pos+1)`.
2. Or load the structure: `npx tsx -e 'import { ITEM_CATALOG } from "./src/itemCatalog"; ...'`.
3. Effects audits: check catalog + `propertyBag/reader.ts` (context blocks) +
   `itemHooks/` + `energyProfiles.ts` — ALL four.
4. Cross-check with a sample: 2-3 known entries must appear in the result,
   otherwise the extraction is broken.

**Anti-patterns.**
- `grep '{ kind: "x",.*cost'` over a file with multi-line entries.
- Sending classification tables to the user without having validated the
  extraction method against known cases.

**Related.**
- [39-verify-memory.md](39-verify-memory.md) (check the current state
  before acting — same basic stance).
- [52-test-conservation-invariants.md](52-test-conservation-invariants.md)
  (verify the guard via a cross-check).
