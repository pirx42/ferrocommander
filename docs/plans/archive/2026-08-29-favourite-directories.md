# Favourite directories — `Ctrl+D`

**Status:** Implemented — all five phases
(`git log --grep "favourite-directories"`)
**Branch:** `claude/next-phase-plan-design-lah4v5`

Total Commander's directory hotlist: one key opens a short list of places you
go often, an arrow and Enter takes you there, and the list is maintained from
inside itself. The pane it moves is the one with the keyboard, so "for the
left and the right pane" is not two features — it is the ordinary rule every
other navigation key here already follows.

## 1. What was decided, and by whom

Three questions were put to the owner before this was written; the answers are
the spine of the design and are recorded here so nobody re-opens them by
accident (skill [30](../../skills/30-document-the-why.md)).

| Question | Answer |
|---|---|
| How does a directory get into the list? | **From the list itself**, Total Commander's way: `Ctrl+D` opens it, and adding and removing happen there. No second key to learn, and the moment you notice a directory is missing is the moment you are looking at the list. |
| What is one entry? | **A path plus a name**, shown in the two-column shape the drive list already uses. |
| Which pane does `Ctrl+D` send? | **The active one.** One key, one binding, the same rule as every other navigation key. |

Four more decisions had no reason to bother the owner with, and are recorded
with their reasoning instead:

- **The order is the user's, not the alphabet's.** A hotlist is a menu: the
  thing you reach for is the one you put at the top. That makes the setting a
  `Vec`, written as TOML's `[[favourites]]` array of tables — not a
  `BTreeMap` like `[drives]` and `[keys]`, which sort themselves and would
  quietly reorder a list somebody arranged. It also removes a problem a map
  would have created: two directories named `src` are two ordinary entries
  rather than a key collision needing a rule.
- **Adding the same directory twice is a no-op**, which is what bounds the
  list. A cap like `COMMAND_HISTORY_LIMIT` would be guarding against a burst
  nobody produces: every entry here is a deliberate keystroke, and the number
  of directories a person cares about is not a number that runs away.
- **A jump `leave_for`s, it does not `go_to`.** A favourite is always a local
  path (see the refusal below), so pressing it while inside an archive has to
  leave the archive stack — exactly what `go_to_drive` does, and for exactly
  the same reason.
- **No `docs/favourites.md`.** The key belongs in
  [keymap.md](../../keymap.md) and the file format in [config.md](../../config.md),
  which is where somebody will look. A third document repeating both is the
  kind of thing skill [29](../../skills/29-one-topic-per-doc.md) is against.

## 2. What this costs the existing code

Checked against the code rather than guessed
(skill [65](../../skills/65-verify-or-ask-never-assume.md)):

| | State |
|---|---|
| `Ctrl+D` | **Free.** Not in `BINDINGS`; the taken Ctrl letters are `a s d̶ e h m q r u z` minus `d`. |
| A pick-one dialog | **Exists.** `dialogs::choose_one` already serves `Alt+F1` and `Ctrl+↓`, keyboard-first, and the end-to-end suite already drives it by title. |
| Sending a pane somewhere | **Exists.** `PaneView::leave_for` + `await_listing_or`, as `go_to_drive` uses them. |
| A settings table the app owns and writes | **Exists.** `[drives]` is one; `render` writes each owned table and leaves everything else alone. |
| A list helper with a de-duplication rule | **Exists.** `config::remember_command` is the same shape, and is the precedent this follows. |
| An action that is bound but unnameable | **Already impossible.** `Action::ALL` and its two tests, from the architecture review, catch a new variant that is missing from `BINDINGS` or `ACTION_NAMES` without anybody remembering to check. |

The one thing that does **not** exist is a chooser that stays open and changes
its own list. `choose_one` closes on activation and hands back one string,
which is right for the drives and wrong for this. So the window is a module of
its own — `dialogs/favourites.rs` — which is the pattern `dialogs/` already
uses for the three windows with state of their own, not a new idea.

Its rows look exactly like the drive list's, so **the row builder comes out of
`choose_one` and both use it** (skill [44](../../skills/44-no-redundancy.md)).
Copying twenty lines of GTK to get the same two dimmed columns is how two
lists start drifting apart.

## 3. The awkward corners, each of which gets a test

A file manager is trusted with the only copy of things
([reliability.md](../../reliability.md)), and a navigation feature's failures are
the quiet ones: a stored path that means nothing, a jump that lands nowhere.

