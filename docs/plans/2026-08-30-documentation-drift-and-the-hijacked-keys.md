# Documentation drift — and the keys the command line lost

Status: In Progress

The owner read `keymap.md` and found a row describing a key as doing
something it stopped doing that morning. This is what came of pulling that
thread.

Fourteen findings so far, and they are not one kind of thing. **Three are the
code being wrong**, one of them a regression this week's work introduced and
confirmed by running it. Eight are documents that now contradict the program.
The rest are gaps — places where a document promises something it does not
deliver.

The order matters and is fixed: the regression ships first, alone. A prose
pass is not a reason for `Ctrl+V` to go on being broken, and the documents
that describe that area should describe code that is already right.

## 1. What was decided, and by whom

Owner spec, 2026-08-30:

- **While the command line is active, the keys belong to it.** `Ctrl+X`,
  `Ctrl+C` and `Ctrl+V` do text cut, copy and paste there.
- **One plan, the bug first.**
- **Finish the sweep** — every file in `docs/` and every `CLAUDE.md`, not only
  the ones recent work touched. Drift is oldest where nobody has looked
  recently, which is exactly where the row the owner found had been sitting.

## 2. The regression, which is the reason this is not a documentation plan

`main.rs` decides what to do with a key while a text field has the focus:

```rust
let commanding = typing_a_command(controller, &shell)
    && modifiers.intersects(CONTROL_MASK | ALT_MASK);
if typing(controller) && !commanding { return Proceed; }
```

So **any modified key the keymap claims is taken from the entry.** Until this
week that was harmless, because the keys it claimed were ones no text field
wants. Then `Ctrl+C`, `Ctrl+X` and `Ctrl+V` were bound, and the three
shortcuts every text field on the desktop owns started reaching the panes
instead.

`Ctrl+V` is the one that bites: pasting a path into a command is something
people do constantly, and it now starts a file copy.

**Confirmed by running it**, not by reading: with the cursor on a file, `→`
into the command line, `Ctrl+C`, then `Tab` and `Ctrl+V` — and the file
lands in the other pane. The clipboard got the file, so the keymap won.

Two more keys are in the same class and were **not** named by the owner; they
are here as an inference to accept or reject:

- `Ctrl+A` — select all, claimed as *mark everything*. This is not new: it
  has never worked in the command line.
- `Ctrl+Z` — undo, claimed as *undo the last multi-rename*.

`main.rs`'s own comment asserts the rule that has stopped being true: "`Ctrl+C`
is the entry's own and stays hers".

### What the fix is

A named list of the shortcuts a text field owns, checked before the keymap is
consulted. `command-line.md` already describes the *intended* rule correctly —
"the command line's own shortcuts" are the exception, meaning `Ctrl+Enter` and
`Ctrl+↓`. The code generalised that into "any claimed modified key" and drifted
from its own documentation. The list makes the exception explicit again, and
the next key somebody binds cannot silently take a text field's.

## 3. Documents that contradict the program

| | Where | What it says | What is true |
|---|---|---|---|
| B1 | `keymap.md:58` | "any unbound letter — Starts a command" | it searches the rows; **row 37 of the same table says so** |
| B2 | `command-line.md:18` | "A printable key that no binding claims starts a command … the only way in from the keyboard" | `→` is the way in; letters search |
| B3 | `command-line.md:83` | "`Ctrl+C` is the entry's own — the keymap does not bind it" | the keymap binds it, which is finding A1 |
| B4 | `vfs.md:11` | the `VirtualFs` listing | omits `fn space()` |
| B5 | `tc-core/src/CLAUDE.md` | the module table | omits `clipboard.rs` |
| B6 | `ui-shell.md:11` | the widget tree | omits the drive bar, the command line, the filter bar and the status line; shows `ApplicationWindow → Paned`, but the window's child is a `Box` |
| B7 | `ui-shell.md:56` | progress dialog answers "cancel" | cancel **or** Background |
| B8 | `archives.md:234` | what an archive refuses | omits `Enter`, `Ctrl+C`, `Ctrl+X` and `Ctrl+V` |

