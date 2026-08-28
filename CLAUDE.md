# CLAUDE.md — TC Clone Linux

A keyboard-centric dual-pane file manager for Linux in the style of
Total Commander (ghisler.com), built from scratch in Rust + GTK4.
Design and v1 scope: [docs/plans/2026-08-28-tc-clone-design.md](docs/plans/2026-08-28-tc-clone-design.md).

**Stack:** Rust + GTK4 (gtk4-rs) · Cargo workspace: `tc-core` (UI-free
engine: VFS, file ops, search, multi-rename, listing) + `tc-app` (GTK shell)
**Target platforms:** Linux (X11/Wayland) **and** Windows — both are
supported build targets; platform differences are confined to dedicated
`platform` modules (see [docs/vfs.md](docs/vfs.md))
**Language:** everything in this repository is English — docs, code,
comments, commit messages.

## Prime directive: speed (owner spec, 2026-08-28)

The app must be fast. Where a decision trades speed against something else —
a prettier surface, a larger feature set, a tidier-looking abstraction —
**speed wins**. The audience is hard-core Total Commander users, and what
they come for is a file manager that never makes them wait.

A performance claim here carries a measurement, and an optimisation carries a
test that pins the property rather than the timing. Rules, measured baseline
and the parts that are deliberately still slow:
[docs/performance.md](docs/performance.md).

## Reliability of operations (owner spec, 2026-08-28)

The operations must be very reliable — **rather more tests than too few**. A
file manager is trusted with the only copy of things, and the failure that
matters is not a crash but a job that reports success over a file it
destroyed. The awkward corner gets a test, not the benefit of the doubt, and
every invariant is checked against its own bug.
Details: [docs/reliability.md](docs/reliability.md).

## Build & Run

```bash
cargo build                # Debug build (workspace)
cargo run -p tc-app        # Run the app
cargo test --workspace     # All tests, including the end-to-end UI suite
cargo fmt --all -- --check # Format gate
cargo clippy --workspace --all-targets -- -D warnings  # Lint gate (CI)
cargo clippy -p tc-core --all-targets --target x86_64-pc-windows-gnu -- -D warnings  # Windows cfg branch
cargo build --release      # Release build
```

**Build prerequisites:** `build-essential`, `pkg-config`, `libgtk-4-dev`
(GTK ≥ 4.12), plus `xvfb` and `xdotool` for the UI tests. Those two are not
optional garnish: without them `cargo test --workspace` fails, by design —
a UI test that quietly skips is worse than no UI test.

The end-to-end suite (`cargo test -p tc-app --test ui`) drives the real
binary with real key presses on a private X server and takes about half a
minute; it runs one app at a time on purpose. See
[docs/ui-shell.md](docs/ui-shell.md).

The full sequence must be green before every commit — see
[docs/skills/25-green-suite-before-commit.md](docs/skills/25-green-suite-before-commit.md).

## Search Defaults (Grep/Glob/Find)

Searches that sweep the whole tree should exclude by default:

- `target/` — Cargo build output.
- `docs/plans/archive/` — historical plan documents.

Practical for bash grep: `grep -r --exclude-dir={target,archive}`.

## "Where do I find what?" — Task Cookbook

The project is in its build-up phase; this table grows with the code.

| Task | Entry point |
|---|---|
| Read/list files, path handling, platform differences | [docs/vfs.md](docs/vfs.md) |
| Sorting, hidden files, cursor, the `..` row | [docs/listing.md](docs/listing.md) |
| Copy/move/delete/mkdir, progress, conflicts, cancel | [docs/ops.md](docs/ops.md) |
| Search by name or content (Alt+F7) | [docs/search.md](docs/search.md) |
| Renaming many files by a rule (Ctrl+M), undo | [docs/multi-rename.md](docs/multi-rename.md) |
| Looking inside a file: F3, paging, encodings, the F4 editor hook | [docs/viewer.md](docs/viewer.md) |
| Widgets, row rendering, GTK version floor | [docs/ui-shell.md](docs/ui-shell.md) |
| Key bindings, navigation actions | [docs/keymap.md](docs/keymap.md) |
| Settings: where they live, what survives a restart | [docs/config.md](docs/config.md) |
| The command line, `cd`, history, command output | [docs/command-line.md](docs/command-line.md) |
| Noticing external changes: the watcher, `Ctrl+R` | [docs/watching.md](docs/watching.md) |
| Look up v1 scope / architecture | [docs/plans/2026-08-28-tc-clone-design.md](docs/plans/2026-08-28-tc-clone-design.md) |
| What is being built right now | [docs/plans/2026-08-28-tc-clone-design.md](docs/plans/2026-08-28-tc-clone-design.md) § 6 (phases 1–5 done, plus 3b and 3c; phase 6 next) |
| Known gaps left open on purpose | [docs/future-improvements.md](docs/future-improvements.md) |
| How fast things are, and what is still slow | [docs/performance.md](docs/performance.md) |
| What "reliable" means, and where the tests live | [docs/reliability.md](docs/reliability.md) |
| Which crate does a thing belong in | [crates/CLAUDE.md](crates/CLAUDE.md) |
| Verify a keystroke really works end to end | `crates/tc-app/tests/ui.rs` (see [docs/ui-shell.md](docs/ui-shell.md)) |
| Working rules / workflow | [docs/good-development-practices.md](docs/good-development-practices.md) + skill triggers below |
| New plan document | `docs/plans/YYYY-MM-DD-<topic>.md` (skill [10](docs/skills/10-plan-lifecycle.md)) |

