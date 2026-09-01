# Field report, round one — twelve findings from build 132

Status: Implemented — substance extracted to `keymap.md`, `ui-shell.md`,
`config.md` and `future-improvements.md`
(commits `14246ce`, `2852c2d`, `f73d35e`, `f223788`, `1f53ae2`, `cdcbaa6`,
`7eebefe`, `85ca267`)

Ten of the twelve findings are done and two were withdrawn by the owner. One
piece of phase 7 — a window listing the running jobs, and the question of
whether the queue should run them concurrently — was carved out rather than
left half-built, and lives in
[future-improvements.md](../../future-improvements.md) with the reason.

The first report from somebody *using* the program rather than testing it.
Build `0.1.0-132` on Ubuntu 24.04, installed from the published `.deb`.
Verdict was "overall works great", followed by twelve findings — which is the
useful half.

They are not one kind of thing. Some are bugs with a mechanism already
identified in the code; some are defects nobody has yet reproduced here; some
are features the program does not have. This plan keeps those apart, because
they carry different risk, and two of the twelve were withdrawn on a second
pass — which is why the sorting is worth doing before the work rather than
after.

## 1. What was decided, and by whom

Owner spec, 2026-08-29, over two rounds of questions.

**Withdrawn on the second round — no change, and the reasons are worth
keeping:**

- **Archives stay read-only.** The extract-modify-repack design was accepted
  and then withdrawn. `F4`/`F6`/`F8` go on reporting `ReadOnly` inside an
  archive. This removes the only phase that carried real revert risk, and
  the only one that would have touched the central VFS contract.
- **`Ctrl+S` stays a substring match.** The subsequence proposal (`pkag`
  finds `package`) is not wanted yet. No tests change, and the rule in
  `listing.md` stands as written.

**Decided:**

- **Background operations** — a full job manager window: several jobs at
  once, each with its own progress and cancel. **Closing the window does not
  stop them**: jobs keep running, the status bar goes on showing progress,
  and reopening finds them. Closing a window never destroys work.
- **Enter on a file** — always the system handler, never direct execution.
  Enter must not start a program because the cursor landed on one. **Inside
  an archive it is refused with a reason**, the way `F4` already is: there is
  no file on disk to hand over, and nothing gets unpacked behind your back.
- **Column widths** — resizable, persisted, and **shared by both panes**.
  The panes go on lining up with each other, which is why the widths were
  constants in the first place.
- **The inactive pane's cursor is an outline** — the same rectangle, not
  filled. The active pane's cursor stays solid.
- **Free space** — free *and* total, right-aligned on the pane's existing
  bottom line. The selection summary keeps the left; the two grow from
  opposite ends and cannot collide.
- **Scroll position** — remembered per directory for the session, however you
  come back to it: Backspace, a favourite, the history. Not persisted; the
  drive bar already remembers this way.

## 2. The twelve, sorted by what is actually known

### Verified in the code — mechanism identified

**F1 · `cd` never reaches the history.** `actions.rs`: the `Typed::Shell`
arm calls `remember_command`, the `Typed::ChangeDirectory` arm does not.
So `cd ..` moves the pane and vanishes; `Ctrl+↓` never offers it again.
Confirmed by reading, not by guessing — the two arms sit twelve lines apart.

**F2 · Space is not what the report suspected.** The report wondered whether
`Space` measures a folder instead of marking it, and asked for a check first.
It does not: `Space` is bound to `Action::ToggleMark` and to nothing else,
and marking never calls the size scan. The *observation* is still real and is
almost certainly F3 — a row appearing to change meaning because the whole
list moved under it. Recorded as answered, not as a defect.

**F3 · Column widths are constants.** `COLUMN_WIDTH_NAME` and its four
siblings, fixed, with a comment saying the panes line up because of it.
Nothing reads or writes them from the settings file.

**F4 · Free space is not computed anywhere.** No `statvfs`, no caller. The
status bar has never known it.

**F5 · Archives refuse every write on purpose.** Every mutating call returns
`VfsError::ReadOnly`, `trash` included, and `docs/archives.md` § 234 lists
`F7`/`F8` among the refusals. **Withdrawn — no change.** Verified so that
the refusal is known to be deliberate rather than assumed to be one; the
finding is answered by the design, and the design stands.

**F6 · Enter on a file does nothing, by design.** `docs/keymap.md`:
"Activating a *file* does nothing" — `F3` views, `F4` edits, an archive is
walked into. Also a deliberate property, and a much smaller one to revoke.

### Reported, not yet reproduced here

