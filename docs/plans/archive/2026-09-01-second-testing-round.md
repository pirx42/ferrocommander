# Testing round two — three bugs, one answer, and the row

Status: **Done**, 2026-09-01 — all seven items, each behind a green gate.
Outcome in § 7.

Seven items from the second round of using the program. Three are the same
subject seen from three angles — a viewport that moves when nothing asked it
to — one is a number that changes every time it is asked for, one is not a
bug at all, and three are what a pane's rows should look like, settled by a
screenshot of Total Commander itself.

## 1. What each one turned out to be

| Reported | Root cause | Where |
|---|---|---|
| The pane scrolls after a copy, and after a delete in the *other* pane | `reload_after_job` ends with `refresh` and restores **nothing** — so both panes land on their cursor after every job | `pane.rs` |
| Same after `Ctrl+R` or a watcher nudge | `reread` restores the offset *synchronously* after `refresh`, against an adjustment the rebuild has just collapsed to zero | `pane.rs` |
| Entering a folder and coming back puts the cursor at the top | not yet known — the arrival path was fixed on 2026-09-01 and this says it is still wrong, so it is either an incomplete fix or a third cause | reproduce first |
| Folder sizes differ every time the key is pressed | a **cancelled** walk still sends its partial count, and `set_measured` overwrites unconditionally — so the next keypress writes a smaller number over a good one | `sizes.rs`, `listing/mod.rs` |
| What does `<value>+` mean? | the count is a **lower bound**: a subdirectory could not be read, or the scan was stopped. Already the documented meaning of `SIZE_PARTIAL_MARKER` — and the answer to the question above, because a cancelled scan is exactly what puts it there | — |
| `AppData` is missing from `C:/Users/pirx` | **not a bug.** Windows marks it hidden and the pane filters hidden entries; `Ctrl+H` reveals it, which the owner confirmed | — |
| Rows too tall, no icons, folder names unbracketed | not built yet | `row.rs`, `pane.rs` |
| The running-job bar changes the window's layout | the button is `set_visible(false)` when idle, so the bottom row has two heights | `indicator.rs` |

## 2. What the screenshot settles

Total Commander on Windows, the owner's own machine:

- **Rows are 17 px apart** — measured off the screenshot, forty-two rows from
  `[..]` at y=93 to the last at y=790. Ours are visibly taller.
- **A 16 px icon leads the Name column**, before the text: a folder for
  directories, and the file type's own icon otherwise.
- **Directory names are bracketed**, `[.git]`, `[Source]`, `[VS2019Build]` —
  **and so is the parent row**, `[..]`.
- **The Size column still says `<DIR>`** for an uncounted directory, so the
  brackets are as well as that marker, not instead of it.
- Marked rows are red, which this program already does.

The screenshot and the owner's answer to the bracket question disagree about
one row: the answer said the parent stays `..`, the screenshot shows `[..]`.
The screenshot wins here because it is what the item pointed at, and it is one
line to reverse.

## 3. Decisions

1. **The icon comes from the desktop theme, by file type.** GIO guesses the
   content type from the name and GTK's icon theme supplies the icon — one
   code path for all three platforms, cached per extension. The cost is
   honest: on Windows and macOS these are the bundled Adwaita icons, not the
   ones Explorer and Finder draw. The alternative — `SHGetFileInfo` and
   NSWorkspace — is two more platform branches that no gate here can test,
   on top of the GIO path, for icons that differ only in style.
2. **Bracketing is display-only.** `Row` gains the brackets; `full_name`
   stays the name the filesystem uses, because rename, type-ahead, the marks
   and every job read that field. A bracket that reached a `VfsPath` would
   be a file operation on a name that does not exist.
3. **`AppData` is closed as working as designed**, and the rule gets written
   down where a reader would look. It cost a bug report, which is the
   evidence that it was not written down anywhere findable.

## 4. Phases

**Phase 0 — coverage pre-check** (skill 43).

