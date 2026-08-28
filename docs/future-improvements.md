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

**A move assumes a single store.**
`ops` tries one `rename` for a whole tree and falls back to copy + delete only
on `CrossDevice`. Both address one backend. Phase 2's UI has exactly one, so
the assumption holds today.
*Home:* design phase 6, which introduces the archive backend and therefore the
first cross-store move.
*From:* [ops.md](ops.md), phase 2 sub-phase B.

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

**Two bindings have no end-to-end coverage.**
`crates/tc-app/tests/ui.rs` drives the real binary with real key events and
covers every binding except `Ctrl+Q` and `Backspace` — quitting would end the
app the test is driving, and going up a directory has no filesystem effect to
assert on. Both were verified by hand.

Also uncovered: the Page Up/Down selection-adoption path, which needs a
directory taller than the viewport and an assertion about which row the cursor
is on — neither of which the filesystem can answer. That one wants a way to
read the pane's state from outside.

The gap this replaces was total until phase 2, and it was not theoretical:
phase 1's manual pass found three bugs that 74 green tests missed, all in the
composition between GTK and the model, and the phase-2 suite found two more.
*Home:* extend the suite as bindings are added.
*From:* [keymap.md](keymap.md), [ui-shell.md](ui-shell.md).

**Entry-building test helpers are duplicated.**
Four test modules across both crates build `Entry` values with their own
small constructors. Sharing them needs a `test-support` feature on `tc-core`,
and today the fixtures are shaped to each test's needs — the cure costs more
than the disease.
*Home:* revisit when design phase 2's operation tests need the same shapes.
*From:* the walking-skeleton refactoring audit.
