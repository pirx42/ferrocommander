# Skill: Architecture audit with parallel subagents

**When.** Whenever a request demands an "architecture review", "code audit",
"refactor roadmap", "senior-architect perspective" or a
sub-system audit over **> 10,000 LOC**. Also fitting when
a plan needs a `Phase 4 — architecture / performance / redundancy +
refactor` block (see
[40-four-phase-cycle.md](40-four-phase-cycle.md)) and the
subsystem scope is so large that a single-thread reading would blow
the context.

**Rule.** Audits at this size are NOT read sequentially by the
main loop, but **split into 4-6 parallel subagents**, each
with a clear, non-overlapping audit axis. Synthesis into a
plan doc afterwards.

**Why.** Three effects:

1. **Token economy.** A subagent reads the hot files itself and
   returns only a 600-800-word report. With 6 agents x
   ~150k subagent tokens, the raw reading does NOT flow into the
   main loop — which stays free for synthesis + user dialogue.
2. **Real parallelism.** 6 agents in 20-30 minutes of wall time deliver
   what would cost 2-3 h sequentially. Wall time is the scarce resource;
   parallel subagents cost only tokens.
3. **Bias reduction.** Each agent only knows its own audit brief — no
   cross-linking with the findings of the others. When three independent
   agents name the same weakness from different angles, it is
   robustly validated (e.g. "App.tsx too big" in both the hook-composition
   audit and the dependency audit).

**How (seven steps).**

1. **Quantitative baseline** (main loop itself).
   - `find src -name '*.ts' -o -name '*.tsx' | grep -v test | wc -l`
   - `find src/__tests__ -name '*.test.ts' | wc -l`
   - LOC per subdirectory cluster:
     `for d in src/{run,components,hooks,renderer,simulation,…}; do
        echo "$(find $d -maxdepth 1 -name '*.ts' -o -maxdepth 1 -name '*.tsx' | grep -v test | xargs cat | wc -l)  $d"
      done | sort -rn`
   - Provides a sense of scale BEFORE the audits start — who
     is big, who is small, where is test pressure.

2. **Cut the audit axes.** Per audit request, choose 4-6 axes
   that do *not* overlap. Examples from the 2026-06-03 run:
   - App.tsx hook composition + state/effect orchestration
   - Sim/engine layer (pure-vs-side-effect, headless parity, determinism)
   - Module layering + dependency cycles (with `madge --circular`)
   - Extension points + plugin patterns (handler maps, catalog, codegen)
   - Test strategy + coverage (pyramid, mock density, snapshot net)
   - Renderer + cross-cutting (i18n, perfMetrics, persistence,
     audio)
   One subagent per axis. If fewer than 4 axes make sense, the
   problem is too small for this skill — read sequentially instead.

3. **Write the subagent briefs.** Each brief contains:
   - **CONTEXT** (2-3 sentences: what is Chimera, where does this subsystem
     live, what does the existing docs say about it).
   - **TASK** (one sentence at senior-architect level).
   - **Concretely to check:** 6-8 sub-questions, each with file paths +
     symbol names as anchors so the agent can dive in directly.
   - **Tool suggestions:** which files to Read, which grep patterns,
     which external tools (madge, vitest --coverage).
   - **Output format:** "max 800 words" + fixed sections
     (quantitative findings / 3-5 strengths / 3-5 weaknesses with
     file:line / improvement suggestions prioritized / comparison with
     standard patterns).
   - **No code changes, research + report only** (explicit, otherwise
     the agent fiddles with the code).
   Send all briefs `run_in_background: true` in a single message block
   so they actually run in parallel.

4. **Create the plan-doc skeleton in parallel** (main loop).
   - File `docs/plans/YYYY-MM-DD-{topic}.md` (skill 10).
   - Sections: executive summary / architecture snapshot /
     standardized principle checks / findings by subsystem /
     cross-cutting themes / improvement suggestions prioritized /
     verdict.
   - Optionally already while the agents run — skeleton without content.

