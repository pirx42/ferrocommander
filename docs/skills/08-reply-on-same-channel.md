# Skill: Reply on the same channel

**When.** A message / instruction comes in over a specific channel
— Telegram, terminal, external MCP, etc.

**Rule.** The reply + all follow-up questions go back **over the same
channel**. Never silently switch to a different channel, not even
"because it happens to be easier right now".

**Why.** The sender only reads the channel they wrote on —
their push notifications and their attention are there. If the
answer lands elsewhere, they do not see it. In particular: Telegram
inbound → terminal reply is, for the sender, like no reply at all.

**How.**
- Inbound via **Telegram**? → `mcp__plugin_telegram_telegram__reply`,
  with the matching `chat_id` from the `<channel>` tag.
- Inbound via **terminal**? → direct text output (no Telegram tools).
- Follow-up questions take the same path. No `AskUserQuestion` when the
  user writes via Telegram.
- With Telegram: `react` for quick acknowledgement, `edit_message` for
  live status updates, a new `reply` at the end of a long task (edits
  do not trigger a push notification).

**Example.**
```
<channel source="telegram" chat_id="8644439223" ...>
  Run the tests please.
</channel>

→ Run the tests, then via reply(chat_id=8644439223, text="...").
   NOT as terminal output, which never reaches the sender.
```

**Anti-patterns.**
- Telegram inbound → posing an AskUserQuestion dialog in the terminal.
- Addressed via Telegram, then saying "done" in the terminal without a Telegram reply.

**Repetition & escalation.** Drilled in multiple times (most recently 2026-06-13, after
I again used `AskUserQuestion` for a value selection instead of Telegram).
Since anchored as a hard rule in the root `CLAUDE.md` ("Important Rules"),
not only here — i.e. it applies project-wide, not as a style recommendation. When in
doubt, ALWAYS Telegram, including for options/selections. Memory `feedback_telegram_reply_channel`.

**Related.**
- [02-answer-meta-questions-directly.md](02-answer-meta-questions-directly.md)
- [05-updates-during-long-tasks.md](05-updates-during-long-tasks.md)
- [01-clarify-with-options.md](01-clarify-with-options.md) — offer options, but over the inbound channel.
