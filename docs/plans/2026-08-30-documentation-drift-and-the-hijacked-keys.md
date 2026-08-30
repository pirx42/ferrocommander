# Documentation drift — and the keys the command line lost

Status: In Progress

The owner read `keymap.md` and found a row describing a key as doing
something it stopped doing that morning. This is what came of pulling that
thread.

Thirty-six findings, and they are not one kind of thing. **Three are the code
being wrong**, one of them a regression this week's work introduced and
confirmed by running it. Twenty-three are documents that now contradict the
program. The rest are gaps — places where a document promises something it
does not deliver.

Fourteen of those were in hand when the plan was written; the sweep in phase 2
found the other twenty-two, which is the answer to whether the sweep was worth
asking for.

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

## 4a. What the sweep added

Phase 2 read every remaining file in `docs/` and all ten `CLAUDE.md` files
against the code. It found **fifteen more contradictions and seven more
gaps**, which more than doubles the list the owner's one row started.

Two mechanical passes ran alongside the reading, both worth keeping: every
backticked identifier in the documentation checked for existence in the
code, and every ALL-CAPS constant likewise. The first produced ten candidates
of which **four were false positives** — a historical rename
(`choose_place`), two dependencies named as a *proposal*
(`rustix`/`utimensat`), and two GTK API names (`activate_action`,
`gtk_column_view_scroll_to`). That ratio is why they are candidates and not
findings: a grep cannot tell a claim from a reference.

### More documents that contradict the program

| | Where | What it says | What is true |
|---|---|---|---|
| B9 | `keymap.md:479` | `activation_target` — "the directory the cursor row leads into, or `None` for a file" | it is `activation_step`, and it returns a `Step` — `Into`, `Enter` or `Out`. The third variant is how you leave an archive, and the doc has no room for it |
| B10 | `ops.md:278` | the probe table names `a_move_within_one_filesystem_reads_no_bytes` | the test is `a_move_within_one_filesystem_neither_reads_nor_walks` |
| B11 | `ops.md:15` | the Jobs table: `Copy`, `Move`, `Delete`, `CreateDir` | `Job::CreateFile` is missing — `Shift+F4`, which `keymap.md` documents at length |
| B12 | `vfs/CLAUDE.md` | "**Only** `to_std_path`, `mount_points`, `render_attributes` and the two `unix_mode` bridges escape it" | fourteen functions leave `platform`, including `space`, `config_dir`, `trash_error` and `root_entries` |
| B13 | `ui-shell.md:387` | `./target/release/tc-app` | the binary is `ferrocommander` — `packaging.md` explains the `[[bin]]` rename two files away |
| B14 | `ui-shell.md:274` | "Both panes open at the user's home directory. Remembering the last directory is config persistence — phase 3" | phase 3 shipped; the panes open where they were left |
| B15 | `ui-shell.md:277` | "Phase D puts the reason in the path bar" | it is in the path bar (`PATH_BAR_ERROR_SEPARATOR`), and `keymap.md` documents the rendering |
| B16 | `vfs.md:69` | "Phase 6's archives **will** not [have a recycle bin]" | they shipped, and they do not |
| B17 | `keymap.md:415` | "`Ctrl+↓` — in phase 3 it **will** mean something else entirely" | it does: the command history |
| B18 | `future-improvements.md` | the suite "covers every binding except `Ctrl+Q` and `Backspace`" | `Backspace` is pressed in three tests, and `Ctrl+Q` is how the harness closes every one of them. `keymap.md`'s own "not exercised" list repeats the `Backspace` half |
| B19 | `CLAUDE.md`, `ui-shell.md`, `future-improvements.md` | 138, 138 and 117 end-to-end tests | **160**. Three files, three numbers, none of them right |
| B20 | `performance.md:183` | "The 12 ms floor in the table is names only" | there is no 12 ms in the table; the reading floor it names is 67 ms |
| B21 | `future-improvements.md` | macOS: "55 F-key bindings" and "53 `Ctrl` bindings" | the table holds **67 bindings in total** — 18 with an F-key, 24 with Ctrl. The two figures add to more than the whole keymap |
| B22 | `packaging.md` | left unchecked: "whether `gh release delete` and `create` in sequence are reliable" | there is no delete: the workflow does `create \|\| edit`, then `upload --clobber` |
| B23 | `archives.md:224` | what an archive refuses: the command line and `F4` | also `Enter` on an ordinary file, `Ctrl+D`, `Ctrl+C`, `Ctrl+X` and paste — six refusal messages exist, two are documented (this is B8, and it is bigger than B8 said) |