- **Adding while inside an archive is refused.** The pane's path there belongs
  to that `ArchiveFs`'s own `Store`, where the same spelling means a
  completely different file. Storing it would write a favourite that fails —
  or worse, resolves against the real filesystem — on the next run. Refused
  with the reason on screen, the same rule the command line already applies
  inside an archive.
- **A favourite whose directory is gone** leaves the pane where it was with
  the reason next to the path. That is `navigate_to`'s existing behaviour and
  the right one here: unlike a drive, a favourite has no mount to fall back
  to, and inventing one would send the pane somewhere nobody named.
- **A favourite the user hand-wrote survives an unrelated save.** Settings are
  built from what was loaded and then overwritten field by field — the rule
  that has already been broken twice, for `[keys]` and for `editor`. A new
  owned table is exactly the shape that breaks it a third time.
- **A malformed entry does not stop the program starting.** A settings file
  from a newer version, or a hand-edit with a typo, falls back rather than
  failing — as an unknown sort key already does.
- **Adding twice adds once**, and removing the last one leaves a list that
  still opens.

## 4. Phases — one phase, one commit

Docs ride in the commit that changes the behaviour
(skill [28](../../skills/28-docs-in-same-commit.md)), so there is no
documentation phase at the end.

### Phase 0 — coverage pre-check

Skill [43](../../skills/43-coverage-before-implementation.md). Before touching
anything, find what covers the code this will change and **probe it** — break
the effect, re-run, and see whether the suite screams
(skill [59](../../skills/59-mutation-probe-over-coverage-percent.md)). The three
places at stake:

- `config::render` — is "a table the app does not own comes back untouched"
  actually pinned, or only the tables that exist today?
- `choose_one` — does anything fail if the row builder renders one column?
- `go_to_drive`'s `leave_for` — does a test notice if a jump from inside an
  archive keeps the archive stack?

Whatever does not bite gets a characterization test **first**, in its own
commit, pinning today's behaviour before any of it moves.

**Done — and it needed no new test.** Two of the three were already pinned,
and the third cannot be pinned by anything this suite can do:

| Probe | What was broken | Result |
|---|---|---|
| `config::render` keeps what it does not own | started the document from empty instead of from the existing file | **Bit.** `saving_leaves_the_users_own_lines_exactly_as_they_wrote_them` and `saving_still_records_what_the_app_owns` both failed, naming the lost comment. |
| `choose_one` renders two columns | — | **Cannot bite.** The harness sends keys and reads the filesystem; it has no way to read a label. See below. |
| `leave_for` leaves the archive stack | spawned on the current backend and left `Transition::Stay` | **Bit.** `a_drive_button_takes_a_pane_out_of_an_archive` failed. |

The residual risk in the first row is not the one the plan guessed. `render`'s
preserve rule is solid; what a *new owned* table risks is the opposite —
`Shell::current_settings` builds from `..self.saved`, so a field nobody
assigns is carried through unchanged rather than zeroed, and the failure mode
for `favourites` is **"the add never reaches the file"**. That is what phase
3's kill-and-relaunch test is for.

**The two-column row is a deliberate blind spot.** The end-to-end suite can
press keys and look at the filesystem; it cannot read the text in a label
(`crates/fc-app/tests/harness/`), which is the same limit the architecture
review recorded in its § 5. So when the row builder comes out of `choose_one`,
what stays covered is the part with logic in it — which value each row maps to
— by the existing drive tests and the new favourites ones. That a row *shows*
its path is checked by running the program, and by nothing else. Written down
rather than papered over.

### Phase 1 — the setting

`fc-core::config`: a `Favourite { name, path }`, `Settings::favourites` as a
`Vec`, and `remember_favourite` / `forget_favourite` beside
`remember_command`, whose de-duplication rule they follow. Round-trip tests,
the hand-written-file test, the malformed-entry test. `config.md` gains the
`[[favourites]]` section.

Headless, and the whole rule set is testable with `cargo test` — no window
involved in any of it.

### Phase 2 — the key and the jump

`Action::Favourites`, named `favourites`, bound to `Ctrl+D`; the list state on
`Shell`, loaded from the settings; `dialogs/favourites.rs` showing the list and
jumping on Enter, with the row builder shared out of `choose_one`.

End to end: `Ctrl+D` in the left pane goes there and the right pane does not
move, and the same again with the panes swapped — which is the whole of "for
the left and the right pane", and worth both halves because a bug that acts on
pane 0 whatever the focus is passes the first test.

