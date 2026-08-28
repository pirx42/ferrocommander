# Skill: Long-running scripts emit ongoing status

**When.** A script/command is expected to run in the background for longer
than a few minutes — training/eval batteries, campaign loops, mass sims,
suite runs, build chains.

**Rule.**

1. **Every self-written long-running script emits continuous progress**
   to stdout/logfile: one line per stage with a timestamp
   (`echo "[$(date +%H:%M)] step 3/12 — eval tutorial-v4"`), plus counters
   (`i/N`) for inner loops. Never work silently for minutes.
2. **Third-party tools that buffer** (vitest writes the report only at the
   end) must not be dropped naked into an hours-long job: unbuffer or
   segment the output — one separate invocation per test file/stage with
   an echo before it, or a reporter/`tee` into the logfile so that
   `tail -f` shows signs of life.
3. **Heartbeat criterion:** an observer must be able to decide from the
   log alone "still running" vs. "hanging" — last line older than the
   longest expectable stage interval = suspicion. Without ongoing output,
   only CPU-load guesswork (`ps`) remains, and that does not distinguish
   between "computing usefully" and "endless loop".

**Why.** Owner instruction 2026-08-23 after the sim-suite incident: a
blanket-started suite run stayed silent for >4 h (vitest buffers; the TPE
driver `optimize.test.ts` ran along unnoticed). Whether the job was hanging
or working could only be detected via CPU forensics. With stage echoes it
would have been clear after two minutes WHAT was running and that it was
the wrong scope.

4. **Actively monitor, don't just wait (owner 2026-08-23):** Whoever
   starts a long runner checks the log output **every ~5 min** — via a
   stall watchdog (alarm when the log hasn't grown for >5 min, see
   example 2) or a direct `tail` check on the status cadence. A completion
   callback does not replace this: a HANGING process never fires it, and
   that is exactly what should stand out.

**Example.**

```bash
for RUN in bot-curriculum tutorial-v4 challenge-v4; do
  echo "[$(date +%H:%M)] eval $RUN starting"
  RL_MANAGER=1 RL_MODEL=... npx tsx scripts/calibration.ts "$RUN" 12 \
    > "/tmp/rl-diag/eval-$RUN.log" 2>&1
  echo "[$(date +%H:%M)] eval $RUN done: $(grep -m1 Bosse /tmp/rl-diag/eval-$RUN.log)"
done
```

Related: [61 — Kill background processes cleanly](61-kill-background-processes-cleanly.md)
(the abort side of the same problem), [05 — Updates during long tasks](05-updates-during-long-tasks.md)
(user communication; here it is about the script output itself).

**Example 2 — stall watchdog (reports ONLY standstill):**

```bash
LAST=0; SILENT=0
while true; do
  SZ=$(stat -c %s "$LOG" 2>/dev/null || echo 0)
  if [ "$SZ" -eq "$LAST" ]; then SILENT=$((SILENT+60)); else SILENT=0; LAST=$SZ; fi
  [ $SILENT -ge 300 ] && { echo "STALL: $LOG unchanged for ${SILENT}s"; SILENT=0; }
  sleep 60
done
```