**B14 through B17 are one kind of thing**, and it is the kind that is hardest
to see while writing: a feature described as future work by the document that
was written before it, and left that way by every reader since because the
sentence is grammatical and the section around it is right. Four of them,
across three files, all pointing at code that shipped weeks ago.

**B19 is the one that indicts the method.** `ui-shell.md` says, of this exact
number, "A number in a document is a claim like any other — this one is dated
by its measurement now." It was 138 when that was written and it is 160 now,
and the dating did not help, because nothing re-reads a number. It is the
argument in § 8 in miniature.

Phase 2's own gate run supplies the replacement rather than leaving phase 3 to
do arithmetic: **160 tests in 527.5 s** on this box, 3.30 s each — against the
442 s the document records for 138, which is 3.20 s each. The per-test cost is
the number that held; the total is the one that moved, and it moved because
the suite grew.

### More gaps

- **C6 · `clipboard.md` promises an entry that does not exist.** It sends the
  reader to `future-improvements.md` for KDE's own cut marker. There is no
  such entry there, and no mention of the clipboard at all.
- **C7 · The root cookbook has no clipboard row**, under a table whose own
  line is "this table grows with the code". `docs/CLAUDE.md` has one; the
  cookbook a reader actually starts from does not.
- **C8 · `reliability.md`'s suite table omits `tests/clipboard.rs`**, which
  pins the format that decides whether a paste copies or moves — the one
  place in this program where misreading a byte deletes somebody's files.
- **C9 · `listing/CLAUDE.md` calls `name.rs` "`split_name`, the one place a
  name is split from its extension".** It also holds
  `contains_ignoring_case`, the matcher behind both the quick filter and
  type-ahead, which is the more load-bearing of the two.
- **C10 · `config.md`'s opening line lists five remembered things**; `Settings`
  carries nine. Every one of the missing four has its own section further
  down, so the file answers its own summary four times over.
- **C11 · `keymap.md` states the stand-down rule absolutely** — "while that
  field has the focus the shell does not dispatch anything" — where
  `command-line.md` states it with two exceptions. Same rule, two strengths.
- **C12 · `good-development-practices.md` marks some Chimera references
  "(Chimera only)" and not others.** `docs/architecture.md`,
  `docs/security.md`, `docs/items.md` and `scripts/gallery.mjs` are named
  without the marker and do not exist here. They are backticked paths rather
  than links, so `check-links.py` cannot see them.

**One correction to § 4.** C1 says fifty-nine action names. There are
**fifty-eight** — the count that made C1 worth writing was itself wrong,
which is the argument for generating the list rather than typing it.

### What was checked and found sound

Recorded because a review that only lists faults says nothing about its own
coverage:

- **Every module table.** `tc-app/src/`, `vfs/`, `listing/`, `ops/` and
  `archive/` list exactly the files that are there. Only `tc-core/src/` is
  wrong, and only by `clipboard.rs` (B5).
- **The skills index and the trigger table.** Every file in `docs/skills/` is
  named by `docs/skills/CLAUDE.md` *and* by the root trigger table, both
  ways round, with nothing dangling.
- `listing.md` end to end — every method it names exists, the `SortKey`
  variants match, and "fourteen selection methods" is exactly right.
- `vfs.md`'s error table, path rules, mount rules and platform table.
- `ops.md`'s conflict resolutions, progress variants and constants.
- `multi-rename.md`, `search.md`, `viewer.md`, `watching.md`,
  `reliability.md` — every identifier checked, all present.
- `python3 scripts/check-links.py`: zero broken links.

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
anything is edited, so the list is a record rather than a diff. **Done** —
§ 4a, twenty-two more findings.

**Phase 3 — the contradictions.** B1–B23. **Done.** Big enough to be worth an
order:
the four shipped-as-future sentences (B14–B17) first, since they are one edit
each and one class; then the three stale counts (B19–B21), which want the
measurement phase 5 takes anyway; then the rest, file by file.