B1 and B2 are the same drift in two files: a rule was replaced and its new
statement was *added* without the old one being removed. Both files now argue
with themselves.

## 4. Gaps — promises not kept

- **C1 · No list of action names anywhere.** `config.md`'s `[keys]` section
  says "the defaults are the keymap in `keymap.md`" and writes
  `"f9" = "create_dir"`. There are fifty-nine action names and `keymap.md`
  lists none of them. Somebody rebinding a key has to read `keymap.rs`.
- **C2 · `keymap.md` contradicts itself about what an operation acts on.**
  `F5` is "copy the entry under the cursor", while `Ctrl+C` three rows below
  is "put what is marked". One rule — everything marked, or the cursor row —
  written two ways in one table.
- **C3 · `Esc` is "stop narrowing"**, and it also abandons a running branch
  walk, which takes precedence. The prose says so 130 lines later; the table
  does not.
- **C4 · `F6` is "move it, or rename it in place"** — "in place" is
  `Shift+F6`'s phrase, two rows above.
- **C5 · `config.md`'s column section breaks the file's own shape.** Every
  other setting has a heading naming its table — `` ## `[drives]` — … `` —
  and sits with the others; this one is called "Column widths" and sits after
  the section about why the crate uses `serde`.

## 5. One measurement that was never taken

Type-ahead runs a case-insensitive substring match over **every visible row on
every keystroke**. In a directory of 50 000 entries — which `performance.md`
says is not exotic — that is 50 000 comparisons per letter, on the main loop.

It is very likely fine. That is not the point: this project's rule is that a
speed decision carries a measurement, and this one carries none.
`performance.md` has a table of such measurements and a place for this row.

## 6. Phases — one phase, one commit

**Phase 0 — coverage pre-check.** What pins the text-field rule today?
`the_history_is_reachable_while_a_command_is_being_typed` pins the exception,
`the_quick_filter_still_gets_its_own_letters` pins the plain-letter case.
Nothing pins what a text field *keeps*, which is the gap phase 1 fills.

**Phase 1 — the keys the command line lost.** The named list, the fix, and a
test per key that fails without it. Ships alone.

**Phase 2 — finish the sweep.** Every remaining file in `docs/` and every
`CLAUDE.md`, read against the code. Findings appended to this plan before
anything is edited, so the list is a record rather than a diff.

**Phase 3 — the contradictions.** B1–B8 and whatever phase 2 adds.

**Phase 4 — the gaps.** C1–C5. The action-name list is the substantial one:
fifty-nine names, and it should be **generated from the table in `keymap.rs`
rather than typed**, or it is one more thing to drift (skill 53).

**Phase 5 — the measurement.** Type-ahead over a large directory, into
`performance.md`'s table, with whatever it turns out to be.

**Phase 6 — refactoring audit** (skill 49).

## 7. Effort

Corrected per skill 45. Phase 1 is a small fix over a clear architecture;
phases 2–4 are documentation work, where the reading is the cost and the
factor does not apply the same way — they are estimated directly.

| Phase | Raw | Factor | Corrected |
|---|---|---|---|
| 0 pre-check | 0.25 d | 0.10 | 0.25 h |
| 1 the hijacked keys | 1 d | 0.10 | 1 h |
| 2 the sweep | — | — | 2 h |
| 3 contradictions | — | — | 1 h |
| 4 gaps | 1 d | 0.10 | 1 h |
| 5 measurement | 0.5 d | 0.10 | 0.5 h |
| 6 audit | 0.5 d | 0.10 | 0.5 h |

**About six hours**, and the first of them is the one that matters.

## 8. What is deliberately not in this

- **A gate check that keeps the tables honest.** Offered and not chosen: a
  script comparing `keymap.md`'s table against `keymap.rs`, and the module
  tables against the directories, so this class of drift fails the gate
  instead of waiting for a reader. It is the durable fix and it is a plan of
  its own — noted here so the choice is on the record.
- **Rewriting documents that are merely old.** A sentence written before a
  feature existed is not wrong; only claims the code contradicts are in scope.
