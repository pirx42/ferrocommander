# Testing round, 2026-09-01 — five findings

Status: Draft — the three decisions settled by the owner, 2026-09-01

Five things a person found by *using* the program rather than testing it:
two panes that scroll when nothing asked them to, a Windows drive root that
lands in the application's own folder, a status line missing its free-space
figure, and a backgrounded job that runs with nothing on screen to say so.

Four of the five were found in the code before this plan was written, and one
of them was reproduced on screen. What follows names each root cause rather
than the symptom, because three of the five are one line and the fourth is
not the line it looks like.

## 1. What each one turned out to be

| Reported | Root cause | Where |
|---|---|---|
| Going back to the parent flickers: `..` at the top for a moment, then the remembered position | `restore_scroll` runs on a **default-priority idle**, which GTK schedules *after* its own layout and paint — so the frame at offset 0 is drawn before the offset is put back | `pane.rs`, since 2026-08-29 |
| `Space` on a view scrolled away from the cursor jumps the list to the top | every marking operation ends in `sync_cursor`, which scrolls the cursor into view **whether or not the cursor moved** | `pane.rs`, since 2026-08-29 |
| On Windows, `..` out of `C:/Users` lands in the application's start folder | `to_std_path` trims the trailing separator off every native path, which turns the drive root `C:\` into `C:` — a **drive-relative** path Windows resolves against the process's current directory | `vfs/platform.rs`, since the first VFS commit |
| No free/total figure at the bottom of a pane | the Windows branch of `space()` returns `None`; `GetDiskFreeSpaceExW` was never called | `vfs/platform.rs`, documented in [future-improvements.md](../future-improvements.md) |
| A backgrounded job runs with nothing to show for it | there is no indicator, and `Background` closes the only window a job ever has | `dialogs/progress_view.rs`, documented in [future-improvements.md](../future-improvements.md) |

**None of the five comes from the quick-view work** that shipped hours
earlier: the two scroll lines date to 2026-08-29, the path trim to the first
VFS commit on 2026-08-28, and the two absences have been written down as
known gaps for days. That was checked rather than assumed, because a
regression and a long-standing bug want different fixes.

## 2. What the probes found, before any of it was planned

**The `Space` jump was reproduced on screen.** `check-scroll-memory.sh`
already builds the exact state it needs — cursor on the first row, the view
wheeled far down — so a variant of it pressed `Space` there and took two
screenshots. Before: rows `dir-21` to `dir-35`, `..` off the top. After: the
list at the top, `..` visible, `dir-01` marked, and the status line reading
`1 of 60 selected`. The cursor never moved; the *view* was dragged back to
it. That is the whole bug, and it is worth having seen rather than deduced.

**The free-space figure is fine on Linux.** The same screenshots show
`4.7 GiB free of 252.0 GiB` in both panes, which places the finding squarely
on the Windows branch and not on the formatting, the trait or the status
line — all of which are already platform-agnostic and already right.

**Windows CI runs no tests at all.** The `windows` job packages and
smoke-starts the binary; the gate is the Linux job's, because Xvfb and
xdotool have no meaning against MSYS2's Win32-backend GTK
([windows.md](../windows.md)). So a `#[cfg(windows)]` unit test would never
run anywhere, on any machine but the owner's. That single fact shapes phases
2 and 3 more than either bug does.

## 3. Decisions

1. **What `Space` should do.** Settled by the owner, and it is not the
   trade-off this plan first offered: *"on pressing the space key the active
   element does not change — so why should there be any change in the scroll
   position?"* Right, and the code is simply wrong. But "marking never
   scrolls" is too blunt a rule: `Insert` and `Shift+↓` mark **and advance**
   through the same `marking` path, and a held-down `Insert` has to keep the
   cursor on screen. So the rule is **the view follows the cursor when the
   cursor moves**, decided by comparing the cursor before and after the
   change rather than by which key it was. `Space` (`toggle_mark(0)`) then
   leaves the view alone by construction, and so do `Num +`, `Num *`,
   `Num /` and the wildcard marking — every one of which jumps today.

