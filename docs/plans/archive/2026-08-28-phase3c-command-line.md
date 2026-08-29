# Phase 3c — the command line, and `Ctrl+↓` for its history

**Status:** Implemented
**Design:** [2026-08-28-tc-clone-design.md](2026-08-28-tc-clone-design.md) —
inserted after phase 3b. The design doc lists no command line at all; this
adds one, on the owner's ruling.

## 1. Why

The owner asked for `Ctrl+↑` and `Ctrl+↓` "like in TC". Neither could be
built, because in Total Commander both are handles on features this program
does not have:

| Key | Total Commander | Here |
|---|---|---|
| `Ctrl+↑` | the directory under the cursor in a **new tab** | no tabs — explicitly out of v1 |
| `Ctrl+↓` | drop down the **command line history** | no command line |

Binding either to something else and calling it by its TC name would be the
"almost the right thing" that is worse than missing — the lesson the dead `+`
binding already taught. So the ruling: build the command line, and `Ctrl+↓`
follows from it. Tabs stay out.

## 2. Decisions taken (owner, 2026-08-28)

| Question | Ruling |
|---|---|
| How a command runs | **`$SHELL -c`**, falling back to `/bin/sh` when unset. Pipes, redirection, globs and `~` all work, and it is what a hard-core user types without thinking. |
| Where output goes | **Captured, and shown when there is any.** A silent success closes silently; anything written to stdout or stderr, or a non-zero exit, opens a scrollable window. `ls` that showed nothing would be baffling, and a failing command that said nothing worse. |

Taken from Total Commander without asking, because it answers them plainly:
the line is **always visible** at the bottom, `cd` is handled **internally**
rather than spawned, `Esc` clears it, and the history opens on `Ctrl+↓` **and**
`Alt+F8`.

Decided here: **nothing blocks**. The command runs off the UI thread and the
window stays live; a second command may start while the first runs. A file
manager whose prime directive is speed does not get to freeze while
`find /` finishes ([performance.md](../../performance.md)).

## 3. Phase 0 — coverage pre-check (skill 43)

| Needed | Already there |
|---|---|
| a text entry that does not steal the keyboard | `PaneView`'s filter bar, and the capture-phase rule that stands down inside a text widget |
| a scrollable output window | `dialogs::show_failures`, which is the same shape |
| pick one row from a list | `dialogs::choose_place`, built for the drive selector — **generalise the name**, it is not about places |
| await a worker on the GLib loop | `watch`, which does it for jobs over `async-channel` |
| both panes re-read after something changed them | `Shell::reload_all` |
| settings that persist and survive hand-editing | `tc_core::config` |

Genuinely new: running a process and capturing its output, and the history.

## 4. Sub-phases

### A. Running a command — `tc-core::command`

`run(shell, directory, line) -> Outcome { status, output }`, spawning
`$SHELL -c <line>` with the working directory set and stdout+stderr captured.

In `tc-core`, not the shell: the UI never touches the filesystem directly and
a process is no different, and it makes the whole thing headless-testable —
which matters here, because "did it run in the right directory" is exactly the
sort of claim that is easy to get wrong and impossible to see.

Tests: a command sees the directory it was given; output comes back whole,
both streams; a non-zero exit is reported as failure with its output; a
command that writes nothing and succeeds reports nothing.

### B. The widget, and running from it

An entry across the bottom of the window, prefixed with the active pane's
directory. `Enter` runs, `Esc` clears. The run happens on a worker thread and
the answer arrives on the GLib loop, like a job; both panes reload afterwards,
because a command is the one thing here that can change anything.

`cd` is handled internally and never spawned — a `cd` in a child process
changes nothing anybody can see.

### C. History, and `Ctrl+↓`

Every line run is remembered, newest first, without duplicates. `Ctrl+↓` and
`Alt+F8` open it in the list chooser; picking one puts it in the entry rather
than running it, so it can be edited first. Persisted in `config.toml`, capped
so the file cannot grow without limit.

### D. Typing goes to the command line

Total Commander's feel, and what makes the line usable without reaching for a
focus key: a printable key that **no binding claims** appends to the command
line instead of being dropped. Plain `a` does nothing today; it should start a
command.

Also `Ctrl+Enter`, which inserts the name under the cursor — the one shortcut
that makes a command line in a file manager worth having.

### E. Docs and refactoring audit

**What changed against the plan**, and why:

- **§D was folded into §B.** The plan had the widget ship first and typing
  reach it later, which would have left a commit where nothing focuses the
  entry — a keyboard-first program with a command line nobody can reach has
  not shipped one.
- **The stand-down inside text fields needed an exception.** Not foreseen:
  `Ctrl+Enter` and `Ctrl+↓` are for use *while typing a command*, which is
  exactly when the entry has the focus, so the shell standing down there made
  them unreachable at the only moment they are wanted. Modified keys the
  keymap claims are now dispatched from inside the command line; a plain
  letter is still text.

**What the audit found:**

- **A redundant guard, found by a probe that did not bite.** An `if commanding
  { Proceed }` in the unbound-key path was unreachable in effect: `commanding`
  is only true for a Ctrl or Alt key, and those are the first thing
  `typed_into_command_line` turns away. Deleted.
- **A test too weak to see its own subject.** The one asserting an inserted
  name is a separate word only checked that *some* output window opened — and
  a failing command opens one too, so gluing the name on passed it. It now
  runs `cat notes.txt > out.txt` and asserts the contents, because the
  redirection creates the file either way and only what is in it tells the two
  apart.
- **A harness limitation, fixed rather than worked around.** `xdotool type`
  read a leading dash as a flag, so a test could not append `-again` to a
  command line and failed with no hint that the text was the problem. It
  passes `--` now.
- **`choose_place` became `choose_one`** when the history needed the same
  chooser. It was never about places.

[keymap.md](../../keymap.md), [config.md](../../config.md), a new
[command-line.md](../../command-line.md), and the audit per skill 49.

## 5. Risks

- **Typing-through (§D) is a change to key handling**, which is where this
  program's bugs have historically lived. It must not break the quick filter,
  and the end-to-end suite is what will say so.
- **`$SHELL -c` inherits the user's shell startup**, which is the point rather
  than a side effect, but it means a broken `.zshrc` shows up here.
- **Long output** — captured whole, so a command printing gigabytes is a
  problem. Capped, with the cap named and the truncation visible.

## 6. Effort

Factor 0.25 per skill 45; §D carries the uncertainty.

| Sub-phase | Corrected |
|---|---|
| A. Running a command | ~45 min |
| B. The widget | ~1 h |
| C. History and `Ctrl+↓` | ~45 min |
| D. Typing goes to the line | ~45 min |
| E. Docs + audit | ~30 min |
| **Total** | **~3.5 h** |