## Documentation

Entry point: [docs/CLAUDE.md](docs/CLAUDE.md) — index of all doc files.
Workspace layout and the tc-core/tc-app boundary: [crates/CLAUDE.md](crates/CLAUDE.md).
Every directory with its own semantics gets its own `CLAUDE.md` with a
`← Parent` link (pattern adopted from the Chimera project).

## Coding Standard

- `rustfmt` defaults (no custom style), `clippy -D warnings` as the gate.
- No magic values — named constants, centralized per subsystem
  (skills [16](docs/skills/16-no-magic-values.md) /
  [17](docs/skills/17-centralize-constants.md)).
- Comments explain the "why", not the "what"
  (skill [18](docs/skills/18-comments-explain-why.md)).
- `tc-core` stays GTK-free and headless-testable; the UI never touches the
  filesystem directly.
- Conventional commits (skill [31](docs/skills/31-conventional-commit.md)).

**Commit-author rule (owner spec, adopted from Chimera):** **EVERY** commit
carries the owner as `author` — `pirx <andreas.rose@framefield.com>`. This
also applies to commits an agent produced entirely on its own: authorship of
the project lies with the owner; the agent's share goes into the
`Co-Authored-By` trailer. Practically, in every fresh working environment
BEFORE the first commit:

```bash
git config user.name "pirx"
git config user.email "andreas.rose@framefield.com"
```

Remote environments otherwise default to `Claude <noreply@anthropic.com>` —
that is the failure case. After the first commit of a session, double-check
with `git log -1 --format='%an <%ae>'`.

Development practices, workflow, and the multi-day cycle (Feature → Tests →
Docs → Refactor) are documented in
[docs/good-development-practices.md](docs/good-development-practices.md) —
read before starting non-trivial work.

## Skill Triggers (when to apply which [docs/skills/](docs/skills/CLAUDE.md))

Short rulebook: which skill applies in which situation. When a trigger
fires, the skill becomes mandatory — index + rationale in the individual
files.

