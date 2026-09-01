# Testing round three — the two viewers, and Enter

Status: Draft — decisions settled by the owner, 2026-09-01

Four findings. Two are the same key doing nothing in two different windows,
one is what the diff view looks like, and one is `Enter` on a file — which
turns out to be the oldest known gap in this program seen from a new angle.

## 1. What each one turned out to be

| Reported | Root cause |
|---|---|
| `Page Up`/`Down` do not work in the `F3` viewer — "always jumps to top" | `last_page()` is `size − 32 KiB`, so for any file **under 32 KiB** it is **0** and `scroll_window`'s `.min(last_page())` clamps the offset to zero. `redraw` then puts the label's own scrollbar back to the top. Both keys therefore land at the top of the file |
| `Page Up`/`Down` do not work in the diff view | they are not bound there at all. The capture handler takes `Escape`, `n` and `p` and passes everything else on, where the focused `TextView` handles a page key by moving its own cursor rather than scrolling the one `ScrolledWindow` both sides share |
| The border between the diff panes is far too wide | the `Box` holding them is `homogeneous(true)` and has **three** children — left, separator, right — so the separator is given a third of the window |
| The diff view has no line numbers | never built |
| `Enter` on a file does nothing on Windows | `command::run` always spawns `$SHELL` or `/bin/sh` with `-c`, and the opener is hard-coded `xdg-open`. Neither exists on Windows — and `xdg-open` does not exist on macOS either, so `Enter` is broken on two of the three platforms |

**The `F3` finding was diagnosed from one word in the report.** "Page up/down
does not work" reproduced as working on Linux — `Page_Down` walks a
200 000-line file 0 % → 2 % → 5 %. It was *"always jumps to top"* that named
the cause, because jumping to the top is not a key doing nothing; it is an
offset being clamped to zero. Asking what the symptom actually looked like
was worth more than any amount of reading.

## 2. What the viewer is really missing

The offset is the viewer's whole design: it holds a position in the file and
reads one 64 KiB window around it ([viewer.md](../viewer.md)). What it does
not have is any idea of the **screen**. A 20 KiB file is one window, so there
is genuinely no next page to turn to — and the label holding it is still
taller than the viewport, so there is a screenful the user cannot reach and a
key that takes them to the top instead.

So the fix is not a bigger `last_page`. Paging has to mean *the screen*
first and *the window* second: scroll the label while there is label left,
and move the offset when there is not. That is what makes a short file page
at all, and it is also what stops a long file skipping the lines between the
bottom of one label and the top of the next.

## 3. Decisions

1. **Line numbers: each side counts its own file.** The left gutter numbers
   the left file's lines, the right gutter the right file's, and a filler
   row opposite a one-sided change carries no number, because there is no
   such line in that file. Two columns that disagree exactly where the files
   do. The alternative — one shared row number down the middle — always
   agrees and is therefore useless for finding anything in an editor.
2. **The runner is fixed for every command, not just for `Enter`.**
   `command::run` gets a Windows branch and the desktop opener gets a
   per-platform answer. That makes `Enter`, `F4`'s editor, the compare tool
   and the command line work on Windows in one change — closing a gap that
   has been in [future-improvements.md](../future-improvements.md) since the
   first Windows run — and fixes `Enter` on macOS at the same time.

## 4. Phases

**Phase 0 — coverage pre-check** (skill 43).

- *The viewer's paging.* `fc-core/tests/viewer.rs` covers the offset
  arithmetic headlessly, and `the_viewer_pages_through_a_file_it_never_read`
  covers `End`/`Home` end to end through the title's percentage. Neither
  covers a file **smaller than half a window**, which is the entire bug —
  a characterisation test is owed before the fix.
- *The diff view.* `fc-core/tests/compare.rs` covers the rows; nothing
  covers the window, and the end-to-end suite can see its title and nothing
  else.
- *The runner.* `fc-core/tests/command.rs` covers running a line on Unix.
  The Windows branch will have no runner here at all — the same position as
  the drive-root and free-space fixes, and the same answer: the part that can
  be a pure function is tested on Linux, the syscall is not.

**Phase 1 — the viewer pages the screen, then the file.** `Page Down`
scrolls the label while it has room and advances the offset when it does not;
`Page Up` the mirror. The offset arithmetic stays in `fc-core` and gets the
short-file case it never had; what belongs to the widget — how much room is
left — stays in the dialog. `redraw` stops resetting the scrollbar
unconditionally, because that reset is half of what the report saw.

**Phase 2 — the diff view: keys, gutter, separator.** The separator is one
line: the `Box` stops being homogeneous and the two text views expand
instead. Page keys, `Home` and `End` join `n`/`p` in the handler that is
already there, scrolling the one scroller both sides share. The line numbers
are a third and fourth column of the same grid, filled from the same `Row`
stream that fills the text — so a one-sided row's gutter is blank by
construction rather than by a special case.

**Phase 3 — a command runs on every platform.** `run` picks `cmd /C` on
Windows and `$SHELL -c` elsewhere; the opener becomes `xdg-open`, `open` or
`start ""` per platform. The shell choice is a pure function of the target
and is unit-tested on Linux; that a `.png` opens and an `.exe` runs is the
owner's to confirm, and the plan says so rather than implying a green gate
covers it.

**Phase 4 — docs.** [viewer.md](../viewer.md) for what paging now means,
[compare.md](../compare.md) for the gutter and the keys,
[command-line.md](../command-line.md) and
[windows.md](../windows.md) for the runner,
[future-improvements.md](../future-improvements.md) for the gap that closes.

**Phase 5 — refactoring audit** (skill 49) and the end-of-plan ritual.

## 5. Effort

Corrected per skill 45; the last plans held at ~0.10.

| Phase | Raw | Corrected |
|---|---|---|
| 0 pre-check | 0.25 d | 0.25 h |
| 1 viewer paging | 1 d | 1 h |
| 2 the diff view | 1.5 d | 1.5 h |
| 3 the runner | 1.5 d | 1.5 h |
| 4 docs | 0.5 d | 0.5 h |
| 5 audit | 0.5 d | 0.5 h |

About five hours, plus a full-gate run (~15 min) per phase commit.

## 6. What would make this wrong

- **Paging the screen means the viewer knows about the widget**, which is
  the boundary this project is most careful about: the offset arithmetic is
  `fc-core`'s and headless-testable, and "how much label is left" is GTK's.
  The split has to stay, or the one part of the viewer that *is* tested stops
  being.
- **The `F3` diagnosis has not been reproduced yet**, only read. It explains
  every word of the report and the arithmetic is unambiguous, but the last
  two rounds both had a first diagnosis that was wrong and a measurement that
  settled it. Phase 1 starts with a file under 32 KiB and a screenshot.
- **The Windows runner cannot be tested where it runs.** Third time in three
  rounds; the answer is the same and so is the honesty about it.
- **`start ""` is a `cmd` builtin, not a program**, and its first quoted
  argument is a window *title* rather than the file — a detail that silently
  opens the wrong thing when forgotten. It only works because the runner
  spawns `cmd` in the first place, which ties the two halves of phase 3
  together more tightly than they look.
- **Running an `.exe` on `Enter` is a real action from a keystroke.** It is
  what Explorer and Total Commander both do and what was asked for, and it
  is worth writing down that the program now starts programs.
