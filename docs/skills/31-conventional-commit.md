# Skill: Conventional Commit Format

**When.** You are writing a commit subject line.

**Rule.** The subject line starts with `feat|fix|refactor|docs|test|chore|perf`
followed by an optional scope and a colon. Consistent across the whole repo.

**Format.**
```
<type>(<scope>): <short description>

<optional body>

<optional footer>
```

**Types.**
- `feat` — new feature / new user-visible code path.
- `fix` — bug fix.
- `refactor` — internal restructuring without behavior change.
- `docs` — documentation only.
- `test` — tests only (catch-up / coverage closing).
- `chore` — build/CI/deps/tooling.
- `perf` — performance optimization (same behavior, faster).

**Scope.** Subsystem / module / file (`tasks`, `runs/pruefstand`, `hud`,
`heatPhysics`). Optional, but recommended.

**Body.** Optional for trivial commits, **recommended from 10+ lines of diff**.

**Footer.** `Co-Authored-By: ...`, `Closes #...`, etc.

**Why.** Makes changelogs, filters, and bisect trivial. Tools like
semantic-release build on it. `git log --grep="^feat:"` shows all
features of a period.

**Examples.**
```
feat(tasks): section SuccessCondition replaces per-task holdMs
fix(runs/pruefstand): no start clause satisfied at t=0 anymore
refactor(hud): circuit-diagram rendering for sector condition
docs: dev-analysis.md — approach for commit statistics
perf(propertyBag): ResolveCache via WeakMap, ~40% faster
test(heatPhysics): coverage 91.78% → 97.26% (paintZone, heater, cable)
chore: bump vite 4.5 → 4.5.1 (security advisory)
```

**Anti-patterns.**
- "WIP", "update", "fix stuff" — gives no hint.
- `feat` for a bug fix, `fix` for a new feature — poisons the filters.
- Subject line > 72 characters — breaks in many tools.

**Related.**
- [32-commit-per-step.md](32-commit-per-step.md)
- [33-never-skip-hooks.md](33-never-skip-hooks.md)