| Trigger / situation | Skill |
|---|---|
| Ambiguous task without clear options | [01](docs/skills/01-clarify-with-options.md) offer named options |
| About to state behavior/code/requirement facts | [65](docs/skills/65-verify-or-ask-never-assume.md) verify or ask — never assume |
| User asks a meta/status question | [02](docs/skills/02-answer-meta-questions-directly.md) answer directly |
| Destructive / shared-state action (push, reset, delete) | [03](docs/skills/03-confirm-destructive-actions.md) get confirmation first |
| Plan approved → multiple phases | [04](docs/skills/04-multi-phase-autonomy.md) execute autonomously + [11](docs/skills/11-multi-phase-commits.md) phase = commit + [49](docs/skills/49-final-phase-refactoring-audit.md) last phase = refactoring audit |
| Task runs > 5 min | [05](docs/skills/05-updates-during-long-tasks.md) interim updates |
| Taking a pause / waiting | [06](docs/skills/06-announce-pauses.md) announce explicitly |
| User asleep / offline | [07](docs/skills/07-overnight-autonomy.md) continue autonomously |
| User writes via another channel | [08](docs/skills/08-reply-on-same-channel.md) reply on the same channel |
| Vague task → before implementation | [09](docs/skills/09-scope-before-implementation.md) scope Q&A |
| Building an expensive artifact yourself (fixture/setup/data) | [51](docs/skills/51-ask-before-expensive-setup.md) first ask whether the user can prepare it faster |
| New plan doc | [10](docs/skills/10-plan-lifecycle.md) `YYYY-MM-DD-`, status, archive + [43](docs/skills/43-coverage-before-implementation.md) phase 0 coverage pre-check + [45](docs/skills/45-calibrate-effort-estimates.md) apply effort factor |
| Major dependency upgrade | [12](docs/skills/12-major-upgrades-isolated.md) own branch/commit |
| Decision trades speed for polish, features or tidiness | [docs/performance.md](docs/performance.md) — speed wins, with a measurement |
| Touching a file operation, or wondering if a corner needs a test | [docs/reliability.md](docs/reliability.md) — it does |
| Refactor with a "quick hack vs clean" choice | [41](docs/skills/41-more-correct-variant.md) the more correct variant |
| New file vs extending an existing one | [13](docs/skills/13-prefer-existing-files.md) edit existing |
| Number/string hardcoded in code | [16](docs/skills/16-no-magic-values.md) named constant + [17](docs/skills/17-centralize-constants.md) centralize |
| Writing a comment | [18](docs/skills/18-comments-explain-why.md) "why", not "what" |
| Check for an already-guaranteed invariant | [19](docs/skills/19-no-defensive-programming.md) leave it out |
| Refactor / rename | [20](docs/skills/20-no-backward-compat-shims.md) no shims, migrate completely |
| Same logic in 2+ places | [44](docs/skills/44-no-redundancy.md) DRY — shared helper |
| Derivable list/value hardcoded | [53](docs/skills/53-generate-instead-of-duplicating.md) generate instead of duplicating |
| Feature / fix implemented | [23](docs/skills/23-tests-accompany-commits.md) tests come along |
| Changing existing tests | [24](docs/skills/24-no-silent-test-changes.md) explain + ask |
| Before commit | [25](docs/skills/25-green-suite-before-commit.md) fmt + clippy + tests + build green |
| Writing a test | [26](docs/skills/26-behavioral-tests.md) behavior over structure |
| Testing file-ops/engine code OR "why did no test catch this bug?" | [52](docs/skills/52-test-conservation-invariants.md) conservation invariants (byte sums, file counts, pack↔unpack roundtrip) over value asserts |
| Coverage gap discovered | [27](docs/skills/27-coverage-gap-triage.md) triage reachable/defensive/dead |
| "Is this covered?" / bug in covered code | [59](docs/skills/59-mutation-probe-over-coverage-percent.md) mutation probe: disable the effect, see if the suite screams |
| Code change with doc impact | [28](docs/skills/28-docs-in-same-commit.md) docs in the same commit |
| New doc file | [29](docs/skills/29-one-topic-per-doc.md) one topic + cross-link |
| Design decision made | [30](docs/skills/30-document-the-why.md) document the "why" |
| Writing a commit message | [31](docs/skills/31-conventional-commit.md) conventional format |
| Multi-step task | [32](docs/skills/32-commit-per-step.md) one step = one commit, immediately |
| Git hook fails | [33](docs/skills/33-never-skip-hooks.md) don't skip, fix the cause |
| `git push --force` on a shared branch | [34](docs/skills/34-no-force-push-shared.md) never without explicit approval |
| `git push` pending | [57](docs/skills/57-push-carries-the-whole-stack.md) `git log @{u}..HEAD` — approval covers the whole stack |
| File with a secret / `git add -A` | [38](docs/skills/38-no-secrets-in-repo.md) add explicitly, no secrets |
| Memory or audit report referenced | [39](docs/skills/39-verify-memory.md) verify state before acting |
| Larger subsystem after a feature burst | [40](docs/skills/40-four-phase-cycle.md) Feature → Tests → Docs → Refactor |
| Aborting a background job / `pkill`/`pgrep -f` in a command | [61](docs/skills/61-kill-background-processes-cleanly.md) kill the process GROUP + ps re-check |
| Statement about ALL catalog/config entries | [58](docs/skills/58-block-extraction-over-single-line-grep.md) block extraction over single-line grep |
| Script with hard-to-reconstruct config | [68](docs/skills/68-commit-important-scripts.md) commit to the repo, not the scratchpad |
| Script/job will foreseeably run > a few minutes | [71](docs/skills/71-long-scripts-report-progress.md) ongoing stage status with timestamps |
| Owner sets a direction + a known better approach exists | [73](docs/skills/73-proactively-suggest-better-approaches.md) suggest proactively — early, not at the plateau |
| Architecture review / code audit (codebase > 10k LOC) | [47](docs/skills/47-architecture-audit-with-subagents.md) parallel subagents, synthesis in a plan doc |

Full index + rationale: [docs/skills/CLAUDE.md](docs/skills/CLAUDE.md).
