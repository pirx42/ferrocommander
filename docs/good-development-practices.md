# Good Development Practices

> **Origin:** adopted from the Chimera project (`../chimera/docs/good-development-practices.md`,
> as of 2026-08-28). The project-specific examples it contains (npm/vitest/Playwright commands,
> RL training, deploy on port 3012, Telegram channel) should be read as illustrations —
> the Rust/Cargo equivalents for this project live in the root [CLAUDE.md](../CLAUDE.md)
> and in [skills/25-green-suite-before-commit.md](skills/25-green-suite-before-commit.md).
> The principles (Part A), the workflow (Part B) and the four-phase cycle (Part C)
> apply unchanged.

Working rules that have proven themselves in this project. The document
is aimed at humans **and** AI agents contributing to the code.
It is deliberately prescriptive: anyone who deviates should have a
conscious reason.

Four parts:

- **Part A — Principles (thematic):** rules per area (code,
  tests, commits, …) with rationale and example.
- **Part B — Workflow (chronological):** an end-to-end flow from
  accepting a task to pushing, in which the principles are put into
  practice.
- **Part C — Four-phase cycle (prescriptive recommendation):** a
  multi-day rhythm with clearly delineated phases (feature → tests →
  docs → refactor) and time frames.
- **Part D — Observed working rhythm from the commit history:**
  empirical baseline from the real commit history of the Chimera project,
  which Part C is oriented on.

**Division of labor with [docs/skills/](skills/CLAUDE.md):** the skills
are the operative rulebook — where content overlaps, the skill file is
the canonical place (SSOT) and this document points there. The GDP
carries the map (Part A as a thematic overview), the workflows
(Parts B/C), the empirical data (Part D) and full text only for rules
without a skill of their own.

---

## Part A — Principles

### 1. Communication with the client

#### 1.1 Ask when the assignment is ambiguous

**Rule.** For ambiguous or option-laden tasks, ask **before**
implementing. Offer named options (A/B/C) with tradeoffs,
not just a list of open questions.

**Why.** Every second of clarification before the work saves hours of
rework. Naming the options is more polite and easier to answer than an
open-ended "how exactly should this be?".

**Example.** If a feature description like "add a skip button"
comes in, clarify:
- How many penalties on skip? (1 random / all / none)
- Active / grayed out on which sectors?
- Confirm dialog or one click?

Then wait for the answer — do not start speculatively.

#### 1.1a Invent nothing — verify or ask (since 2026-07-18)

**Rule.** Statements about game behavior, code or requirements are
NEVER written from assumption: first verify against the implementation
(code/maintained docs); if it remains unclear, ALWAYS ask instead of
guessing (Owner msg 15477/15478). "Plausible" is not evidence.

**Why.** Handbook incident 2026-07-18: ~9 assumption errors (cable
button, "ammunition", infiltration backwards, …) — each correction round
costs more than the verification. Details in
[docs/skills/65-verify-or-ask-never-assume.md](skills/65-verify-or-ask-never-assume.md).

---

#### 1.2 Keep feedback short and concrete

**Rule.** Replies in the chat/terminal interface are short by default.
Details only when they are directly decision-relevant. An end-of-turn
summary fits in 1–2 sentences.

**Why.** Long summaries at the end of every iteration cost time,
are barely readable and repeat information that is already in the
diff.

**Example.** "Deployed (abc1234). 1580 tests green." is enough. No
paragraph about every changed selector.

#### 1.3 Answer meta-questions directly

**Rule.** For questions like "pushed?" / "committed yet?" / "in which
file?", deliver the answer first, then context if needed.

**Why.** When someone asks "pushed?", "yes, commit abc1234" helps
immediately — a paragraph of preamble is irrelevant.

#### 1.4 Confirmation for destructive / shared-state actions

**Rule.** Operations with non-local effects need explicit consent:
`git push` **of production code**, force-push, dependency
downgrades, DB migrations, restarting services, messages to third
parties, API calls that cost money. A one-time approval applies only to
the concrete scope.

**Push approval (as of branch workflow 2026-07-18, §6.7 / Skill 64):**
EVERY push to dev or main needs an approval — the acceptance merge of a
plan counts as approval. Only topic-branch pushes are free (no
CI/deploy effect). `git push --force`: ALWAYS ask. History: the earlier
exception "push pure test/doc changes directly" (user spec
msg 11745, 2026-06-09) is hereby SUPERSEDED (see §6.7).

**Why.** An accidental `git push --force` to main can destroy other
people's work. A restart at the wrong moment can cut users off in the
middle of a level. The cost of asking is small; the cost of a mistake
is often large.

**Example.** Even if CLAUDE.md says "perform git commits
autonomously", a push to dev/main is still a separate step with its
own approval — because it has outward visibility.

#### 1.5 Multi-phase plan: work through all phases, interrupt only for decisions

**Rule.** Once a plan has been approved by the client, work through all
phases in one go. Interruptions happen exclusively for real
decisions (tradeoffs, schema breaks, new requirements). After
each phase, push, give a brief update, keep going.

**Why.** Checking back after every sub-step produces unnecessary
waiting time and makes it hard to hold a plan in your head as one
piece. If the plan was agreed beforehand, working it through is the
default.

**Anti-pattern.** "Phase 1 done — should I start phase 2?" when the
plan contained phases 1+2+3+4 and there are no new findings.

#### 1.6 Updates during long tasks (~20-min cadence + milestones)

**10-minute checkpoint (since 2026-07-18, msg 15522).** Longer runs
are checked SUBSTANTIVELY after at most 10 minutes (scope/parameters/
throughput/remaining time) — not just progress-reported. CPU load only
proves "it's computing", not "the right thing" (simulateAll incident:
1.9 h with the wrong suite scope). Details Skill [05](skills/05-updates-during-long-tasks.md).

**Rule.** For tasks that take longer than ~5 minutes of pure working
time, give a brief update roughly every **~20 minutes** — what is
happening right now, what is the next step — **plus** a status ping
after every completed milestone (plan sub-task/phase/sub-agent).
Cadence raised by the user on 2026-07-09 from ~5 min to ~20 min
(memory `feedback_long_task_updates`); no longer a rigid 5-minute
beat. For sub-agent spawns, the stagger update applies to the main
thread waiting for the reply.

**Why.** Silence for more than a few minutes creates uncertainty for
the client — is it still working, is it stuck, has it been forgotten?
Brief visibility costs little and reassures a lot.

**Make background jobs observable** (2026-07-16): output goes into a
log FILE (`> job.log 2>&1`), never only through `| tail`; before any
hang diagnosis, check the run configuration/defaults (100 % CPU
only proves "computing", not "terminating"). Details Skill
[05](skills/05-updates-during-long-tasks.md).

#### 1.7 Announce pauses explicitly

**Rule.** When the work reaches a stopping point (waiting for a user
decision, done with no further tasks, sub-agent running in the
background with the main thread idle), report that **explicitly**.
Never fall silent quietly.

**Why.** From the client's point of view, "no new output" is identical
to "crashed", "forgotten" or "waiting for me" — they have to
ask themselves to see the difference. A single line "waiting for a
decision on X" or "all tasks done — ready for the next round"
resolves the ambiguity.

#### 1.8 Night/offline autonomy

**Rule.** When the client explicitly announces going offline
("I'm going to sleep", "back tomorrow", similar), work autonomously
until the stated return time (typically ~7:00) on decision-free
tasks. Do not wait idle. Tasks that need clarification are NOT
started — those wait until morning.

**Why.** The client's waking hours are the only time in which
decisions can be clarified. Blocking decisions-required tasks
during the night would waste that time. Low-risk tasks
(docs, test backfill, clear bug fixes, translation, audit follow-ups)
are ideal instead — the client finds progress in the morning.

**Behavior.**
- Before starting: load existing memory entries (mandatory rules,
  conventions) and follow them — do not take the opportunity to break
  house rules.
- Risk filter: no schema breaks, no refactors that touch many
  consumers, no destructive git operations without explicit
  prior consent. If a decision comes up: note it down (memory /
  plan doc) and leave it be.
- Stop when all decision-free tasks are done — do not tackle
  risky tasks just to fill the time. A short
  end-of-night status is enough.

#### 1.9 Reply on the same channel

**Rule.** The answer + all follow-up questions go back over the
**channel the message came in on**. Telegram inbound → Telegram
reply. Terminal inbound → terminal output. Never switch silently,
not even "because it happens to be easier right now".

**Why.** The sender only reads the channel they wrote on —
their push notifications and their attention are there. If the
answer lands elsewhere, they do not see it. In particular: Telegram
inbound with a terminal reply is, for the sender, the same as no reply.

**Behavior.**
- Telegram inbound → `reply` tool with `chat_id` from the `<channel>` tag;
  `react` for acknowledgment, `edit_message` for live status, a **new**
  `reply` at the end of a long task (edits do not trigger push
  notifications).
- Terminal inbound → direct text output, no Telegram tools.
- No `AskUserQuestion` dialogs in the terminal when the request came
  via Telegram.

#### 1.10 Ask briefly before laboriously building things yourself

**Rule.** Before an artifact is **laboriously assembled by hand**
that the user could produce faster and more reliably interactively
(in the game, in the tool, from their head), first ask briefly: "Can
you prepare/export X faster?". Then the user decides whether they
prepare something or whether I build it myself. Do not silently start
tinkering.

**Why.** User spec msg 11713 (2026-06-08). Trigger: an
infiltration save for e2e tests was to be produced synthetically
(bot-sim dump + hand-editing the JSON + several failed attempts for
`activeBossBody` + energy network + infiltrator charge). The user built
the same setup in the game in seconds and sent it as a save —
exactly correct. One question up front would have saved the detour.

**Behavior.**
- Recognize: "I am about to build this laboriously by hand" — especially
  for savegames, fixtures, setups that arise interactively in the tool.
- Ask a short, concrete question over the active channel + say where I
  will put the result.
- Wait for the answer; on "you build it", take the do-it-yourself route.
- Do **not** ask for trivial or purely code-side artifacts,
  or when the user is offline (night autonomy → choose the cheapest
  do-it-yourself route, present the result for review later).

See Skill [51](skills/51-ask-before-expensive-setup.md).

---

#### 1.11 Create preview images for visual feedback

**Rule.** If a task produces a **visual result** (icon/emblem,
map graphic, layout, color palette, rendering variant), first render a
**preview image of all variants side by side**, inspect it yourself via
`Read` and then send it to the user — instead of making them hunt for
it in the game. **Commit the gallery script to the repo**, because you
need it again on every tweak.

**Why.** User spec msg 13771 (2026-07-02, upgrade medals): "this
preview is really good. please remember to always create previews for
feedback on tasks like this. it's much faster than in the game
itself." In the game, not all events/upgrades/items occur per run — you
would never see the variants together and would have to play up to the
boss. A side-by-side board costs one render cycle and makes subjective
design questions (emblem, contrast, color) immediately decidable. That
way you don't finish building N motifs "blind" before the user sees one.

**Behavior.**
- Gallery entry with the **real component** for ALL variants (no
  rebuild → no divergence); instances via existing factories
  (`createModule(def)` for items, effect literals for events/upgrades).
- Screenshot headless: short-lived Vite dev server + Playwright
  `fullPage`. In Chimera ready-made as **`npm run gallery`**
  (`scripts/gallery.mjs` → `docs/visual-gallery.png`, `src/devPreview/`).
