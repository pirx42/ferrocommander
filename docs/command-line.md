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

## Getting the keyboard into it

`→` from a pane, and that is the only way in from the keyboard — see
[the section below](#getting-into-it-from-the-keyboard) for why it is a key
rather than a letter.

**Letters used to be the way in**, which is the shape the rest of this file
was written around: a printable key no binding claimed started a command
rather than being dropped. Type-ahead took them ([listing.md](listing.md)),
and `→` landed first so that it could.

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

## `Ctrl+↓` and `Alt+F8` open the history

Total Commander's two ways to the same list, and both are muscle memory.
Picking a line **puts it in the entry rather than running it**, with the cursor
at the end — editing a previous command is most of why anybody opens a history
at all.

- **Newest first, no duplicates.** A line run again moves up rather than
  appearing twice; a history listing `make` eleven times is one you read past
  to find anything else.
- **Remembered before it runs, whatever it does.** A command that failed is
  the one most worth getting back to and correcting.
- **It survives a restart**, in `command_history` in
  [config.md](config.md) — a history that forgot everything when the app closed
  would be one in name only. Capped at `COMMAND_HISTORY_LIMIT`, because the
  settings file is rewritten whenever anything changes and an unbounded list
  would make that write grow without limit.
- **An empty history opens nothing.** Nothing run yet is not worth a window
  listing nothing.

The list is the same chooser the drive selector uses (`dialogs::choose_one`),
which is why it was renamed from `choose_place` — it was never about places.

## `Ctrl+Enter` inserts the name under the cursor

The one shortcut that makes a command line in a file manager worth having:
act on the file you are looking at without typing its name. Appended as a
**separate word** — the difference between `lsnotes.txt` and `ls notes.txt` —
unless the line already ends in a space, because reaching for the space bar
first would make the shortcut not worth using.

**It follows the active pane**, and typing does not change which one that is:
the focus moves into the entry, but the pane the keyboard came from is still
the one being looked at. A name taken from the other side would be plausible
right up until it named a file that exists on both — so the test presses Tab
first, and uses a name both panes have with different contents to say which
one it came from.

## The shell keeps its hands off a text field — with one exception

While a text field has the focus the shell dispatches nothing, or every letter
would become a command. The command line's own shortcuts are the exception,
and they earn it: `Ctrl+Enter` and `Ctrl+↓` are for use **while typing a
command**, which is exactly when the entry has the focus. Standing down there
would make them unreachable at the only moment anybody wants them.

Only modified keys, only ones the keymap claims, and **never one a text field
owns**. A plain letter is text; so are `Ctrl+A`, `Ctrl+C`, `Ctrl+V`, `Ctrl+X`,
`Ctrl+Z` and `Ctrl+Y`, which select, copy, paste, cut, undo and redo *in the
entry* whatever the keymap says about them.

That last clause had to be added. The rule was once "only ones the keymap
claims", which was safe for exactly as long as the keymap claimed nothing a
text field wants — and then `Ctrl+C`, `Ctrl+X` and `Ctrl+V` were bound for the
clipboard ([clipboard.md](clipboard.md)) and started reaching the panes from
inside the entry. Pasting a path into a command began a file copy.

The list names **what a text field owns**, not what collides today: `Ctrl+Y`
is in it although nothing claims it. That is the difference between fixing
this and meeting it again the next time somebody binds a key.

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

**On Windows this runs nothing at all.** Neither `$SHELL` nor `/bin/sh`
exists there, so a command fails before it starts. What should take their
place is a product question and not just a constant, because `cmd.exe` does
not do `~` or globbing — the things the paragraph above calls the reason the
feature exists ([future-improvements.md](future-improvements.md)).

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

## Getting into it from the keyboard

`→` focuses the command line. A pane has no horizontal movement to spend the
key on — `Ctrl+←`/`→` clone a pane and plain `Left` and `Right` were unbound —
and the line needs a way in that does not cost the letter keys.

That last part is the point. Typing a letter has always been the way in, and
that is what makes the letters unavailable for anything else; type-ahead wants
them ([listing.md](listing.md)). This key exists so the reason letters type
here stops applying, which is why it landed before type-ahead rather than
after.

`Escape` is still the way out, and it **clears** the line on the way — one key
for "never mind" rather than select-all-and-delete followed by a reach for
Tab. So "text in the line while the rows have the keyboard" is not a state
this program has, and `→` focusing rather than clearing is a property with
nowhere to show itself today.
