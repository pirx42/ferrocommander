# Phase 3b — the rest of Total Commander's selection keys, pane commands, and a configurable keymap

**Status:** Implemented
**Design:** [2026-08-28-tc-clone-design.md](2026-08-28-tc-clone-design.md) — inserted
between design phases 3 and 4, because it finishes phase 3's territory
(selection commands and the keymap) rather than starting the viewer.

## 1. Why

Phase 3 shipped the marking keys that were easy to reach from the model that
already existed: `Space`, `Insert`, `Num ±`, `Num *`, `Ctrl+A`. An audit of
Total Commander's own selection keys against ours found five it has and we do
not, one of which — `Shift`+cursor — is how most TC users actually select a
run. The owner's ruling: all of them, plus three pane commands, plus a
keymap that a user can change.

A hard-core TC user's fingers are the specification here. A key that does
almost the right thing is worse than one that is missing, because the missing
one gets noticed.

## 2. Phase 0 — coverage pre-check (skill 43)

What already exists and is reused rather than rewritten:

| Needed | Already there |
|---|---|
| mark / unmark one row | `Listing::set_selected`, `toggle_selected` |
| mark everything visible | `Listing::select_all`, `clear_selection` |
| invert everything visible | `Listing::invert_selection` |
| mark by wildcard | `Listing::select_matching` + `glob::matches` |
| split a name from its extension | `listing::name::split_name` |
| pane ordering / hidden flag survive a reload | `PaneView::adopt`, `state`, `restore` |
| send a pane somewhere | `PaneView::go_to` |
| settings file, atomic write | `tc_core::config` |

What is genuinely new: a files-only inversion, selection by extension, a
selection that can be restored after an operation, cursor moves that mark as
they go, exchanging two panes' contents, and the keymap overlay.

**Gap found by the pre-check:** the app has **no "unmark everything" key at
all**. `Ctrl+A` marks everything and the only way back is `Num *` twice. That
is a hole rather than a decision, and `Ctrl+Num −` closes it.

## 3. Decisions taken (owner, 2026-08-28)

| Question | Ruling |
|---|---|
| `Shift`+cursor semantics | **Faithful TC**: mark the row being *left*, then move. Reversing direction unmarks, because the second pass toggles the same row again. Explorer users will overshoot by one; that is TC's own quirk and it is the point. |
| `Num *` vs `Shift+Num *` | **Faithful TC**: `Num *` inverts **files only**, `Shift+Num *` inverts files **and** folders. Changes what `Num *` does today. |
| `Ctrl+←/→` | **TC-relative**: sends the **active** pane's directory to the other one; the arrow points at the *target*, so pressing it toward the pane you are already standing in does nothing. |
| Keymap configuration | A **`[keys]` table inside `config.toml`**, **overlay** semantics: only overridden bindings appear, everything else keeps its default. |

## 4. Consequence the ruling on `config.toml` forces

`config.toml` is now written **whenever something changes**, not on exit
([config.md](../../config.md)). A serde round-trip through `Settings` reproduces
the *data* and nothing else — so a user who hand-writes a `[keys]` table with
comments would lose them the first time they moved the cursor, about half a
second after opening the app. That is unacceptable for a file the user is now
being invited to edit.

**Therefore the save becomes format-preserving**: `config.toml` is parsed as a
document (`toml_edit`), the tables the app owns (`window`, `panes`,
`active_pane`) are updated in place, and everything else — comments, ordering,
the whole `[keys]` table — is written back untouched. The app never writes
`[keys]` at all; it only reads it.

This is the one place the plan adds a dependency, and it is what makes the
owner's chosen layout safe rather than a trap.

## 5. Sub-phases

Each is one commit with its tests and its doc changes (skills 11, 23, 28).

### A. Selection primitives in `tc-core`

- `Listing::invert_selection_files()` — inverts visible **file** rows only,
  leaving directories and `..` alone. The existing `invert_selection` keeps
  its meaning (everything visible) and becomes what `Shift+Num *` calls.
- `Listing::select_same_extension(selected: bool)` — takes the extension of
  the row under the cursor via `split_name` and marks or unmarks every visible
  **file** with that extension. A file with no extension matches the other
  extension-less files, which is what TC does.
- `Listing::selected_names()` / `set_selected_names(&[String])` — a selection
  snapshot **by name, not by index**. Every operation rebuilds the listing and
  every sort reorders it, so indices are not a thing that can be restored.

Tests: conservation-style rather than value asserts (skill 52) — inverting
files twice is the identity; a files-only inversion never changes the number
of marked directories; `selected_names` → `set_selected_names` round-trips
across a sort and a reload.

### B. The remaining selection keys

New actions and their bindings:

| Key | Action |
|---|---|
| `Shift+↑` / `Shift+↓` | mark the current row, then move one row that way |
| `Shift+Home` / `Shift+End` | mark from the cursor to the first / last row |
| `Shift+PgUp` / `Shift+PgDn` | the same, over one page |
| `Ctrl+Num −` | unmark everything visible |
| `Alt+Num +` / `Alt+Num −` | mark / unmark every file with the cursor row's extension |
| `Num /` | restore the selection from before the last operation |
| `Shift+Num *` | invert including folders |
| `Num *` | **changed**: invert files only |

