# Future improvements

← Parent: [CLAUDE.md](CLAUDE.md)

Known gaps that were deliberately left open, with the reason and the place
they belong. Each was found while implementing something else; none is a bug
in what shipped.

## Engine

**Windows copies keep only the read-only flag.**
`std::fs` can set that and nothing else, so hidden, system and archive are
read and displayed but not restored onto a copy. Setting them needs
`SetFileAttributesW`, and this project has no Win32 binding yet. Unix keeps
the full permission bits.
*Home:* whenever a Win32 dependency is justified on its own merits.
*From:* [vfs.md](vfs.md), phase 3 sub-phase E.

**A copied directory does not keep its date.**
`VirtualFs::set_modified` needs a handle opened for writing, which no platform
hands out for a directory, so `LocalFs` supports files only. A copied
directory carries the time the copy created it. Files keep their date, which
is what the date column shows.
*Home:* would need `utimensat`-level access (a `rustix`/`libc` dependency) or
the Windows equivalent, weighed against how much a directory's mtime is worth.
*From:* [vfs.md](vfs.md), phase 2 sub-phase A.

**`.7z` and `.rar` are not read, and probably never will be here.**
`.rar` has no freely licensed extractor — unrar's licence forbids using it to
build a competing archiver — and a 7z decoder is a large dependency with no
evidence anybody here wants it. An archive in an unsupported format stays an
ordinary file: it is listed, viewed and copied, it simply does not open as a
directory.
*Home:* only if somebody asks with a use for it.
*From:* [archives.md](archives.md), phase 6 sub-phase A.

**A symlink inside a tar is not listed.**
Nor are hard links, devices or fifos. A pane has nowhere to show what a link
inside an archive points at, and their recorded size is zero — so listing one
would unpack it as an empty file, which is worse than not listing it. A zip
made on Unix stores a symlink as a file whose contents are the target, and
those *are* listed, as the files they claim to be.
*Home:* wants an `Entry` that can carry a link target the pane can render, and
an unpacker that creates links rather than files.
*From:* [archives.md](archives.md), phase 6 sub-phase B.

**Undo is one multi-rename deep.**
`Ctrl+Z` puts the last batch back and forgets it. Nothing else in the program
is undoable, and a rename stack would be the visible half of a general
operation history — which needs somewhere to record what a copy or a delete
did, and an answer for what "undo a delete" means once the trash is involved.
One batch is what the multi-rename tool actually needs to be safe to use.
*Home:* whenever an operation history is worth its own phase; it is not in v1.
*From:* [multi-rename.md](multi-rename.md), phase 5 sub-phase D.

**The keymap table in the docs is kept by hand.**
`docs/keymap.md` lists every binding, and nothing checks it against the
`BINDINGS` table it describes — the only doc-and-code pair in the repository
with no test between them. A test could read the file and insist every action
appears, but only if the doc carried action names beside the keys, which is a
column for the program's benefit rather than the reader's.
*Home:* generating the table from `BINDINGS` at build time, if it ever drifts
in practice rather than in principle.
*From:* the phase 7 audit.

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