- Look at it yourself first (`Read` on the PNG), then send it over the
  active channel ([1.9](#19-reply-on-the-same-channel)) with a brief legend.
- Delete throwaway one-off previews again; the reproducible board stays.
- Complements — does NOT replace — the final in-game pilot for
  interaction/feel questions ([7.6](#76-pilot-deploy-before-mass-rollout--for-critical--visible-changes-since-2026-07-01) / Skill 54).

See Skill `55-vorschau-bilder-fuer-visuelles-feedback.md` (Chimera only).

---

### 2. Planning and scope

#### 2.1 Nail down the scope before implementation

**Rule.** Before the first line of code: write down for yourself (or
tell the client) what belongs to this task. If a scope expansion comes
up during implementation ("we should clean that up too while we're at
it"), no yes before asking back.

**Why.** Scope creep produces large, hard-to-review diffs,
dilutes the fix with unrelated changes and makes git-blame
useless.

**Example.** Task: "Fix bug X". While reading the bug, you notice a
messy module. Do not refactor it along the way — either as a separate
PR afterwards, or only after asking.

#### 2.2 Multi-phase changes into separate commits

**Rule.** Split non-trivial refactors into phases — each phase one
commit, tested + build-green individually. No batch commits with several
independent changes.

**Why.** Revert granularity + review readability. A bisect finds
the error more easily in 10 small commits than in 1 big one. Separate
phases force you to consider whether the intermediate steps are really
runnable.

**Example.** A larger refactor "replace data model X": phase 1 =
new types + loader + JSON migration. Phase 2 = switch the runtime.
Phase 3 = update the UI. Each phase green on its own.

#### 2.3 Major dependency upgrades in isolation

**Rule.** No major-version bumps of a dependency in the course of
another task or in an `npm update` sweep. Every major is its own
task with its own regression window.

**Why.** Major bumps bring breaking changes that cost debugging
time. Mixed with another task, the error becomes untraceable.

**Example.** TypeScript 5 → 6 belongs in its own day with a
`strict` check, not in a bug-fix commit.

#### 2.4 The more correct variant instead of quick-and-dirty

**Rule.** In refactor scope decisions, choose the **substantial path**,
not the scaffolding minimum. If two variants are on offer — one
"does it right", the other "only sets up the skeleton",
the right variant is the default, provided effort and risk
are acceptable.

**Quick fixes need explicit user approval** (user spec msg 10982,
2026-05-31): if (b) seems justified, the assistant asks
back — it does not make the decision silently. Concrete
form: "a quick fix would be possible here that works around X — should
I, or should I implement the proper variant?" and wait for approval.
Exception: the cases named below (spike, hot-fix, pure migration)
may be implemented by the assistant as a quick fix without new approval —
but the user's assignment must recognizably target one of those cases.

**Why.** Scaffolding solutions have the property of staying
permanently. "Quick-and-dirty now, clean later" is rarely followed
up in practice, because the next pressure comes from a different
direction. A little more time now saves a refactor in 6 months
plus the side effects (several places have to be migrated,
tests differ, consumers change). The approval requirement forces the
assistant to lay the real cost/benefit calculation open instead of
encapsulating it away — and rules out the pattern "build the quick fix
first, then have to follow up", which has already led to correction
iterations several times in this codebase (most recently the
pulseMod display-layer quick fix before msg 10982).

**When the minimum _is_ right.** For genuinely exploratory spike code
that is explicitly marked as throwaway and is also deleted again (not
merged into main). Or for a hot-fix under time pressure — then with an
explicit follow-up story.

**Example.** For a property-bag extension, two variants are
possible: (a) add the new field directly to `VALUE_KEYS` and integrate
it in the resolver with all rules, or (b) special-case handling in the
caller. (a) is substantial, (b) is scaffolding. Choose (a) unless an
explicit reason justifies (b).

#### 2.5 New item kind: mandatory and optional touch set

**When.** The assignment demands a new item in the `ITEM_CATALOG` — whether
a modifier sub-module or a main item (producer/consumer/storage).

**Rule.** Work through the touch set by class. Never commit a catalog entry
without i18n labels (memory `feedback_chimera_i18n_rule`).

**Mandatory (shared, all items):**
1. `src/types/base.ts` — extend the `ItemKind` union.
2. `src/itemCatalog.ts` — `ItemDef` entry with mandatory fields (`kind`, `icon`, `subSlotCount`, `category`, `color`, `botRole`, `maxCables`). **No `label` field** — the user-facing name lives exclusively in i18n since 2026-06-05 (`items.<kind>`, via `getItemLabel`).
3. `src/i18n/de_DE.ts` + `src/i18n/en_US.ts` — `items.<kind>` = the item name (single source) + optional `help`.
4. `docs/items.md` — narrative entry + afterwards `npm run docs:items` for the AUTO-GENERATED block (plan 2026-06-02-propertybag-modularisation part B).
5. `npm run generate:norms` — on EVERY catalog property change (including on existing items, e.g. a new category), otherwise `propertyNormFactors.test.ts` breaks in CI (incident 2026-07-11: nova → damage cat → intrinsic trait changed).
6. `src/simulation/rlObservation.ts` — append the new kind at the END of `KIND_ORDER` (append-only!) + length guard in `rlObservation.test.ts`. The guard runs ONLY in the sim suite, not in `npm test` — after appending, explicitly run `npx vitest run --config vitest.simulation.config.ts src/simulation/rlObservation.test.ts` (missed twice: nova/firewall 07-12, catalyst/harvester 07-17).

**Modifier-specific:**
- `subSlotCount: 0` (or 1 if it takes sub-mods itself), `botRole: "modifier"`, no energy profile needed.
- `modifierMultiplierEffect` and/or `modifierValueEffect` with `output`/`cost`/`stealth`/`damage`/`health`/`experience`/`spatial`/`shield`/`diffusion` as a number or `"invert"`.
- If special math is needed (context-/time-dependent): `src/itemHooks/<kind>.ts` as a hook (heatExtractor pattern).
- If a container applies the kind's contribution N-fold: `hooks: { childApplyCount: 2 }` (duplicator pattern, transparent expansion).

**Main-item-specific:**
- `botRole: "container"` (consumer) or `"producer"`, its own energy profile in `src/energyProfiles.ts`.
- `categories: { output: [...], cost: [...], stealth: [...], health: [...], experience: [...] }` — lists the VALUE_KEYS per cat (`output: ["production"]` for generator, `cost: ["demand", "heat"]` for engine).
- `drawDecoration` in `src/renderer/itemDecorations.ts` (almost always needed).
- `defaultMaxHP` if the item should have HP/destruction (default 100 via DEFAULT_MAX_HP fallback).
- `damage`/`damageRate`/`visualEffect: "laser"` for bolt spawners.
- `rootOnly: true` for items that do not fit into sub-slots (infiltrator).
- `defaultThreshold` + `src/probeTypes.ts` entry for diagnostic items (auto-derived DIAGNOSTIC_KINDS).
- Save-format bump in `src/persistence.ts` ONLY if a new PropertyBag field is needed (rare).

**Auto-derived constants** (NO manual maintenance needed): `VALID_LOCK_KINDS`, `DIAGNOSTIC_KINDS`, `DEFAULT_THRESHOLDS`, `CONTAINER_KINDS`, `PRODUCER_KINDS`, `MODIFIER_KINDS` are generated from the catalog. Setting the correct `botRole`/`category`/`lockable` is enough.

**Workflow.**
1. Catalog + types + i18n in one commit.
2. Decoration + energy profile in the same commit.
3. Hooks/codex if needed.
4. Tests in `src/__tests__/`.
5. `npm run docs:items` for SSOT regeneration.
6. Build + deploy + playtest.

**Effort.** Historical observations: pure modifier 3 files ~1 h;
main item without special math 5-6 files ~2-3 h; main item with its own
sub-mechanic 8-12 files, belongs in a plan doc. For plan estimates,
the lived factor rule from Skill
[45-calibrate-effort-estimates.md](skills/45-calibrate-effort-estimates.md)
applies (refactor ×0.10, feature ×0.25) — do not perpetuate these absolute numbers.

**Why.** Adding items is a recurring task type with high
drift susceptibility (forget i18n → UI fallback, forget energyProfile
→ item produces nothing, forget categories map → overclocker has no effect,
wrong botRole → bot heuristic confused). The checklist avoids silent
errors that only show up during the playtest. Full details in
`docs/skills/46-neues-item-hinzufuegen.md` (Chimera only).

#### 2.6 Last plan phase = refactoring audit with correction

**When.** Every multi-phase plan in `docs/plans/`.

**Rule.** The concluding phase of every plan is a **refactoring
audit + correction** over the **entire** implementation delivered by the plan —
checked against architecture and redundancy; improvements found are
**implemented** in the same phase (not just noted). It is already planned
as the last phase when the plan is written; a plan does not count as
"Implemented" before this phase is done.

**Audit axes.** Redundancy/DRY (same logic across several plan commits →
shared helper/type/constant), architecture (special cases that hide a common
pattern, handler map instead of switch, pure reducers, oversized
functions), consistency (naming, layer placement, interface shape),
dead remnants (shims, unused exports, scaffolding).

**Scope.** Only what THIS plan touched — not the whole codebase
(that is what the subagent audit from
[docs/skills/47-architecture-audit-with-subagents.md](skills/47-architecture-audit-with-subagents.md) is for).

**Why.** Across several phases, structures arise that are locally sensible
but suboptimal in sum (duplicated logic, inline unions that belong
centralized, hidden patterns). In the overall view at the plan's end — suite
green, behavior verified — consolidation is cheapest and safest.
Full details in
[docs/skills/49-final-phase-refactoring-audit.md](skills/49-final-phase-refactoring-audit.md).

#### 2.7 Game-rule semantics: one learnable rule, no special cases (since 2026-07-05)

**Rule.** Game-mechanic semantics questions (what does a value/modifier
MEAN on an item?) are FIRST discussed via Telegram, THEN implemented —
and the solution must be a GLOBAL rule learnable by players, no
per-item special cases. Litmus: *Can a player learn the rule from one
sentence and apply it to all items?* (User spec msg 14116.)

**Why.** An item that silently interprets a declared mechanic
differently breaks the players' mental model. Example: fan(inverter)
was supposed to suck — the special-case solution (`inverterDirectionCategories`
flag) was reverted; the right answer was the global sign rule (the inverter
makes values genuinely negative, every device interprets the sign
physically). Full details in
`docs/skills/56-spielregel-semantik-ohne-sonderfaelle.md` (Chimera only).

---

#### 2.8 Catalog/config audits: block extraction instead of single-line grep (since 2026-07-11)

**Rule.** Never derive statements about ALL entries of a catalog/config
file (classifications, "which items have X?") via single-line regex
— entries can span multiple lines and silently drop out.
Block extraction (anchor to next anchor) or load the structure
directly; for effect audits additionally check `propertyBag/reader.ts`
(context effects), `itemHooks/` and `energyProfiles.ts`.
Cross-check the extraction with 2-3 known cases. Incident: wrong
Q3b remainder list in the XP-level plan (msg 14802). Details in
[docs/skills/58-block-extraction-over-single-line-grep.md](skills/58-block-extraction-over-single-line-grep.md).

---

#### 2.9 User-provided objects: adopt wholesale or replace fresh (since 2026-06-12)

**Rule.** If the user provides concrete items/builds/JSON templates to embed
or transform, then either pass through the COMPLETE property set
or build a fresh instance of the same type — never selectively strip
properties (only unambiguously runtime-local fields like `currentHP` may go).
Preserve explicitly empty sub-slots `{ "module": null }` in templates 1:1 —
auto-pad only applies to container kinds.

**Why.** Partial adoption produces a Frankenstein state
(user spec msg 12100): on 2026-06-12, stripping `xp` broke the energy
balance the user had tuned via item levels; on 2026-05-08 an
explicitly empty generator slot was missing in the library. Details in
`docs/skills/62-user-objekte-ganz-oder-frisch.md` (Chimera only).

### 3. Code quality

#### 3.1 Edit existing files, do not create new ones lightly

**Rule.** If a change fits into an existing file, go in there.
New files only for new conceptual units.

**Why.** File proliferation hampers navigation. The first question
should be: "Where is the natural place?", not "Where do I create a
new file?".

#### 3.2 Separate pure reducers from stateful hooks

**Rule.** Business logic in pure functions (deterministic, no
side effects) — IO / state / timing in hooks or thin wrappers. The hook
delegates to the reducer, not the other way around.

**Why.** Pure functions are trivially testable and composable. Hooks
are hard to test; if hooks contain logic, the logic is tied
to React rendering.

**Example.** Reducer `advanceToNextSection(state, runDef, gameTimeMs)`
has no side effects — a hook calls it via `setGameState(gs =>
advanceToNextSection(gs, …))`.

#### 3.2a Large hooks: extract tick sub-functions as pure modules

**Rule.** As soon as a hook exceeds several hundred LOC and the
central tick function contains 5+ inline sub-functions (`applyXXX()`),
pull these out into `src/<area>/<concern>Tick.ts` as pure functions
(`runXXXTick(input)`). The hook remains the orchestrator: setup effects,
HeatManager/state refs, order of the calls. Every extracted
function receives **all deps** (HM refs, charge maps, shield maps,
callbacks) as an input object — no hidden closure captures.

**Why.** Inline sub-functions inherit the entire closure state of the
tick() function and are therefore not testable in isolation; the main
purpose of the hook (what gets called when) disappears behind the
sub-logic. Pure-module extraction makes every tick path individually
testable, reduces the hook LOC drastically and forces explicit
dependency lists that make hidden couplings visible.

**Example.** `useHeatSimulation` was 982 LOC with 7 inline sub-functions
(applyEmergencyShutdown, applyExtremeTemperatureDamage,
applyFieldLockDisplacement, applyThermoInjections, applyRepairPaste,
applyShooterBolts, applyLaserCutters). After extraction into
`src/run/*Tick.ts` the hook is 637 LOC; every sub-function has a
typed input interface, no longer closes over any refs and is
testable in isolation. tickMitigateCombined travels as a callback through
the input objects — the refactor made explicit which paths need shield
mitigation.

**Order.** One extraction per commit, each with green tests.
Do not batch — the reducer traffic (the order of the tick steps is
semantically important) otherwise becomes opaque. With every extraction,
also clean up unused imports and now-dead code (constants, closures) in
the hook.

#### 3.3 No magic values

**Rule.** Numbers / strings with meaning become named constants,
not inline. Thresholds, timeouts, units — everything named.

**Why.** Named constants are documentation. `const
TASK_START_DELAY_MS = 5000` explains itself; `5000` in a
timer call does not.

#### 3.3a Centralize constants as soon as they are thematically related

**Rule.** When three or more constants belong to the same subsystem
(heat sim, energy pool, cable physics, tutorial-run timing) and wander
across several files, collect them in a dedicated
`<area>Constants.ts` file. Feature-specific UI/timing constants
(`BEAM_FLASH_COOLDOWN_MS`, `FIELD_LOCK_SNAP_DURATION_MS`) stay in
their feature modules — they are not sim parameters and belong to the
respective logic.

**Why.** Scattered constants drift: two places hold the same
concept with minimally different values, or the docs cite a value
that no longer exists anywhere in that form. A central collection point
makes game-mechanic tuning local and simplifies doc references.

**Example.** `src/heatConstants.ts` holds `HEATMAP_DIFFUSION_RATE`,
`BORDER_COOLING_EXTRA`, `ITEM_HEAT_DURATION_MS`, `CABLE_HEAT_FULL_FLOW`,
`ZONE_HEAT_DAMAGE_THRESHOLD`. Previously these were spread across
`heatPhysics.ts`, `useHeatSimulation.ts`, `thermoInjectionTick.ts` and
`extremeTemperatureTick.ts`; `docs/heat-system.md` references
them as game-mechanic parameters — only sensible with a single home.

#### 3.3b Balance/content values data-driven, not hard-wired (since 2026-06-24)

**Rule.** If a value or behavior is something that **authors/players should
tune per content** (zone bonuses, boss/run parameters), it belongs in the
**content schema (JSON)**, not in a code constant — not even a central
`…Constants.ts`. The solver reads the value from the data; code holds only
the mechanic, not the desired value.

**Why.** A code constant forces a build + deploy for every balance change and
makes per-chassis/per-boss variance impossible. Example (msg 12940): the
torso production bonus was hard-coded in the solver as `TORSO_PRODUCTION_BONUS = 1.5`.
Converted to `ZoneBonusEntry.productionMultiplier` / `BossZoneDef.productionMultiplier` →
freely adjustable per zone/chassis/boss; the solver receives a `zoneProductionMul`
map. The former arm "bonus" (×1.5 consumption) was effectively a penalty and was
removed without replacement — hard-wired "bonuses" escape visibility and
tuning. Distinction from 3.3a: pure sim/UI timing constants stay in code;
what is meant are **authoring/balance quantities**.

#### 3.3c Generate derived lists/values instead of duplicating them (against drift, since 2026-06-30)

**Rule.** A list or value that can be **derived from a source of truth**
(existing files in the build output, `package.json`, installed versions)
is **generated at build time**, not maintained by hand in a second place.
The generated file is checked in (so dev works without a build) AND regenerated
in `npm run build` (always fresh). Pattern: `src/generated/propertyNormFactors.ts`,
`scripts/generate-*.ts`.

**Why.** Every hand-maintained copy drifts away from its source. Examples
(2026-06-30, each following a user report):
- **Run list:** `NewRunScreen` probed a hard-coded `ALL_RUN_IDS` list via
  `fetch` → 404 blind probes for stripped runs in the public build. Fix: a
  build manifest (`runs/index.json`, `generate-run-manifest.ts`) lists the
  runs actually shipped → only what exists gets loaded.
- **Credits license list:** the hard-coded tool/version list was 6/10 outdated
  + incomplete (Playwright/fonts missing). Fix: `generate-licenses.ts` reads
  `package.json` × `node_modules` → versions, licenses and license texts can no
  longer go stale.

**Distinction.** Deliberately static values (e.g. the README test-count badge,
user wish 2026-06-23) remain hand-maintained — that is a decision, not a drift
bug. What is meant are **derivable** data whose hand-copy is only a source of
error. Follow-up trap: what gets generated/stripped must not be hard-referenced
by the app elsewhere (see 7.4). Skill: [53-generate-instead-of-duplicating.md](skills/53-generate-instead-of-duplicating.md).

#### 3.4 Comments explain the "why", not the "what"

**Rule.** No comment that paraphrases the code line. Comments
are for hidden invariants, workarounds, design decisions that
would not be clear to the reader without context.

**Why.** The code says what it does; the comment should say why.
"Increments counter" is noise; "Retry counter — provides the jitter in
the reconnect flow (issue #412)" is context.

#### 3.5 Trust framework guarantees, validate only at system boundaries

**Rule.** No defensive programming for scenarios the framework
rules out. Validation only at outer boundaries (user input,
external APIs, file parsers).

**Why.** Over-defensive code buries the actual business logic under
null checks and try/catches. If the compiler or the framework
guarantees that X cannot happen, do not waste a line on it.

**Example.** For a TypeScript type `{ foo: string }`, do not add an `if
(typeof obj.foo !== "string")` check.

#### 3.6 No backward-compatibility shims when refactoring

**Rule.** When you replace something, remove the old thing completely. No
`_unused` leftover, no `// removed` comment, no re-export of the
old API "just in case". Old code rots.

**Why.** Dead code is worse than no code: it looks like living
code and sends readers in wrong directions.

**Exception.** Public APIs and persisted data with external
consumers often need migrations / deprecation paths. Internal code
does not.

#### 3.6a No redundancy / no duplicated code (since 2026-05-31)

**Rule.** The same logic occurs **once** — whatever repeats in several
places moves into a shared helper, a function or a
constant (centralized, see [3.3a](#33a-centralize-constants-as-soon-as-they-are-thematically-related)).
Even 3-5 lines are worth the extraction, BEFORE the second copy is
committed. Boundary: same **meaning**, not same appearance —
do not prematurely abstract semantically different similarities.

**Why.** Duplicates diverge: the burst-window pattern
`(now - fireTime) < THRESHOLD` lived in three places (heatPhysics,
instanceStatRows, engine); when the time base moved to gameTimeMs, only
one was converted — the others silently collapsed, the bug was only
noticed by the user (msg 10991). Application checklist, smell test
("do I have to touch a second place when changing this?") and
exceptions: Skill [44-no-redundancy.md](skills/44-no-redundancy.md).
Source: user spec msg 11002 (2026-05-31).

#### 3.7 Discriminated unions: handler map instead of scattered `kind` switches

**Rule.** As soon as a discriminated union (`kind`/`type`/`action`) branches
on the discriminator at more than two call sites, the behavior moves into
a **handler map**: one handler file per variant, a central
`Record<Kind, Handler>` map with TS-enforced completeness, callers
dispatch via `HANDLERS[e.kind].method?.(...)`. The pattern is the
default choice — new discriminated unions are laid out this way from the
start, not refactored only after the third switch.

**Why.** Scattered `kind` switches must all be found and extended for
every new variant — a miss often only becomes visible at runtime;
the map turns it into a compile error and bundles the knowledge per
variant into one file. Three Chimera refactors (NPC_HANDLERS, TASK_HANDLERS,
EFFECT_HANDLERS) resolved ~140 scattered switch sites this way.

Construction guide (four conventions), contraindications (parsers/loaders,
type-narrowing guards, single local lookups), auxiliary rules and
mini skeleton: Skill
`21-handler-map-pattern.md` (Chimera only).
Live examples: `src/run/effectHandlers/`, `src/run/taskHandlers/`,
`src/run/npcHandlers/`.

#### 3.8 i18n requirement for user-facing texts (since 2026-05-22)

**Rule.** Every new user-facing text must be i18n-capable. Two paths:
text defined in TS/TSX code goes through `t()` from `useTranslation()` (or
`getStaticTranslation` outside React) with keys in `src/i18n/en_US.ts`
AND `de_DE.ts`; text defined in JSON as a `LocalizedString`
(`{ de_DE, en_US }`; loader `parseLocalizedFromAny`, display
`resolveLocalized`). Forbidden: hardcoded UI literals, DE fallback via `??`,
missing locale variant.

**Why.** Every string that slips through is a bug in the EN locale, which
the user so far had to catch via screenshot ("[object Object]", TaskHUD
"ODER", "Hitzeschaden") — check before the commit instead of following up.
Full details (JSON file list, yes/no examples, migrator +
translation scripts): Skill `22-i18n-pflicht.md` (Chimera only);
architecture: `docs/architecture.md` → "Internationalization".

#### 3.9 Know and respect design invariants — do not fix them as bugs

Reports of the form "X has no effect on Y" / "the value doesn't pulse/scale as
expected" are first held against the **deliberate system invariants** before
touching the engine. What looks like an error in isolation is often
intended semantics.

Known Chimera invariants (as of 2026-06-13):

- **Boss event effects without a duration (`durationMs`/`recoveryMs` = null) are
  PERMANENT — intended engine semantics, not a bug.** On a loop boss
  (`loopTimeMs`) the event fires anew every cycle and the instances STACK
  (no replace). An accumulating hazard (fieldLock locks more fields per loop,
  zoneCompression collapses the zone) is a FORGOTTEN duration in the
  boss JSON, not an engine error. Cautionary example 2026-06-13: on a hunch I
  built a "replace-on-refire" engine fix (fieldLock, then wrongly generalized to
  zoneCompression) — completely reverted by the user; the real fix
  was `durationMs:6000` in the JSON. Additional error: the lifetime field differs
  per kind (fieldLock `durationMs`, **zoneCompression `recoveryMs`**) — my
  scan checked only `durationMs` and wrongly read compressions as permanent. Lesson:
  first check design intent + data (missing field?), not the engine; and look up
  the RIGHT field per kind.
- **Modifiers act only child→parent, never on siblings.** A sub-module
  modifies its parent, not its siblings in the same container.
  `mux(pulseMod, repairPaste)` does NOT make the repair pulse (siblings) —
  the pulsing composition is `repairPaste(pulseMod)` (nesting). Technically:
  two-pass aggregation in `src/propertyBag/resolve.ts` + owner scope (only the
  category owner receives child multipliers on its values). User
  decision msg 11370: "childs only act on parents but not on
  siblings" — the engine change was explicitly rejected.
- **`inverterMod` is category-selective and is consumed by the first ancestor with
  a matching category factor OR a `categories` declaration.** It
  flips the values/multipliers of exactly ONE category. Containers WITHOUT a
  factor/declaration in the target category are transparent — the inverter
  propagates up through them. **An item with a `categories` declaration is an
  absorption boundary (owner scope, `phaseC` `hostOwnsCategories`):** it
  absorbs ALL remaining inverter counts → the inverter no longer reaches the
  parent. Verified: `engine(duplicator(inverterMod))` → speed 0
  (propagated through the category-less duplicator), but `engine(sensor(inverterMod))`
  → speed 50 (sensor has `categories` → absorbs). Consequence for item builders: if
  an inverter is to flip a host VALUE (cooler/heatSink: heatCooling→heat), it must
  be a DIRECT child of the host; a `categories` map on an intermediate item
  deliberately makes it an inverter boundary. Detail skill:
  `46-neues-item-hinzufuegen.md` (Chimera only) →
  "Inverter propagation".
- **Drag must never change or block the sim** (sim output identical to
  not-dragged).

Procedure: reproduce the path empirically (throwaway dump test that
logs `resolveTree(...)` / `buildInstanceStatRows(...)` for the reported setup),
hold it against the invariant, only then decide bug vs. intended.
If intended → no fix, but a diagnosis + recommending the correct composition;
name a genuine engine change as a deliberate design question with its blast
radius (see Skill
`48-design-invarianten-respektieren.md` (Chimera only)).

---

#### 3.10 Browser-shared modules: no `require` — `process.getBuiltinModule` (since 2026-07-11)

**Rule.** Modules shared by browser AND Node/sim must not use `require` for
synchronous Node APIs (`node:fs` etc.) — under ESM/tsx this silently returns
`null` and the sim path loses the functionality, while vitest
(CJS interop) masks it: **unit-green ≠ sim-green**. Instead use the
shared helper (`getNodeSyncFs` via `process.getBuiltinModule`)
and verify the sim path (`npm run simulate` smoke) separately. See
memory `project_chimera_esm_require_getbuiltinmodule`.

---

### 4. Testing

#### 4.1 Tests accompany features and fixes

**Rule.** A new feature commit brings tests for the new logic.
A bug fix brings a regression test that fails without the fix.

**Why.** Without tests you only know "works right now", not
"stays correct". Bug regression tests additionally document what the
reported problem was.

#### 4.2 Explain + ask before changing existing tests

**Rule.** Tests are not simply "adjusted so they go green".
An existing test has a reason. If a change makes it
fail: (a) first consider whether the change is really supposed to break
the documented behavior. (b) If yes, explain the change
and — for real behavioral rules — align with the client.

**Why.** Tests are codified requirements. Changing a test
to repair a build often deletes exactly the invariant
the test protects.

#### 4.3 Green suite before every commit

**Rule.** `tsc --noEmit`, the complete test suite, **the e2e tests
(`npm run e2e`, Playwright — they do NOT run as part of `npm test`!)**,
**`npm run lint`** and the production build run green before a
commit is made. If one is red: fix it, do not commit.
(e2e added since 2026-06-07, user spec msg 11609; lint added since
2026-07-06 after CI fail msg 14129 — a single eslint ERROR, even in
test files, makes the GitHub workflow fail; pre-existing warnings
are ok. See memory `feedback_e2e_before_commit`.)

**Why.** Red commits burn time for everyone who checks out
afterwards. Bisect breaks. CI credits are wasted. e2e used to run
out-of-band and stayed red unnoticed (outdated save version) — hence
from now on part of the gate for app-code changes.

**Example sequence:**
```
npx tsc --noEmit && npm run lint && npm test -- --run && npm run e2e && npm run build
```
For pure doc/test-file commits without app-code changes, `npm run e2e`
may be skipped. Also avoid values in e2e tests that drift with the source
(e.g. read `CURRENT_SAVE_VERSION` at runtime from `src/persistence.ts`).

**Pipe-exit trap:** `npm test | tail -3` masks the exit code (the pipe
returns the status of `tail`) — an `&&` gate keeps running despite red tests
(happened 2026-07-05: 3 failures unnoticed). Set `set -o pipefail`
or check the summary line explicitly.

**After a merge: no second run without changes** (owner spec
2026-08-14). The gate runs BEFORE the merge on the topic branch; if
`git diff --quiet topic/<plan> dev` is empty after the merge (tree identical
to the tested state), the suite is NOT run again. If the diff is
not empty (foreign dev commits, conflict resolution), the full gate runs
on the merge result. Details: Skill
[25-green-suite-before-commit.md](skills/25-green-suite-before-commit.md) →
"After a merge" + Skill 64 rule 4.

#### 4.4 Behavioral tests before structural tests

**Rule.** Tests check observability — what the code does, not
how it is built internally. No test that has to assert a file path or an
internal class hierarchy in order to survive.

**Why.** Structural tests break on every harmless-looking
refactor. Behavioral tests survive refactors and protect real
invariants.

#### 4.5 Coverage gaps: test reachable paths, document defensive paths

**Rule.** When closing coverage gaps, distinguish three cases:
(a) **reachable via API call** → write a test; (b) **defensive guard**
against structurally excluded cases → remove, or explicitly name it as such
in the commit body; (c) **hook code for a not-yet-integrated
feature** → activate via `vi.mock(... importOriginal())` with a synthetic item
def (testable spec documentation instead of deleting).

**Why.** 100 % branch coverage for dead code produces mock theater or
strips protective code. Triage flow + Chimera example (heatPhysics
91.78 % → 97.26 %, last 4 branches = documented defensive guards):
Skill [27-coverage-gap-triage.md](skills/27-coverage-gap-triage.md).

#### 4.5b Coverage measures execution, not assertion — the mutation probe

**Rule.** A line counts as "covered" as soon as any test passes through it
— regardless of whether anything is ever asserted about its result. A feature
can have **100 % coverage and be 0 % tested**. Therefore NEVER infer
protection from a coverage number. The only reliable
proof is the **mutation probe**: switch off the effect and see whether the
suite screams.

| Signal | Significance |
|---|---|
| Coverage 0 % | Real gap. Reliable. |
| Coverage 100 % | **Says nothing** about protection. |
| Mutation survives | Real hole. Reliable. |
| Mutation dies | A guard exists. Reliable. |

**Why.** 2026-07-14: the per-tick XP grant (`energyStep`) was completely
untested. With the grant hard-disabled, **8482 tests ran green** —
after months of coverage checks and coverage improvements. The reason is
structural: the XP lines run in EVERY test that ticks the sim, so they are
excellently "covered" — and by definition never appear in a gap
triage (4.5), because it lists only NON-covered lines. The
housekeeping was blind to this class by design.

**How.** Per subsystem, list the **observable outputs** (sim core: `xp`,
`charge`, `heat`, `itemHP`, shield charge, fire timestamps, siphon net,
heat bomb), one mutation each that deletes exactly this effect (neutralize
the assignment / cut the body / setter no-op), full suite per mutation, restore
in `finally`, afterwards `git diff` = empty. Details + script pattern:
[skills/59-mutation-probe-over-coverage-percent.md](skills/59-mutation-probe-over-coverage-percent.md).
The mutation pass has been a fixed part of the
idle housekeeping since 2026-07-14 (Skill 42, pass 2).

#### 4.6 e2e tests (Playwright): pyramid + mechanics

**Rule.** Take the test pyramid seriously: effect/logic coverage belongs in
fast **unit/integration tests** (vitest, ~1–5 ms); e2e remains a
**thin smoke layer** for the UI path (browser render, click flows). Do
NOT "massively expand" e2e to check logic — that is slow (2–5 s/test) and
flake-prone. Per feature, a few representative e2e smokes; the
behavioral breadth is covered by the unit level (e.g. table-driven across all
variants). Detail plan: `docs/plans/archive/2026-06-08-event-card-test-expansion.md`.

**Conventions.**
- Selectors exclusively via `data-testid` (NOT text — i18n breaks
  otherwise). If a test anchor id is missing, add it in the component code (+ if
  needed `data-<state>` for states, e.g. `data-filled`).
- Canvas drag via `page.mouse.down()/move()/up()`, not `dragTo()`.
- Waiting strategy explicit (`waitForSelector`/`waitForFunction`/`toBeVisible`),
  no fixed `waitForTimeout` values (flake source).

**Chimera-specific state mechanics** (learned the hard way, msg 11676):
- **The running run is NOT debounce-saved while playing**
  (tick starvation: the 500 ms auto-save debounce is constantly reset
  by the state updates). Persistence only happens via the `pagehide` handler
  on reload/navigation. Whoever needs the running run in localStorage must
  therefore trigger a reload first.
- **Save-resume does NOT lead automatically into the console.** After a reload
  with a run save you land on the StartScreen; click `start-continue` to return
  to the running console. Reaching the console the regular way:
  `clickThroughRunStart` (start flow) — `tests/e2e/helpers.ts`.
- **State injection** (e.g. placing an item into a run slot): read the save →
  modify → set via `context.addInitScript(...)` → `reload`. A
  direct `localStorage.setItem` BEFORE the reload is overwritten by the
  `pagehide` save of the old page; `addInitScript` runs on the next
  navigation AFTER the pagehide save and before the app scripts, and thus wins.

**Why.** e2e is the topmost, most expensive pyramid level — it verifies "the
real browser renders + reacts", not the effect math. The
save/resume/injection mechanics are not obvious and cost a debug round
every time without a note; hence firmly documented. Skill:
`docs/skills/50-e2e-tests-erstellen.md` (Chimera only).

#### 4.6a jsdom render tests: `cleanup()` is mandatory + scope queries (isolate:false, since 2026-07-16)

**Rule.** Every jsdom test file (`// @vitest-environment jsdom` + `render(...)`
from `@testing-library/react`) MUST call `afterEach(() => cleanup())` — there is
NO auto-cleanup (the vitest config has no `globals: true`, so Testing
Library does not register its `afterEach` hook itself). Additionally: prefer
scoping DOM queries in the test to your own render container
(`within(container).getByText(...)` instead of `screen.getByText(...)`); only
portal content (`createPortal(..., document.body)`, e.g. the `InfoTip`
bubble) stays with `screen`.

**Why.** The suite runs with `pool: "threads"` + **`isolate: false`**
(vite.config.ts, boss-registry scan cost) → all files of ONE worker share
`document` and the React module state. Without `cleanup()` the rendered
tree remains in the shared `document.body` and **leaks into the next file
of the same worker**. Two failure modes, both **green locally / red in CI**
(depends on worker/file order):
- **`Found multiple elements`** — a leaker renders the same text (e.g.
  `taskHud.sectorGoal`), the victim finds it twice via `screen.getByText`.
  Exactly this on 2026-07-16 (`taskHudNameFallback` → `taskHudTooltips`, commit
  8f6427ab): fix on two fronts — `cleanup()` in the leaker + `within(container)`
  in the victim.
- **`NotFoundError: The node to be removed is not a child of this node`** — a
  later portal test crashes on unmount on the dangling React root (commit
  e4833605). `document.body.innerHTML = ""` alone is NOT enough — first
  `cleanup()` (React unmount), then optionally wipe the remaining DOM.

Reproduce:
`npx vitest run --no-isolate --no-file-parallelism <leaker>.test.tsx <victim>.test.tsx`
forces one worker + order; the honest counter-check remains the full
`npm test`, because vitest does occasionally reset the `document` depending on
file scheduling and the leak then only strikes under the real worker
packing. See `src/__tests__/CLAUDE.md` (hook tests) + memory
`project_chimera_jsdom_cleanup_isolate_false`.

#### 4.7 Content/sim invariant tests for authored content (since 2026-06-13)

**Rule.** For authored content (boss/run JSONs), unit tests with
SYNTHETIC inputs are not enough — a boss JSON can parse cleanly and still
BEHAVE wrongly. Two suite types for this:
- **Sim invariants** (`bossContentSimInvariants.test.ts`): loads EVERY real
  boss, ticks it (e.g. 90 s cold start) and checks physical invariants
  (no self-cook, `firesAtPlayer` emitter fires + survives, shields charge,
  generators produce).
- **Parse completeness** (`bossEventsParse.test.ts`): checks that EVERY event
  of EVERY boss parses (no silently rejected event). The parser discards invalid
  payloads only via `console.warn` + skips them — the sim/content suite otherwise
  does not notice (the item/boss still ticks).

**Why.** Both caught real content bugs that synthetic unit tests let through:
fireTimestamps `gameTimeMs` heat, stripped `xp` (level-0 shield does not charge),
r2-cold `energyEater` `zones`→`zone`. User question msg 12100 ("which test class
did we forget?") → exactly this: real authored behavior against invariants,
not just functions with constructed inputs. When adding new content types,
bring such load-and-tick guards along.

#### 4.8 Conservation/physics invariants instead of value asserts (solver/sim code)

**Rule.** For code with a **physical/structural invariant**
(energy, heat, pressure, flows, aggregation), test the **invariant** across
many/combined inputs — not just concrete output values of individual cases.
`expect(cableFlow).toBe(15)` enshrines exactly the wrong value as
"correct" when there is a bug; `expect(export).toBeLessThanOrEqual(netProduction)` trips over
EVERY topology that violates the conservation.

**Why.** Value tests often arise characterizing (observe current behavior
+ pin it down). If the implementation leaks, the test protects the bug.
Exactly that happened 2026-06-24: a `generator(sensor)` exported GROSS instead of
NET (delivered 20 out of 15 production). ~5200 tests did not catch it — they assert
values, and the leak looked functionally healthy (shield charged, sensor ran); even
the boss-sim invariants stayed green, BECAUSE the bug produced "working"
behavior. Only a display that SUMMED production vs. consumption made it
visible. The regression test is an invariant ("source exports ≤
effProduction − effDemand", 38 topologies) — cross-checked red on buggy code,
green with the fix.

**Mirror case — the same invariant, reversed (2026-07-12).** Not "too much
flow", but legitimate flow SILENTLY blocked: a siphon-hit non-source
node (ventilator) became a source, but `applyStorageBottlenecks` gave every
non-source node a hard export capacity of `0` → the surplus was counted as
"out of nowhere", the outflow zeroed; a cabled capacitor never charged. Again
~6800 tests did not catch it: the new contribution was correct in the reader AND
in `buildHydraulicNode`, but the third solver stage did not know about it — no test
checked end-to-end that a producer surplus REACHES a cabled storage.
Lesson: the conservation holds in both directions (`delivered ≤ produced` AND
"surplus arrives"), and a new contribution must be threaded through ALL stages
(reader → node build → bottleneck → charge) + secured by a system-level test.

**How.**
- Formulate the conservation explicitly: Σ output ≤ Σ input (+ storage
  discharge); in both directions — a connected surplus must also reach the
  storage/consumer.
- A new contribution must arrive through ALL pipeline stages (reader → node
  build → bottleneck → charge) — system-level test instead of per-function only.
- Property-/table-driven across combinations (components × wiring ×
  initial states), checking the invariant in EVERY one — across several ticks.
- Measure at the right point: the real flow/charge quantity (cable flow, charge
  delta), NOT a display aggregate metric that mixes pool draw + self-discharge
  (`totalActualConsumption` counts the entire internalDrain when the
  ChargeSink is full).
- **Verify the guard:** briefly revert the fix → the test MUST go red.

**Related principle (also outside tests).** Verify the INVARIANT, not
the surface pattern. Content migration (46 JSONs): the first regex pass on
`"zone":"torso"` also hit item `location`s + events; caught via structure check
(only objects with `maxHp` = real zone defs) → revert → anchor on `maxHp` +
count verification. Skill:
[docs/skills/52-test-conservation-invariants.md](skills/52-test-conservation-invariants.md).

#### 4.9 Diagnostics/display use the real computed values (no re-compute)

**Rule.** A display that "breaks down" an internal computation (energy
balance, resolve trace) must not compute the values ANEW — it emits the
real solver intermediate values at the point of computation (gated/optional so
that headless/sim stays allocation-free). Otherwise the display drifts from the
actual behavior and "explains" something wrong.

**Why.** A parallel re-compute is a second strand of truth that silently
diverges with every solver change. The energy breakdown display (msg 12895)
passes `breakdownOut` INTO the solver and reads back its real values — that is
what made the `generator(sensor)` leak visible in the first place, instead of
papering over it with a beautified parallel computation.

#### 4.10 Sim↔game divergence: the game is the reference

**Rule.** If headless sim/bot and the real game show different behavior for
the same mechanic, the GAME counts as correct (better tested) —
the sim is aligned, never the other way around. The first suspect is logic
that lives only in the React hook and never made it into the shared engine
(proven cases: zoneEnergyProduction, satellite hits, visibilityTier). An
RL retrain does not fix placement sectors (`planSector` = heuristic).
Details in
`docs/skills/63-sim-game-divergenz-spiel-ist-referenz.md` (Chimera only).

#### 4.x Ref flags that steer setState updaters belong in the queue

**Rule.** If a ref flag steers the behavior of setState updaters
(e.g. "merge the next pushes"), the flag flips themselves must run as
no-op updaters through the SAME setState queue — never synchronously beside it.

**Why.** React batches: updaters run later than the synchronous code that
enqueued them. A synchronous `end()` in the `finally` of a drop handler
would run BEFORE the still-queued drop push — the gesture would again fall apart
into several undo entries. Example: `useGameHistory.beginHistoryGesture/
endHistoryGesture/resetHistory` (2026-07-06, msg 14271). Twin of the
existing rule "no ref mutations in updaters without an idempotency guard"
(e2e undo flake): that one was about multiple invokes, this one is about order.

#### 4.y Effect cleanups capture their objects in the closure

**Rule.** A useEffect cleanup unsubscribes from the SAME object it
subscribed to — capture the object in a closure variable at mount time,
do not reach through the global `window`/singleton again in the cleanup.

**Why.** Test harnesses (and hot reload) can swap the global object between
mount and unmount: on CI the RTL auto-unmount ran AFTER
`vi.unstubAllGlobals()` — `window.speechSynthesis` was gone, the cleanup threw
(green locally, red in CI, because the afterEach order varies with the file
distribution across workers; 2026-07-06, msg 14250).

#### 4.v Couple reward/counter emission to the same guard as the state mutation

An unconditional `reward += X` (or stat counter) directly AFTER an idempotent
state transformer is a divergence trap: if the loop re-enters the transformer
(because a sub-state like `pendingLoot` blocks the advance), the reward
counts, the state counter does not (multi-pick boss bug 2026-07-08, engine.ts
if→while). Always bind the emission to the same guard as the mutation.

#### 4.w Signature-/parse-heavy persistence reads NEVER uncached in the hot path

`loadProfiles`/`chimeraSettings` reads go through HMAC-SHA256 + bounded parse
over the whole blob. Uncached in the frame path (even hidden: `getStaticTranslation`
→ settings → profile) this cost 75 % CPU at 8-12 fps (bug 2026-06-29, fix:
in-memory cache in profileStorage). Rule: cache such reads; when hunting perf,
first Chrome profiler bottom-up (self-time) instead of guessing collection sizes —
"slow from the start" + flat collections = recompute hotspot, not a leak.

#### 4.z Analysis tools as env-gated vitest files

**Rule.** Balance/statistics tools that need the real engine pipeline
(loader bootstrap!) are created as `*.analysis.test.ts` with
`describe.runIf(process.env.X === "1")` — they never run in the
normal suite, but start with an env flag without infrastructure of their own.

**Why.** Standalone scripts (tsx) fail on the CJS `require` of the
staticJsonLoader; the vitest environment brings the complete bootstrap
for free. Example: loot-odds Monte Carlo (`LOOT_ODDS=1`, 2026-07-06) —
pattern analogous to the slow suite.

---

### 5. Documentation

#### 5.1 Docs in the same commit as the code change

**Rule.** If a feature / a refactor / a fix changes the documented
reality, update the docs in the same commit (or in one
immediately following).

**Why.** Asynchronous docs never go stale by themselves — they go stale
because nobody maintains them along the way. Staying in the same commit
forces the author to look at the docs.

#### 5.2 One topic per file, clear linking

**Rule.** The `docs/` folder is organized by topics (architecture,
testing, deploy, one file per large subsystem). `CLAUDE.md` or
`README.md` as an index with a short description + link.

**Why.** Nobody reads a 3000-line monster doc. Small,
topic-focused files get read and maintained.

#### 5.3 Document the "why" behind design decisions

**Rule.** If an architecture decision cannot be read from the code,
it belongs in the docs: "Why pure reducers instead of OOP?
Why SQLite instead of Postgres? Why this state machine and not
another one?".

**Why.** Six months later, nobody remembers why X is the way it is.
Without docs, the temptation arises to optimize X away and relive the
same old pitfalls.

#### 5.4 No duplicates of code/git information

**Rule.** Never repeat content in docs that can be found in the code or
in `git log`. API signatures, file paths, commit
history — that lives in the original sources.

**Why.** Duplicated information drifts. The copy goes stale, readers
trust the wrong one.

#### 5.5 Doc conventions (binding since 2026-05-05)

Apply drive-by on the next touch of each doc file:

**Language:** English (repo-wide rule for this project — the Chimera
original mandated German here). Everything in this repository is English:
docs, code, comments, commit messages. Files that slip in another language
are translated on the next substantial edit.

**H1 style:** `# <Topic>` for subsystem docs.

**Test-coverage section:** if present, always `## Test-Coverage`
(not "Tests" or "Test-Abdeckung"). Subsystem docs without a test section
get at least a pointer at the end ("Tests in
`src/__tests__/<file>.test.ts`").

**Linking:**
- Internal refs to Markdown files: relative `.md` paths with anchor:
  `` `runs.md → Loot-Pools` `` (Chimera only).
- Plan refs as Markdown links, not as code spans: `[plan-name.md](plans/...)`
  instead of `` `plans/...` ``.
- Active plans: `docs/plans/...md`. Archived: `docs/plans/archive/...md`.
- Always redirect cross-refs when a plan is archived.

**File granularity:** subsystem docs should stay at 200-1500 lines.
Under 100 lines: check whether the section would be better placed in a
larger file. Over 2000 lines: check whether a standalone
sub-topic can be extracted (see as an example: `tasks.md`
NPC subsystem → `npcs.md`).

---

### 6. Commits and git

#### 6.1 Conventional-commit prefix

**Rule.** The subject line starts with `feat|fix|refactor|docs|test|chore|
perf` followed by an optional scope and a colon. Consistent across the
whole repo.

**Why.** Makes changelogs, filters and bisect trivial. Tools like
semantic-release leverage it.

**Example.**
```
feat(tasks): section SuccessCondition replaces per-task holdMs
fix(runs/pruefstand): no start clause fulfilled at t=0 anymore
refactor(hud): circuit-diagram rendering for sector condition
docs: dev-analysis.md — approach for commit statistics
```

#### 6.2 The commit message explains why + what

**Rule.** Subject line (short, precise). Blank line. Body with context:
which problem, which solution approach, which consequences. The body is
optional for trivial commits, but recommended from 10+ lines of diff.

**Why.** In 6 months, "fix bug" is worth nothing. "fix: sticky
fulfilled canceled the debounce → sector never ended" gives you the
entire debugging result back.

#### 6.3 One logical step = one commit

**Rule.** A commit should contain exactly one conceptual change.
Two independent fixes = two commits, even if they were noticed in the
same context.

**Why.** Revert granularity (see 2.2). Easier code review.
Clearer bisect.

#### 6.3a Commit IMMEDIATELY per step, do not collect at the end

**Rule.** In multi-step tasks (a plan with N phases, a bug round with
several fixes, a refactor in tranches), **every completed
individual step is committed directly** — not collected at the end of a day
or a multi-phase plan.

**Why.** Batch commits lose the phase granularity that 6.3
and 2.2 are meant to protect. If three phases are implemented in one go and
then committed together, bisect/revert no longer helps — and the
push-per-phase from 1.5 (multi-phase autonomy) also becomes impossible.

**Behavior.**
1. Step N: implement → `tsc + tests + build` green.
2. **Immediately** `git add <files> && git commit -m "..."`.
3. Push (if non-trivial; see 1.4).
4. Continue directly with step N+1.

**Anti-pattern.** Working through seven steps, then one gigantic
`feat: damping + count + tests + docs` commit at the end. Bisect finds nothing anymore.

#### 6.4 Do not skip hooks, do not amend commits

**Rule.** Pre-commit hooks exist to catch errors. If one
fires, fix the error — not `--no-verify` or `--no-gpg-sign`.
After a hook fail, make a NEW commit, do not amend (the
faulty commit never existed).

**Why.** Hooks are the last protection against broken things in the repo.
Bypassing them undermines team trust. Amending already-pushed commits
destroys the git history of other branches.

#### 6.5 No `--force` pushes to shared branches

**Rule.** `git push --force` to `main` or other shared branches
is forbidden — even if your own branch would merely be "prettier".

**Why.** Destroys other people's work. Done once, painful
forever. OK for your own feature branches; never for shared branches.

#### 6.6 A push carries ALL unpushed commits along

**Rule.** Before every push, check `git log @{u}..HEAD --oneline`: the
push-approval rule applies to the ENTIRE commit stack — a "harmless"
docs push would otherwise carry along unapproved commits. Details in
[docs/skills/57-push-carries-the-whole-stack.md](skills/57-push-carries-the-whole-stack.md).

---

#### 6.7 Branch workflow: dev / topic branches / main (since 2026-07-18, test phase)

**Rule** (Owner msg 15495/15497). main deploys automatically to REAL
testers — work never happens directly on main anymore:

- **Every plan starts with its own topic branch on dev**
  (`topic/<plan-slug>`); during the plan, commits go only there.
  **Topic pushes are free** (no CI/deploy effect).
- **Acceptance = `git merge --no-ff topic/<plan>` into dev** — the plan
  stays in the history as a unit. **dev pushes only after approval**
  (the earlier test/docs-free exception is superseded); small stuff
  outside of plans is committed directly on dev, pushed after approval.
- **Release = `merge dev → main` exclusively on explicit
  owner instruction** (+ `release/<n>` tag) — the push deploys immediately. The
  dev→main merge MUST be **`--ff-only`** (Owner msg 15904: keeps
  `BUILD_COUNT` on main identical to dev — a merge commit produced
  an off-by-one against release-notes names + tag; details Skill 64 rule 5).
  Do not confuse: topic→dev remains `--no-ff`.
- **Locally, dev is always checked out** (acceptances within a plan naturally
  deploy the topic state, afterwards dev again). CI (ci.yml) runs on
  main AND dev pushes.

In full: `docs/skills/64-branch-workflow-dev-topic-main.md` (Chimera only).

---

### 7. Deployment

#### 7.1 Reproducible build/deploy procedure

**Rule.** Deploy is a documented chain of steps —
ideally a single script or an explicit CLAUDE.md
section. No secret handshakes, no manual sequence you have to
memorize.

**Why.** So that someone else (human or agent) can do it exactly the
same way tomorrow. Every "you just have to know that" secret is a
bus factor of 1.

**Chimera standard:** the 5-step commit gate from §4.3, then build +
restart as an `&&` unit:
```
npx tsc --noEmit && npm run lint && npm test -- --run && npm run e2e \
  && npm run build && systemctl --user restart chimera
```
Build and restart belong together (build only = new code in the filesystem,
old code in the service; restart only = the service serves the previous state) —
details in Skill `35-reproducible-deploy.md` (Chimera only)
+ `36-build-vor-restart.md` (Chimera only).

#### 7.2 Verify the restart visibly

**Rule.** After a restart, briefly call `systemctl status` or
`curl localhost:<port>/health` to make sure the
new process is running. No "I typed restart" without
confirmation.

**Why.** Build errors that became runtime errors can only be
discovered this way before the user sees them.

**For remote/static deploys, verify the CONTENT, not just the status
(since 2026-06-30).** HTTP 200 only says "something is being served", not "the NEW
build". After a GitHub Pages deploy, check the **actually delivered artifact
content**: the new build hash, or exactly the changed piece in the bundle
(e.g. fetch the content-hashed chunk via `curl` + grep for the expected string).
Examples this week: confirmed that the privacy-policy "15+" version is live,
that the plain-text e-mail is NOT in the delivered bundle and that the OFL license
text is in the chunk. **Lazy chunks** are not referenced in `index.html` → fetch
them directly via their content hash (Vite hashes content-based → the local chunk
name often matches the remote chunk). Only report "live + correct" once that has
been checked.

#### 7.3 Production deploy = release merge with announcement (revised 2026-07-18)

**Rule.** The public deployment (GitHub Pages, real testers) hangs
off the main branch. Since the branch workflow (§6.7), the only path there
is the **release merge dev→main on explicit owner instruction**; the
earlier rule "production-code push after asking" has been absorbed into it.
Local deploys (build + restart) serve dev, or the topic state
of an ongoing acceptance.

#### 7.4 Multiple deployment targets: local build ≠ delivered build (since 2026-06-29, 4-target model 2026-07-01)

**Rule.** When different deployments contain different amounts (stripped
content, hidden dev UI, different base path), this is solved via **a single
target enum** (`VITE_BUILD_TARGET` → `getBuildTarget()`, targets `local|test|demo|full`)
plus a **pure SSOT** (`src/buildTargetConfig.ts`: base path + run sets per target)
— NOT via scattered ad-hoc flags. Two reduction paths: **UI predicates**
(`isPublicBuild()`/`isDemoBuild()`/`showBugReportButton()`) for gating in the bundle and a
**content strip AFTER the build** (`scripts/strip-build.ts <target>`, pulls the run list
from the SSOT) for data/assets. The local `npm run build` contains everything (`target=local`
→ `"all"` → no strip); the strip runs only in the deploy.

**Why.** Unreleased content must not be retrievable by URL, but dev
needs the full scope. A target enum instead of N flags, because otherwise every new
difference (base path, CTA, bug button) would need another independent flag, and
they can contradict each other. **Follow-up traps:** (a) what the strip removes must
not be hard-referenced by the app → see 3.3c (manifest-driven instead of a fixed
list); (b) the **base path must hang target-aware at ONE place** (SSOT `baseForTarget`),
otherwise `vite base`, `index.html`, manifest, service worker and precache drift apart;
(c) a **deploy gate** aborts when strip-mandatory content is missing or the hardening
(no source maps, CSP `<meta>`) is violated. Reference: CLAUDE.md → "Build-Varianten",
`docs/plans/2026-07-01-vier-deployment-targets.md`, `docs/security.md`.

#### 7.5 Public/commercial launch: mandatory pages + licenses (since 2026-06-30)

**Rule.** As soon as the app is publicly reachable (and all the more so
commercially), the following belong to it: **legal notice + privacy policy**
(DE, § 5 DDG / GDPR), the **complete open-source license texts** of the shipped
components (MIT permission notice per package, the full OFL text of bundled
fonts, the Apache text) and **age-appropriate/correct claims** (privacy-policy age
classification, store description).

**Why.** A legal obligation, not a nicety — and it ripples: a target-audience change
(~12 → 15+) had to be pulled through the privacy policy, README and docs. What is
shipped bundled (runtime libs, fonts) needs its license text along; pure dev/build
tools do not. Not legal advice — but the *technical* mandatory display (pages +
license texts) is implementable and belongs before the launch. Reference: `docs/legal.md`.

#### 7.6 Pilot deploy before mass rollout / for critical + visible changes (since 2026-07-01)

**Rule.** For **critical** or **visible UX/design changes** (a new
interaction pattern, tooltip/popover looks, layout) — and especially for **mass
migrations** pulling the same pattern through many places — first a **pilot**
at ONE place is **deployed locally** (`npm run build && systemctl --user restart chimera`,
check HTTP 200) and shown to the user for testing. Only after their approval of look &
feel does the rest follow. **Deploy ≠ push:** for viewing, the local deploy is enough; a push
is separate (prod code → ask, 7.3).

**Why.** The main target device is mobile; how something feels (tap area, bubble
position, underline weight) is only visible in the running game, not in code or
in tests. Building 20 places in the wrong style is expensive rework — a pilot costs one
deploy cycle and prevents the dead end. Skill
`54-kritische-changes-lokal-deployen-testen-lassen.md` (Chimera only), memory
`feedback_local_deploy_test_before_rollout`.

---

#### 7.7 Abort background processes cleanly: process group + verification (since 2026-07-17)

**Rule.** Abort running background jobs (vitest, sim, Playwright) via the
process GROUP (`kill -- -<pgid>`), not via `pkill` on the
command pattern — forked workers often do not match the pattern, survive
as orphans and keep computing. Afterwards ALWAYS verify via
`ps -eo pid,etime,%cpu,cmd --sort=-%cpu | head` that nothing
from the run is left. With `pkill`/`pgrep -f` in the Bash tool, use the
bracket trick (`"[p]attern"`), otherwise the pattern matches its own wrapper and
kills the task itself (exit 144).

**Why.** Incident 2026-07-17 (msg 15351): an orphaned vitest fork worker
burned one core at 100 % for 13.5 h. Details in
[docs/skills/61-kill-background-processes-cleanly.md](skills/61-kill-background-processes-cleanly.md).

---

### 8. Memory / context

#### 8.1 Store user preferences permanently

**Rule.** When a user expresses corrections or preferences
("no emojis", "commit messages in German", "never silently change
tests"), persist that in a memory system or a
team guideline. Do not ask anew every time.

**Why.** Saves cycles. Shows that feedback is taken seriously.

#### 8.2 Read the context before you ask

**Rule.** Before asking back: go through CLAUDE.md, documented repo
conventions, memory files and relevant files. Ask only when the
answer is not retrievable.

**Why.** Repetitive clarifications are expensive. Well-prepared
questions are targeted and respectful.

#### 8.3 Verify before acting from memory

**Rule.** Memories can go stale — before acting on "X exists in
file Y", briefly check against the current state (`grep`,
`ls`, file read). A memory is a snapshot, not a source; "it used to be
that way" is not enough. Details: Skill
[39-verify-memory.md](skills/39-verify-memory.md).

#### 8.4 Verify analysis/audit reports before implementing them

**Rule.** Findings from sub-agents / external analyses are also checked
against the current code before implementation (a targeted `grep` or a
30-line read of the named location). If the finding is wrong:
inform the client or re-identify the real potential —
do not blindly "deliver".

**Why.** Reports are helpful, but not authoritative. Example
(2026-04-27): "pressureSolver has no early break" — the break had long
been there; the real lever was the epsilon calibration (1e-6 → 1e-4).
Procedure details: Skill
[39-verify-memory.md](skills/39-verify-memory.md).

---

### 9. Security and secrets

#### 9.1 No secrets in the repo

**Rule.** `.env`, `credentials.json`, API keys do not belong in
commits. A pre-commit hook or `.gitignore` enforces that; when in doubt,
better to ask once too often than once too little.

**Why.** Secrets pushed once are compromised, no matter how
quickly you retract the commit. Roles and keys must then be
rotated.

#### 9.2 Specific files instead of `git add -A`

**Rule.** `git add <explicit-file>` instead of `git add -A` or
`git add .`. The latter picks up unwanted temporary files, builds,
secrets.

**Why.** Small typo, big damage. Explicit staging is
an extra thinking step that catches mistakes.

---

## Part B — Workflow (end-to-end)

The typical flow of a single task, applying the principles from Part A.

### Step 0 — Understand the task

Read the task twice. Identify:
- What is the concrete goal?
- Are there unspoken assumptions?
- Which parts of the code are affected?
- Are there ambiguities (scope, options)?

On ambiguity: Part A.1.1 applies — ask with named options
and **wait** for the answer. No speculative implementation.

### Step 1 — Gather context

Read deliberately:
- The relevant source files (do not guess)
- The docs for the subsystem (if any)
- The recent commits in that area (for current conventions)
- Memory / team guidelines for user preferences

Note the current state briefly — mentally or in tasks.

### Step 2 — Record the plan

For non-trivial work: communicate a short plan to the client
(or in a task list). It has three jobs:
- Shows that you understood
- Gives the client the chance to correct before time is burned
- Cuts large activities into verifiable phases

**Plan lifecycle (binding since 2026-05-05):** plans that are stored as their own Markdown file under `docs/plans/` follow this lifecycle:

0. **Filename with a leading date (binding).** `docs/plans/*.md` MUST begin with `YYYY-MM-DD-` — e.g. `2026-05-24-item-consolidation-audit.md`. Sorts chronologically + shows age at a glance.
1. **Status header mandatory.** First or second line after the H1: `Status: Entwurf | In Arbeit | Umgesetzt | Deferred | Archiviert`. For "Umgesetzt" + "Archiviert" additionally the implementation commit hashes ("`commits abc123/def456`") or a reference "(commits see `git log --grep ...`)".
2. **Plan-end ritual** as soon as the implementation is complete:
   - **(a) Extract the substance.** If the plan contains durably useful conceptual material (tables, rationales, API schemas), it moves into the matching `docs/` entry (e.g. `architecture.md`, `property-system.md`, `items.md`). Plan status set to "Umgesetzt + substance extracted to `<file>`".
   - **(b) Archive.** If the plan was essentially a step-by-step guide whose traces are in the code, `git mv` to `docs/plans/archive/` with an updated status header. **Never** archive before (a) has been cross-checked — otherwise a two-pass operation with a git gap arises.
   - **(c) Delete.** Only if the plan contained pure implementation to-dos without lasting value (rare — usually the plan document is too weighty for "without value").
3. **Periodic audit every 4 weeks or after big sprints.** Three questions per plan: (i) is the status header still correct? (ii) does implemented substance still live in the plan that would belong in `docs/`? (iii) should the plan be archived?

**4. Phase 0 — coverage pre-check (binding since 2026-05-29).** As soon as the scope and phase plan are set and BEFORE the first code change of phase A happens:

- For every file/function the plan modifies, review the existing test coverage and classify gaps per Part A.4.5 (`coverage-gap-triage`, Skill 27) (reachable / defensive / dead).
- Reachable paths without tests get characterization tests **before** phase A that pin down the *current* behavior. Style: behavioral test, not structural test. These tests rest on today's implementation and become the regression boundary against which the planned change must measure itself with respect to old behavior.
- Phase 0 is documented as its own plan phase (header `Phase 0 — Coverage-Pre-Check`); its commit carries `test(<area>): pre-impl characterization` and is cleanly separated from the refactor commit.
- When *not* needed: pure bug-fix patches (the regression test per A.4 replaces the pre-check); the function is already strongly covered (> 80 % branch + visible behavioral tests); pure doc/style changes.

Rationale: tests written AFTER a change are unconsciously oriented on the new implementation — they check what the function does *now*, not what it *should* do. Characterization tests before the implementation turn a "refactor with confidence" into a "refactor with a net" and immediately expose silent contract deviations.

Operational details in `docs/skills/43-coverage-before-implementation.md`.

Convention for cross-refs: active plans as `docs/plans/...md`, archived ones as `docs/plans/archive/...md`. When you archive a plan, redirect the cross-refs in code/docs to the new path at the same time (otherwise dead links arise).

### Step 3 — Implementation

Stick to Part A.2.1 (scope) and A.3 (code quality).

While working:
- Prefer editing existing files (A.3.1)
- Keep the diff size manageable — if it grows, sort it into
  phases (A.2.2)
- No emojis, no over-documented strings, no unrequested refactors

### Step 4 — Add tests

For new logic: at least one unit test that would check the central
invariant. For fixes: a regression test that fails without the fix.

Never adjust existing tests without reason (A.4.2).

### Step 5 — Local verification

The 5-step commit gate from A.4.3:

```
npx tsc --noEmit && npm run lint && npm test -- --run && npm run e2e && npm run build
```

All must be green. On red: fix, do not commit. For pure
doc/test-file commits without app-code changes, `npm run e2e`
may be skipped (A.4.3).

### Step 6 — Update the docs

If the change alters the documented reality (architecture,
APIs, configuration, deploy), update the affected docs (A.5.1).
In the same commit.

### Step 7 — Commit

Subject + body in conventional-commit format (A.6.1, A.6.2). Explicit
`git add <files>` instead of `-A` (A.9.2). The commit body explains the why
— especially for bug fixes: what was the error, what was the cause,
how was it fixed.

Example:

```
fix(successCondition): sticky fulfilled + run end for boss-less runs

Two regressions in the sector-completion path:

1) The condition evaluator was not sticky — as soon as a task lost
   currentlyMatched during a tick, heldMs reset to 0 and the
   status flipped from "fulfilled" back to "ongoing". That could
   cancel the 1.5 s debounce in the transition effect.
   Fix: a clause stays fulfilled after reaching the hold time.

2) Boss-less single-section runs never ended the run.
   Fix: second path for "last section without a boss + fulfilled".

Regression tests in successCondition.test.ts (2 new cases).
```

### Step 8 — Deploy (if relevant)

For live services: run the documented deploy procedure (A.7.1),
then verify the status (A.7.2).

### Step 9 — Push (after approval)

`git push origin <branch>` — but **only** with the client's explicit
approval, unless agreed otherwise (A.1.4). A "pushed?"
in 2 minutes is better than an unwanted push.

### Step 10 — Report back

Short + concrete (A.1.2):
- What was done (1 sentence)
- Commit hash / PR link
- What is still open (if anything)
- If waiting on review / test: a clear "please test"

### Special workflow: RL training iterations — analysis sim first

For autonomous training-improvement loops (retrains, continuation cycles,
manager training), the owner workflow applies (msg 16204/16209, 2026-07-26; Skill
`66-trainings-iteration-analyse-sim-zuerst.md` (Chimera only)):

1. **Eval measurement point** on the final model (deterministic, 12 seeds) — never
   judge from stochastic rollout logs.
2. **Diagnostic SIM before every adjustment:** on misbehavior, run a simulation
   with analysis at the problem spot (spy bot/trace/recording) —
   which actions does the bot really choose, what does the mask offer, what does
   the step fail on. Evidence instead of conjecture.
3. **Decide the adjustment on the basis of the analysis** (reward wiring, capability,
   mask, content — whatever the analysis supports), implement, **commit**.
4. Start the next training; the machine may be kept fully loaded
   continuously. Stop: target gate reached (report immediately) · plateau (2 continuations
   without eval progress) · agreed end of night (morning summary).

Rationale: premature parameter tuning against training variance burns
iterations; diagnostic runs find foundational errors (examples: zone
geometry parity bot↔engine, buffer-source capability, weapon-chain
curriculum — all from analysis sims, none from tuning).

---

## Part C — Four-phase cycle (prescriptive recommendation)

Parts A and B describe how a single task is implemented.
This section prescribes the **multi-day rhythm**: four
clearly delineated phases run through cyclically, with
time frames oriented on the actual commit history of the
Chimera project.

### C.1 Overview

One cycle yields a lean, consolidated piece of software:
**features are built, tests cover them, docs are consistent, the code is
architecturally clean.** Afterwards the next cycle starts with
phase 1.

| Phase | Focus | Recommended duration |
|---:|---|---|
| 1 | Feature addition | **1–3 days** |
| 2 | Test-coverage backfill | **~½ day** |
| 3 | Doc consistency check | **~2 hours** |
| 4 | Architecture / performance / redundancy analysis + refactor | **1 day** (rarely 2) |

**Full cycle:** roughly 3–5 days of work spread over ~1 calendar week.
That is in the range where in Chimera ~8–10 days lay between two
hardening days (04-11 → 04-19).

### C.2 Phase 1 — Feature addition

**Duration:** 1–3 consecutive working days.
**Goal:** implement a thematically coherent feature package.
Tests and docs flow along **within each commit** (baseline from
Parts A.4.1 and A.5.1). The further phases are backfill phases, not
a substitute for the per-commit duty.

**Typical activities:**
- New features, new modes, new UI components, new subsystems
- Bug fixes discovered along the way
- Per-commit tests for new logic (baseline)
- Per-commit doc updates when architecture/APIs are touched

**Boundary signal:** after 3 consecutive days with a feature-dominant
profile (>40% of the commits are `feat:`), switching to phase 2 is
mandatory. Chimera never ran purely on features for more than 3 days
in a row — the signals for this are solid.

**Chimera evidence:** feature burst 04-14 to 04-18 (3 intensive days
with 73 + 9 + 55 = ~137 feature commits), likewise 03-25/26, 03-29/30,
04-06/07, 04-18. All within the 1–3-day window.

**Exit criterion:** phase 1 ends when
(a) the planned feature package is functionally complete, OR
(b) the 3-day limit is reached, even if more was planned.

### C.3 Phase 2 — Test-coverage backfill

**Duration:** ~½ working day (2–4 hours).
**Goal:** close gaps that accumulated in phase 1 despite per-commit
tests. No new feature code.

**Typical activities:**
- Check the coverage report or `npm test -- --coverage`
- Identify edge cases for new modes that were not in the
  per-commit tests
- Regression tests for bugs discovered in phase 1 but only
  shallowly repaired
- Integration tests across several new subsystems
- Clean up test utilities, consolidate redundant test setups

**Exit criterion:** coverage for the paths introduced in phase 1
is close to gap-free. No absolute % value — what counts is
that the next phase can build on a green suite.

**Chimera evidence:** in the observed history, test backfills happened
interleaved with phase 4 (e.g. 5 test commits on
04-14, 6 on 04-16, 4 on 04-19). The separation recommended here
makes the test work more visible and gap-free.

### C.4 Phase 3 — Doc consistency check

**Duration:** ~2 hours.
**Goal:** ensure that docs/ reflects the new reality.
No code — docs only.

**Typical activities:**
- Check all subsystems changed in phase 1 against their docs/*.md
  file
- Add cross-references if new concepts touch other documents
- Delete or mark stale passages
- Check CLAUDE.md / README / architecture docs for currency
- If the phase uncovers gappy docs, mini-fixes directly there

**Exit criterion:** no discrepancies between documented and
actual behavior. Doc examples match current
code behavior.

**Chimera evidence:** 40 `docs:` commits over the whole period,
often concentrated in the hardening days (7 on 04-18, 5 on 04-19).
The separation recommended here turns scattered doc backfill into
a deliberate consistency session.

### C.5 Phase 4 — Architecture / performance / redundancy + refactor

**Duration:** 1 working day (rarely 2).
**Goal:** pay down technical debt accumulated in phase 1.
No functional changes — structure/perf only.

**Procedure:**
1. **Analysis step (morning):**
   - Obtain a hotspot report: `chimera-complexity-analysis.md`
     or equivalent — files > 500 LOC, unhealthy
     dependency directions, duplicated logic.
   - Performance measurements on hot paths (browser profiler,
     `console.time`, render counts).
   - Mental architecture scan: layer cleanliness (reducers pure,
     hooks thin), dependency direction, clarity of state flow.
   - **From ~10 k LOC or at the first "real" architecture audit**:
     instead of a mental scan, parallel subagents — 4-6 audit axes,
     each agent returning 600-800 words, synthesis in the plan doc. See
     [Skill 47](skills/47-architecture-audit-with-subagents.md). Lifts
     you to senior-audit level without token pain (madge cycles,
     SOLID/DRY/hexagonal checks, quick-win/mid/big roadmap).
   - List the findings (short note per finding).

2. **Implementation step (afternoon):**
   - Refactor commits with the conventional prefix `refactor:`
   - One finding per commit (Part A.6.3)
   - After every commit `tsc + tests + build` green (A.4.3)
   - Split larger rebuilds into phases (A.2.2)

**Exit criterion:** all findings noted in the analysis are
either implemented or explicitly deferred to "later"
(then in `docs/future-improvements.md`). Tests green.

**Chimera evidence:**
- 04-11: 33 commits, 23 of them `refactor:` — after a
  10-day mixed phase.
- 04-19: 67 commits, 27 of them `refactor:` — directly after the
  feature burst 04-14 to 04-18.
Both show that the refactor volume of a hardening day
lands in the range of 20–30 commits and is doable in one working day.

### C.6 Phase transitions (gating)

Between phases stand clear gates — not simply "carry on":

| Transition | Gate |
|---|---|
| Phase 1 → 2 | Per-commit suite green. Feature package functionally complete or 3-day limit reached. |
| Phase 2 → 3 | `npm test` shows no coverage gaps introduced in phase 1 anymore. |
| Phase 3 → 4 | A quick doc spot-check against a randomly chosen subsystem yields no discrepancy. |
| Phase 4 → 1 | All analysis findings implemented or booked into `future-improvements.md`. Build + tests green. |

If a gate cannot be held (changes too big, unclear, etc.),
the rule from Part A.2.1 applies: pull back the scope, do not overstretch it.

### C.7 Cycle frequency

A full cycle takes roughly 3–5 working days (~1 calendar week at
part-time engagement). That yields a **hardening cadence of
~7–10 days** — exactly what the Chimera history showed
(04-11 → 04-19: 8 days apart).

Longer cycles (>2 weeks without phase 4) create technical debt
that is harder to pay down. Shorter cycles (<4 days) become
unprofitable through the overhead of phases 2–4.

### C.8 What is NOT part of the cycle

- **No bug fixes as a phase.** Bugs are fixed in the phase in which
  they show up — phase 1 (feature) or phase 4 (refactor). An
  isolated bug-fix sprint is not a component of the cycle.
- **No release phase.** Deploy runs continuously (Part A.7); there
  is no code freeze at the end of a cycle.
- **No kickoff/retro meetings.** The cycle is a working rhythm,
  not process ceremony.

### C.9 Nightly housekeeping (daily) — coverage + mutation probe + doc + skills drift

The four-phase cycle (C.1–C.7) is the **weekly cadence** for the
substantial hardening. Complementary to it, a small four-pass runs
**daily, whenever the task queue is empty** (typically at night or in
idle phases) — autonomously, without a user decision,
because there are no conflict risks and the effort is small:

1. **Test-coverage pass.** Run the coverage report, triage gaps per Skill 27
   (reachable → behavioral test / defensive → marker /
   dead → delete). **First check whether a report was produced at all**
   (`coverage/coverage-summary.json`) — vitest only writes it on a
   green run, a single timeout makes it drop out and the pass then looks
   like "nothing to do". Exactly like that it ran into the void for two
   weeks in 2026-08 (housekeeping finding 2026-08-21).
2. **Mutation pass** (since 2026-07-14, user assignment msg 15097; §4.5b
   + Skill 59). Switch off central state writes one at a time,
   full suite — if it stays green, a guard is missing → add a test.
   Always roll back the mutations (`git diff` = empty).
3. **Doc-drift pass.** Check all files under `docs/` (recursively) and all
   `CLAUDE.md` files against the current code state; also
   **regenerate the generated doc blocks** (`npm run docs:items` +
   `npm run docs:deployment`, Owner msg 15459 — changed output =
   drift, commit it along). READ the diff while doing so: a generator that
   reads a directory can suck up machine-local artifacts and write them into
   a committed doc (Skill 53, follow-up trap 3). A link walk
   over all `*.md` belongs in the same pass — doc moves break the
   relative links IN the moved file, not only those pointing to it. Commit fixes immediately (one commit per file,
   Skill 32).
4. **Skills/practices drift pass** (since 2026-07-06, user spec
   msg 14133). Check `docs/skills/` + this umbrella document against the
   learnings accumulated since the last pass, incl. the
   **memory→skills/GDP full reconciliation** (user assignment msg 15430,
   2026-07-17). Every change synchronized threefold: skill file +
   skills index + umbrella document.

Operational full details of all four passes (triggers, step lists,
anti-patterns): Skill
`42-housekeeping-coverage-doku-drift.md` (Chimera only).

**Why daily instead of only weekly.** Drift, unlike
architecture debt, is linear in time — code changes every day,
docs age every day. If the correction happens on the same day as
the code change, it is trivial; if it accumulates over 1–2
weeks, it becomes its own hardening subproject. Daily housekeeping
pushes the drift half-life from weeks down to a day and
thereby relieves phases 2 and 3 of the four-phase cycle.

**Branch context (since 2026-07-18):** housekeeping commits are small stuff
→ commit directly on dev; the push to origin/dev needs an
approval (§6.7) — commit locally at night, report the push bundle in the morning.

**Autonomy.** Fully autonomously triggerable, without a user decision. As soon as
the user kicks off tasks again, the pass is cleanly interrupted at the next
commit boundary — which is why the per-step commit
rule (`32-commit-per-step.md`) is doubly important here.

---

## Part D — Observed working rhythm from the commit history

Parts A and B describe how a single task is implemented;
Part C the prescriptive four-phase cycle. This section
describes how tasks have **actually strung together** over longer
periods — grounded in the real commit history
of the Chimera project (03-18 to 04-27, 1055 commits excluding merges). Not a
target state, but an empirical baseline that the four-phase
cycle from Part C is oriented on.

### D.1 Mini cycle — per change (hours)

The smallest unit. Per change, one commit that is already complete:
feature code + accompanying tests + doc backfill if needed. That means:
**tests and docs are not phases but a component of every single
commit**. In the project, 5–30 such mini cycles per day are common,
with outliers up to 70+ on intensive feature days (see D.3).

See also Part A.5.1 (docs in the same commit) and A.4.1 (tests
accompany features/fixes).

### D.2 Micro rhythm — feature bursts (1–4 days)

The second-smallest unit. A thematically coherent
feature package — typically a new subsystem, a new
condition grammar, a new UI component — stretches over
1–4 consecutive days with a feature-dominant commit profile
(at least 35–40% of that day's commits are `feat:`).

**Observed feature bursts in Chimera:**

| Period | Duration | Commits/day | Content (in brief) |
|---|---:|---:|---|
| 03-25/26 | 2d | 10, 10 | First feature wave after bootstrap |
| 03-29/30 | 2d | 7, 15 | — |
| 04-06/07 | 2d | 11, 8 | — |
| 04-14 to 04-16 | 3d | 73, 9, 55 | Big feature package (~137 commits) |
| 04-18 | 1d | 48 | Feature follow-up |
| 04-22 to 04-25 | 4d | 37, 53, 34, 61 | Zone-HP system + RL-bot gap closure + heatExtractor (~185 commits) |

**Average:** ~2 days per feature burst, max. 4 consecutive days
(04-22 to 04-25, the longest burst observed so far). Long pure
feature sequences beyond that do **not** exist — fixes and refactors
mix in between.

### D.3 Macro rhythm — hardening days (1–2 days)

After a larger feature package, a **hardening day** typically follows:
a concentrated day with a refactor-/perf-/test-dominant profile that
pays down the technical debt accumulated in the feature phase.
Typically 15–30 refactor commits on a single day, often
complemented by a perf pass and a test-coverage backfill.

**Observed hardening days in Chimera:**

| Date | Refactor commits | Total commits | Trigger |
|---|---:|---:|---|
| 04-11 | 23 | 33 | After a 10-day feature/mixed phase |
| 04-19 | 27 | 67 | Directly after the 04-14 to 04-18 feature burst |
| 04-26 | 16 | 50 | Architecture clarification (types split, handler maps, RNG injection) |
| 04-27 | 8 | 46 | Perf/test/doc backfill (heat constants, ResolveCache, coverage) |

Typical activities on a hardening day:
- **Architecture review:** check whether new features are properly
  embedded in existing subsystems (layer cleanliness,
  dependency direction, state flow). Examples from 04-26:
  handler-map families (NPC, tasks, effects), types-monolith split,
  RNG injection through boss-event activation.
- **Performance pass:** measure hot paths, eliminate unnecessary
  re-renders / hooks, memoize expensive operations. Examples from 04-27:
  per-tick `ResolveCache` (WeakMap) for `resolveTree`, pressureSolver
  iteration and epsilon calibration.
- **Redundancy elimination:** merge duplicated logic, extract helper
  functions, fuse parallel data structures. Examples from
  04-27: centralize heat-sim constants in `heatConstants.ts`,
  7 inline `apply*` functions moved out of `useHeatSimulation` into their own
  pure modules under `src/run/` (982 → 637 LOC).
- **Test-coverage backfill:** close gaps that were not covered during
  the feature phase. Examples from 04-27:
  zoneHealAll defensive guards, propertyBag zeroDemand paths via
  ItemDef mock, heatPhysics branch 91.78% → 97.26%.
- **Doc consistency check:** go through docs/ to see whether new concepts are
  described in the relevant subsystem files. In Chimera, docs/ is laid
  out so that every subsystem has its own Markdown file
  (`tasks.md`, `game-flow.md`, `runs.md`, etc.). Additionally, new
  insights flow into `good-development-practices.md`
  (see commit `0f3bc91` of 04-27).

**Rule of thumb:** one hardening day per 5–10 feature days. Observed
intervals: 04-11 → 04-19 (8 days), 04-19 → 04-26 (7 days),
04-26 → 04-27 (1 day — 04-27 is actually the second half
of a two-day hardening sequence). That fits the loose
weekly rhythm, but is not enforced.

### D.4 What is NOT done (deliberately)

The Chimera rhythm forgoes some common structures:

- **No dedicated test days.** Tests arise in the same commit
  as the behavior to be checked (Part A.4.1). The test-coverage
  backfill happens within the hardening days, not as a
  separate activity.
- **No dedicated doc days.** Docs are part of every commit that
  touches them (A.5.1). Consistency reviews happen along with the
  hardening day.
- **No sprint cycles.** There are no 1- or 2-week sprints with
  kickoff/retro. The work flows; the rhythm emerges from the
  alternation between the urge to build features and the pressure of
  accumulated debt.
- **No pre-release lockdown phases.** There is no dedicated
  "code freeze" before a deploy — the service is restarted several
  times every day (see A.7).

### D.5 Triggers for a hardening day

How do you recognize that the next day should be a hardening day?
Observed signals from Chimera:

1. **Several feature bursts in a row** without a refactor day
   in between (>5 feature-dominant days in a row). Concrete example:
   04-22 to 04-25 — four feature-dominant days in a row → 04-26
   hardening day that systematically pays down the accumulated debt.
2. **Size signal:** a file has grown beyond ~500 LOC or
   another has become unclear (the hotspots from
   `docs/analysis/complexity-analysis.md` make that visible). Concrete example:
   `useHeatSimulation.ts` had grown to 982 LOC → on 04-27 decomposed into
   7 pure modules (637 LOC remaining).
3. **New feature on a shaky base:** if the next planned
   extension rests on a mushy subsystem, the refactor is worth doing
   first. Example: the types monolith with 441 LOC was split on
   04-26 into 5 thematic files before further bot features
   were put on top.
4. **Docs drifting visibly:** when, while getting oriented, you notice
   that the docs are several steps behind, or that new
   practices/insights from the last days are not reflected in
   `good-development-practices.md`.

### D.6 Summary of the cadences

| Cadence | Duration | Content | Frequency |
|---|---|---|---|
| **Mini** (per change) | Minutes–hours | 1 commit with code + tests + docs | 5–70/day |
| **Micro** (feature burst) | 1–4 days | Thematically coherent feature package | ~every 3–7 days |
| **Macro** (hardening) | 1 day (occasionally 2) | Refactor + perf + redundancy + test backfill + doc check | ~every 7–10 days |

No phase is sharply delineated from the next — commits of one
cadence can reach into another (e.g. a refactor commit
in the middle of a feature burst when it blocks progress, or
a feature commit on a hardening day when the refactor takes a small
functional extension along). The rhythm is the pattern across many
commits, not a strict rotation.

### D.7 Observations from 41 days (03-18 to 04-27)

**Distribution of the commit categories (1055 commits excluding merges):**

| Category | Commits | Share |
|---|---:|---:|
| feature | 373 | 35.4% |
| misc | 216 | 20.5% |
| fix | 178 | 16.9% |
| refactor | 153 | 14.5% |
| doc | 88 | 8.3% |
| test | 47 | 4.5% |

**What that says about the rhythm:**
- Feature dominates (35%), but no single day has a pure
  feature profile — even the strongest feature day (04-14 with
  73 commits) has 14 fix and 14 refactor commits interspersed.
- Refactor (14.5%) is high enough to make the continuous
  debt reduction visible — but concentrates in
  the hardening days (04-11, 04-19, 04-26 alone carry
  ~66 of the 153 refactor commits, i.e. ~43%).
- Test (4.5%) looks low — but it is not: most tests
  ride piggyback with `feat:`/`fix:` commits (see Part A.4.1).
  Pure `test:` commits are coverage backfills, i.e. primarily
  hardening activity.
- Doc (8.3%) is in the expected range, since per feature usually one
  or two pure doc commits accrue (in addition to the docs embedded
  in feature commits).

---

## Anti-pattern collection (short list)

Things that should **never** happen:

- A commit with failing tests, "I'll fix it right after"
- `git push --force origin main`
- Silent edits to existing tests so that the new code goes green
- Doc update "later at some point" (→ never)
- A "while I'm at it" refactor in a bug-fix commit
- A major-version bump without an explicit task for it
- Secrets via `git add -A`
- **Running a tool in the working tree that touches files while you keep
  working alongside** (mutation sweep, codemod, auto-formatter in
  watch mode). The tree is then deliberately broken for a while: `git status` shows
  foreign changes, `git add -A` commits them, and a blanket
  cleanup step (`git checkout -- src/`) deletes your own uncommitted
  work along with it. Happened exactly like that 2026-07-15 (half a test refactor was
  gone). Rule: start such tools only on a clean tree; they must roll back exactly
  what they themselves wrought — and nothing else.
- Sending catalog classifications from single-line greps to the user (multi-line entries = silent false negatives)
- Emojis in code or commits (unless explicitly wanted)
- Taking reminder messages / system messages into account in the
  reply to the user
- Acting from memory without verifying that the content is still true
- Implementing analysis/audit reports 1:1 without briefly verifying the
  named finding against the real code
- "Closing" a coverage gap via synthetic mocks even though the
  affected path is structurally unreachable (the real choice: delete,
  or mark it in the commit body as a defensive guard)
- Leaving large inline sub-functions in hooks when they obtain all their
  deps via closure capture — the extraction to pure modules with an
  explicit input interface only forces itself on you at the next bug fix
  in that tangle
- Spreading constants for one subsystem across 3+ files instead of
  opening a `<area>Constants.ts`
- Replying to Telegram inbound with terminal output (or an AskUserQuestion
  dialog) — the sender does not see it
- Working through a multi-step task completely and making one batch
  commit at the end, instead of committing immediately per step
- A plan file in `docs/plans/` without a leading `YYYY-MM-DD-` in the filename
- Running `npm run build` and `systemctl restart` separately instead of as
  one `&&` unit (either the old state gets served, or the service starts
  without the new files)
- Choosing the scaffolding minimum in a refactor scope decision
  ("quick-and-dirty, clean later") instead of the substantially right
  variant
- "Straightening out" a core engine semantic (aggregation order, owner scope,
  gate propagation, event lifetime) based on a single bug report/screenshot
  without checking whether the behavior is a deliberate
  design invariant (e.g. modifiers child→parent never sibling; boss event
  without a duration = permanent → an accumulating hazard is a forgotten
  `durationMs` in the JSON, not an engine bug — reverted 2026-06-13)
- Re-layouting/clamping user data (save/export) when adopting it instead of
  taking it 1:1; or back-calculating values from a screenshot even though the
  source file is available
- Setting a plan to "Umgesetzt" as soon as the last feature runs —
  without actually performing the concluding refactoring-audit phase
  (architecture/redundancy over the whole implementation)

---

## Revision

If a rule causes problems in practice, question it
instead of silently working around it. Practices should serve the goal —
shipping working, maintainable software fast and with a low
error rate — not the other way around.

Changes to this document: via PR, short rationale, approval of
at least one other team member.