- *The viewport.* Nothing automated, and nothing can be: a scroll offset is
  not a window title, a file or a key press ([ui-shell.md](../../ui-shell.md)).
  `scripts/check-scroll-memory.sh` is the check, and it covers the memory and
  the marking case — not the two job paths, and not the navigate-and-return
  case this round reports.
- *Folder sizes.* `fc-core/tests/sizes.rs` covers the walk and the cancel;
  what it does not cover is what a *cancelled* walk delivers, which is the
  bug. That is a characterisation test owed before the fix.
- *The row.* `row.rs` is pure and unit-tested — the one part of the shell
  that is. Brackets and the icon name belong there for that reason; the
  icon *widget* does not, and the end-to-end suite cannot see it.
- *The indicator.* Nothing. Its click could not be tested last round either,
  and the layout question is a pixel question on top of that.

**Phase 1 — the viewport moves only when asked.** The two known causes are
one line each: `reload_after_job` never restores, and `reread` restores into
a collapsed adjustment. Both route through `restore_offset`, the deferred
helper added on 2026-09-01 — one place that knows *when* an offset may be put
back, rather than three that each get it wrong differently.

The third case is reproduced **before** it is fixed. Last round's `Space` bug
was diagnosed wrong three times from reading the code, and what settled it was
instrumenting the adjustment; this phase starts there rather than ending
there. `check-scroll-memory.sh` grows both job cases, and whatever the third
turns out to be.

**Phase 2 — a folder's size stops changing.** Two fixes, because they fail
differently: a cancelled walk **sends nothing** (the partial count is not an
answer anybody asked for any more), and `set_measured` **never replaces a
complete count with a partial one** (a late answer from a walk that was
cancelled a keypress ago must not win). Tests: a cancelled walk delivers no
answer, and the same tree measures the same twice — a conservation invariant
(skill 52) rather than a value assert.

**Phase 3 — the row: height, icon, brackets.** The two pure halves first,
in `row.rs`, where they can be tested: the bracketing, and which icon name a
row asks for. Then the cell, which binds a `gtk::Image` before the label in
the name column — the one column that already has a `Stack` for the rename
editor, so the layout there grows a box rather than a widget.

The icon lookup is cached by extension. A listing of fifty thousand rows must
not become fifty thousand content-type guesses, and only the rows on screen
bind — but the cache is what keeps scrolling cheap
([performance.md](../../performance.md)), and the phase carries a measurement
rather than a claim.

Row height is CSS: the padding on the cells and the row, taken down until a
row is as close to the screenshot's 17 px as the font allows. Measured on
screen rather than asserted, because that is the only place it exists.

**Phase 4 — the indicator stops moving the layout.** The bar already draws
its text inside itself; what changes the row's height is the button appearing
and disappearing. It stays in the layout always and goes transparent when
idle, so the bottom row has one height for the life of the window.

**Phase 5 — docs.** [listing.md](../../listing.md) for the brackets and the
hidden-file rule that `AppData` ran into, [ui-shell.md](../../ui-shell.md) for
the icon and the row metrics, [performance.md](../../performance.md) for the
icon cache's number, [ops.md](../../ops.md) for what a `+` means beside the
count it belongs to.

**Phase 6 — refactoring audit** (skill 49) and the end-of-plan ritual.

## 5. Effort

Corrected per skill 45; the last plans held at ~0.10.

| Phase | Raw | Corrected |
|---|---|---|
| 0 pre-check | 0.25 d | 0.25 h |
| 1 the viewport | 1.5 d | 1.5 h |
| 2 folder sizes | 1 d | 1 h |
| 3 the row | 2.5 d | 2.5 h |
| 4 the indicator | 0.75 d | 0.75 h |
| 5 docs | 0.5 d | 0.5 h |
| 6 audit | 0.5 d | 0.5 h |

About seven hours, plus a full-gate run (~15 min) per phase commit.

## 6. What would make this wrong

- **The third viewport case is unknown**, and the plan says so rather than
  assuming it is the same as the other two. If it turns out to be the arrival
  path again, the fix from last round is incomplete and this phase is where
  that is found — by instrumenting, not by reading.