2. **How Windows is asked for free space.** Settled: **a hand-declared
   `extern "system"`** for `GetDiskFreeSpaceExW`, beside the `libc::statvfs`
   unsafe block the Unix branch already has, rather than a Windows API crate
   for one number. Alternative considered and not taken: `windows-sys`,
   which would be right if more Win32 calls were coming — and none is.

3. **What the background indicator is.** Settled: a small **progress bar
   with its percentage at the right of the command-line row**, visible only
   while a job is running, and **clicking it reopens the full progress
   window** with its Cancel and Background buttons. That second half is what
   closes the gap `future-improvements.md` calls *"a window listing the
   running jobs"*: once backgrounded, a job today can be neither watched nor
   cancelled, because nothing on screen refers to it.

## 4. Phases

**Phase 0 — coverage pre-check** (skill 43). What already covers the four
seams, and what cannot:

- *The two scroll bugs.* Nothing automated, and nothing can be: a scroll
  offset is not a window title, a file or a key press, so the end-to-end
  suite cannot see it ([ui-shell.md](../ui-shell.md)). The existing check is
  `scripts/check-scroll-memory.sh`, which drives the real app and leaves
  screenshots for a person. The marking half is a *new* case for it.
- *The Windows drive root.* `platform.rs`'s own tests cover the root
  mapping and a native round trip, but both are written to pass on whichever
  platform runs them, and no platform that runs them is Windows.
- *Free space on Windows.* Untested by construction — `space()` returning
  `None` is what the current test expectations were written around
  ([future-improvements.md](../future-improvements.md) lists the row).
- *The job indicator.* The `Meter` behind it is unit-tested in
  `progress.rs`; the window it feeds is not, and neither is `Background`.

Nothing here owes characterisation tests. What phase 0 changes is the plan
itself: **two of the five fixes cannot be verified where they run**, so
phases 2 and 3 are shaped around getting as much of each as possible into a
form a Linux gate *can* check.

**Phase 1 — the two scroll bugs.** They are one commit because they are one
subject: who is allowed to move the viewport.

*Marking.* `marking` reads the cursor before and after the change. Moved →
`sync_cursor` as today. Not moved → re-assert the widget's selection without
scrolling, or nothing at all if the splice already leaves it in place, which
is worth finding out rather than guarding against (skill 19).

*The flicker.* `restore_scroll` queues its work with
`glib::idle_add_local_once`, which is `G_PRIORITY_DEFAULT_IDLE` — behind
GTK's resize pass and behind its redraw. The restore therefore lands one
painted frame late, and that frame is the flicker. The fix is to run it
**between** the two: after layout, so the adjustment's `upper` is real and
the offset is not clamped to zero, and before the paint, so no frame ever
shows the top. If GTK turns out to compute the column view's extent later
than that, the fallback named here rather than discovered later is to stop
emptying the store at all — `refresh` does `remove_all` then one `append`
per row, and emptying is what drops the adjustment to zero in the first
place; one `splice` of the whole store never does.

*The check* is a script, committed rather than left in a scratchpad (skill
68): `check-scroll-memory.sh` grows the marking case, and the flicker gets a
burst of screenshots taken as fast as the display will give them, so the
*first* frame after the key is the one under inspection. Honest about what
that is worth: a one-frame flicker caught by racing it is a weaker check
than a test, and nothing in CI runs it.

