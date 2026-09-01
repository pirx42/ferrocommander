# Future improvements

← Parent: [CLAUDE.md](CLAUDE.md)

Known gaps that were deliberately left open, with the reason and the place
they belong. Each was found while implementing something else; none is a bug
in what shipped — except the two Windows entries under *Platform coverage*,
which are real ones, and are recorded here because there is nowhere better.

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

**The Windows app needs `GSK_RENDERER=cairo` to start.**
Built and run on Windows 11 for the first time on 2026-08-30 — MSYS2 MINGW64,
GTK4 4.22.4, `x86_64-pc-windows-gnu`. The workspace compiles clean, and under
cairo the app works: both panes list, the drive bar offers the real drives,
the status line and the command line are live. With the renderer GTK picks by
itself it dies about six seconds in with `0xC0000005`, and dies quietly —
empty stderr, no window ever presented, five runs out of five.

The obvious explanation does not survive the evidence: an explicit
`GSK_RENDERER=vulkan` runs fine, so this is not a GPU that cannot do Vulkan.
`gl` and `ngl` refuse to realize at all — "OpenGL requires Direct
Composition" — which makes GTK's fallback path, rather than any one renderer,
the thing to suspect. The machine it was found on has an AMD Radeon 880M with
NVIDIA Optimus Vulkan layers loaded, which is not the simple case.
*Home:* needs a debugger on Windows, and a second Windows machine to tell a
driver problem from a code one.
*From:* the first Windows run, 2026-08-30 ([ui-shell.md](ui-shell.md),
[windows.md](windows.md)).

**The command line does not run anything on Windows.**
`command.rs` reads `$SHELL` and falls back to `/bin/sh` — a constant whose own
comment says "present on every Unix by definition", which is true and is
exactly the assumption Windows breaks. There is no `$SHELL` there and no
`/bin/sh`, so every command fails with *"the system cannot find the path"*
before it starts. Nine of the fifteen `fc-core` failures in the first Windows
test run were this one cause.

The fix is a platform branch to `%COMSPEC%` (`cmd.exe`) with `/C` in place of
`-c`, which belongs in `vfs::platform` beside every other such difference. It
is not merely swapping the name: the section it would break is
[command-line.md](command-line.md)'s "through a shell, on purpose" — pipes,
globs and `~` are the reason the feature exists, and `cmd.exe` does not do
`~` or globbing. Whether the answer is PowerShell instead, or the MSYS2 shell
when one is present, is a product decision rather than a porting detail.
*Home:* with the Windows startup crash above — both want one session on
Windows rather than two.
*From:* the first Windows test run, 2026-08-30.

**macOS is wanted, to the standard a Mac user would accept.**
Decided 2026-08-29; groundwork 2026-08-30
([plan](plans/archive/2026-08-30-macos-groundwork.md)), packaging 2026-08-31:
the gate cross-checks `aarch64-apple-darwin`, `platform.rs` carries the macOS
branches (`getfsstat` mounts, `~/Library/Application Support`, `UF_HIDDEN`),
`cmd` is an expressible modifier with the Cmd layer shipped dormant as data —
and since CI run #24 the program *runs* there: every commit builds a signed
relocatable `.app` on an arm64 runner and starts it for twenty seconds under
GTK's default renderer ([packaging.md](packaging.md)). What no one has done
yet is *use* it: twenty smoke-tested seconds prove startup, mounts read and
a window presented, not that a file manager behaves.

What genuinely remains, and each needs a person at a Mac:

| | What it is | Why it waits |
|---|---|---|
| the first session | review `platform.rs`'s macOS lists, the `getfsstat` reader against real external disks, the `remove_file` error branch — the groundwork plan's § 5 list | the smoke test executed this code without killing the app; only a person confirms the *answers* are right |
| the keymap's feel | whether Cmd bindings should *move* rather than add, and what the text-field shortcuts mean under Cmd | taste, unjudgeable from here |
| notarization | ad-hoc signing ships today, with the right-click sentence in the bundle's README; the double-click costs an Apple Developer account and `notarytool` plumbing | not worth 99 USD/yr before real Mac users appear |
| **the end-to-end driver** | the suite drives a real binary through `Xvfb` and `xdotool`, both X11-only; macOS wants the Accessibility API against a real GUI session | the part that decides whether a macOS release is honest, and the least certain effort here |

**The open question is still GTK4 itself**, and it is a product question:
non-native chrome, no menu bar, the least maintained of GTK's backends. If
that does not clear the bar, this entry becomes a different one — a second
front end over the same `fc-core`, which is the one thing the crate split
makes possible ([archives.md](archives.md) keeps the score on what the split
is worth).

*Home:* groundwork and packaging are in; the rest starts the day a person
sits at a Mac.
*From:* the phase 7 discussion, against the v1 scope's Linux-and-Windows
line; groundwork per the 2026-08-30 plan, the package per the 2026-08-31
plan.

