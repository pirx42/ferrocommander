# Viewer — looking inside a file without holding it

← Parent: [CLAUDE.md](CLAUDE.md)

`F3` opens a window on the file under the cursor; `F4` hands the same file to
the editor from [config.md](config.md). Neither does anything on a directory or
on `..`, which is what Total Commander does too.

## It never reads the file

The requirement decides the design: **a 4 GB file must open instantly**
([performance.md](performance.md)). So the viewer holds an **offset**, asks for
a window around it, and moves the offset. Opening reads the file's *size* and
nothing else.

A viewer that read a file to show it could not open a disk image, and one that
read the first megabyte and stopped would be lying about what is in the file.

That needs random access, which is `VirtualFs::read_at` — added for this, and
random access rather than a seekable reader because a seekable reader is a
promise phase 6 cannot keep: an entry inside a compressed archive has no cheap
seek, and a trait method some backends must fake is worse than one they
implement honestly and slowly.

**Paging is by byte offset, not by line.** A line index over four gigabytes is
the thing being avoided. Moving a line forward is a scan of the window for a
newline; moving one back is a scan of the window *before* the offset, which is
why moving up through very long lines costs the same as moving down. Text mode
starts a window just after a newline so a page never opens mid-line — except at
offset zero, where there is nothing to snap to.

A file with no newline in it at all still moves: "a line down" can only mean a
window there, and a viewer stuck at the top of a minified file forever would be
worse than one that guesses.

## Encoding detection is a validity test

Valid UTF-8 **is** UTF-8: the byte sequences that make it valid do not happen by
accident, so the test *is* the detection. Anything else is read as Latin-1,
which maps every byte to some character and so cannot fail — the honest answer
for a file whose encoding nobody recorded.

**UTF-16 comes out as mojibake.** Said here rather than pretended away: telling
it from Latin-1 needs a real detector, and that is a dependency and a later
decision.

## Keys

| Key | |
|---|---|
| `↑` / `↓` | a line |
| `PgUp` / `PgDn`, `Space` | a window |
| `Home` / `End` | the ends |
| `1` / `2` | text / hex |
| `Esc`, `F3` | close |

Total Commander switches modes from a menu, and its numbering is 1/2/3. `N` and
`P` for the next and previous file wait for a phase that has a file list to
walk.

The window is titled `<name> — <percent>%`, which is both how a person tells two
open viewers apart and the only thing about the viewer the end-to-end suite can
see — so it is what the paging test asserts on.

## F3 works inside an archive; F4 does not

The viewer is handed the pane's own backend, so `F3` on a file inside an
[archive](archives.md) reads it through the archive and shows it — never
unpacking it, because it never reads more than a window at a time.

`F4` hands the file to an editor, and an editor takes an operating-system
path, which a file inside an archive has none of. It says so rather than
handing over what the archive calls the file: that path also exists on the
disk, and the editor would create it there when saved.

## What is where

The arithmetic — where the offset goes, what the bytes say, how a hex dump is
laid out — is `tc-core::viewer` and is tested without a window. `dialogs::Viewer`
is the widget, the keys, and the reader the view asks for bytes.
