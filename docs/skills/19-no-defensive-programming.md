# Skill: Trust framework guarantees — validate only at system boundaries

**When.** You are wondering whether to add a null check, type check, or
try/catch.

**Rule.** **No defensive programming** for scenarios the framework /
compiler rules out. Validation **only at outer boundaries**:
- user input
- external APIs
- file parsers / JSON loaders
- network boundaries

**Why.** Overly defensive code buries the actual business logic under
null checks and try/catches. If the compiler or framework guarantees
that X cannot happen, don't waste a line on it. Defensive code in
internal places signals "something unexpected happens here" — the reader
then hunts for a risk that doesn't exist.

**How.**
- Trust TypeScript types. A param of type `string` is not a `number`.
- Trust React guarantees. `useEffect` does not fire multiple times
  "at random".
- Trust your own invariants. If `bag.kind === "engine"` has already been
  checked once, don't re-check it in every sub-function.
- At system boundaries: validate explicitly with clear error messages
  (schema parsers, JSON loaders, URL parsers).

**Example.**
```ts
// YES — system boundary: JSON loader validates
function parseRun(raw: unknown): RunDef {
  if (typeof raw !== "object" || raw === null) throw new Error("not an object");
  // ...
}

// NO — internal code, TS guarantees string
function formatLabel(s: string): string {
  if (typeof s !== "string") throw new Error("s not string"); // superfluous
  return s.toUpperCase();
}

// YES — no check needed, the TS type is the guarantee
function formatLabel(s: string): string {
  return s.toUpperCase();
}

// NO — paranoid try/catch
try {
  return state.items.length; // when would state.items ever become null?
} catch { return 0; }
```

**Anti-patterns.**
- `if (typeof obj.foo !== "string")` on a TS type `{ foo: string }`.
- `try { obj?.method?.() }` when `obj` comes from your own reducer.
- Fallback values for "in case X is missing", although X is always there.

**Related.**
- [20-no-backward-compat-shims.md](20-no-backward-compat-shims.md)
- [27-coverage-gap-triage.md](27-coverage-gap-triage.md) (defensive
  guards in the coverage audit).
