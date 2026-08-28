# Skill: Commit important scripts, not scratchpad

**When.** Whenever a script carries **hard-to-reconstruct parameters or
configuration** — RL retrain recipes (staging/novelty/entropy schedule/
episodes), measurement/campaign commands, data-migration one-liners with
carefully tuned flags, repro harnesses that will be needed again.

**Rule.**
- Config-carrying scripts belong **in the repo** (`scripts/` or `rl/`),
  parameterized + briefly documented — NOT in the scratchpad (gitignored,
  ephemeral) and not only as a nohup command line in the transcript.
- Throwaway previews/one-off debug snippets may stay in the scratchpad;
  the boundary is: "Would losing it cost reproducibility?"
- Gallery/preview scripts that get reused for design tweaks also fall
  under "commit" (see skill 55).

**Why.**
The `-scope8` RL retrain lived only in `scratchpad/retrain-b.sh`
(gitignored) and was lost; the exact config (2-stage 30+60, novelty 0.05,
episodes 6, ent 0.03→0.015→0.005) had to be laboriously salvaged from the
session transcript (owner msg 16000, 2026-07-25). Ephemeral scratch
scripts = lost reproducibility.

**Related.** Skill 55 (commit the gallery script), skill 35 (reproducible
deploy), `docs/bot-training.md` (training recipes).