These four are UI-level and cannot be confirmed by reading. Each gets a
failing test in phase 0 **before** anything is changed, and any that turns
out not to reproduce is reported as such rather than quietly fixed.

**F7 · Marking the first row moves every row down by one row height.**
Marking the *second* row does not. Unmarking the first moves them back.
A layout effect, not a model one — the mark is a style class, and a style
class that changes a row's height would do exactly this.

**F8 · `Shift+F6` shows the extension twice** — in the edit field (correct,
the whole name is being edited) and still in the Ext column behind it.

**F9 · `Shift+F6` loses the cursor.** After Enter the cursor jumps to the top
row instead of staying on the file just renamed.

**F10 · The two panes' cursors look the same.** The inactive pane's cursor
is to be an **outline** — the same rectangle, unfilled — against the active
pane's solid one. This extends the existing decision rather than reversing
it: the active pane is marked on its path bar because "in a dual-pane manager
the inactive side must stay fully readable", and an outline keeps the
inactive rows readable while saying where its cursor is.

**F11 · Going back up loses the scroll position.** Scroll down a long list,
enter a subdirectory, come back: the cursor lands on the directory just left
(that part works, and is tested), but the viewport is not where it was.

### Withdrawn

**F12 · `Ctrl+S` matching.** A subsequence match (`pkag` finds `package`) was
proposed and withdrawn. The filter stays a case-insensitive substring, the
rule in `listing.md` stands, and no test changes.

## 3. What this costs the existing code

With archives and the quick filter withdrawn, **nothing in this plan changes
an existing rule.** Every remaining item adds behaviour where there was none,
or fixes behaviour that was already meant to work. No existing test should
need its assertions changed; if one does, that is a signal to stop and say so
(skill 24) rather than a chore.

Column widths move five constants into the settings file. `config.md`'s rule
is that a setting the app owns is written as it changes — five more of them
is not new machinery.

The job manager is now the largest item, and the only one with genuine
design in it. The queue already exists in `fc-core` and already runs jobs one
after another, so what is missing is a window over it, not an engine under
it — but "several at once" is a change to the queue's own shape, and the
rule that closing the window leaves jobs running is the part that has to be
true under a test rather than by inspection.

Free space is the one item needing a platform call, so it lands in
`vfs::platform` with both branches — `statvfs` on Unix,
`GetDiskFreeSpaceEx` on Windows — and the Windows branch is covered by the
cross-target clippy step the gate already runs.

## 4. The awkward corners, each of which gets a test

- A column dragged to zero width, and one dragged wider than the window.
- Free space on a filesystem that reports zero blocks — a full disk is a
  number, not an absent one — and on a path that has gone (an unplugged
  disk), where there is no number at all.
- Enter on a file the system has no handler for, and Enter on a file inside
  an archive, which is refused with a reason.
- The scroll position of a directory that shrank while you were below it:
  the remembered offset is past the end and must clamp, not panic.
- A remembered scroll position for a directory that has since been deleted.
- Closing the job manager while a job runs — the job finishes, and the
  status bar still knows about it.
- Two jobs writing into the same directory at once.
- The status line when free space and a long selection summary compete for
  one line.

## 5. Phases — one phase, one commit

**Phase 0 — reproduce.** Failing UI tests for F7–F11, and the coverage
pre-check over the quick filter, the inline rename, and the pane's cursor
memory. This phase changes no behaviour and its commit says so. Anything
that does not reproduce is reported back rather than fixed.

**Phase 1 — the three small verified ones.** `cd` into the history (F1),
Enter through the system handler (F6), the inactive pane's cursor (F10).

**Phase 2 — the inline rename.** F8 and F9 together: they are the same
widget and the same commit.

**Phase 3 — the list that moves.** F7, once phase 0 says what moves it.

**Phase 4 — what the pane remembers.** F11, the scroll position beside the
cursor, per directory for the session.

**Phase 5 — the status line.** F4: free and total space, right-aligned, and
the platform call behind it — with a Windows branch, because `platform` has
two.

**Phase 6 — columns.** F3: draggable, shared, persisted.

**Phase 7 — the job manager.** Background operations, several at once, and
a window whose closing does not stop them.

**Phase 8 — refactoring audit** (skill 49), and the documentation pass:
`keymap.md`, `config.md`, `ui-shell.md` and `ops.md` all make claims this
plan falsifies.

## 6. Effort

Estimated by phase, then corrected (skill 45). Phases 0–6 are features with
a clear architecture and a test net around them; phase 7 is a UI feature with
iteration and takes the feature factor.