### Phase 3 — maintaining the list from inside it

The trailing `+ Add current directory` row, `Delete` on a favourite to remove
it, and the window staying open while both happen. The three refusals from
§ 3, each with its test. `keymap.md` gains the key and what the list does.

End to end: add, and the row is there; kill the app and relaunch, and it is
still there — the same `App::relaunch` shape
`settings_survive_the_app_being_killed` already uses, because a favourites
list that does not survive a restart is not a favourites list.

### Phase 4 — refactoring audit

Skill [49](../../skills/49-final-phase-refactoring-audit.md): re-read the whole
change as one diff, looking for what the phases introduced and nobody stepped
back to see — the last audit found a formatter left behind in the wrong
module. Plus the cookbook row in `CLAUDE.md`, and `scripts/check-links.py`.

## 5. What is deliberately not in this

- **Separators and submenus.** Total Commander's hotlist has both. They turn
  the setting from a list of entries into a tree and the dialog into a nesting
  one, for a list that will have ten rows in it.
- **Reordering from the dialog.** The order is the file's, and the file is
  editable. If moving a row up becomes something anybody wants, it is a
  keystroke on a list that already exists.
- **A favourite that points inside an archive.** Refused, per § 3 — and it
  would need the archive's whole entry stack stored, not a path.

## 6. Effort

Factor 0.25 per skill [45](../../skills/45-calibrate-effort-estimates.md) — the
factor the architecture review used, and its nine items came in near it.

| Phase | Raw | Corrected |
|---|---|---|
| 0 — coverage pre-check | ~2 h | ~30 min |
| 1 — the setting | ~2 h | ~30 min |
| 2 — the key and the jump | ~4 h | ~1 h |
| 3 — maintaining the list | ~3 h | ~45 min |
| 4 — audit | ~2 h | ~30 min |
| **Total** | | **~3.25 h** |

The end-to-end suite is the slow part rather than the code: it runs one app at
a time and each new test costs about five seconds of it.

## 7. What was actually done

One commit per phase, and every behavioural claim probed — the effect broken
on purpose, the suite re-run, and only a probe that bit counted as coverage.

| Phase | Commit | Deviation |
|---|---|---|
| 0 — coverage | `9481798` | no new test: two of three were pinned, and the third **cannot** be — see § 4 |
| 1 — the setting | `5aaba56` | none |
| 2 — the key and the jump | `8c9d798` | four end-to-end tests rather than two: the archive exit and the missing directory earned their own |
| 3 — the list edits itself | `bd1cb14` | the empty-list note from phase 2 was **removed**, not kept — see below |
| 4 — audit | this one | three findings, below |

### What the probes were worth

Two paid for themselves outright:

- Removing the archive refusal kept `/` — the archive's own root — as a
  favourite. On the next run that resolves against the real filesystem, so
  the check is not tidiness: without it, `Ctrl+D` on a zip quietly bookmarks
  the root of the disk.
- Removing the `remember` call left a list that adds, draws and works
  perfectly until the program is restarted. Nothing but the relaunch test
  sees it.

And one probe was wasted, which is worth recording too: the first attempt
broke the refusal *and* the save in one run, so the refusal test passed for
the wrong reason. One probe at a time, or a probe proves nothing.

### The empty-list note, added in phase 2 and gone in phase 3

Phase 2 gave the empty list a dim "no favourites yet" row. Phase 3 made every
row past the favourites the command row as far as the activation handler is
concerned — so pressing Enter on that note would have *added a favourite*.
It is deleted rather than special-cased: the command row is a better empty
state than a note explaining that there is nothing above it.

### Audit phase

Re-reading the four commits as one diff found three things, all now fixed:

- **Two stale counts.** `dialogs/` held four windows with state of their own
  and now holds five; `crates/fc-app/src/CLAUDE.md` and
  [ui-shell.md](../../ui-shell.md) both said four. The dialog inventory in
  ui-shell.md had no row for `Ctrl+D` either.
- **A rationale written twice.** `favourites.rs` restated `choose_one`'s
  "a modal window, not a popover, because the suite cannot drive a popover"
  argument in full. It now points at it.
- **Borrows held across a callback.** Three places read out of the
  `RefCell` and then called a hook while the guard was alive — including
  `*list.borrow_mut() = (hook)(…)`, where the place expression is evaluated
  first. All safe today, and all exactly the shape
  `crates/fc-app/src/CLAUDE.md` says this crate does not write, which is the
  point: the rule is what keeps it safe tomorrow.
