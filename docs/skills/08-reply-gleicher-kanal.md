# Skill: Auf dem gleichen Kanal antworten

**Wann.** Eine Nachricht / Anweisung kommt ueber einen bestimmten Kanal
herein — Telegram, Terminal, externe MCP, etc.

**Regel.** Antwort + alle Rueckfragen gehen **ueber denselben Kanal**
zurueck. Nie stillschweigend auf einen anderen Kanal wechseln, auch
nicht „weil das gerade einfacher ist".

**Warum.** Der Sender liest nur den Kanal, ueber den er geschrieben hat —
seine Push-Notifications und seine Aufmerksamkeit sind dort. Wenn die
Antwort woanders landet, sieht er sie nicht. Insbesondere: Telegram-
Eingang → Terminal-Reply ist fuer den Sender wie kein Reply.

**How.**
- Eingang ueber **Telegram**? → `mcp__plugin_telegram_telegram__reply`,
  passend mit `chat_id` aus dem `<channel>`-Tag.
- Eingang ueber **Terminal**? → direkter Text-Output (keine Telegram-Tools).
- Rueckfragen folgen demselben Pfad. Keine `AskUserQuestion` wenn der
  User per Telegram schreibt.
- Bei Telegram: `react` fuer schnelle Bestaetigung, `edit_message` fuer
  Live-Status-Updates, neuer `reply` am Ende einer langen Task (Edits
  triggern keine Push-Notification).

**Beispiel.**
```
<channel source="telegram" chat_id="8644439223" ...>
  Run die Tests bitte.
</channel>

→ Tests laufen lassen, dann via reply(chat_id=8644439223, text="...").
   NICHT als Terminal-Output, der den Sender nicht erreicht.
```

**Anti-Pattern.**
- Telegram-Eingang → AskUserQuestion-Dialog im Terminal stellen.
- Per Telegram angesprochen, dann im Terminal „done" sagen ohne Telegram-Reply.

**Wiederholung & Eskalation.** Mehrfach eingeschaerft (zuletzt 2026-06-13, nachdem
ich erneut `AskUserQuestion` fuer eine Wert-Auswahl statt Telegram benutzt habe).
Inzwischen als harte Regel in der Root-`CLAUDE.md` („Important Rules") verankert,
nicht nur hier — d.h. sie gilt projektweit, nicht als Stil-Empfehlung. Im Zweifel
IMMER Telegram, auch fuer Optionen/Auswahlen. Memory `feedback_telegram_reply_channel`.

**Verwandt.**
- [02-meta-fragen-direkt.md](02-meta-fragen-direkt.md)
- [05-updates-bei-langen-tasks.md](05-updates-bei-langen-tasks.md)
- [01-clarify-with-options.md](01-clarify-with-options.md) — Optionen anbieten, aber ueber den Eingangs-Kanal.