- **Nothing here can see a scroll offset, a row's height or an icon.** Three
  of the seven items are pixels, and the suite reads window titles, the log,
  the settings file and the filesystem. What can be tested is the pure half —
  which name a row shows, which icon it asks for — and the rest is a
  screenshot and a person. That is the same split as the scroll memory, and
  it is stated here so a green gate is not read as more than it is.
- **The icon cache is a guess until measured.** Every row asking GTK's icon
  theme for a name is the shape of thing that makes scrolling a large
  directory feel heavy, and this plan asserts a cache fixes it without having
  measured either. Phase 3 measures before and after.
- **17 px is a Windows screenshot's number.** GTK's default font and theme
  are not Total Commander's, and a row forced to a pixel height that the font
  does not fit in clips text. The target is "as compact as the font allows",
  with the screenshot as the direction rather than the specification.
- **The bracket is display-only, and five things read a row's name.** Rename,
  type-ahead, the marks, the pack prefill and every job. The plan says
  `full_name` protects them; what would make it wrong is a sixth reader that
  takes the displayed name instead, which is exactly the kind of thing that
  is invisible until somebody renames a folder to `[Documents]`.

## 7. Outcome

Seven items, five commits, six green gates. What the plan got right, and the
two things it did not.

**The plan's own § 6 said the third viewport case was unknown and would be
reproduced rather than assumed. That was the right call, and it paid.** The
two known causes — `reload_after_job` restoring nothing, `reread` restoring
into a collapsed adjustment — were fixed first, and *neither fix worked* for
the third case. Instrumenting every painted frame's offset is what showed
why: a watcher nudge still moved the view from 630 px to 1170, the cursor row
at the top, which is exactly the wording of the report.

Two more fixes were written and measured before the one in the commit:

- restoring the offset at the deferred priority after `refresh` — still 1170,
  because `scroll_to` after a rebuild does not scroll minimally; with no
  anchor it puts the row at the top, and GTK applies it during the frame's
  own layout, after every idle a caller could hook;
- also suppressing that `scroll_to` — the view then went to **0**, because
  the allocation reconfigures the adjustment from the anchor either way.

So the cause was never *when* the offset was put back. It was that `refresh`
emptied the store, and `remove_all` takes the list's scroll anchor with it.
`sync_rows` keeps the objects and rewrites their contents; nothing is
replaced, so nothing is anchored to a row that has gone. All three cases then
hold at 630, measured the same way.

That is the second round running in which the first diagnosis was wrong and a
measurement settled it. The pattern is worth naming: **reading GTK code
predicts what a widget is asked to do, not what it does at allocation time.**

**The folder-size report was one bug and its symptom**, which the plan had
already worked out: `+` means a lower bound, and what was putting it there
was the previous keypress cancelling a walk whose partial count was then
delivered and written over a good one. Two guards, because they fail
separately. The first test written for the first guard proved nothing — it
cancelled the token *before* `spawn`, which the pre-existing check catches —
and was replaced with one that cancels from inside the first `read_dir`.

**`AppData` was not a bug**, and the rule is written down now.

**The three visual items went as planned**, and the one thing worth
recording is that the plan asserted an icon cache would be needed without
having measured. It is 5× cheaper and neither figure was ever a problem: a
whole screenful is 87 µs against a 16 ms frame. `docs/performance.md` says
that rather than implying the cache rescued something.

**Three existing tests changed.** Two row tests asserted the unbracketed
display name; the third was the one the gate caught rather than I did —
`a_dragged_column_width_reaches_the_settings_file` aims at the header band by
pixel, and a shorter header moved it. It is the single place in the whole
suite that knows a pixel of the pane's layout, which is now said beside the
constant so the next row-height change expects it.

The audit found one thing, and it is a *non*-finding worth writing down:
`refresh_marks` and `sync_rows` both answer "which rows say something
different now", and they stay separate on purpose. `sync_rows` builds each
row to compare it, which is right after a re-read where the rows have to be
built anyway; a mark changes two flags, and asking the expensive question
about it would format fifty thousand dates on every `Space`. Two functions
that look like duplication and are not now say so.