5. **Synthesis after all reports arrive.**
   - Per subsystem, take over 3-5 strengths + 3-5 weaknesses with
     file:line anchors.
   - Find cross-cutting themes: what did several reports name in
     common? (e.g. "locale bypass" in the renderer audit + "i18n without
     schema validation" in the extension audit → one cross-cutting finding).
   - **Standardized principle checks** as their own section: SOLID
     (S+O+I+D, L usually n/a for TS discriminated unions), DRY,
     layering/onion/clean, hexagonal, CQRS, test pyramid,
     plugin patterns. Each with a score + 1-sentence finding.
   - Improvement suggestions in three buckets: **Quick Wins** (each < 4 h),
     **Mid** (each 1-3 days), **Big** (each 1-2 weeks, its own plan). Per
     suggestion: measure + effort + effect.

6. **Verdict + recommended first steps.** Make the last section explicit:
   what is the codebase overall, where does it stand compared with its
   size class, and which 5 steps are concretely recommended first
   (in order). This makes the plan doc actionable for the user.

7. **Telegram report + push.** Commit + push the plan doc, then
   a Telegram reply with:
   - File path + LOC + commit hash.
   - 5-7 line summary (strengths-weaknesses verdict).
   - Question about the next step (usually: pull out one of the quick-win
     clusters as a follow-up plan, see example
     `2026-06-03-arch-review-qw1-qw2-qw4.md`).

**When it is _not_ needed.**
- Codebase < 10,000 LOC or audit scope < 3 subsystems — then
  single-thread reading + plan suffices.
- Pure bug audit or single-hot-spot diagnosis — a subagent would be
  overhead.
- The user explicitly asked for "brief" / "overview-style" — subagent effort
  does not match the request.

**Anti-patterns.**
- Starting subagents sequentially (foreground): kills the parallelism
  advantage. **Send all in one tool-use block.**
- Briefs without concrete file paths — the agent then greps blindly and
  misses the important spots.
- Forgetting the output format → reports come back in 4 different styles,
  synthesis becomes unnecessarily hard.
- Copying the synthesis section "findings by subsystem" 1:1 from the reports
  without adding cross-cutting themes + standardized principle checks —
  the main loop has to deliver the added value, otherwise
  parallelism was an expensive token choice.
- Forgetting the verdict section → the plan becomes a wall of findings, the user
  does not know where to start.

**Related.**
- [10-plan-lifecycle.md](10-plan-lifecycle.md) — the output is a
  plan doc with a `YYYY-MM-DD-` prefix and a later archive move.
- [09-scope-before-implementation.md](09-scope-before-implementation.md) — the 4-6 audit axes
  are the scope; do not extend with further aspects midway.
- [40-four-phase-cycle.md](40-four-phase-cycle.md) — fits into
  phase 4 (architecture/refactor) as an analysis tool.
- `42-housekeeping-coverage-doku-drift.md` (Chimera only)
  — audit findings identified as "organic drift"
  move into daily housekeeping.
- [45-calibrate-effort-estimates.md](45-calibrate-effort-estimates.md)
  — the Quick-Win/Mid/Big classification of the suggestions uses the
  empirical effort factors.

**Example run (2026-06-03).** Request: "create an architecture analysis
at high + mid level with a principles check". 75k LOC, 5271 tests. 6
subagents in ~25 minutes of wall time, ~710k subagent tokens, result:
[`docs/plans/archive/2026-06-03-architecture-review.md`](../plans/archive/2026-06-03-architecture-review.md)
(481 LOC). Findings: 63 cycles, App.tsx 5 forward-ref bridges, 3
determinism leaks, 0 UI tests, plus a concrete Quick-Wins/Mid/Big roadmap.
Follow-up plan
[`2026-06-03-arch-review-qw1-qw2-qw4.md`](../plans/archive/2026-06-03-arch-review-qw1-qw2-qw4.md)
implemented the first three quick wins (commits `4ce131d4`, `23e162fa`,
`285b1959`).
