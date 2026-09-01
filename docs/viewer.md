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
promise not every backend can keep. That turned out to be exactly right: an
entry inside a compressed [archive](archives.md) has no cheap seek, and its
`read_at` decodes from the entry's start — honest and slow, rather than a
`Seek` that would have had to be faked.

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

## Quick view — `Ctrl+Q`, the same engine in the other pane

`Ctrl+Q` turns the pane *without* the keyboard into a window onto whatever
the cursor is on, following it as it moves: arrow through a directory and
watch the files go by. The content is the same `View::open` plus one
`render` the `F3` window makes — same engine, second surface, which is why
this is a section here rather than a document of its own.

It is **strictly a preview**. The keyboard never enters it, `Tab` keeps its
plain meaning, and the preview simply follows the keyboard to whichever pane
it is not in — so there is no second focus state and no key whose meaning
depends on a mode. `F3` is one key away when somebody wants to read rather
than glance, and it already has the paging, the hex mode and the encodings.

The mode is **transient**: a file manager that starts with one pane showing
the head of a text file has to be explained, and the key is cheap to press
again.

The listing underneath is never torn down. The pane is a `gtk::Stack` with
two pages, so leaving quick view restores the pane — cursor, marks, scroll
position and directory watch — rather than rebuilding it.

What the cursor is on decides what is shown, and that decision
(`preview_for` in `actions.rs`) is a pure function with a unit test per case:

| Under the cursor | The other pane shows |
|---|---|
| A file | its head, as `F3` would |
| A directory | its name, then what it holds once counted |
| `..` | a line saying so — the directory being left is not a thing to count |
| A file that cannot be opened | a line saying so |
| Nothing (an empty listing) | a line saying so |

A directory is the case that costs something: counting one is the recursive
walk `Alt+Shift+Enter` exists as a deliberate key for. So **the summary is
asked for, never waited for** — the walk goes to `fc_core::sizes` on a worker,
the name appears immediately, the figures when they land, and each cursor move
cancels the walk the last one started. That is cancellation rather than a
delay, and the measurement is why: starting a walk costs 0.058 ms, so a held
arrow key is not slowed by starting them — only by letting them pile up
([performance.md](performance.md) § *What one step of the quick view costs*).

A cancelled walk can still deliver, because the token is checked between
folders rather than inside one. So an answer is matched against the folder
the preview is showing *now* before it is drawn, which is what keeps one
folder's size from appearing under another folder's name.

**What no test can assert is the text on the screen.** The end-to-end suite
reads window titles, the log, the settings file and the filesystem — never a
pane's contents ([ui-shell.md](ui-shell.md)). So the engine's own tests carry
what is shown, the unit tests carry which of the five cases applies, and the
end-to-end tests carry what is left: that `Ctrl+Q` stopped quitting, that a
previewed pane has no rows to click and gets them back, and that the preview
follows the keyboard.

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
laid out — is `fc-core::viewer` and is tested without a window. `dialogs::Viewer`
is the widget, the keys, and the reader the view asks for bytes.
