# Field report, round one — twelve findings from build 132

Status: Draft

The first report from somebody *using* the program rather than testing it.
Build `0.1.0-132` on Ubuntu 24.04, installed from the published `.deb`.
Verdict was "overall works great", followed by twelve findings — which is the
useful half.

They are not one kind of thing. Three are bugs with a mechanism already
identified in the code; four are defects nobody has yet reproduced here; five
are features the program does not have, two of which are refusals it makes on
purpose. This plan keeps those apart, because they carry different risk: a
bug with a known cause is an afternoon, and "archives should be writable" is
a change to the central contract of the codebase.

## 1. What was decided, and by whom

Owner spec, 2026-08-29, in answer to four questions:

- **Archives** — extract, modify, repack. `F4`/`F6`/`F8` work inside an
  archive by rewriting it. Not a writable VFS backend.
- **Background operations** — a full job manager window: several jobs at
  once, each with its own progress and cancel.
- **Enter on a file** — always the system handler, never direct execution.
  Enter must not start a program because the cursor landed on one.
- **Column widths** — resizable, persisted, and **shared by both panes**.
  The panes go on lining up with each other, which is why the widths were
  constants in the first place.

## 2. The twelve, sorted by what is actually known

### Verified in the code — mechanism identified

**F1 · `cd` never reaches the history.** `actions.rs`: the `Typed::Shell`
arm calls `remember_command`, the `Typed::ChangeDirectory` arm does not.
So `cd ..` moves the pane and vanishes; `Ctrl+↓` never offers it again.
Confirmed by reading, not by guessing — the two arms sit twelve lines apart.

**F2 · Space is not what the report suspected.** The report wondered whether
`Space` measures a folder instead of marking it, and asked for a check first.
It does not: `Space` is bound to `Action::ToggleMark` and to nothing else,
and marking never calls the size scan. The *observation* is still real and is
almost certainly F3 — a row appearing to change meaning because the whole
list moved under it. Recorded as answered, not as a defect.

**F3 · Column widths are constants.** `COLUMN_WIDTH_NAME` and its four
siblings, fixed, with a comment saying the panes line up because of it.
Nothing reads or writes them from the settings file.

**F4 · Free space is not computed anywhere.** No `statvfs`, no caller. The
status bar has never known it.

**F5 · Archives refuse every write on purpose.** Every mutating call returns
`VfsError::ReadOnly`, `trash` included, and `docs/archives.md` § 234 lists
`F7`/`F8` among the refusals. This is a deliberate design property being
revoked, not a bug being fixed — which is why it is last and alone.

**F6 · Enter on a file does nothing, by design.** `docs/keymap.md`:
"Activating a *file* does nothing" — `F3` views, `F4` edits, an archive is
walked into. Also a deliberate property, and a much smaller one to revoke.

### Reported, not yet reproduced here

These four are UI-level and cannot be confirmed by reading. Each gets a
failing test in phase 0 **before** anything is changed, and any that turns
out not to reproduce is reported as such rather than quietly fixed.

**F7 · Marking the first row moves every row down by one row height.**
Marking the *second* row does not. Unmarking the first moves them back.
A layout effect, not a model one — the mark is a style class, and a style
class that changes a row's height would do exactly this.

**F8 · `Shift+F6` shows the extension twice** — in the edit field (correct,
the whole name is being edited) and still in the Ext column behind it.

**F9 · `Shift+F6` loses the cursor.** After Enter the cursor jumps to the top
row instead of staying on the file just renamed.

**F10 · The two panes' cursors look the same.** The inactive pane's cursor
should be visibly different from the active pane's. The active pane is
currently marked on its path bar only, deliberately — "in a dual-pane manager
the inactive side must stay fully readable" — so this is a change to that
decision, made where it was made.

**F11 · Going back up loses the scroll position.** Scroll down a long list,
enter a subdirectory, come back: the cursor lands on the directory just left
(that part works, and is tested), but the viewport is not where it was.

### A behaviour change, precisely specified

**F12 · `Ctrl+S` should match a subsequence, not a substring.** Today the
filter "matches anywhere in the name and ignores case" — a substring. `pkag`
therefore matches nothing. Wanted: `*` between every typed character, so
`pkag` finds `package` and `packages` but **not** `page`. That last clause is
the whole specification: `page` fails because `k` must appear after `a` and
before `g`, and it has no `k`.

## 3. What this costs the existing code