`Num /` needs the snapshot taken where the selection is *spent*: `PaneView`
records `selected_names()` when a job is submitted, because
`reload_after_job` builds a fresh listing and the marks are gone by the time
anything could ask for them.

`Shift+PgUp/PgDn` are the awkward pair: paging is deliberately the widget's
job ([keymap.md](../../keymap.md)) because the model does not know how tall the
viewport is. The pane measures a page from the column view's allocated height
and its row height and hands the model a row count. **If that measurement
proves unreliable under the UI harness, this is the item that gets reported
rather than fudged** — a page key that marks the wrong range is worse than one
that is not bound yet.

`Num *`'s change of meaning is a deliberate change to existing behaviour and
is called out in its commit message (skill 24). No end-to-end test asserts the
old meaning; the `invert_selection` unit tests stay valid because that function
is unchanged — only the key that reaches it moves.

### C. Pane commands

| Key | Action |
|---|---|
| `Ctrl+→` | the left pane's directory, shown in the right pane |
| `Ctrl+←` | the right pane's directory, shown in the left pane |
| `Ctrl+U` | exchange the two panes |

TC-relative, per the ruling: `Ctrl+→` acts only when the **left** pane is
active, `Ctrl+←` only when the right one is. Pressing either toward the pane
already holding the keyboard does nothing, rather than guessing.

`Ctrl+U` exchanges the panes' **contents**, not their widgets: the widgets are
children of a `Paned` and moving them would be a reparent for no reason.
`PaneView::exchange_with(&mut other)` swaps the `Listing` (which carries the
directory, the cursor and the marks), the `sort` and `show_hidden` fields and
the filter text, then both panes refresh. Swapping the listing whole is what
makes the marks and the cursor come along for free.

### D. The configurable keymap

- **`tc-core`** gains `Settings::keys: BTreeMap<String, String>` — plain
  strings on both sides. It parses no key names and knows no actions, because
  a key name is a `gdk::Key` and `tc-core` stays GTK-free ([crates/CLAUDE.md](../../../crates/CLAUDE.md)).
- **`tc-app`** owns both halves of the translation:
  - a key spec `"ctrl+shift+kp_add"` → `(Key, ModifierType)`, case-insensitive,
    modifiers in any order;
  - an action name `"invert_marks"` → `Action`, through **one** table that a
    test walks to prove every `Action` variant has a name (skill 53) — a new
    action nobody named fails the tests rather than being unbindable.
- **Overlay**: the defaults are the `BINDINGS` table; a `[keys]` entry replaces
  the binding for that key, and an empty value unbinds it. A key nobody
  mentions keeps its default, so a binding added in a later version reaches
  people who already have a config file.
- **Bad entries do not stop the program** ([config.md](../../config.md)): an
  unknown key name or action is reported once and skipped, and the rest of the
  table still applies.
- The format-preserving save from §4.

### E. Docs and refactoring audit

**What the audit found**, and what came of it:

- **`Settings` gained a field the shell defaulted away.** `current_settings`
  built its value with `..Settings::default()`, which zeroed the new `keys`
  map — so for anyone with a `[keys]` table it never equalled what was loaded,
  and the first keystroke of every run wrote their file for no reason. The
  bindings are echoed back as they were loaded now. It costs one redundant
  write and nothing else observable (after that write the two agree again), so
  it carries no test of its own: pinning it would take a contortion that tests
  the contortion.
- **The `Testing` section of [keymap.md](../../keymap.md) had gone stale** —
  it claimed every binding but two was pressed for real, which stopped being
  true three sub-phases ago. It now names what is left out and why, and two
  keys that had no reason to be missing (`Shift+PgUp`, `Alt+Num −`) were
  covered rather than excused.
- **The six "do something to the marks" methods on `PaneView` were left
  alone.** They look like duplication and are not: each is a delegation to a
  differently-named `Listing` call, and only the three that depend on the
  cursor call `adopt_selection` first. A shared helper would either force that
  call on all six or take a flag to say which, and both are worse than the
  three lines they would save.

[keymap.md](../../keymap.md) gets the full table and the TC-faithfulness notes;
[config.md](../../config.md) gets `[keys]` and the format-preserving save;
[listing.md](../../listing.md) gets the selection primitives. Then the audit per
skill 49: the selection commands will have grown to a dozen actions that all
do "something to the marks", and that is exactly where redundancy hides.

## 6. Risks

- **`Shift+PgUp/PgDn` measurement** (§5B) — the only item with genuine
  uncertainty. Reported, not fudged, if it does not come out clean.
- **`Num *` changing meaning** — a user who learned our old behaviour in phase
  3 loses it. Accepted: TC's meaning is the specification, and the phase-3
  behaviour was never the considered choice, just the easy one.
- **`toml_edit` as a second TOML crate** — `toml` stays for reading into
  `Settings`, `toml_edit` does the writing. Two crates for one format is worth
  one line of justification in [config.md](../../config.md), and the alternative
  is destroying the user's own file.

## 7. Effort

Feature plan on a clear architecture with a strong test net; the keymap
overlay is the only genuinely new mechanic. Factor 0.25 applied per skill 45.

| Sub-phase | Corrected |
|---|---|
| A. Selection primitives | ~45 min |
| B. Selection keys | ~1 h |
| C. Pane commands | ~30 min |
| D. Configurable keymap | ~1.5 h |
| E. Docs + audit | ~30 min |
| **Total** | **~4 h** |