**Phase 2 — the Windows drive root.** `to_std_path` builds `C:\` and then
trims the separator that makes it a root. The fix is to stop trimming — join
the components onto the drive rather than appending a separator after each
and cutting the last one off, which has no trailing separator to remove and
so cannot remove the wrong one.

Because Windows CI runs no tests, the mapping moves out of the
`#[cfg(windows)]` module into an always-compiled pure function that takes
the separator as an argument, tested on Linux with `\` passed in — the
pattern `parse_mount_table` already set for the mount table. The cases: the
drive root, one level down, two levels down, and the round trip `C:\` →
`/C:` → `C:\` that the current test only exercises for a home directory
several levels deep, which is exactly why it never caught this.

**Phase 3 — free space on Windows.** One `extern "system"` declaration, the
path encoded as UTF-16 with its NUL, and the *first* of the three out
parameters — bytes available **to the caller** — for the same reason the
Unix branch takes `f_bavail` over `f_bfree`: the question a status line
answers is "will my copy fit". A zero return is `None`, not zero bytes.

What can be checked here is thin and the plan says so: the encoding helper
gets a unit test that runs anywhere, and the call itself is verified by the
owner on a real machine. Extending the Windows package's smoke start to read
the status line was considered and not taken — it would mean a screenshot
and an OCR on a runner, to check a number that a person looking at the app
sees immediately.

**Phase 4 — the job indicator.** A `ProgressBar` at the end of the row the
command line is in, fed from the same `Meter` the progress window uses, so
the caption has one formatter and not two (skill 44). It appears under the
same rule the window does — `PROGRESS_DELAY` and `has_work()` — because a
bar that flashes for thirty milliseconds is the noise that rule exists to
prevent, and two rules for "is this job worth mentioning" would drift.

The real work is ownership. Today the progress window lives in a local of
the future that drains the job's events, which is why nothing else can
reopen it. It moves to the shell, where the click handler and the future can
both reach it — and that is the change most likely to alter *when* the
window closes, so the phase carries a test that a job still finishes, still
reports its failures and still reloads both panes when the window is not
open at all.

**Phase 5 — docs.** [ui-shell.md](../ui-shell.md) for the indicator and the
rule about who moves the viewport, [windows.md](../windows.md) and
[future-improvements.md](../future-improvements.md) for the two gaps that
close, [ops.md](../ops.md) for the way back to a backgrounded job.

**Phase 6 — refactoring audit** (skill 49) and the end-of-plan ritual.

## 5. Effort

Corrected per skill 45; the last plans held at ~0.10.

| Phase | Raw | Corrected |
|---|---|---|
| 0 pre-check | 0.25 d | 0.25 h |
| 1 the two scroll bugs | 1.5 d | 1.5 h |
| 2 the Windows drive root | 0.75 d | 0.75 h |
| 3 free space on Windows | 0.75 d | 0.75 h |
| 4 the job indicator | 2 d | 2 h |
| 5 docs | 0.5 d | 0.5 h |
| 6 audit | 0.5 d | 0.5 h |

About six hours, plus a full-gate run (~15 min) per phase commit.

## 6. What would make this wrong

- **The flicker fix rests on GTK's idle priorities**, which are a documented
  ordering rather than an API contract. If GTK reorders its passes, the
  flicker comes back silently and only a person running the screenshot
  script would ever know. That is the honest state of anything about a
  scroll offset in this program, and it is why the fallback is named in
  phase 1 rather than left to be invented under pressure.
- **Two fixes cannot be verified where they run.** The Windows job has no
  test step, so the pure halves are tested on Linux and the two Win32-facing
  lines — the drive root's native string and the free-space call — are
  verified by the owner starting the program. A green gate will not mean
  they work.
- **The marking fix changes five keys, not one.** `Num +`, `Num *`, `Num /`
  and the wildcard marking all jump today for the same reason `Space` does,
  and all of them stop. That is the fix being right rather than wide, but it
  is more behaviour changed than the report asked about, and the screenshots
  should cover more than the reported key.
- **The indicator moves the progress window's ownership** out of the future
  that drains the job. Everything about when that window opens and closes is
  currently implied by a local variable's lifetime, and implied lifetimes
  are what break quietly when they become explicit.
- **The wheel is not the only way the cursor leaves the screen**, and this
  plan fixes marking rather than scrolling in general. A scrollbar drag, a
  window resize and a filter that shortens the list all leave the cursor
  somewhere the view is not; what phase 1 settles is that a keystroke which
  moves nothing must not move the view either.
