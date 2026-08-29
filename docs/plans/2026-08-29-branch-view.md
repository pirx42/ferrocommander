# Branch view — `Ctrl+B`

**Status:** In Progress — approved 2026-08-29; phase 0 done, phases 1–4 to go
**Branch:** `claude/next-phase-plan-design-lah4v5`

Total Commander's branch view: one key flattens the whole tree below the pane
into a single list of files, and everything a pane does — the quick filter,
the marks, the sort, the file operations — goes on meaning exactly what it
meant.

That last clause is the whole difficulty. A branch view that is a *separate
mode* with its own rules is a second file manager inside the first. This plan
is about making it **the same listing with different rows in it**.

## 1. What was decided, and by whom

Four questions were put to the owner; all four took the recommendation.

| Question | Answer |
|---|---|
| What does a row show? | **The relative path** — `nested/inner.txt` — in the name column, Total Commander's way. Sorting by name therefore groups each directory's files together. |
| What happens while a big tree is walked? | **The walk runs on a worker and the pane keeps showing what it has**, exactly as a slow directory read already behaves. Escape cancels, so `Ctrl+B` on `/` is not a trap. |
| What does Enter do on a row? | **Goes to that file's own directory**, cursor on the file, branch view off. Enter on a plain file does nothing today, so nothing is displaced. |
| Do F5/F6/F8 work on marks made here? | **Yes, and the view is rebuilt afterwards.** Copies land flat, because a destination is built from each source's own file name. |

Six more decisions had no reason to bother the owner with:

- **Files only, no directory rows.** Total Commander's rule, and the one that
  makes the view make sense: a flat list is a list of leaves, and a directory
  row in it would be a row whose contents are also rows.
- **A file is hidden if it *or any directory above it* is hidden.** The walk
  descends everything and records the flag; the view filters as it always
  does. That keeps the existing property that `Ctrl+H` costs no filesystem
  access — the alternative, skipping hidden directories during the walk, would
  make the toggle need a second walk.
- **The `..` row stays and means what it always meant**: the parent directory,
  as an ordinary listing. Any navigation leaves the branch view, because what
  arrives is an ordinary listing of somewhere.
- **The path bar says it is a branch view.** The root with a marker after it,
  or the pane is lying about what it is showing.
- **Branch mode does not survive a restart.** It is a way of looking right
  now, not a place. The pane already records its directory, which is the root,
  so a restart gives the plain listing and nothing extra has to be written.
- **The watch stays on the root directory.** Watching a whole tree is a
  different feature with a different cost (one inotify watch per directory);
  a change deeper in the tree is noticed on `Ctrl+R`, which re-walks.

## 2. What this costs the existing code

Checked against the code rather than guessed
(skill [65](../skills/65-verify-or-ask-never-assume.md)):

| | State |
|---|---|
| `Ctrl+B` | **Free.** Not in `BINDINGS`. |
| A cancellable breadth-first walk | **Exists**, in `search::spawn` — a queue rather than recursion, because a deep enough tree turns recursion into a crash over somebody else's directory layout. |
| A listing built from something other than `read_dir` | **Foreseen.** `listing/mod.rs`'s own header says a `Listing` is constructible from a `Vec<Entry>` "and, later, from streaming search results". |
| A pane that keeps showing the old listing while a read runs | **Exists.** `Loading`/`Arrival`, and `PaneView::start`. |
| A path with separators in it | **Works.** `path_at` is `dir.child(&entry.name)`, and `VfsPath::new` normalises component by component, so `child("nested/inner.txt")` is the right path. |

**The one thing that has to give is an invariant.** `Entry::name` is
documented as "Final path component. Never contains a separator." A branch row
breaks that by design. So the plan is not to break it quietly: the doc comment
has to say that a branch row's name is a path relative to the listing's
directory, and **every reader of `name` has to be visited** rather than
assumed harmless. The list, from the code:

| Reader | Verdict |
|---|---|
| `split_name` → the Name/Ext columns | Fine. `nested/inner.txt` splits into `nested/inner` and `txt`. |
| `index_of` / `focus_entry`, `selected_names` / `set_selected_names` | Fine, and better than fine: relative paths are unique where bare names would not be, so marks survive a re-walk by name exactly as they survive a reload today. |
| `select_same_extension` | Fine — it goes through `split_name`. |
| The quick filter and `select_matching` | **A decision, not a bug.** `*` matches across separators, so `n*` marks everything under `nested/`. That is what filtering on what you can see means, and it is what makes "filter, then mark, then F5" work. Stated, and tested. |
| `Destination::of` → `source.file_name()` | Fine, and it is what makes the flat copy fall out for free. |
| `jobs::prefilled_archive` (Alt+F5) | **Wrong today.** It prefills from `split_name(cursor.name).0`, which in a branch view is `nested/inner` — an archive name containing a directory that may not exist. Needs the last component. |
| `PaneView::begin_rename` (Shift+F6) | **Must refuse.** Renaming a row whose name is a path would either fail or move the file; `..` is already refused there, and this joins it. |

That table is the real content of this plan. The feature is a walk and a key;
the work is those seven rows.

## 3. The awkward corners, each of which gets a test

- **Alt+F5 in a branch view** prefills a name, not a path (§ 2).
- **Shift+F6 in a branch view** does nothing, like `..`.
- **A file in a hidden directory** is hidden, and `Ctrl+H` reveals it with no
  second walk.
- **A tree that cannot be fully read** lists what it could. One unreadable
  subdirectory must not cost the view, the same rule the copy engine keeps.
