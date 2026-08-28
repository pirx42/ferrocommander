# Skill: Updates during long tasks (~20-min cadence + milestones)

**When.** The task needs more than ~5 minutes of pure working time
(big build, long test suite, batch refactor, sub-agent running).

**Rule.** Roughly every **~20 minutes** a short line — what is happening right
now, what is the next step (cadence raised by the user on 2026-07-09 from ~5 min
to ~20 min, memory `feedback_long_task_updates`). **Additionally**, at
every completed milestone (plan sub-task, phase, sub-agent result)
a short status ping — no longer on a rigid 5-minute beat. For
sub-agent spawns, the stagger update applies to the main thread waiting
for the reply.

**Why.** Silence for more than a few minutes creates uncertainty:
is it still on it, is it stuck, has it been forgotten? Brief visibility costs
little and reassures a lot.

**How.**
- One line is enough: "still building, 70% through", "waiting for sub-agent X,
  ETA ~3 min", "tests running, green so far".
- At every notable progress step: mini-update.
- Report obstacles immediately, not after the plan has imploded.
- **Make background jobs observable** (incident 2026-07-16, msg 15330):
  redirect output into a log FILE (`> job.log 2>&1`), never only through
  `| tail` — otherwise progress is invisible and "working" cannot be
  distinguished from "hung". BEFORE diagnosing a hang, check the run
  configuration (default parameters! the `SIM_RUNS_PER_BOT` default of 100 turned
  a smoke run into an hours-long campaign); 100% CPU only proves "computing",
  not "terminating".
- **10-minute checkpoint (MANDATORY, owner msg 15522):** every run
  > ~10 min gets a substantive check after AT MOST 10 minutes — is it
  running AS INTENDED (correct scope/parameters, plausible throughput,
  expected intermediate output, remaining-time projection)? On deviation,
  abort/correct immediately. Trigger: the full sim-config suite
  unnoticedly included the simulateAll statistics campaign and burned 1.9 h
  (2026-07-18) — a 10-min glance at the log would have shown it immediately.
- **Monitoring needs no asking back** (blanket permission 2026-04-30,
  memory `feedback_monitor_autonomy`): just start `Monitor`/`tail -F`/`watch`/
  `inotifywait` on arbitrary processes/files under `/home/pirx/projects`
  + `/tmp` + accessible logs — proactively, without asking first.
  External hosts/SSH remain permission-gated.

**Example.**
```
[t=0]   Starting build:rl-worker + 50-run sweep (~40 min total).
[t=8]   10-min checkpoint: correct scope, throughput plausible.
[t=20]  Milestone: build done, sweep at 20/50, reward 12.3, no crashes.
[t=40]  Done, 50/50 green — result ping.
```

**Anti-patterns.**
- Half an hour of silence, then "done" without any interim report.
- The opposite: content-free pings on a 5-minute beat (the cadence was
  deliberately relaxed).
- "wait" / "almost there" — meaningless filler.

**Related.**
- [06-announce-pauses.md](06-announce-pauses.md)
- [04-multi-phase-autonomy.md](04-multi-phase-autonomy.md)
