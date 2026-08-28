# Search — Alt+F7

← Parent: [CLAUDE.md](CLAUDE.md)

`Alt+F7` finds files below the active pane's directory, by name pattern and
optionally by the text inside them.

## It streams, and it stops

A search over a home directory finds its first hit in milliseconds and its last
in minutes. Both properties follow, and both are the speed requirement rather
than polish ([performance.md](performance.md)):

- **Results appear as they are found.** Waiting for the whole walk before
  showing anything makes the fast case feel like the slow one.
- **The same button stops it.** A search and its cancel are one act from the
  user's side, and two buttons where one will do is one more thing to look at.
  Cancellation is checked between *entries*, not between directories: a single
  directory with a hundred thousand files in it is exactly where a search feels
  stuck.

## What the walk does and does not do

- **Through a queue, not recursion.** A directory tree is user input, and a
  deep enough one turns recursion into a stack overflow — a crash in a file
  manager, over somebody else's directory layout.
- **A directory it cannot read is skipped.** Half a home directory is
  unreadable on any real machine, and a search that stops at the first refusal
  is a search nobody can use.
- **Content matching never holds a file.** It reads windows through `read_at`,
  the same way the [viewer](viewer.md) does, with the windows overlapping by
  the needle less one byte so a match lying across a boundary is still found.

## Choosing a result goes to the file

Enter on a result sends the pane to the file's directory **and puts the cursor
on it**. Getting to the right directory and leaving somebody to hunt for the
row is half the job.

The first result takes the keyboard as it arrives, so Enter goes to it rather
than back to the field it was typed in — which would start the search again and
look like the window ignoring you.

## The list is capped, and says so

A `ListBox` is not virtualised: every row is a widget, so a hundred thousand
results would be a hundred thousand widgets and would take the window down with
them. Past `SEARCH_LIST_LIMIT` the count keeps climbing and the list does not,
and the status line says both numbers. That is the honest version of a limit —
the alternative is a window that appears to have found five thousand files when
it found ninety.