- **Escape during the walk** leaves the pane exactly where it was, showing
  what it was showing.
- **A job over marks from several directories** copies all of them, flat, and
  the view comes back with the same marks restorable by `Num /`.
- **Enter on a row** lands in that file's directory *with the cursor on it* —
  landing in the right directory and leaving somebody to hunt for the row is
  half the job, which is the rule the search results already follow.

## 4. Phases — one phase, one commit

Docs ride in the commit that changes the behaviour
(skill [28](../skills/28-docs-in-same-commit.md)).

### Phase 0 — coverage pre-check

Skill [43](../skills/43-coverage-before-implementation.md), and probes rather
than reading (skill
[59](../skills/59-mutation-probe-over-coverage-percent.md)). What is at stake
is not the new code — it is the seven readers of `name` in § 2, all of which
this change walks past. So:

- Is `Destination::of` building from the source's **file name** actually
  pinned, or would building from the whole path pass?
- Is the rename refusal on `..` pinned by anything but its own comment?
- Do the marks-survive-a-reload tests pin *by name*, or would an index do?
- Does anything fail if `split_name` returns the whole string as the name?

Whatever does not bite gets a characterization test first, in its own commit.

**Done. Four of five were pinned; one was not, and it is one of the two
things phase 3 changes.**

| Probe | What was broken | Result |
|---|---|---|
| `Destination::of` uses the source's file name | built the destination from the whole path | **Bit**, 23 tests in `ops.rs`. |
| `split_name` finds the extension | returned the whole string as the stem | **Bit**, 4: the name unit test, the row column test, and both extension-marking tests. |
| Marks survive a reload **by name** | kept them by position instead | **Bit**, `a_reload_keeps_the_marks_on_the_names_that_survive`. |
| `jobs::prefilled_archive` drops the source's extension | used the whole name | **Did not bite.** `notes.txt` offered `notes.txt.zip` and the whole suite stayed green. |
| `begin_rename` refuses `..` | removed the guard | **Did not bite** — and was not expected to. `pane.rs` already records that a probe could not get an editor to open on `..`, and this re-ran that probe rather than trusting the note. |

The prefill gap is now closed by two unit tests, re-probed so they bite
(`5f4ff23`). The rename guard is a different case and gets no phase 0 test:
its current behaviour is *unreachable*, which is not a behaviour a test can
pin. **Phase 3 is what makes it reachable** — a branch row is the first name
with a separator in it — so the test that proves the refusal belongs there,
where it will bite.

### Phase 1 — the walk, and a listing made of it

`tc-core`: a cancellable walk yielding `(relative path, Entry)`, and
`Listing::branch` / `spawn_branch` built from it. `reload` re-walks in branch
mode rather than re-reading one directory. Hidden inheritance. The `name`
invariant re-documented.

**The walk and `search::spawn`'s walk are the same loop** — a queue, a
`read_dir`, unreadable skipped, cancel checked twice. If after writing it the
two really are the same, they become one (skill
[44](../skills/44-no-redundancy.md)); if the bodies have diverged, they stay
two and the plan says why. That is a judgement to make with both in front of
you, not now.

**This phase carries a measurement**, because the prime directive says a
performance claim does ([performance.md](../performance.md)): the branch view
of the 20 000-file tree the benchmark already uses, against the 80 ms it costs
to list one directory of 50 000.

### Phase 2 — the key, and the pane

`Action::BranchView` on `Ctrl+B`, the walk started on the active pane, Escape
cancelling it, the path bar marker, and any navigation leaving the mode. The
end-to-end test presses `Ctrl+B` and copies a file from two directories down —
which is the whole feature in one keystroke sequence.

### Phase 3 — the seven readers

Everything in § 2's table that needs changing, and every corner in § 3, each
with its test. This is the phase the plan exists for; the two before it are
the easy half.

### Phase 4 — refactoring audit

Skill [49](../skills/49-final-phase-refactoring-audit.md): the whole change
re-read as one diff. Plus the cookbook row in `CLAUDE.md`,
[listing.md](../listing.md), and `scripts/check-links.py`.

## 5. What is deliberately not in this

- **Copying with the subfolder structure preserved.** Total Commander has it
  as an option; it changes how a Copy job builds destinations, and it deserves
  its own decision rather than riding along here.
- **`Ctrl+Shift+B`, the branch view of the marked entries only.** The same
  machinery over a different root set, once the machinery exists.
- **Streaming rows in as they are found.** The bigger project that
  [performance.md](../performance.md) already names; sorting and marking over
  a list that is still filling is the hard part, and it is not this feature's
  to solve.
- **Watching the whole tree.** One inotify watch per directory is a different
  cost model. `Ctrl+R` re-walks.

## 6. Effort

Factor 0.25 per skill [45](../skills/45-calibrate-effort-estimates.md).

| Phase | Raw | Corrected |
|---|---|---|
| 0 — coverage pre-check | ~3 h | ~45 min |
| 1 — the walk and the listing | ~5 h | ~1.25 h |
| 2 — the key and the pane | ~4 h | ~1 h |
| 3 — the seven readers | ~6 h | ~1.5 h |
| 4 — audit | ~2 h | ~30 min |
| **Total** | | **~5 h** |

Phase 3 is the one to watch. The estimate assumes the § 2 table is complete —
it was built by reading every use of `Entry::name`, but a reader that reaches
it through a `Listing` method rather than the field would not have shown up in
that sweep, and the phase 0 probes are partly there to flush one out.
