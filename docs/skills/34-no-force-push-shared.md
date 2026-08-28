# Skill: No `--force` Push on Shared Branches

**When.** You are considering `git push --force` or `git push --force-with-lease`
on a branch used by multiple people.

**Rule.** `git push --force` on `main` or other shared branches is
**forbidden** — even if your own branch would just be "a bit prettier".
Memory `feedback_git_autonomy` explicitly excludes force-push from the
auto-pass.

**Own feature branches:** OK, if nobody else is collaborating.

**Why.** A force-push destroys other people's work — other people's
commits vanish without warning, local clones hang on unreachable refs,
PR review context is lost. Done once, painful forever.

**How.**
1. If you thought you needed a force-push: consider **why**.
   - Commit messy → better way: new commit (e.g. "revert" or
     fix-up), no force.
   - Pushed secret → see [38-no-secrets-in-repo.md](38-no-secrets-in-repo.md)
     and rotate the secret; a force-push doesn't remove the secret
     (history remains in clones).
   - Rebase onto main → if the branch is shared: no force-push, use a
     normal merge.
2. If really intended: obtain **explicit user consent** with a
   clear rationale.

**Anti-patterns.**
- "I'll quickly force-push, it was only me anyway" — and then it wasn't
  "only me".
- `git push --force-with-lease` as the "safer" force → it is safer,
  but still deletes commits when the lease matches.
- Force-push as the default for rebase-after-pull.

**Related.**
- [03-confirm-destructive-actions.md](03-confirm-destructive-actions.md)
- [33-never-skip-hooks.md](33-never-skip-hooks.md)
- Memory `feedback_git_autonomy`.