The quick filter is the one item that changes a *rule* rather than adding to
one, and it has the most tests pinned to the current rule — including
`quick_filter::what_is_shown_plus_what_is_excluded_is_everything`, a
conservation invariant that must go on holding under the new matcher. It
will, because subsequence matching is still a predicate over one name; the
tests that must change are the ones asserting substring semantics, and
changing them needs saying so out loud (skill 24).

Column widths move five constants into the settings file. `config.md`'s rule
is that a setting the app owns is written as it changes — five more of them
is not new machinery.

The job manager is the largest UI addition: the queue already exists in
`tc-core` and already runs jobs one after another, so what is missing is a
window over it, not an engine under it.

Archives are the one item that touches the central boundary. Extract-repack
is confined to `tc-core::archive` and is testable headlessly, but the failure
mode is the one the reliability doc exists for: **a repack that fails must
never destroy the original archive.** That single sentence is the reason F5
is its own phase with its own conservation tests, and the reason it is last.

## 4. The awkward corners, each of which gets a test

- A repack that runs out of disk halfway leaves the original intact.
- A repack of an archive that is open in the other pane.
- `F8` on the last entry of an archive — an archive with nothing in it is
  still an archive, not a corrupt file.
- `F4` on an entry inside an archive whose editor writes nothing back.
- The subsequence filter with an empty pattern (matches everything, as now),
  and with a pattern longer than every name.
- A column dragged to zero width, and one dragged wider than the window.
- Free space on a filesystem that reports zero blocks — a full disk is a
  number, not an absent one.
- Enter on a file the system has no handler for.
- The scroll position of a directory that shrank while you were below it.

## 5. Phases — one phase, one commit

**Phase 0 — reproduce.** Failing UI tests for F7–F11, and the coverage
pre-check over the quick filter, the inline rename, and the pane's cursor
memory. This phase changes no behaviour and its commit says so. Anything
that does not reproduce is reported back rather than fixed.

**Phase 1 — the three small verified ones.** `cd` into the history (F1),
Enter through the system handler (F6), the inactive pane's cursor (F10).

**Phase 2 — the inline rename.** F8 and F9 together: they are the same
widget and the same commit.

**Phase 3 — the list that moves.** F7, once phase 0 says what moves it.

**Phase 4 — the quick filter.** F12, with the tests that change called out.

**Phase 5 — what the pane remembers.** F11, the scroll position beside the
cursor.

**Phase 6 — the status bar.** F4, free space, and the platform call behind
it — with a Windows branch, because `platform` has two.

**Phase 7 — columns.** F3: draggable, shared, persisted.

**Phase 8 — the job manager.** Background operations, several at once.

**Phase 9 — archives that can be written.** F5: extract, modify, repack,
with the conservation tests first.

**Phase 10 — refactoring audit** (skill 49), and the documentation pass:
`keymap.md`, `archives.md`, `config.md`, `listing.md`, `ops.md` all make
claims this plan falsifies.

## 6. Effort

Estimated by phase, then corrected (skill 45). Phases 0–7 are features with
a clear architecture and a test net around them; phase 8 is a UI feature with
iteration; phase 9 carries genuine revert risk and takes no factor.

| Phase | Raw | Factor | Corrected |
|---|---|---|---|
| 0 reproduce | 1 d | 0.10 | 1 h |
| 1 three small | 1 d | 0.10 | 1 h |
| 2 inline rename | 0.5 d | 0.10 | 0.5 h |
| 3 the moving list | 0.5 d | 0.10 | 0.5 h |
| 4 quick filter | 1 d | 0.10 | 1 h |
| 5 scroll position | 0.5 d | 0.10 | 0.5 h |
| 6 free space | 1 d | 0.10 | 1 h |
| 7 columns | 1.5 d | 0.10 | 1.5 h |
| 8 job manager | 3 d | 0.25 | 18 h |
| 9 archives | 3 d | 1.0 | 3 d |
| 10 audit + docs | 1 d | 0.10 | 1 h |

**Phases 0–7: about 7 hours.** Phase 8: about two days. Phase 9 is the one
that could take what it says.

## 7. What is deliberately not in this

- **A writable archive VFS.** Decided against: extract-repack instead.
- **Per-pane column widths.** Decided against: the panes line up.
- **Enter executing a binary.** Decided against, in both available forms.
- **Multi-rename or search reached from the job manager.** The manager shows
  file operations; the other two have their own windows already.
