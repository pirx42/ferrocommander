# Compare by content — `Ctrl+Shift+C`

← Parent: [CLAUDE.md](CLAUDE.md)

Two files, one verdict: their lines side by side with the differences
marked, or — when the settings name a real diff tool — that tool, handed
both paths. The internal view is deliberately simple, and the external hook
is what makes simple acceptable: anyone who outgrows it writes one config
line ([the plan](plans/archive/2026-08-31-compare-by-content.md) is the record of
that decision and the three beside it).

## Which two files

Total Commander's cascade, in priority order:

1. **Exactly two marked files in the active pane** are the pair. Two,
   exactly: one marked file is not a pair, and three is not an answer to
   "which two". Marked *directories* do not count toward the two — a
   directory has no content to compare, and one marked earlier for a copy
   should not spoil the gesture.
2. Otherwise, **the cursor's file against its namesake** — the file of the
   same name in the other pane, when one exists. The everyday case: two
   versions of a tree, one file in question.
3. Otherwise, **cursor against cursor**, one file from each pane.

No pair at all — the cursor on a directory and nothing marked — is told to
the user in one line rather than silently ignored: unlike `F3` on a
directory, a compare that does nothing looks broken, because the gesture
named two things.

The cascade is a pure function (`compare_pair` in `actions.rs`), and each
arm is a unit test.

## The internal view

A window titled with both names, holding the two files' lines side by side
in one scrolled pair — one scrollbar by construction, so the sides cannot
drift. Rows are tinted by what happened to them: reddish for a line only
the left file has, greenish for only-right, amber for a changed pair — and
inside a changed pair, the character runs that actually differ carry a
stronger amber. The filler line opposite a one-sided row wears that row's
color, so a hole reads as part of the change it belongs to.

Each side carries its **own file's line numbers** in a gutter beside it, so
the two columns disagree exactly where the files do: a filler row opposite a
line only the other side has carries no number, because there is no such line
in that file to number. The gutter is a text view rather than a label, so its
lines are the same height as the text beside them by construction rather than
by matching two fonts.

| Key | |
|---|---|
| `n` / `p` | next / previous block of differences |
| `PgUp` / `PgDn`, `Home` / `End` | move the shared viewport |
| `Esc` | close |

The page keys had to be bound explicitly (2026-09-01): unbound, they reached
the focused `TextView`, which answers a page key by moving its *own* cursor
rather than scrolling the one `ScrolledWindow` both sides share — so the view
did not move and the key looked dead.

**The separator between the sides is one pixel, and lives inside the right
half.** The box holding the two sides is homogeneous, which is what keeps
them equal — and homogeneous means every child gets the same width, so a
separator added as a third child was given a third of the window.

The rows come from `fc-core::compare` — reading through the VFS, so a file
inside an archive compares like any other — and the diff runs on a worker
thread, because the row path reads both files whole and the main loop does
not wait on that ([performance.md](performance.md) § *What a compare by
content costs* has the measured numbers).

## Binary and huge files get a verdict, not a view

A NUL byte in either file's first 8 KiB, or either file over the engine's
64 MiB ceiling, and the answer is one line — *identical*, or *the first
difference is at byte N* — from a streaming byte comparison that never
holds either file whole. When one file is a prefix of the other, the first
difference is the shorter one's length: the first position where one file
has a byte and the other has ended.

The ceiling is where the viewer's never-read-the-file rule
([viewer.md](viewer.md)) meets a line diff's need to hold both files in
memory. Text read for the row view is decoded lossily on purpose: the NUL
sniff has already routed real binaries away, so what remains is text in
some encoding, and a replacement character beats refusing to diff.

## The external tool: `compare_tool` in the settings

```toml
compare_tool = "meld %1 %2"
```

A command line with `%1` and `%2` standing for the two files, each replaced
by a quoted path and run the way a typed command is
([config.md](config.md) § `compare_tool`). **A non-empty setting is the
compare command** — the internal view is what an empty one means. One key,
one meaning, the settings line decides; there is no second binding to
learn.

The substitution is single-pass on purpose: sequential `replace` calls read
their own output, so a path *containing* the text `%2` would have the other
file's path spliced into its middle. Anything else after `%` passes through
as written — the template is the user's own line. A placeholder used twice
is filled twice; one left out is left out.

The tool takes operating-system paths, so a pair with a file inside an
archive is refused with the reason — `F4`'s rule, one key over
([archives.md](archives.md)). The internal view has no such limit.

## What the tests pin

The engine's two conservation invariants ([reliability.md](reliability.md)
lists the suite): every side's rows reconstruct that side's file exactly,
and a changed pair's texts with their differing spans removed are equal —
each probed by breaking the code on purpose. The spans count **chars** end
to end — the diff finds them in chars and the text buffer paints them in
chars, so bytes never enter to need converting back out — and a multi-byte
fixture pins that a count secretly done in bytes would select the wrong
characters and fail the equality. End to end, the suite opens the window on a real pair, walks
the cascade's marked arm, runs a recording script as the external tool and
reads both paths out of its log.
