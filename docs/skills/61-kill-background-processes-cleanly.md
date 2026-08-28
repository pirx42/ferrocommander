# Skill: Kill background processes cleanly — process group + verification + bracket trick

**When.** A running background job (vitest, npm sim, Playwright, Vite,
RL training) is to be aborted — or any `pkill`/`pgrep -f` runs inside a
Bash tool command.

**Rule.**

1. **Kill the process GROUP, not the pattern:** `kill -- -<pgid>` (PGID via
   `ps -o pgid= -p <pid>`). `pkill -f "<pattern>"` only kills processes
   whose cmdline matches — **forked workers** (vitest `workers/forks.js`,
   browsers, Node forks) often do NOT match, get reparented as orphans and
   keep computing. Alternatively, additionally pkill the worker signature
   (`pkill -f "[w]orkers/forks.js"`).
2. **ALWAYS re-check:** `ps -eo pid,etime,%cpu,cmd --sort=-%cpu | head` —
   no surviving process of the run may still draw CPU. Only then is the
   abort complete.
3. **Bracket trick with `pkill`/`pgrep -f`:** put one character of the
   pattern into a character class (`pgrep -f "vite [p]review"`). Otherwise
   the raw pattern appears literally in the cmdline of your own Bash
   wrapper → the call matches and kills ITSELF (exit code 144). Also never
   bundle `pkill` in the same command as other critical steps.

**Why.** Process trees do not die with the parent kill. Incident
2026-07-17 (owner msg 15351): an orphaned vitest fork worker of an aborted
sim burned **one core at 100 % for 13.5 h** until the owner noticed the
load. The self-match case happened twice on 2026-07-06 (background gate
shot down, and the kill command itself).

**Example.**
```bash
# Know the job PID (background task id or pgrep with bracket trick):
pgrep -f "[s]imulate" | head -1        # → 12345
ps -o pgid= -p 12345                    # → 12300
kill -- -12300                          # whole group
ps -eo pid,etime,%cpu,cmd --sort=-%cpu | head   # verification: nothing >0 %
```

**Anti-patterns.**
- `pkill -f "npm run simulate"` and moving on — the fork workers live on.
- Reporting a kill as "done" without the ps re-check.
- `pkill -f` with a raw pattern in the Bash tool → exit 144, own task dead.

**Related.**
- [05-updates-during-long-tasks.md](05-updates-during-long-tasks.md)
  (make background jobs observable, log file instead of `| tail`).
- Memory `feedback_kill_process_tree_verify` +
  `feedback_pgrep_pkill_self_match`.
