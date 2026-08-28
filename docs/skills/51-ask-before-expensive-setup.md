# Skill: Ask briefly before expensive hand-building

**When.** A work step boils down to **laboriously assembling an
artifact by hand** that the user could produce faster and more reliably
interactively (in the game, in the tool, from their head). Typical
cases:
- **Test fixtures / save states** with complex inner state
  (e.g. an infiltration save: `activeBossBody` + energy network +
  loaded infiltrator — error-prone by hand, several failed attempts).
- **Data sets / setups** that can be clicked together in the UI in seconds.
- Concrete **content** whose "right" shape only the user knows
  (example inputs, realistic configurations).

**Rule.** Before starting the laborious hand-build, **ask briefly on the
active channel**: "Can you prepare/export X faster?"
Then the user decides whether they prepare something or whether I build it
myself. Do not silently start tinkering.

**Why.** User spec msg 11713 (2026-06-08). Trigger: I wanted to
synthetically create an infiltration save for e2e tests (bot-sim dump,
hand-editing the JSON, several attempts). The user built the same setup
in the game in seconds and sent it as a file — exactly correct, without
all the fiddling. One question up front would have saved the detour.

**Adopt user artifacts 1:1, do not rebuild or re-layout them
(2026-06-13).** When the user has sent a setup (save/export), take its
data over EXACTLY — do not redistribute, clamp or "improve" it.
Trigger: while wrapping a user export into a boss, I had redistributed it
onto other zones via a handmade "place map" → it did not match his
build; the correct approach is position-faithful adoption (`placeFaithful` in the
wrap script, export coordinates 1:1). Related reflex: when the user shows via
screenshot "this is how it should look", do NOT laboriously back-calculate the
values from the image — ask for the **source file** (the export already existed and
contained the exact positions). Memory `feedback_user_items_all_or_fresh`.

**When _not_ to ask.** Pure code artifacts, trivial fixtures, things
that are faster to build yourself than to explain, or when the user is offline
and overnight autonomy applies (then choose the cheapest self-built path and
present the result for review later).

**How.**
1. Recognize: "I am about to build this laboriously by hand" — especially with
   save states/setups that originate in the tool.
2. Ask a **short, concrete** question about what is needed
   ("I need a save in the boss sector with an active infiltrator — can
   you export a setup like that?").
3. Say **where** I will put/use the result, so the user knows
   what it is for.
4. Wait for the answer; on "you build it", go the self-built path.

**Related.**
- [01-clarify-with-options.md](01-clarify-with-options.md) (questions
  instead of assumptions).
- `50-e2e-tests-erstellen.md` (Chimera only) (fixtures are
  a main use case — real exported saves beat
  synthetics).
- Memory `feedback_ask_before_laborious_build`,
  `feedback_prefer_correct_over_quick`.