## Testing

**Six engine tests assert Linux rather than the engine**, and one of the six
has since gone: free space is answered on Windows as of 2026-09-01
([windows.md](windows.md)), so `a_real_filesystem_reports_a_total_and_something_free`
should pass there now. Should — nothing here runs it, which is the point the
row below its neighbours is really making.
The first Windows run of `cargo test -p fc-core --no-fail-fast` (2026-08-30)
passes 331 of 346. Nine failures are the `/bin/sh` gap above. The others
are the suite's own assumptions:

| Test | What it assumes |
|---|---|
| `a_file_under_a_hidden_directory_is_hidden…` | a leading dot hides a directory — [vfs.md](vfs.md) says it does not on Windows |
| `removing_a_directory_as_a_file_reports_is_a_directory` | `IsADirectory`; Windows gives `PermissionDenied` |
| `trashing_something_that_is_gone_reports_not_found` | `NotFound`; `trash` hands back a raw Win32 code |
| `reading_a_subdirectory_is_not_a_change_to_the_directory` | inotify staying quiet on a read; `ReadDirectoryChangesW` fires |
| `a_column_width_reaches_the_file_as_a_number…` | that a `VfsPath` string is a real path — it hands `/C:/…` to `std::fs` |

Only the last is a plain test bug. The rest are real differences, and `cfg`
guards would hide them rather than settle them — the two error-mapping rows
are [vfs.md](vfs.md)'s "Windows keeps the raw code" decision meeting tests
that want a named error, which is worth deciding once rather than skipped
five times.

**The macOS groundwork settled three of the six** (2026-08-30), because macOS
shares two of the differences and sharpened one: `remove_file` on a directory
now answers `IsADirectory` on every platform — POSIX lets `unlink(2)` say
`EPERM` and macOS does, so the engine settles the spelling on the error path
rather than each test skipping it; the trash-of-a-missing-path test asserts
`NotFound` on Linux and "an error" elsewhere, which is vfs.md's own per-target
decision written into the assertion; and the trash suite is `target_os =
"linux"` rather than `unix`, because what it asserts is the *freedesktop
layout* — on a Mac it would trash its fixtures into the account's real bin.
The hidden-directory and watcher rows still wait for Windows; the
free-space one should have stopped failing on 2026-09-01, unverified.
*Home:* whenever Windows gets a session of its own.
*From:* the first Windows test run, 2026-08-30.

**Which bindings the suite presses is now counted, not claimed.**
Kept as a record of a gap that closed, because the entry outlived it in both
directions and the replacement overclaimed in turn. `Backspace` was listed
here as uncoverable — "going up a directory has no filesystem effect to assert
on" — and three tests press it today, reaching the effect through what happens
*next* in the parent directory. `Ctrl+Q` was listed as uncoverable because
quitting would end the app under test; it is how the harness closes every one
of the 160.

**"Every binding is now exercised" replaced that on 2026-08-30 and was also
wrong**, by eight. The real figure — 59 of 67 bindings pressed, the other
eight excused with a reason each — was arrived at by counting rather than
reading, and it is now `UI_UNPRESSED` plus
`every_binding_is_pressed_end_to_end_or_says_why_not`
([keymap.md](keymap.md)) rather than a sentence here.

Three sentences about this in three days, each written in good faith, none of
them right. That is the argument for a test over a paragraph, more than any
of the drift found by reading was.

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
small constructors. Sharing them needs a `test-support` feature on `fc-core`,
and today the fixtures are shaped to each test's needs — the cure costs more
than the disease.
*Home:* revisit when design phase 2's operation tests need the same shapes.
*From:* the walking-skeleton refactoring audit.

**KDE's cut marker is not written.**
`Ctrl+X` is recorded in the first line of the `x-special/gnome-copied-files`
payload, which Nautilus, Nemo, Thunar and Caja read
([clipboard.md](clipboard.md)). KDE's file managers use a marker of their own
(`application/x-kde-cutselection`), and this writes neither it nor reads it —
so a cut made here and pasted in Dolphin arrives as a copy, which is the safe
direction of being wrong. Reading and writing one more small format is the
whole of the work; nobody has asked yet.
*Home:* beside the two formats in `fc-core::clipboard`, which is where the
encoding lives and is tested.
*From:* [clipboard.md](clipboard.md).

## A window listing the running jobs

`Background` closes a progress window and leaves its job running, which is
what the field report asked for. What it does not yet have is the way back:
once backgrounded, a job cannot be watched again or cancelled, because
nothing lists it.

The engine is ready for that list — every job's progress, cancel token and
report already hang off its own `JobHandle`, and the shell already holds one
per running job. What is missing is a non-modal window over them.

The queue behind it still runs jobs **one at a time**, which is a documented
property rather than an oversight: two copies writing into one directory at
once is a reliability question (docs/reliability.md), not a convenience one,
and it deserves deciding on its own rather than as a side effect of adding a
window.
