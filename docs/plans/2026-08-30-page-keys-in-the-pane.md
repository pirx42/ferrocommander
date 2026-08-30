# Page Up and Page Down — the keys the model never hears about

Status: Proposed

`Page Up` and `Page Down` already move the cursor in a pane. They are the
only cursor keys in this program that do it **without the model finding
out**, and this is about closing that — plus the one live bug it has already
caused.

## 1. What is true today

They are **not bound**. `BINDINGS` has `Shift+Page_Up` and `Shift+Page_Down`
(marking across a screenful) and nothing for the plain keys, so an unbound
`Page_Down` falls through the window controller to the `ColumnView`, which
moves *its own* selection and scrolls. Visually it works.

The model cursor does not move. It catches up lazily: `dispatch` calls
`PaneView::adopt_selection` before every action, so the next bound key reads
the widget's selection back into the listing. That contract is written on the
method and is genuinely well kept —

> **The contract: the active pane's selection is adopted once per dispatched
> action, before the action runs.** So nothing reached from `dispatch` — no
> handler, no method on this type — has to call this, and a new action gets
> it for free.

— and two end-to-end tests hold it: `page_down_then_F5_copies_…` and
`page_down_then_space_marks_the_row_the_widget_moved_to`.

## 2. The bug: one path is not a dispatched action

**Type-ahead is not reached through `dispatch`.** A key no binding claims goes
to `typed_into_the_pane` in `main.rs`, which calls `PaneView::type_ahead`
directly, and nothing on that path adopts anything:

```rust
let Some(action) = shell.borrow().keymap.action_for(key, modifiers) else {
    return typed_into_the_pane(&shell, key, modifiers);   // ← no adopt
};
```

`type_ahead` then reads `self.shown.listing.cursor()` to decide where to
search from. So:

> **Page Down, then type a letter, and the search starts from the row you
> were on before the page** — off the top of the screen — rather than from
> the row you are looking at.

Everything about type-ahead's own rules is right; it is asking a stale
question. The contract did not break — **the ground under it moved**. It was
written when every route into a pane was an action, and it stayed true until
the letter keys stopped being one, which happened on 2026-08-30 in the
type-ahead work. Nothing failed, because the contract's own words are still
accurate about `dispatch`.

**Paging is not the only way in.** A mouse click also gives the widget a
selection of its own — `ui-shell.md` says so — so click, then type a letter,
is the same bug without touching a page key. That is why this ships before
anything else here and on its own: binding the page keys would hide the
instance that prompted this and leave the hole.

## 3. The reason for not binding them has expired

`keymap.md` argues the design:

> **Page Up / Page Down are the widget's job.** They are deliberately **not**
> in the table. Paging depends on how many rows fit on screen, and the model
> has no idea how tall the viewport is — the `ColumnView` does.

That was true and is not. `PaneView::page_rows()` measures exactly that —
content height over row count against viewport height — and has since
`Shift+PgUp`/`PgDn` needed it. The measurement the argument says is
impossible is a method on the type, already used, already tested end to end
on a screen tall enough to hold every row.

Its own doc comment still repeats the argument it refutes:

> Measured rather than assumed: the model has no idea how tall the viewport
> is, which is exactly why plain Page Up/Down are left to the widget.

A decision whose reason was removed by later work, and which nobody went
back to. Exactly the shape the 2026-08-30 documentation review was about,
found the same way — by reading the code against the prose.

## 4. What to build

Bind them, as the ordinary cursor keys they are:

| | |
|---|---|
| `Action::CursorPageUp` / `CursorPageDown` | two variants beside `CursorUp`/`CursorDown` |
| `cursor_page_up` / `cursor_page_down` | the `[keys]` names, matching `extend_mark_page_up` next to them |
| `page_rows()` | the same measurement `extend_by_page` already uses — one place decides what a page is (skill 44) |

The move is `move_cursor_by(±page_rows())`, which already **clamps at both
ends rather than wrapping** — Total Commander's behaviour, and the same rule
the arrow keys follow. The `..` row needs no special case: it is row 0, and
paging up lands on it exactly as `Home` does.

