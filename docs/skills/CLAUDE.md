# docs/skills/

← Parent: [../CLAUDE.md](../CLAUDE.md)

Individual, focused skills — derived from
[../good-development-practices.md](../good-development-practices.md).
Each file is a **self-contained skill** with trigger (`When`), rule,
rationale (`Why`), application (`How`), anti-patterns, and cross-links.

One skill per file. Cross-links between related skills.

> **Origin:** adopted from the Chimera project (as of 2026-08-28) and
> translated to English. The original numbering is kept so cross-references
> stay stable — gaps in the sequence are Chimera-specific skills (React
> hooks, item catalog, RL training, server deploy, i18n mandate) that were
> not adopted. Examples mentioning npm/vitest/tsc apply analogously with
> the Cargo equivalents (fmt/clippy/test/build). References to `msg NNNNN`
> are provenance from the Chimera history. New skills for this project get
> numbers from 100 upward.

## Index

### Communication & Workflow
- [01-clarify-with-options.md](01-clarify-with-options.md) — Clarify ambiguous tasks with named options.
- [02-answer-meta-questions-directly.md](02-answer-meta-questions-directly.md) — Answer meta questions directly.
- [03-confirm-destructive-actions.md](03-confirm-destructive-actions.md) — Confirmation for destructive / shared-state actions.
- [04-multi-phase-autonomy.md](04-multi-phase-autonomy.md) — Execute multi-phase plans autonomously.
- [05-updates-during-long-tasks.md](05-updates-during-long-tasks.md) — Updates during long tasks (~20-min cadence + milestone pings).
- [06-announce-pauses.md](06-announce-pauses.md) — Announce pauses explicitly.
- [07-overnight-autonomy.md](07-overnight-autonomy.md) — Overnight/offline autonomy.
- [08-reply-on-same-channel.md](08-reply-on-same-channel.md) — Reply on the same channel.
- [51-ask-before-expensive-setup.md](51-ask-before-expensive-setup.md) — Before building expensive artifacts yourself (fixtures/setups/data), ask whether the user can prepare them faster.
- [65-verify-or-ask-never-assume.md](65-verify-or-ask-never-assume.md) — NEVER state behavior/facts from assumption: verify or ask.
- [73-proactively-suggest-better-approaches.md](73-proactively-suggest-better-approaches.md) — ALWAYS put known better approaches next to the given direction, early; the decision stays with the owner.

### Planning & Scope
- [09-scope-before-implementation.md](09-scope-before-implementation.md) — Pin down scope before implementation.
- [10-plan-lifecycle.md](10-plan-lifecycle.md) — Plan filenames + lifecycle (Draft → Archived).
- [11-multi-phase-commits.md](11-multi-phase-commits.md) — Multi-phase changes in separate commits.
- [12-major-upgrades-isolated.md](12-major-upgrades-isolated.md) — Major dependency upgrades in isolation.
- [41-more-correct-variant.md](41-more-correct-variant.md) — The more correct variant over quick-and-dirty.
- [43-coverage-before-implementation.md](43-coverage-before-implementation.md) — Plan phase 0: review existing coverage per modified function, close gaps first.
- [45-calibrate-effort-estimates.md](45-calibrate-effort-estimates.md) — Correct plan effort estimates with empirical factors.
- [49-final-phase-refactoring-audit.md](49-final-phase-refactoring-audit.md) — Last plan phase = refactoring audit of the whole implementation.

### Code Quality
- [13-prefer-existing-files.md](13-prefer-existing-files.md) — Edit existing files instead of creating new ones.
- [16-no-magic-values.md](16-no-magic-values.md) — Magic values as named constants.
- [17-centralize-constants.md](17-centralize-constants.md) — Centralize subsystem constants.
- [18-comments-explain-why.md](18-comments-explain-why.md) — Comments explain the "why".
- [19-no-defensive-programming.md](19-no-defensive-programming.md) — Trust framework/type-system guarantees.
- [20-no-backward-compat-shims.md](20-no-backward-compat-shims.md) — No backward-compat leftovers when refactoring.
- [44-no-redundancy.md](44-no-redundancy.md) — No duplicates / shared logic goes into shared helpers.
- [53-generate-instead-of-duplicating.md](53-generate-instead-of-duplicating.md) — Generate derivable lists/values from the source of truth at build time instead of hardcoding duplicates.
- [58-block-extraction-over-single-line-grep.md](58-block-extraction-over-single-line-grep.md) — Never audit catalogs/configs with single-line regexes; extract blocks or load the structure.
- [68-commit-important-scripts.md](68-commit-important-scripts.md) — Commit config-carrying scripts to the repo, not the scratchpad.
- [71-long-scripts-report-progress.md](71-long-scripts-report-progress.md) — Long-running scripts emit ongoing stage status with timestamps.

