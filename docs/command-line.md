# Command line — typing a command without leaving the file manager

← Parent: [CLAUDE.md](CLAUDE.md)

An entry across the bottom of the window, running commands in the directory
the **active pane** is showing. Total Commander has one and a hard-core user's
fingers expect it: `chmod +x *.sh` without opening a terminal, finding the
directory again, and coming back.

The running lives in `tc-core::command`, not in the shell — the UI does not
reach past its own layer, and a process is no different from a file. It also
makes the part that matters testable: **"did it run in the directory the pane
was showing"** is the one claim a command line is useless without, and the one
no amount of looking at the screen can check. `rm *.txt` has to mean the
pane's files.

## Typing goes there on its own

A printable key that **no binding claims** starts a command instead of being
dropped — Total Commander's feel, and the only way in from the keyboard.
Before this, plain `a` did nothing.

**A modified key does not type.** `Ctrl+J` and `Alt+J` still report the letter
J, and somebody reaching for a shortcut this program does not have meant a
shortcut. Typing `j` into the line would turn a missing feature into a wrong
answer, and then Enter would run it.

`Enter` runs; `Esc` empties the line and hands the keyboard back. **Running is
also the end of typing**: the line clears and the rows take the keyboard the
moment the command is away, before it finishes, so a long command does not
hold the keyboard for as long as it runs. Leaving the focus in the entry meant
the next `F7` was typed into it rather than opening a dialog, which reads as
the program ignoring you — a test found that.

## The prompt says which directory

The line follows the active pane, so which directory it means changes under
the user with every `Tab`. That is shown rather than left to be remembered: a
command line that did not say is one you check by running something.

## `cd` is read, never run

A `cd` in a child process changes nothing anybody can see, so a command line
that spawned one would look broken. `cd` moves the pane instead — relative
against the pane's directory, `~` and a bare `cd` to the home directory, and
`..` for free because [`VfsPath`](vfs.md) normalizes lexically.

A command that merely *starts* with those letters is not a `cd`: `cdparanoia`
and `cdrecord` are real programs, and swallowing them would be a silent wrong
answer.

## Through a shell, on purpose

`$SHELL -c`, falling back to `/bin/sh`. Pipes, redirection, globs and `~` all
work, which is most of what anybody types into a file manager's command line
and the reason the feature exists. It also means the user's shell startup
applies — the point rather than a side effect, though it does mean a broken
`.zshrc` shows up here.

## Nothing blocks

The command runs on a thread of its own and the answer arrives back on the
GLib loop, like a job. The window stays live and a second command may start
while the first runs: a file manager whose prime directive is speed does not
get to freeze while `find /` finishes ([performance.md](performance.md)).

A plain thread rather than the job queue, because a command is not a file
operation — no progress to report, no conflicts to answer, and no reason to
wait behind a copy.

Both panes reload when it ends. A command is the one thing here that can
change anything, and nothing says which side it touched.

## Output is shown only when there is some

Both streams together, as a terminal shows them and as the user thinks about
them: an error printed between two lines of output belongs between them.

- **A silent success opens nothing.** Otherwise every `touch` costs a dialog
  to dismiss, and a command line that interrupts after every command is one
  nobody uses twice.
- **A silent failure still opens.** "It did nothing and said nothing" must not
  be indistinguishable from "it worked".
- **A shell that cannot start at all** is a failed command carrying the
  reason, not a separate kind of error: from the user's side "it did not run"
  and "it ran and failed" want the same window.
- **Output is capped** at `OUTPUT_LIMIT`, and the cut is marked. A command may
  print gigabytes, and a truncated tail must never read as the end of what it
  said.
