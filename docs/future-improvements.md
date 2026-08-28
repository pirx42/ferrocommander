# Future improvements

← Parent: [CLAUDE.md](CLAUDE.md)

Known gaps that were deliberately left open, with the reason and the place
they belong. Each was found while implementing something else; none is a bug
in what shipped.

## Engine

**A vanished entry fails the whole listing.**
If a file disappears between `read_dir` enumerating it and `stat` reading its
metadata, `LocalFs` returns `NotFound` for the entire directory instead of
omitting the entry that is gone. Skipping it cleanly needs an injection seam
to be testable at all, so the version with no untested code shipped.
*Home:* the refresh logic in design phase 3.
*From:* [vfs.md](vfs.md), walking-skeleton sub-phase A.

**A copy loses the original's permission bits.**
`Entry` carries no mode, so an executable script copied through the operation
engine arrives without its `+x`. Modelling permissions portably — Unix mode
bits against Windows ACLs — is a design question of its own, and the size and
date columns were the phase-2 promise, not the attributes column.
*Home:* phase 3, which adds the attributes column and therefore has to answer
the modelling question anyway.
*From:* [vfs.md](vfs.md), phase 2 sub-phase A.

**A copied directory does not keep its date.**
`VirtualFs::set_modified` needs a handle opened for writing, which no platform
hands out for a directory, so `LocalFs` supports files only. A copied
directory carries the time the copy created it. Files keep their date, which
is what the date column shows.
*Home:* would need `utimensat`-level access (a `rustix`/`libc` dependency) or
the Windows equivalent, weighed against how much a directory's mtime is worth.
*From:* [vfs.md](vfs.md), phase 2 sub-phase A.

## Platform coverage

**The Windows GTK build is unverified.**
The green gate cross-compiles `tc-core` for `x86_64-pc-windows-gnu`, which
covers every platform-divergent line in the project — `tc-app` has no `cfg`
branches. It cannot cover `tc-app` itself, because GTK's `-sys` build scripts
need a mingw libgtk-4 through pkg-config that a Linux box has no way to
provide. Nothing has been *run* on Windows.
*Home:* needs a Windows or mingw toolchain in the loop.
*From:* [vfs.md](vfs.md), [ui-shell.md](ui-shell.md).

## Testing

**The keypress-to-pane path has no automated coverage.**
The keymap table and the navigation targets are unit-tested, but the wiring
between a physical keypress and those functions — the GTK controller, its
capture phase, the focus handling — has no test behind it.

Every phase-1 binding was manually confirmed by the owner on 2026-08-28. That
same session also found a bug no unit test could see — stepping up left the
cursor on `..` instead of the directory just left — while 74 tests passed.
Each unit was correct; the composition was not. That is the argument for this
gap mattering, and for the manual pass being repeated whenever the shell
grows. A second one followed: Page Up/Down moved the widget's selection while
the model's cursor stayed behind, so the next keystroke acted on a stale row.
A third followed: the view never scrolled to the cursor, so it walked off
screen. All three lived in the composition between GTK and the model, which
is exactly the layer with no test. So the wiring is known good today; what is missing is a regression
net, and a future refactor could break it silently.
*Home:* would need a way to synthesize keystrokes in the session, or
`gtk::test` smoke tests as the design doc's testing section anticipates.
*From:* [keymap.md](keymap.md).

**Entry-building test helpers are duplicated.**
Four test modules across both crates build `Entry` values with their own
small constructors. Sharing them needs a `test-support` feature on `tc-core`,
and today the fixtures are shaped to each test's needs — the cure costs more
than the disease.
*Home:* revisit when design phase 2's operation tests need the same shapes.
*From:* the walking-skeleton refactoring audit.
