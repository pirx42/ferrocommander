# Quick view — `Ctrl+Q` shows the cursor's file in the other pane

Status: Draft — §2's decisions are open

Total Commander's `Ctrl+Q`: the opposite pane stops being a directory and
becomes a window onto whatever the cursor is on, following it as the cursor
moves. Same content `F3` shows, without the window and without leaving the
rows. And `Ctrl+Q` is where quit lives today, so quit moves to `Alt+F4` on
Linux and Windows, `Cmd+Q` on macOS.

## 1. What exists to build on

- **The viewer engine is already what quick view needs.**
  `fc_core::viewer::View` opens by reading a file's *size* and nothing else,
  and `render` reads one 64 KiB window on demand
  ([viewer.md](../viewer.md)). A preview that follows a cursor is the same
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
  bindings and action tables in [keymap.md](../keymap.md) are checked
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
  contents ([ui-shell.md](../ui-shell.md)). What an end-to-end test can
  assert about quick view is therefore limited, and decision 2 below
  changes the answer.

## 2. Decisions before implementation

1. **Can the keyboard get *into* the quick view?** Recommended: **no —
   it is strictly a preview.** The cursor stays in the rows, `Tab` keeps
   its meaning, and the preview always shows the head of the file. That is
   the whole gesture: arrow through files, watch content go by. `F3` is one
   key away when somebody wants to read rather than glance, and it already
   has the paging, the hex mode and the encodings. Alternative: Total
   Commander lets `Tab` move into the panel so it can be scrolled, which
   means a second focus state, a second set of key meanings, and a `Tab`
   whose behaviour depends on a mode.
2. **Does the mode survive a restart?** Recommended: **no, it is
   transient.** A file manager that starts with one pane showing the head
   of a text file instead of a directory has to be explained; `Ctrl+Q` is
   cheap to press again. The cost is honest and worth naming: the settings
   file is the *only* thing the end-to-end suite can read, so a transient
   mode is one the suite can only observe indirectly (§3, phase 4).
   Alternative: persist it like `active_pane`, which buys a direct
   end-to-end assertion and a mode that outlives a restart.
3. **What the preview shows when the cursor is not on a readable file** —
   a directory, `..`, or a file that cannot be opened. Recommended:
   **a short line saying so**, in the same place the content would be
   (`(directory)`, `(cannot be read)`), so the pane never looks broken or
   stale. Alternative: leave the previous file's content until the cursor
   reaches another readable one, which is what a stale preview looks like.

## 3. Phases

**Phase 0 — coverage pre-check** (skill 43). The seams: the keymap tables
(covered by the gate's own tests), `viewer::View` (covered headlessly by
`fc-core/tests/viewer.rs`), the pane's widget tree (covered only by the
end-to-end suite), and quit (pressed today as `ctrl+q`). Nothing here is
unpinned enough to owe characterisation tests; what is owed is a
*measurement*, which is phase 1.

**Phase 1 — the cost of following a cursor.** Before the feature: a
benchmark for what one preview step costs — `stat` plus one 64 KiB
windowed read — with the input layout stated as part of the claim (skill
74), including the case that cannot flatter it: a directory of large files
on a cold cache, and an archive member, where a "read one window" is a
decompression. The number decides whether the preview may update straight
from the key handler or has to be debounced, and it goes in
[performance.md](../performance.md) either way.

**Phase 2 — the keys move.** `Action::Quit` rebinds to `Alt+F4`;
`ctrl+q` becomes `Action::QuickView`, which for this phase only toggles a
flag and redraws nothing. The macOS layer keeps `cmd+q → quit` and gains
nothing. The gate's tables follow in the same commit, and the end-to-end
quit test moves to the new key — the one existing test whose subject
this phase moves out from under it (skill 74's re-probe rule).

**Phase 3 — the preview itself.** The `gtk::Stack` in the pane, the text
widget, and the wiring: which file the *other* pane shows is a pure
function of the active pane's cursor, unit-tested per case from decision 3
— a file, a directory, `..`, an empty listing. The preview updates where
the cursor already reports having moved, so nothing new has to remember to
call it (the lesson the adoption plan paid for). Leaving quick view puts
the pane back exactly as it was, because the listing was never torn down.

**Phase 4 — the end-to-end tests, and what they can honestly assert.**
That `Ctrl+Q` does not quit any more and `Alt+F4` does; that the pane in
quick view stops responding to the keys that would move a listing, which
is observable through the settings file's per-pane directory; and that a
second `Ctrl+Q` restores the pane where it was. What no test can assert is
the *text on screen* — that is stated in [ui-shell.md](../ui-shell.md)
beside the two things the suite already cannot see, rather than left as a
gap somebody rediscovers.

**Phase 5 — docs.** [keymap.md](../keymap.md) (forced by the gate),
a quick-view section in [viewer.md](../viewer.md) — same engine, second
surface, so one document rather than a new one (skill 29) — and
[performance.md](../performance.md)'s number from phase 1.

**Phase 6 — refactoring audit** (skill 49) and the end-of-plan ritual.

## 4. Effort

Corrected per skill 45; the last plans held at ~0.10.

| Phase | Raw | Corrected |
|---|---|---|
| 0 pre-check | 0.25 d | 0.25 h |
| 1 measurement | 0.5 d | 0.5 h |
| 2 the keys | 0.5 d | 0.5 h |
| 3 the preview | 2 d | 2 h |
| 4 end-to-end | 1 d | 1 h |
| 5 docs | 0.5 d | 0.5 h |
| 6 audit | 0.5 d | 0.5 h |

About five hours of work, plus a full-gate run (~15 min) per phase commit.

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
- **What the suite cannot see** is the feature's whole point. The tests
  can prove the mode is on, that the pane stopped being a listing and came
  back — not that the right bytes are on the screen. The engine's own tests
  carry that half, and the honest statement of the split belongs in the
  docs rather than in a commit message nobody re-reads.
