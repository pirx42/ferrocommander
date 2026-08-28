# Skill: More correct variant instead of quick-and-dirty

**When.** A refactor/implementation decision offers two
variants:
- (a) does it **substantially right** — new fields are properly
  integrated into the responsible pipeline, all rules apply, tests
  cover the paths uniformly.
- (b) **scaffolding minimum** — special-casing at the caller, workaround
  at the edge, "clean variant later".

**Rule.** **(a) is the default.** Choose the substantial path, provided
effort and risk are acceptable.

**Quick fixes require explicit user approval** (user spec msg 10982,
2026-05-31, after a pulseMod display-layer quick fix that triggered a
corrective bag-level iteration). If (b) really seems justified,
that is a decision that belongs back with the requester —
not one the assistant makes silently.
Concretely: before every quick-fix commit, explicitly ask ("a quick
fix would be possible here that bypasses X — should I, or should I
implement the proper variant?") and wait for approval.

**Why.** Scaffolding solutions have the property of staying permanently.
"Quick-and-dirty now, clean later" is rarely followed up in practice,
because the next pressure comes from a different direction.
A little more time now saves a refactor in 6 months
plus the side effects (multiple places have to be migrated,
tests changed, consumers changed). The approval requirement forces the
assistant to lay the real cost/benefit calculation open,
instead of encapsulating it away.

**When the minimum _is_ acceptable.** These exceptions are the only cases
in which (b) may be chosen without a fresh approval — a user statement
must still confirm with "do the quick fix" or similar that one of
these cases applies:
- **Spike code**, explicitly marked as throwaway and actually
  deleted again (not merged into main).
- **Hot fix under time pressure** — with an explicit follow-up story that
  is worked off within a few days.
- **Pure data migration**, where the "proper" path adds no value.

**Example.**
Property-bag extension with two variants:
- (a) Add the new field to `VALUE_KEYS`, integrate all rules
  (multipliers, inverters, clamp) in the resolver.
- (b) Special-casing in the caller, which manually computes the field.

Choose (a). (b) only if an explicit reason (e.g. the field is
structurally different) justifies it.

Concretely from Chimera: for the damping generalization, phase C was
switched directly to the count model (the substantial variant), instead
of counting per caller at each call site — that saves consumer changes
and defines the rule in one place.

**Anti-patterns.**
- Building a special-case branch at the caller with `// TODO: integrate
  into resolver` — the TODO stays for 6 months.
- Attaching a boolean flag "zeroDemand" that only a single caller
  understands, because the resolver integration would be "too much effort".
- Wrapping an adapter layer around a new API instead of migrating the
  callers to the new API.

**Related.**
- [09-scope-before-implementation.md](09-scope-before-implementation.md) (estimate scope honestly,
  then choose the right variant).
- [11-multi-phase-commits.md](11-multi-phase-commits.md) (the substantial
  variant may be multi-phase — that is OK).
- [20-no-backward-compat-shims.md](20-no-backward-compat-shims.md)
  (scaffolding leftovers are often backward-compat shims).
- Memory `feedback_prefer_correct_over_quick`.
