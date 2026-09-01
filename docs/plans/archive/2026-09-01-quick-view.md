# Quick view — `Ctrl+Q` shows the cursor's file in the other pane

Status: **Done**, 2026-09-01 — all six phases, each behind a green gate.
Outcome in § 6.

Total Commander's `Ctrl+Q`: the opposite pane stops being a directory and
becomes a window onto whatever the cursor is on, following it as the cursor
moves. Same content `F3` shows, without the window and without leaving the
rows. And `Ctrl+Q` is where quit lives today, so quit moves to `Alt+F4` on
Linux and Windows, `Cmd+Q` on macOS.

## 1. What exists to build on

- **The viewer engine is already what quick view needs.**
  `fc_core::viewer::View` opens by reading a file's *size* and nothing else,
  and `render` reads one 64 KiB window on demand
  ([viewer.md](../../viewer.md)). A preview that follows a cursor is the same
  call the `F3` window already makes, so nothing about *what is shown* is
  new work — only where it is drawn.
- **The pane is a vertical box** — path bar, filter bar, the scrolled
  column view, status line (`pane.rs`). Swapping the column view for a text
  widget is a `gtk::Stack` around that one child: two pages, the listing's
  own state left alive underneath, so leaving quick view restores the pane
  rather than rebuilding it.
- **Quit's other half already exists.** `MACOS_LAYER` maps `cmd+q` to
  `quit` today and is applied over the defaults, so the macOS half of the
  move is done: what changes is the default `ctrl+q` binding underneath it.
- **The keymap gate will not let this be half-done**: a binding that no
  end-to-end test presses is red unless `UI_UNPRESSED` excuses it, and the
  bindings and action tables in [keymap.md](../../keymap.md) are checked
  against the code.

Three things the plan has to decide rather than discover:

- **`Alt+F4` may never reach us.** On a desktop with a window manager,
  `Alt+F4` is conventionally the WM's, not the application's — it closes
  the window without the app seeing the key. Under the end-to-end suite's
  Xvfb there is *no* window manager, so the app sees it and the binding is
  what quits. Both paths end in a closed window; the plan states this
  rather than discovering it when the test passes for the wrong reason.