### Testing
- [23-tests-accompany-commits.md](23-tests-accompany-commits.md) — Tests accompany features and fixes.
- [24-no-silent-test-changes.md](24-no-silent-test-changes.md) — Explain + ask before changing tests.
- [25-green-suite-before-commit.md](25-green-suite-before-commit.md) — fmt + clippy + tests + build green before every commit (adapted to Cargo here).
- [26-behavioral-tests.md](26-behavioral-tests.md) — Behavioral tests over structural tests.
- [27-coverage-gap-triage.md](27-coverage-gap-triage.md) — Coverage gaps: reachable/defensive/dead.
- [52-test-conservation-invariants.md](52-test-conservation-invariants.md) — Test conservation invariants instead of value asserts (here e.g.: byte sums on copy, file counts on move, pack→unpack roundtrip).
- [59-mutation-probe-over-coverage-percent.md](59-mutation-probe-over-coverage-percent.md) — Disable the effect and see whether the suite screams — coverage measures execution, not assertion.
- [74-worst-case-inputs-for-tests-and-benchmarks.md](74-worst-case-inputs-for-tests-and-benchmarks.md) — A test or benchmark that picks its own inputs picks flattering ones; place the data where the property is contended, and re-probe a test whose trigger was rebound.

### Documentation
- [28-docs-in-same-commit.md](28-docs-in-same-commit.md) — Docs in the same commit as the code.
- [29-one-topic-per-doc.md](29-one-topic-per-doc.md) — One topic per file + cross-linking.
- [30-document-the-why.md](30-document-the-why.md) — Document the design "why".

### Commits & Git
- [31-conventional-commit.md](31-conventional-commit.md) — Conventional-commit format.
- [32-commit-per-step.md](32-commit-per-step.md) — One logical step = one commit, immediately.
- [33-never-skip-hooks.md](33-never-skip-hooks.md) — Don't skip hooks, don't amend commits.
- [34-no-force-push-shared.md](34-no-force-push-shared.md) — No force-push on shared branches.
- [57-push-carries-the-whole-stack.md](57-push-carries-the-whole-stack.md) — Before every push check `git log @{u}..HEAD`: the approval rule covers the whole commit stack.

### Safety & Verification
- [38-no-secrets-in-repo.md](38-no-secrets-in-repo.md) — No secrets, explicit git-add.
- [39-verify-memory.md](39-verify-memory.md) — Verify memory + audit reports before acting on them.
- [61-kill-background-processes-cleanly.md](61-kill-background-processes-cleanly.md) — Abort background jobs via the process GROUP + ps re-check.

### Multi-Day Rhythm
- [40-four-phase-cycle.md](40-four-phase-cycle.md) — Four-phase hardening cycle (Feature → Tests → Docs → Refactor).
- [47-architecture-audit-with-subagents.md](47-architecture-audit-with-subagents.md) — Decompose architecture audits > 10k LOC across parallel subagents.

## Not adopted (Chimera-specific)

14, 15, 60, 67 (React hooks/jsdom) · 21 (TS handler map — Rust's `match`
covers it) · 22 (Chimera i18n mandate) · 35–37, 54 (server deploy) ·
42 (Chimera housekeeping commands; concept in GDP part C.9) ·
46, 48, 50, 55, 56, 62, 63, 64, 66, 69, 70, 72 (game content, Playwright,
RL training, Chimera branch model).

## Source

Direct source: [../good-development-practices.md](../good-development-practices.md).
Skills are the executable individual practices; the umbrella document
remains the overall narrative.
