# Phase 4 — the viewer (F3) and the external editor (F4)

**Status:** Implemented
**Design:** [2026-08-28-tc-clone-design.md](2026-08-28-tc-clone-design.md) § 6, phase 4.

## 1. Why

`Enter` on a file still does nothing, and the design doc has said "F3/F4 arrive
in phase 4" since phase 1. A file manager whose only way to look inside a file
is to launch something else is not one a hard-core user keeps.

The design asks for **text with encoding detection plus a hex mode** (F3), and
**F4 launching a configurable external editor** — which phase 3c already built
for `Shift+F4`, so F4 is a binding rather than a feature.

## 2. The requirement that shapes it

`docs/performance.md`: **a 4 GB file must open instantly.** That single line
decides the whole design. A viewer that reads a file to show it is a viewer
that cannot open a disk image, and "read the first megabyte and say so" is a
viewer that lies about what is in the file.

So the viewer never holds the file. It holds an **offset**, reads a window
around it, and moves the offset. Which needs something the VFS does not have.

## 3. Phase 0 — coverage pre-check (skill 43)

| Needed | Already there |
|---|---|
| the size of a file | `VirtualFs::stat` |
| reading bytes | `VirtualFs::open_read` — **sequential only**, no offset |
| a modal window with a key handler | `dialogs::shell` |
| launching a configured program | `command::open_in_editor`, `Settings::editor` |
| a scrollable, monospaced text area | `dialogs::show_output` |

Genuinely new: random access on the VFS, and the viewer itself.

## 4. Sub-phases

### A. `read_at` on the VFS

`fn read_at(&self, path: &VfsPath, offset: u64, len: usize) -> Result<Vec<u8>, VfsError>`.

Random access rather than a seekable reader, because a seekable reader is a
promise phase 6 cannot keep: an entry inside a compressed archive has no cheap
seek, and a trait method that some backends must fake is worse than one they
implement honestly and slowly.

A read past the end returns what is there — short, not an error. That is what
every caller wants at the end of a file, and the alternative is every caller
clamping against a size that may have changed since they asked.

Tests: reading at an offset returns the same bytes as reading the whole file
and slicing; a read past the end comes back short; a read entirely past the end
comes back empty; a directory is an error rather than a panic.

### B. The viewer

A window over one file, holding an offset and a mode.

- **Text** — the window decoded as UTF-8 where it is, and as Latin-1 where it
  is not. Detection is that test and nothing more: a file that decodes as UTF-8
  is UTF-8, because the byte sequences that make it valid do not happen by
  accident. Anything else is shown as Latin-1, which maps every byte to *some*
  character and so never fails. No dependency, and honest about what it does.
- **Hex** — offset, sixteen bytes, and their printable forms.

Paging by byte offset, not by line: a line index over 4 GB is the thing being
avoided. Text mode snaps the window start to just after a newline so a page
never opens mid-line, except at offset zero where there is nothing to snap to.

Keys: `↑`/`↓` a line, `PgUp`/`PgDn` a window, `Home`/`End` the ends, `1` text,
`2` hex, `Esc`/`F3` closes. Total Commander switches modes from a menu and its
numbering is 1/2/3; `N`/`P` for the next and previous file wait for a phase
that has a file list to walk.

### C. F3 and F4

`F3` opens the viewer on the row under the cursor. `F4` hands the same file to
the editor from the settings, which `Shift+F4` already built.

Neither does anything on `..` or on a directory: there is nothing to view, and
TC does not either.

### D. Docs and the audit

**What the audit found**, all of it in the test harness rather than the app:

- **`xdotool search --name` is a regex and an unreliable one.** A title of `..`
  matched every window on the display, so a test asserting a viewer was
  *absent* could never fail — and a window plainly named `notes.txt — 0%` was
  not found by `notes.txt` at all. The harness now lists every window and
  compares names itself, which has no such surprises.
- **Focus was checked by name, and a focused window often has none.** One GTK
  window is several X windows and the focus lands on a child, which carries no
  name — so a perfectly focused viewer read as a failure. Compared by id first
  now, by name second.

Neither was reachable before phase 4, because every window the suite had ever
looked for was a dialog with a plain ASCII title.

A new `docs/viewer.md`, the keymap table, and the audit per skill 49.

## 5. Risks

- **Encoding detection is a heuristic and will be wrong sometimes** — UTF-16
  will read as Latin-1 mojibake. Named in the docs rather than pretended away;
  a real detector is a dependency and a later decision.
- **A file that changes while it is open** — the viewer reads windows on
  demand, so it will show a mixture. Acceptable: it is a viewer, not a lock.

## 6. Effort

Factor 0.25 per skill 45.

| Sub-phase | Corrected |
|---|---|
| A. `read_at` | ~30 min |
| B. The viewer | ~1.5 h |
| C. F3 / F4 | ~30 min |
| D. Docs + audit | ~30 min |
| **Total** | **~3 h** |