What this buys beyond the fix:

- **Rebindable.** Every other key in this program can be moved in `[keys]`;
  these two could not, and there was no way to say so.
- **In the tables.** They appear in the generated action-name table and in
  the bindings table — and the two checks added on 2026-08-30 will *fail the
  gate* until they do, so the documentation cannot be forgotten. That is the
  first time those checks are load-bearing for a change rather than for a
  review.
- **One rule for the cursor.** The model becomes authoritative for every key
  that moves it, which is what `adopt_selection` exists to paper over.

### The one thing to check by eye

A bound key returns `Propagation::Stop`, so the widget stops paging itself
and `sync_cursor` does the scrolling — select, focus and scroll-into-view in
one `scroll_to`. The widget keeps the cursor row at the same screen position
when *it* pages; scroll-into-view may instead put the row at an edge. Whether
that reads the same is not something the end-to-end suite can see — it
asserts on the filesystem, not on pixels — so it goes the way the scroll
memory went: `scripts/check-scroll-memory.sh`'s method, screenshots, by hand,
and the answer written down either way.

## 5. Phases — one phase, one commit

**Phase 0 — coverage pre-check** (skill 43). What holds paging today:
`page_down_then_F5_copies_…` and
`page_down_then_space_marks_the_row_the_widget_moved_to` pin the *dispatched*
path, and `shift+Next`/`shift+Prior` pin the marking keys. **Nothing pins the
non-dispatched path**, which is the gap phase 1 fills — and the reason the
bug shipped past a green suite.

**Phase 1 — the adoption hole.** Type-ahead adopts the widget's selection
before it searches. A test that pages and then types, and a second that
clicks and then types if the harness can click. Ships alone: it is a bug, it
is reachable without a page key, and binding the page keys would hide it.

**Phase 2 — bind the two keys.** The actions, the dispatch arms, the tests,
the probes, and the documentation in the same commit (skill 28) — which the
bindings-table check makes mandatory rather than optional. `keymap.md`'s
"the widget's job" section is rewritten to say what is true *and why it
changed* (skill 30), and `page_rows()` loses the comment that argues against
its own existence.

**Phase 3 — the scroll, by eye.** Screenshots before and after, and a
paragraph in `ui-shell.md` recording what was checked and how — the honest
version of a property no test in this repository can see.

**Phase 4 — refactoring audit** (skill 49).

## 6. Effort

Corrected per skill 45, factor 0.10 as in the last two plans.

| Phase | Raw | Factor | Corrected |
|---|---|---|---|
| 0 pre-check | 0.25 d | 0.10 | 0.25 h |
| 1 the adoption hole | 1 d | 0.10 | 1 h |
| 2 bind the keys | 1.5 d | 0.10 | 1.5 h |
| 3 the scroll by eye | 0.5 d | 0.10 | 0.5 h |
| 4 audit | 0.5 d | 0.10 | 0.5 h |

**About three and three-quarter hours**, and the gate run is most of it: each
phase costs a nine-minute suite before it can be committed.

## 7. What was considered and not chosen

- **Fix the adoption and leave the keys unbound.** Smaller, and it fixes the
  bug — phase 1 alone is exactly this. It was not chosen as the whole answer
  because it leaves two keys that cannot be rebound, are absent from both
  tables, and rely on the widget agreeing with the model about what a page
  is. The `..` row is the tell: the widget will happily page onto it and the
  model has rules about that row everywhere else.
- **Make `adopt_selection` automatic** — a selection-changed signal writing
  every widget move into the model. It removes the class of bug rather than
  this instance, and it is a bigger change than this asks for: the model
  would start hearing about selections the widget invents while repopulating,
  which `refresh` currently works to suppress. Worth its own plan if a third
  instance turns up.
- **Match the widget's scroll exactly**, by computing the offset rather than
  calling `scroll_to`. Deferred until phase 3 says whether anybody would
  notice.
