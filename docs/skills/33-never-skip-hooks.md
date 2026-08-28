# Skill: Don't Skip Hooks, Don't Amend Commits

**When.** A pre-commit hook fails, or you are considering changing a
previous commit with `git commit --amend`.

**Rule.**
- **Don't skip** pre-commit hooks (`--no-verify`,
  `--no-gpg-sign`).
- After a hook failure, make a **NEW commit**, don't amend —
  the failed commit "never existed".
- If the hook fails: **fix the error**, don't work around the hook.

**Why.**
- Hooks are the last protection against broken things in the repo.
  Bypassing them undermines team trust.
- Amending already-pushed commits **destroys the Git history** of other
  branches.
- After a hook failure, the commit was never created — `--amend`
  modifies the **previous** commit, and can therefore destroy earlier
  work.

**How.**
1. Hook fails → read the error message, identify the cause.
2. Fix the code (or test, or docs — whatever the hook complains about).
3. `git add <files>` again.
4. `git commit -m "..."` again — a fresh commit, not amend.
5. Never `--no-verify` out of convenience.

**Exception.** If the user **explicitly** requests the hook skip (rare,
with a reason) — then allowed, but note in the commit body why.

**Example — hook-fail flow.**
```
$ git commit -m "feat: new thing"
pre-commit hook: tsc --noEmit failed

# FIX:
# - file X line 42 has a type error
# - correct it, then:
$ git add src/X.ts
$ git commit -m "feat: new thing"
# Success. No --amend, no --no-verify.
```

**Anti-patterns.**
- `git commit --no-verify -m "wip, fix later"` — and "later" never comes.
- `git commit --amend` after a pushed commit → push --force needed →
  other people's branches broken.
- Ignoring hook errors out of convenience — the error is real.

**Related.**
- [25-green-suite-before-commit.md](25-green-suite-before-commit.md)
- [34-no-force-push-shared.md](34-no-force-push-shared.md)