| Phase | Raw | Factor | Corrected |
|---|---|---|---|
| 0 reproduce | 1 d | 0.10 | 1 h |
| 1 three small | 1 d | 0.10 | 1 h |
| 2 inline rename | 0.5 d | 0.10 | 0.5 h |
| 3 the moving list | 0.5 d | 0.10 | 0.5 h |
| 4 scroll position | 0.5 d | 0.10 | 0.5 h |
| 5 status line | 1 d | 0.10 | 1 h |
| 6 columns | 1.5 d | 0.10 | 1.5 h |
| 7 job manager | 3 d | 0.25 | 18 h |
| 8 audit + docs | 1 d | 0.10 | 1 h |

**Phases 0–6 and 8: about 6.5 hours.** Phase 7 is about two days, and is
most of the plan.

Withdrawing archives took three days of revert risk out of this plan, and
withdrawing the quick filter took out the only change that would have made
existing tests wrong.

## 7. What is deliberately not in this

- **Anything that writes into an archive.** Withdrawn on the second round.
  `F4`/`F6`/`F8` go on being refused inside one.
- **Subsequence matching in `Ctrl+S`.** Withdrawn on the second round.
- **Per-pane column widths.** Decided against: the panes line up.
- **Enter executing a binary.** Decided against, in both available forms.
- **Persisting scroll positions across restarts.** The memory is a session's,
  which needs no cap and no settings-file growth.
- **Multi-rename or search reached from the job manager.** The manager shows
  file operations; the other two have their own windows already.

## 8. What was actually done

Phases 0–6 are implemented and phase 7 is half of itself. Every fix carries a
test that was run against the *unfixed* code first and seen to fail — the
probe discipline, applied to eight changes in a row.

| | Finding | Outcome |
|---|---|---|
| F1 | `cd` not in the history | fixed |
| F2 | Space "measures instead of marking" | answered: it does not, and never did |
| F3 | column widths fixed | fixed — draggable, shared, persisted |
| F4 | no free space | fixed — free and total, Unix only |
| F5 | archives read-only | withdrawn |
| F6 | Enter on a file did nothing | fixed — the desktop's handler |
| F7 | rows move when the first is marked | **did not reproduce** |
| F8 | extension shown twice while renaming | fixed |
| F9 | cursor jumps to the top after a rename | fixed |
| F10 | the idle pane's cursor | fixed — an outline |
| F11 | scroll position lost | fixed — per directory, per session |
| F12 | `Ctrl+S` matching | withdrawn |
| — | operations cannot go to the background | half fixed — Background works, nothing lists what is running |

### F7 did not reproduce, and that is a finding

Marking the first row moves nothing here. Screenshots before and after
differ by 918 pixels, all of them the row's text turning red; the mark is
colour only, deliberately, because bold was once wide enough to push the date
out of its column. Marking and unmarking leaves the left pane
pixel-identical.

Three explanations were checked and dropped: the mark's style class changes
no size, the status label is below the rows rather than above them, and it
already holds text before anything is marked, so it never grows from empty.

What would settle it is the reporter's own case: which row, how the list was
sorted, and whether the pane was scrolled.

### What the tests could not see, and what was done instead

Two of these are invisible to the end-to-end suite, and both were checked
another way rather than left on trust.

A **scroll offset** is not a window title, a file on disk or a key press, so
`xdotool` cannot read one. `scripts/check-scroll-memory.sh` is committed for
it, and sets up the one case the cursor cannot explain.

Whether the **other pane** followed a column drag is equally unreadable. The
test asserts the file; the mirroring was checked by eye, and holds by
construction because both panes are set from one field.

### Three bugs the work found in itself

* Marking a **`bin/` directory** into the test home made it a row in the
  pane, and two fixtures that count rows counted it. The private `PATH` lives
  in `.local/bin` now.
* `focus_on_arrival` was set and **changed nothing**, because
  `reload_after_job` never consulted it. Found by running the app, not by
  reading it.
* A column width written into the shell's copy of the settings file made
  `current_settings() == saved` true, and `remember` skips the save when they
  are equal — so every drag was silently dropped. It looked exactly like a
  drag GTK had failed to report, and three screenshots said otherwise before
  an `eprintln` said where it stopped.

### What is left

The **job manager window**. `Background` sends a job on, and nothing lists it
afterwards, so it cannot be watched again or cancelled. The engine is ready
— every job's progress, cancel and report hang off its own handle — and what
is missing is a non-modal window over them.

The queue still runs jobs **one at a time**. The owner asked for concurrent
jobs; that was not done tonight and was not a slip. Two copies writing into
one directory at once is a reliability question rather than a convenience
one, and it should be decided on its own rather than arrive as a side effect
of adding a window.
