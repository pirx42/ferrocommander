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

**`.tar.zst` is not read or written.**
The v1 scope named it beside `.tar.gz`, and nothing in building the archive
layer turned up a reason for it: zstd is another compressor and another
dependency, and `.tar.gz` is what the world actually ships. The format table
is one match arm and the wrapper is one enum variant, so adding it is small —
it just has not earned itself.
*Home:* whenever somebody has a `.tar.zst` they want to open.
*From:* the phase 7 audit, against the v1 scope list.

**A job can be cancelled but not paused.**
"pause/cancel" was one phrase in the scope list. What a pause *means* for a
job holding an open file across a conflict prompt is not one phrase, and
nothing since has asked for it. Cancel is implemented, tested, and rolls back
the file it was in the middle of.
*Home:* if a long copy over a slow mount ever makes somebody want it.
*From:* the phase 7 audit, against the v1 scope list.

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

**macOS is wanted, to the standard a Mac user would accept.**
Decided 2026-08-29. Not "it compiles" — the engine very nearly does already,
and a second unverified platform beside Windows would make the claim weaker
rather than stronger. The bar is a program somebody would keep.

Four separate pieces of work, and the engine is the cheap one:

| | What it is | Rough |
|---|---|---|
| `tc-core` | three Linux-only functions in the `unix` branch | ~half a day |
| the keymap | macOS defaults, shipped as a `[keys]` layer | ~half a day |
| packaging | an `.app` bundle with the GTK dylibs inside it | ~1–2 days |
| the end-to-end suite | a second driver; `Xvfb` and `xdotool` are X11-only | **days, and the least certain number here** |

The engine is nearly free because macOS *is* `unix`: `VfsPath`, `Entry`, the
attribute bits, `ops`, `archive`, `listing`, `search` and the viewer compile as
they stand, `trash_error` already carries a `macos` arm, and `notify` is
already built with `macos_kqueue`. Three functions assume Linux specifically —
`mount_points` reads `/proc/self/mounts`, so **the drive bar would be empty**;
`config_dir` follows XDG where macOS puts things under `~/Library`; and
`is_hidden` knows about the leading dot but not `UF_HIDDEN`. The
`parse_mount_table` split already separates reading the table from judging it,
so the macOS version is a different reader against the same filter and the
same fixture tests.

The keymap is a second *default*, not new machinery: 55 F-key bindings on a
platform that gives F1–F12 to hardware unless the user says otherwise, and 53
`Ctrl` bindings where a Mac user reaches for `Cmd`. The `[keys]` table
([config.md](config.md)) already carries exactly this shape.

**The test suite is the part that decides whether this is honest.** The 117
end-to-end tests are this project's main defence — six of the defects in
[reliability.md](reliability.md) were caught by them and by nothing else — and
they drive a real binary through `Xvfb` and `xdotool`, both X11-only. macOS has
no headless equivalent; the same job wants AppleScript or the Accessibility
API against a real GUI session with screen-recording permission granted. So
either macOS ships untested at the exact layer where every bug in this project
has lived, or that driver gets written. Shipping without it would contradict
[reliability.md](reliability.md), so it is not a corner to cut quietly.

**The open question is GTK4 itself**, and it is a product question rather than
an engineering one: non-native window chrome, no menu bar, and the least
maintained of GTK's backends. If that does not clear the bar, this entry
becomes a different one — a second front end over the same `tc-core`, which is
the one thing the crate split makes possible at all
([archives.md](archives.md) keeps the score on what that split is worth).

*Home:* **verify Windows first.** It is already a compile target in the green
gate, it shares the whole second-platform apparatus, and running it would
price the platform boundary for real before a fortnight is spent on the third
one. The estimates above were read off the code by someone with no Mac to try
it on, and the harness number is the one to distrust.
*From:* the phase 7 discussion, against the v1 scope's Linux-and-Windows line.

## Testing

**Two bindings have no end-to-end coverage.**
`crates/tc-app/tests/ui.rs` drives the real binary with real key events and
covers every binding except `Ctrl+Q` and `Backspace` — quitting would end the
app the test is driving, and going up a directory has no filesystem effect to
assert on. Both were verified by hand.

*(The Page Up/Down selection-adoption path used to be listed here as needing
"an assertion about which row the cursor is on, which the filesystem cannot
answer". It can: F5 with nothing marked copies the cursor row, so the
filesystem says which row that was. Covered since the architecture review.)*

**Text on screen is not covered either.** The path bar and the command-line
prompt are GTK labels, and the suite can see window titles and the filesystem
and nothing else — so "the prompt inside an archive reads `…/bundle.zip`
rather than `/`" was verified by screenshot and is not pinned by a test. A
test written for it would have asserted on the command it *ran*, which is
already covered, while looking like it checked the label.
*Home:* wants the pane's state readable from outside, the same thing the Page
Up/Down gap wants.
*From:* the phase 7 audit's smoke run.

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

## Free space on Windows

The status line's disk figure is Unix-only. `statvfs` has no Windows
equivalent in `std`, and `GetDiskFreeSpaceExW` would need a Windows API crate
this workspace does not otherwise want — for one number. The Windows branch
returns `None`, so the figure is simply absent there.

A status line with nothing in it is honest; one showing a made-up number is
not. Whoever adds the crate gets the figure for free — the trait method, the
call site and the formatting are already there and platform-agnostic.
