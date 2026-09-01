# Type-ahead, the Right arrow, and the system clipboard

Status: Implemented — substance extracted to `keymap.md`, `listing.md`,
`command-line.md` and the new `clipboard.md`
(commits `1df3a1d`, `e7e634f`, `5cf254f`, `597b7b4`, `22bd399`)

Three features asked for together, and they are not three independent
things: the first takes the keys the command line currently lives on, and the
second is what gives it back. Doing either alone would leave the program
worse than it is now, which is why they are one plan and why their order is
fixed.

## 1. What was decided, and by whom

Owner spec, 2026-08-30, in answer to four questions:

- **Letters search, and `Right` is the way into the command line.** Type-ahead
  wins the letter keys. This is only safe because the same round asked for
  `Right` — the reason letters typed into the command line is that it was
  otherwise unreachable, and that reason stops applying.
- **Matching is a case-insensitive substring of the whole name**, extension
  included. `tes` finds `notes.txt`, and so does `no` and `.txt`.
- **A cut from another application is honoured as a move**: copy, then delete
  the source once the copy is verified, through the engine that already does
  exactly that.
- **GNOME-style clipboard, plus plain text.**
  `x-special/gnome-copied-files` — which Nautilus, Nemo, Thunar and Caja all
  read and write — plus `text/uri-list` and a plain-text path, so something
  usable reaches anything else. KDE's own cut marker is not in this plan.

## 2. What is already true, checked rather than assumed

- **Plain `Right` and `Left` are unbound.** Only `Ctrl+Right`/`Ctrl+Left` are
  taken, by the two clone-to-the-other-pane commands. So the Right arrow is
  free, and nothing has to be given up to take it.
- **The matcher already exists.** `listing/name.rs` matches the quick filter
  with a case-insensitive `contains` over the whole name — the exact rule
  decided for type-ahead. It is reused, not rewritten (skill 44); one rule
  for "does this name match what was typed" is the point.
- **The keymap is a table.** A new action is a row in it and a match arm, and
  the user can rebind it. Nothing about these three features needs a new way
  of reaching an action.

## 3. What this costs the existing code

This plan changes a **stated rule**, and that is its one real cost. Typing a
letter puts it in the command line today, deliberately: *"the only way into
the command line from the keyboard — without it a keyboard-first program has
a command line nobody can reach"* (`main.rs`). Three tests pin that:

- `typing_a_letter_starts_a_command_and_enter_runs_it`
- `a_shortcut_this_program_does_not_have_types_nothing`
- `the_quick_filter_still_gets_its_own_letters`

The first must change, and its change is the feature. The other two are about
where a letter *does not* go and should go on passing untouched — if either
needs editing, that is a signal to stop and say so (skill 24), not a chore.

The clipboard is the largest piece and the only one touching the ops engine,
but it touches it the way every other key does: by building a `Job` and
handing it to the queue. Nothing new about copying or moving is invented
here. What is new is a *source* of paths that did not come from a pane, which
is why the corners below are mostly about what arrives on the clipboard
rather than about what is done with it.

## 4. The awkward corners, each of which gets a test

**Type-ahead**

- The buffer must expire, or a letter typed a minute later continues a search
  nobody remembers starting. A timeout, and `Escape` clears it now.
- Searching starts *below* the cursor and wraps, so pressing the same letter
  repeatedly walks the matches rather than sitting on the first.
- No match leaves the cursor exactly where it was. Nothing jumps to row zero.
- A pane whose filter hides the only match finds nothing, which is correct:
  type-ahead searches what is shown.
- The buffer is per pane, and switching panes drops it.

**The Right arrow**

- With text already in the command line, `Right` focuses it and puts the
  cursor at the end rather than replacing anything.
- Inside an archive the command line already refuses to run — `Right` may
  still reach it, because refusing to focus a widget teaches nothing.

**The clipboard**

- A percent-encoded URI (`file:///a%20name.txt`) is a space, not a literal
  `%20`. Round-tripping our own paths through the clipboard is the test.
- A URI that is not `file://` — `trash://`, `smb://` — is refused with a
  reason rather than turned into a nonsense local path.
- Paste into the directory the source is already in: the engine refuses a
  copy onto itself, and this must reach that refusal rather than a new one.
- Cut, then paste into a subdirectory of what was cut. The engine refuses.
- A cut that is pasted twice: the clipboard is cleared after a move
  completes, so the second paste has nothing to move rather than failing over
  files that are gone.
- Copy inside an archive: the entries have no operating-system path, so
  `Ctrl+C` is refused with a reason, as `F4` and Enter already are.
- Paste while the pane is inside an archive: refused, since the archive
  backend is read-only.
- Nothing on the clipboard at all, and something on it that is not files.

## 5. Phases — one phase, one commit

