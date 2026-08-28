# Skill: Hintergrund-Prozesse sauber killen — Prozessgruppe + Verifikation + Bracket-Trick

**Wann.** Ein laufender Hintergrund-Job (vitest, npm-Sim, Playwright, Vite,
RL-Training) soll abgebrochen werden — oder irgendein `pkill`/`pgrep -f`
laeuft in einem Bash-Tool-Kommando.

**Regel.**

1. **Prozess-GRUPPE killen, nicht das Muster:** `kill -- -<pgid>` (PGID via
   `ps -o pgid= -p <pid>`). `pkill -f "<muster>"` toetet nur Prozesse, deren
   Cmdline matcht — **geforkte Worker** (vitest `workers/forks.js`, Browser,
   Node-Forks) matchen oft NICHT, werden zu Waisen umgehaengt und rechnen
   weiter. Alternativ pkill zusaetzlich auf die Worker-Signatur
   (`pkill -f "[w]orkers/forks.js"`).
2. **IMMER nachpruefen:** `ps -eo pid,etime,%cpu,cmd --sort=-%cpu | head` —
   kein ueberlebender Prozess des Laufs darf noch CPU ziehen. Erst dann ist
   der Abbruch abgeschlossen.
3. **Bracket-Trick bei `pkill`/`pgrep -f`:** ein Zeichen des Patterns in eine
   Zeichenklasse setzen (`pgrep -f "vite [p]review"`). Das rohe Pattern steht
   sonst woertlich in der Cmdline des eigenen Bash-Wrappers → der Aufruf
   matcht und killt SICH SELBST (exit code 144). `pkill` ausserdem nie im
   selben Kommando wie andere kritische Schritte buendeln.

**Warum.** Prozessbaeume sterben nicht mit dem Eltern-Kill. Vorfall
2026-07-17 (Owner msg 15351): ein verwaister vitest-Fork-Worker einer
abgebrochenen Sim brannte **13,5 h lang einen Kern auf 100 %**, bis der
Owner die Last bemerkte. Der Selbst-Match-Fall passierte 2026-07-06 zweimal
(Hintergrund-Gate abgeschossen bzw. das kill-Kommando selbst).

**Beispiel.**
```bash
# Job-PID kennen (Background-Task-Id oder pgrep mit Bracket-Trick):
pgrep -f "[s]imulate" | head -1        # → 12345
ps -o pgid= -p 12345                    # → 12300
kill -- -12300                          # ganze Gruppe
ps -eo pid,etime,%cpu,cmd --sort=-%cpu | head   # Verifikation: nichts >0 %
```

**Anti-Pattern.**
- `pkill -f "npm run simulate"` und weitergehen — die Fork-Worker leben.
- Kill ohne ps-Nachkontrolle als „erledigt" melden.
- `pkill -f` mit rohem Pattern im Bash-Tool → exit 144, eigener Task tot.

**Verwandt.**
- [05-updates-bei-langen-tasks.md](05-updates-bei-langen-tasks.md)
  (Hintergrund-Jobs beobachtbar machen, Log-Datei statt `| tail`).
- Memory `feedback_kill_process_tree_verify` +
  `feedback_pgrep_pkill_self_match`.
