# Skill: Never make things up — verify or ask

**When.** Whenever a substantive statement is made about game behavior,
code, or requirements (docs, manual, Telegram replies, commit
descriptions) — and especially when it "sounds plausible".

**Rule.**

1. **No statement from assumption.** Every behavioral claim is verified
   against the implementation beforehand (read the code; docs/features.md
   and the subsystem docs are maintained and citable).
2. **If it remains unclear afterwards → ASK instead of guessing** (owner
   msg 15477/15478: "if anything is unclear, always ask instead of
   guessing") — follow-up question via Telegram, ideally with concrete
   options (skill [01](01-clarify-with-options.md)).
3. Applies equally to requirement gaps (skill
   [09](09-scope-before-implementation.md)) and memory contents (skill
   [39](39-verify-memory.md) — memory describes the state of back then,
   not necessarily today's).

**Why.** Incident 2026-07-18: the player manual contained ~9 errors
written from assumptions (a non-existent cable button, "ammunition",
infiltration exactly backwards, wrong recycle and run-end semantics,
"topology doesn't matter" …). Every single assumption would have been
prevented by a glance at the code or a follow-up question — the correction
round cost more than the verification ever would have. Plausibility is not
evidence.

**How.**
- Before writing: be able to name the source for each claim (file/line,
  doc section, owner statement). No source → check or ask.
- When documenting unfamiliar subsystems, first read the maintained docs
  (features.md, energy-system.md, …), then do targeted code greps.
- Take uncertainty signals seriously: "presumably", "should actually",
  "typically" in your own head = stop word.

**Anti-patterns.**
- "It will surely work like in other games."
- Extrapolating a mechanic from UI appearance instead of reading the
  handler.
- Trying to save the follow-up question and collecting a correction round
  instead.

**Related.**
- [01-clarify-with-options.md](01-clarify-with-options.md),
  [09-scope-before-implementation.md](09-scope-before-implementation.md),
  [39-verify-memory.md](39-verify-memory.md),
  `56-spielregel-semantik-ohne-sonderfaelle.md` (Chimera only).
- Memory `feedback_ask_instead_of_guessing`.
