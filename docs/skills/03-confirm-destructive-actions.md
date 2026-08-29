# Skill: Obtain confirmation for destructive / shared-state actions

**When.** Before an action with **non-local effect**:
- `git push` **of production code** (`src/` runtime, server, build config),
  `git push --force`, deleting a branch
- running a DB migration, schema break
- dependency downgrade / major bump
- restarting a production service
- messages to third parties (Slack, mail, external APIs with costs)
- committing sensitive material to the repo

**Rule.** Obtain the owner's explicit consent for the **concrete
scope** — even if CLAUDE.md grants blanket approval. One-time
consent does not apply to the next case.

**Why.** An accidental `git push --force` destroys other people's work.
A restart at the wrong moment cuts users off mid-level. The cost
of asking is small, the cost of mistakes often large.

**How.**
- Name action + target + effect in one sentence.
- Ask the question, then wait.
- Even with a blanket grant (e.g. "you may make local commits"), the
  separate step still applies for operations with external effect.

**Example.**
```
Ready to `git push origin main` (commits abc1234 + def5678). OK?

Before restarting the chimera service: are users still in active sessions?
If unsure, better to wait briefly.
```

**Exception.** If the user has explicitly anchored a standing grant
("you may perform this action without asking back"):
- Local git ops + commits under /home/pirx/projects without asking back
  (grant `feedback_git_autonomy`, today in
  `workspace/agent-arbeitsregeln.md` in the Chimera project, where this skill
  came from — not a file in this repository).
- **Topic-branch pushes are free** (they trigger neither CI nor deploy) —
  see `64-branch-workflow-dev-topic-main.md` (Chimera only).

**History (superseded):** the earlier exception "`git push` of pure
test/doc changes without asking back" (user spec msg 11745, 2026-06-09) has
NOT applied since the branch workflow of 2026-07-18 (owner msg 15495/15497) —
today EVERY dev/main push requires approval (the acceptance merge of a
plan counts as approval). `git push --force` ALWAYS requires asking.

**Anti-patterns.**
- "I already pushed — was that okay?".
- Reapplying a one-time "push ok" to the next push without fresh
  consent.

**Related.**
- [34-no-force-push-shared.md](34-no-force-push-shared.md)
- `37-restart-verifizieren.md` (Chimera only)
- `64-branch-workflow-dev-topic-main.md` (Chimera only)