Two notes from doing it. **B3 needed no edit**: phase 1 had already rewritten
that section correctly, and a paragraph explaining the old sentence was drafted
before anybody checked whether the sentence was still there. It was removed —
a documentation review can invent drift as easily as it finds it. And **the
first attempt at B15 was wrong in the same direction as the bug it fixed**: the
sentence is in the *Startup* section, where "the pane stays where it was" means
nothing, since there is no previous directory to stay in. Startup falls back to
the nearest readable ancestor; only navigation stays put. Both halves are now
in the file, and the difference between them named.

**Phase 4 — the gaps.** C1–C12. **Done.** The action-name list was the
substantial one, and it is **generated from `ACTION_NAMES` and `BINDINGS`**
rather than typed (skill 53): `action_catalogue` renders the table, the
document carries it between markers, and
`the_action_name_table_in_the_docs_is_the_one_the_code_generates` fails when
they disagree. It is a table of *names and their default keys* rather than
names alone, because the key is the other half of what somebody rebinding
needs.

**The count settled the argument for generating it.** § 4 said fifty-nine
names; § 4a "corrected" that to fifty-eight; the generator says **fifty-nine**,
so the original was right and the correction was the error. Three counts of one
list, two of them by hand, one right. The document now states no number at all
— a count beside a generated list is the part of it that is not generated.

Both new tests were probed. The drift test goes red on a stale row, a deleted
row, and an action added in code with no row; the round trip goes red when
`key_spec` drops its modifiers.

**Phase 5 — the measurement.** **Done**, in a section of its own in
`performance.md` and reproducible with
`cargo run --release -p tc-core --example bench_type_ahead`.

It turned out to be fine, which was the expectation and is not the point:
**0.2 µs for a hit a few rows down, 1.2 ms for a letter that matches nothing**
over 50 000 entries. Three orders of magnitude between the two, because the
search stops at the first match — so the number the § 5 worry was about is the
one that only happens when the search is failing, and at ten keystrokes a
second it is about 1% of the time.

Two things fell out that were not asked for. The quick filter costs **1.1 ms**
on the same needle, so "one answer to *does this name match*" is a claim about
cost as well as correctness. And one non-ASCII name among 50 000 costs the
other 49 999 nothing, because the fast path is chosen per comparison rather
than per directory — which was the design and is now checked.

**Phase 6 — refactoring audit** (skill 49).

## 7. Effort

Corrected per skill 45. Phase 1 is a small fix over a clear architecture;
phases 2–4 are documentation work, where the reading is the cost and the
factor does not apply the same way — they are estimated directly.

| Phase | Raw | Factor | Corrected |
|---|---|---|---|
| 0 pre-check | 0.25 d | 0.10 | 0.25 h |
| 1 the hijacked keys | 1 d | 0.10 | 1 h |
| 2 the sweep | — | — | 2 h — **spent, and it landed on the estimate** |
| 3 contradictions | — | — | 1 h → **2 h**, for 23 rather than 8 |
| 4 gaps | 1 d | 0.10 | 1 h → **1.5 h**, for 12 rather than 5 |
| 5 measurement | 0.5 d | 0.10 | 0.5 h |
| 6 audit | 0.5 d | 0.10 | 0.5 h |

**About seven and a half hours** now, and the first of them is still the one
that matters. The sweep is the only phase whose estimate survived contact,
which is what happens when the cost is reading a known number of lines rather
than fixing an unknown number of things.

## 8. What is deliberately not in this

- **A gate check that keeps the tables honest.** Offered and not chosen: a
  script comparing `keymap.md`'s table against `keymap.rs`, and the module
  tables against the directories, so this class of drift fails the gate
  instead of waiting for a reader. It is the durable fix and it is a plan of
  its own — noted here so the choice is on the record.

  **The sweep strengthened the case rather than settling it.** Two of the
  three checks that would have caught most of this were run by hand in phase 2
  and took seconds: every backticked identifier against the code, and every
  module table against its directory. `future-improvements.md` already carries
  an entry saying the keymap table is kept by hand and offering to generate it
  "if it ever drifts in practice rather than in principle" — B1, C2, C3 and C4
  are that condition, met. What a script cannot check is the larger half of
  this list: a sentence that describes a shipped feature as future work is
  well-formed prose naming nothing.
- **Rewriting documents that are merely old.** A sentence written before a
  feature existed is not wrong; only claims the code contradicts are in scope.
