# Skill: Overnight / offline autonomy

**When.** The owner explicitly announces going offline
("I'm going to sleep", "back tomorrow", "away until 9").

**Rule.** Until the stated return time (typically ~7:00), work autonomously
on **decision-free** tasks. Do not wait idle. Tasks that
need questions answered are **not** started — those wait until morning.

**Why.** The owner's waking hours are the only time for
decisions. Blocking decision-required tasks during the night would be
a waste of that time. Low-risk tasks (docs, catching up on tests,
clear bugfixes, translation, audit follow-ups) are ideal — the owner
finds progress in the morning.

**How.**

1. **Before starting:** load and follow existing memory entries (mandatory
   rules, conventions) — do not take the opportunity to break house rules.
2. **Risk filter:**
   - no schema breaks
   - no refactors that touch many consumers
   - no destructive git operations without explicit prior consent
   - when a decision comes up: note it down (memory / plan doc) and leave
     it be
3. **Stop once all decision-free tasks are done** — do not
   take on risk tasks just to fill the time. A short
   end-of-night status is enough.

**Suitable night tasks (examples).**
- Docs catch-up after a completed refactor
- Reading audit reports + identifying open follow-up tasks
- Closing coverage gaps (reachable paths)
- i18n upkeep (missing translation keys)
- Adding tests for existing behavior
- Archiving plan files (status: Implemented → archive/)

**Unsuitable night tasks.**
- New features with an unclear spec
- Refactors with multi-way tradeoffs ("option A or B?")
- Major dependency bumps
- Production push without prior consent

**Anti-patterns.**
- 4 hours idle because "there was no task without questions".
- Starting a risk task and presenting it in the morning with "is this okay?".

**Related.**
- [03-confirm-destructive-actions.md](03-confirm-destructive-actions.md)
- [06-announce-pauses.md](06-announce-pauses.md)
- [40-four-phase-cycle.md](40-four-phase-cycle.md) (phase 2/3 tasks
  are particularly suitable).