**Phase 0 — coverage pre-check.** *Done, and it added nothing.* Both rules
this plan touches are already covered: the letter path by three end-to-end
tests, and `contains_ignoring_case` by a table that already includes the
extension cases type-ahead depends on (`".txt"`, `"port.t"`). Skill 43 says
to skip the phase when the cover is there, so no characterization tests were
written — the check is the deliverable, not more tests.

**Phase 1 — `Right` opens the command line.** First, and deliberately: it is
the replacement route, so it must exist before the letters stop being one.
A commit in between where letters search and nothing reaches the command line
would be a program worse than the one we have.

**Phase 2 — type-ahead.** The letter keys change meaning, the matcher is
reused, the buffer expires, and the one test that pins the old rule is
rewritten with its reason.

**Phase 3 — the clipboard, inside FerroCommander.** `Ctrl+C`/`X`/`V` between
the two panes, through the existing copy and move jobs.

**Phase 4 — the clipboard, with everything else.** The GNOME format,
`text/uri-list`, plain text; reading what another application put there, and
honouring its cut as a move.

**Phase 5 — refactoring audit** (skill 49) and the documentation pass:
`keymap.md`, `command-line.md` and `listing.md` all state rules this plan
changes.

## 6. Effort

Estimated by phase, then corrected (skill 45). Phases 0–2 are features over
a clear architecture with a test net; phases 3 and 4 are features with real
iteration in them, and phase 4 talks to other programs, which is where the
surprises live.

| Phase | Raw | Factor | Corrected |
|---|---|---|---|
| 0 pre-check | 0.5 d | 0.10 | 0.5 h |
| 1 Right arrow | 0.5 d | 0.10 | 0.5 h |
| 2 type-ahead | 1.5 d | 0.10 | 1.5 h |
| 3 clipboard, inside | 2 d | 0.25 | 4 h |
| 4 clipboard, outside | 3 d | 0.25 | 6 h |
| 5 audit + docs | 1 d | 0.10 | 1 h |

**About thirteen and a half hours**, most of it the clipboard.

## 7. What is deliberately not in this

- **KDE's cut marker.** `application/x-kde-cutselection` is not written or
  read; a cut in Dolphin will paste as a copy. Named here so that it is a
  decision rather than a discovery.
- **Drag and drop.** The clipboard formats are most of what it would need,
  but it is a different set of gestures and a different plan.
- **Type-ahead across directories.** It searches the pane it is in. Finding a
  file somewhere below is what `Alt+F7` and `Ctrl+B` are for.
- **A configurable type-ahead timeout.** One value, in the constants, until
  somebody says otherwise.

## 8. What was actually done

All three features are in, and the order the plan insisted on turned out to
matter for a reason it did not foresee.

### The ordering earned itself twice

`→` first, then the letters — that part went as planned. What the plan did
not see is that **phase 1's test could not discriminate until phase 2
landed**. Pressing `→` and typing a command passed with the binding deleted,
because a letter reached the command line either way. The probe caught it;
the test now goes through `Space`, whose meaning differs at the only moment
focus differs.

That is the second test in this repository caught proving nothing, after the
stylesheet one. Both were caught the same way — by breaking the thing on
purpose and watching the test stay green.

### The prediction about the three tests held exactly

The plan named three tests pinning the letter rule and said one must change
while the other two must not. That is what happened: the one whose rule *is*
the feature was rewritten, and the two about where a letter must **not** go
passed untouched. The stop-signal was armed and did not fire.

Fifteen further tests changed, which the plan did not predict and should
have: they typed a command as a *means*, so they now press `→` first with
their assertions untouched. Two more were rewritten rather than patched,
because they proved a rename guard *through* the fallback this work removed
— "with no editor open the typing falls to the command line".

### Phases 3 and 4 collapsed into one, and then did not

The plan separated "inside FerroCommander" from "with everything else". Using
the *system* clipboard for both — which is the right design — made the split
an accounting fiction, so they were committed together.

Then the commit message overclaimed: two of phase 4's corners were still
open, and a follow-up finished them. Saying so cost less than leaving the
claim standing.

### What the audit found

The plain-text encoder was in the shell while its two siblings were in
fc-core. Moved.

And a real one, found by asking what the API promises rather than by a test:
`read_bytes_async` reads *up to* its limit, so a full buffer and a truncated
one are indistinguishable — and the code used whatever came back. A copy
would have silently missed files; a **cut would have moved a subset and then
cleared the clipboard holding the rest**. A read that comes back exactly full
is refused now.

### What is not here

- **KDE's cut marker**, as decided. A cut in Dolphin pastes as a copy.
- **Drag and drop**, which the formats would mostly serve.
- **Windows**, where the clipboard is `CF_HDROP` and none of this applies.
  The module says so rather than pretending to be portable.
