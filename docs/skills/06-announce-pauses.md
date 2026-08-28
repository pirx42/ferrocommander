# Skill: Announce pauses explicitly

**When.** The work reaches a stopping point:
- waiting for a user decision
- all tasks completed, nothing in the pipeline
- sub-agent running in the background, main thread idle
- you are ending active processing

**Rule.** Report the stopping point **explicitly**. Never fall silent
without a word.

**Why.** From the owner's perspective, "no new output" is identical
to "crashed", "forgotten", or "waiting on me". They have to ask
themselves to see the difference. One line resolves the
ambiguity.

**How.**
- When pausing: one sentence, why + what is being waited for.
- When idle: "all tasks done — ready for the next round".
- With a background sub-agent: "sub-agent X is running, I'll report when it
  finishes; idle in the meantime".

**Example.**
```
Waiting for the decision on phase 3 (option A vs B).
Holding back phases 4-5 until then.

All 5 planned phases committed + pushed.
Ready for the next task.

Sub-agent "coverage audit" running in the background (ETA ~10 min).
Main thread idle until the result is in.
```

**Anti-patterns.**
- Phase 2 done, no output, user wonders after 20 min "are you still alive?".
- "done" without saying with what.

**Related.**
- [05-updates-during-long-tasks.md](05-updates-during-long-tasks.md)
- [04-multi-phase-autonomy.md](04-multi-phase-autonomy.md)
