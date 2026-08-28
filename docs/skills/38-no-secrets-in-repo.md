# Skill: No secrets in the repo + explicit git-add

**When.** Before every `git add` / `git commit`.

**Rule — Secrets.**
- `.env`, `credentials.json`, API keys, service-account JSONs do
  **not belong in commits**.
- Pre-commit hook and `.gitignore` enforce this.
- When in doubt: ask once **too often** rather than too rarely.

**Rule — Specific files instead of `-A`.**
- `git add <explicit-file>` instead of `git add -A` or `git add .`.
- The latter drags in unwanted temporary files, builds, secrets.

**Why.**
- Secrets that were pushed once are **compromised**, no matter how quickly
  you retract the commit. Roles and keys must then be rotated.
- A force-push does **not** remove the secret — it remains in every clone
  and in GitHub's object store (even after "the commit was deleted").
- Explicit staging is an additional deliberate thinking step that catches
  mistakes.

**How.**
1. Read `git status`, decide **consciously** what should go in.
2. `git add src/foo.ts src/bar.ts docs/baz.md` — by name.
3. Read `git diff --cached` before the commit.
4. If a secret lands in there by accident:
   - **Before push:** `git restore --staged <file>` + clean up locally.
   - **After push:** treat the secret as compromised, **rotate** it.

**Example.**
```bash
# YES — deliberate
git status
git add src/propertyBag.ts src/__tests__/inverter.test.ts docs/property-system.md
git diff --cached
git commit -m "..."

# NO — takes everything, including .env / build artifacts
git add -A
git commit -am "..."
```

**Anti-patterns.**
- `git add .` out of convenience — and the `.env` ends up in the next
  commit.
- `git add -A` in a repo with untracked **artifacts**: on 2026-07-29
  ~80 intermediate RL models (binary + JSONL, 7.5k lines) landed this way
  in a 5-file docs commit — rebuilt before the push via `git reset --soft` +
  `git restore --staged models/`. This applies not only to secrets:
  everything untracked is NOT commit material until proven otherwise.
- `git mv` after an edit: `git mv` stages the file CONTENT as of the
  moment of the move — changes made before/after remain unstaged
  (happened 2x, 2026-07-28). After every `git mv`:
  check `git status` + `git diff` on the new path.
- A secret "documented" in the commit body (e.g. "API key was XYZ, now
  rotated") — the body stays in the repo.
- A secret in a JSON fixture "just for the example".

**Related.**
- [25-green-suite-before-commit.md](25-green-suite-before-commit.md)
- [33-never-skip-hooks.md](33-never-skip-hooks.md)
- [03-confirm-destructive-actions.md](03-confirm-destructive-actions.md)