- **A read per cursor move** is new: the prime directive's territory.
  Holding `Down` through a large directory must not stall, and each step
  costs a `stat` plus one windowed read. Phase 1 measures it before
  anything is built on it (skill 74's rules for what the input looks like).
- **The suite cannot read a pane's text.** The harness observes the window
  title, the log, the settings file and the filesystem — not widget
  contents ([ui-shell.md](../../ui-shell.md)). What an end-to-end test can
  assert about quick view is therefore limited, and decision 2 below
  changes the answer.

## 2. Decisions before implementation

1. **Can the keyboard get *into* the quick view?** Settled: **no — it is
   strictly a preview.** The cursor stays in the rows, `Tab` keeps
   its meaning, and the preview always shows the head of the file. That is
   the whole gesture: arrow through files, watch content go by. `F3` is one
   key away when somebody wants to read rather than glance, and it already
   has the paging, the hex mode and the encodings. Alternative: Total
   Commander lets `Tab` move into the panel so it can be scrolled, which
   means a second focus state, a second set of key meanings, and a `Tab`
   whose behaviour depends on a mode.
2. **Does the mode survive a restart?** Settled: **no, it is
   transient.** A file manager that starts with one pane showing the head
   of a text file instead of a directory has to be explained; `Ctrl+Q` is
   cheap to press again. The cost is honest and worth naming: the settings
   file is the *only* thing the end-to-end suite can read, so a transient
   mode is one the suite can only observe indirectly (§3, phase 4).
   Alternative: persist it like `active_pane`, which buys a direct
   end-to-end assertion and a mode that outlives a restart.
3. **What the preview shows when the cursor is not on a readable file.**
   Settled, against the plan's recommendation of a placeholder line: **a
   directory shows what it holds** — the count and the byte total
   `Alt+Shift+Enter` already produces. `..` and a file that cannot be
   opened still get the short line (`(cannot be read)`), because neither
   has contents to summarise.

   This is the decision that costs something, and the cost lands on the
   cursor. Counting a folder is a *recursive walk*: `Alt+Shift+Enter`
   exists as a deliberate, on-demand key precisely because the answer is
   not free, and a walk started on every cursor step over a
   directory-heavy tree is the stall the prime directive forbids. What
   makes it affordable is that the machinery already exists and was built
   for exactly this shape (the space-counts plan): `fc_core::sizes` walks
   on a worker with a cancel token, and the pane already knows how to
   recognise and drop an answer that arrives after the cursor moved on.
   So the rule is: **the summary is asked for, never waited for.** The
   preview shows the directory's name immediately and the figures when
   they arrive; a cursor that moves first cancels the walk it started, and
   a late answer is dropped rather than drawn. Phase 1 measures the step
   that decides whether even *starting* a walk per keystroke is too much,
   and phase 3 carries the debounce if it is.

## 3. Phases

**Phase 0 — coverage pre-check** (skill 43), *done, and it moved phase 2*.
The seams are covered: the keymap tables by the gate's own tests,
`viewer::View` headlessly by `fc-core/tests/viewer.rs`, the pane's widget
tree by the end-to-end suite alone, and the directory walk by
`fc-core/tests/sizes.rs`. Nothing owed characterisation tests.

**What it found is bigger than the plan assumed.** `ctrl+q` is not pressed
by a *test* — it is pressed by the **harness**: `App::close()` sends it to
quit the app, because settings are written from GTK's close handler and a
killed process never runs one. Eleven tests close an app that way and
seven of those relaunch it afterwards to read what was written, so moving
quit off `ctrl+q` moves the mechanism that proves the settings survive at
all. The gate's binding-pressed test scans the harness as well as the
suite, which is why the binding counts as pressed today and why it will
keep counting once the harness says `alt+F4`.

So phase 2 is not "rebind two keys": it is rebinding two keys *and* the
one line every closing test depends on, with the whole suite as the check
that it still shuts down cleanly. That is the phase's real risk, and it is
in front of it rather than behind it.

**Phase 1 — the cost of following a cursor** *(done; the numbers are in
[performance.md](../../performance.md), and they settled the debounce
question: cancellation, not delay).* Before the feature: a
benchmark for what one preview step costs — `stat` plus one 64 KiB
windowed read — with the input layout stated as part of the claim (skill
74), including the two cases that cannot flatter it: an archive member, where
a "read one window" is a decompression, and — from decision 3 — a *deep
directory* under the cursor, where the preview starts a recursive walk.
The second is the one that decides whether a walk may start from the key
handler at all, or needs the delay that lets a held-down arrow key pass
over a folder without ever asking. The number decides whether the preview may update straight
from the key handler or has to be debounced, and it goes in
[performance.md](../../performance.md) either way.

**Phase 2 — the keys move, harness included.** `Action::Quit` rebinds to
`Alt+F4` and `App::close()` sends that instead — the change phase 0 found,
and the one the whole suite checks by continuing to shut down cleanly. The
macOS layer keeps `cmd+q → quit` and gains nothing.

`Ctrl+Q` is left **unbound** here rather than pointed at a `QuickView` that
does nothing yet. The plan first proposed the latter, and the gate would
have refused it: a binding no end-to-end test presses is red, and a test
pressing a key with no visible effect is a test proving nothing — which
this repository has twice caught itself writing. Unbinding needs no
temporary excuse in `UI_UNPRESSED` and leaves the risky half of this
phase — one harness line that eleven closing tests lean on — alone in its
own commit. Quick view claims the key in phase 3, together with the
preview and the tests that press it.

**Phase 3 — the preview itself.** The `gtk::Stack` in the pane, the text
widget, and the wiring: what the *other* pane shows is a pure function of
the active pane's cursor, unit-tested per case from decision 3 — a file,
a directory, `..`, an unreadable file, an empty listing. The directory
case is where the phase's weight is: the walk goes to `fc_core::sizes` on
a worker with the pane's cancel token, the answer is matched against what
the cursor is on *now* before it is drawn, and phase 1's measurement
decides whether the walk starts on the keystroke or after a pause. The preview updates where
the cursor already reports having moved, so nothing new has to remember to
call it (the lesson the adoption plan paid for). Leaving quick view puts
the pane back exactly as it was, because the listing was never torn down.

**Phase 4 — the end-to-end tests, and what they can honestly assert.**
That `Ctrl+Q` does not quit any more and `Alt+F4` does; that the pane in
quick view stops responding to the keys that would move a listing, which
is observable through the settings file's per-pane directory; and that a
second `Ctrl+Q` restores the pane where it was. What no test can assert is
the *text on screen* — that is stated in [ui-shell.md](../../ui-shell.md)
beside the two things the suite already cannot see, rather than left as a
gap somebody rediscovers.

**Phase 5 — docs.** [keymap.md](../../keymap.md) (forced by the gate),
a quick-view section in [viewer.md](../../viewer.md) — same engine, second
surface, so one document rather than a new one (skill 29) — and
[performance.md](../../performance.md)'s number from phase 1.

**Phase 6 — refactoring audit** (skill 49) and the end-of-plan ritual.

## 4. Effort

Corrected per skill 45; the last plans held at ~0.10.

| Phase | Raw | Corrected |
|---|---|---|
| 0 pre-check | 0.25 d | 0.25 h |
| 1 measurement | 0.75 d | 0.75 h |
| 2 the keys | 0.75 d | 0.75 h |
| 3 the preview | 2.5 d | 2.5 h |
| 4 end-to-end | 1 d | 1 h |
| 5 docs | 0.5 d | 0.5 h |
| 6 audit | 0.5 d | 0.5 h |

About six hours of work, plus a full-gate run (~15 min) per phase commit.
Decision 3 is what moved phases 1 and 3: a directory summary is a walk, and
a walk behind a cursor key needs measuring and cancelling rather than just
calling.

## 5. What would make this wrong

- **`Alt+F4` under a real window manager** is the WM's key before it is
  ours. The suite will pass because Xvfb has no WM, and a person on GNOME
  will still see the window close — for a different reason. That is fine,
  and it means the end-to-end test proves less than it appears to; the plan
  says so rather than letting the green tick imply otherwise.
- **The preview's cost is measured on this machine's page cache.** A
  directory on a network mount, or an archive member, is the case that can
  make the cursor feel heavy, and phase 1's benchmark has to include one
  rather than reporting the flattering number.
- **Decision 3 puts a recursive walk behind an arrow key**, which is the
  one thing in this plan that could make the pane feel slower than it does
  today. The mitigations are named — worker, cancel on move, drop late
  answers, and a delay if the measurement asks for one — but the honest
  version is that this is the part most likely to need a second pass after
  somebody holds `Down` through a real tree.
- **What the suite cannot see** is the feature's whole point. The tests
  can prove the mode is on, that the pane stopped being a listing and came
  back — not that the right bytes are on the screen. The engine's own tests
  carry that half, and the honest statement of the split belongs in the
  docs rather than in a commit message nobody re-reads.

## 6. Outcome

Six phases, six commits, six green gates. What the plan got right, and the
four things it did not:

**The measurement moved the design, as it was meant to.** Phase 1 asked what
one preview step costs and got two answers rather than one: 0.004 ms for a
file of any size, and 0.058 ms to *start* a directory walk whose 5.745 ms
over 2 000 entries runs on behind the keystroke. That second pair is what
settled decision 3's open question — cancellation, not delay — and a
benchmark of the keystroke alone would have reported 0.058 ms and said the
walk was free. The first version of that benchmark did exactly that by
pointing at a 26-entry folder: 0.231 ms, 25 times flattering, and the third
time in this repository a benchmark has had to be told what a hard case looks
like (skill 74).

**Phase 0 found the risk, and it was not in the feature.** `Ctrl+Q` was not
pressed by a test but by the *harness*: `App::close()` quit the app with it,
eleven tests close that way and seven relaunch afterwards to read what was
written. So rebinding quit moved the mechanism that proves the settings
survive at all. Isolating that in its own commit — with `Ctrl+Q` left
unbound rather than pointed at an action that did nothing — is what kept the
suite's verdict on it readable.

**Phases 3 to 5 became one commit**, which phase 2's own text had already
predicted: a binding no end-to-end test presses fails the gate, so the key,
the preview and the tests that press it cannot be separate commits. The
docs followed in the same one, as skill 28 asks.

**What the plan did not foresee** is that the preview is redrawn after every
keystroke, not only the ones that move the cursor — the direct consequence
of hanging it beside `follow_active` instead of asking each action to
remember, which is the arrangement the adoption plan paid for. Restarting a
folder's walk unconditionally would therefore throw away a finished count
and go back to "counting…" whenever anybody pressed a key. `preview_folder`
returns `None` for the folder it is already counting. It is the one part of
the feature no test covers, and honestly so: it is a property of what is on
the screen, and the screen is what this suite cannot read.

**Two probes, both screaming** (skill 59): a `Ctrl+Q` that does nothing, and
a `show_listing` that stops the walk but never switches the page back. The
test that catches both — `a_previewed_pane_has_no_rows_to_click` — works
because a click moves a pane's cursor without giving it the keyboard, and
`F5` on `..` opens no dialog; the copy dialog is therefore the answer to
"did the click find a row". The other three end-to-end tests are honest
about proving less: they say the key stopped quitting, that a previewed pane
comes back with its cursor intact, and that the preview follows the keyboard.

**The audit's one finding** was a second source of truth: the shell derived
"which folder is being counted" from the listing while the pane already held
it, so a late answer was matched against a recomputation rather than against
the walk that produced it. The pane answers now, and `Shell::previewed_folder`
is gone. Also recorded rather than left to be rediscovered: why the preview's
walk is not part of `abandon_background` (`Escape` acts on the pane with the
keyboard, and the preview is by definition the other one).

Two unit tests were wrong before they were right, in the way this repository
keeps finding: `/dir` has a parent, so the model offers `..` and starts the
cursor on it — and both "the cursor is on a file" tests were quietly asserting
about the parent row.
