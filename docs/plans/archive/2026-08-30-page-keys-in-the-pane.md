# Page Up and Page Down — the keys the model never hears about

Status: Implemented

`Page Up` and `Page Down` already move the cursor in a pane. They are the
only cursor keys in this program that do it **without the model finding
out**, and this is about closing that — plus the one live bug it has already
caused.

## 0. What was decided, and by whom

Owner, 2026-08-30, on the three questions the proposal left open:

- **Phases 1 and 2 both.** The bug and the binding.
- **The scroll must be identical**, not close enough. So phase 2 does not hand
  the scrolling to `scroll_to`: it reproduces what the widget does today and
  the plan is wrong if anybody can tell the difference.
- **`cursor_page_up` / `cursor_page_down`** as the `[keys]` names.

The scroll decision is the one that changes the work, and it changes it in a
way worth writing down before starting: **phase 2 begins with a measurement,
not an implementation.** What the `ColumnView` does to the scroll offset on a
page is not written down anywhere, and reproducing it from a guess is how you
ship a key that is subtly worse than the one it replaced. So the first thing
phase 2 does is instrument the adjustment and press the key.

The mechanism is available. `scroll_to` takes a `ScrollInfo`, and
`ScrollInfo::set_enable_vertical(false)` — GTK 4.12, inside this project's
floor — makes it select and focus **without scrolling**, leaving the offset to
be set deliberately rather than inferred. Checked before promising it.

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
| `page_step()` | the same measurement the marking twins use — one place decides what a page is (skill 44) |

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

### The scroll, which the owner settled

A bound key returns `Propagation::Stop`, so the widget stops paging itself and
the shell owns the scroll. `sync_cursor`'s `scroll_to(.., None)` scrolls
*minimally* — enough to bring the row into view, which puts it at an edge. The
widget instead keeps the cursor row where it was on screen and moves the view
under it. Those are different, and **the owner's call is that they must not
be** ( § 0).

So phase 2 is measure, then reproduce:

1. **Measure.** Instrument the vertical adjustment — value, page size, upper —
   around a real `Page_Down` on a tall directory, under the end-to-end
   harness's own X server. What the widget does is not documented and must not
   be guessed at.
2. **Reproduce.** `scroll_to` with a `ScrollInfo` whose
   `set_enable_vertical(false)` is set, so it selects and focuses and does not
   scroll; then set the adjustment to whatever step 1 says.
3. **Compare.** Screenshots before and after, the way the scroll memory is
   checked (`scripts/check-scroll-memory.sh`), because the end-to-end suite
   asserts on the filesystem and cannot see a pixel.

If step 1 says the widget's rule is something the shell cannot reproduce
exactly, that is a finding and it goes in the plan rather than being rounded
off — the honest outcome is "not identical, here is how it differs", not a
quiet approximation.

## 5. Phases — one phase, one commit

**Phase 0 — coverage pre-check** (skill 43). What holds paging today:
`page_down_then_F5_copies_…` and
`page_down_then_space_marks_the_row_the_widget_moved_to` pin the *dispatched*
path, and `shift+Next`/`shift+Prior` pin the marking keys. **Nothing pins the
non-dispatched path**, which is the gap phase 1 fills — and the reason the
bug shipped past a green suite.

**Phase 1 — the adoption hole. Done.** `typed_into_the_pane` adopts before it
searches, and `page_down_then_a_letter_searches_from_the_row_on_screen` holds
it. The test was written first and watched to fail against the unfixed code —
it copied `row000.txt`, the first `row*.txt` in the listing, which is exactly
what searching from the stale cursor at `..` finds. That is the probe, in its
strongest form: a test that has been red for the real reason rather than for a
mutation of it.

The second test — click, then type — was not written. `xdotool` can click, but
placing a click on a *row* means knowing where that row is in window
coordinates, which the harness has no way to compute; and the paging test
already covers the mechanism the click would exercise. Said here rather than
left looking like coverage.

The contract comment on `adopt_selection` and its paragraph in `ui-shell.md`
now say why nothing broke: the rule "once per dispatched action" stayed true
while a route appeared that is not a dispatched action. The question for a new
route is not whether the contract covers it, but whether it is an action.

**Phase 2 — bind the two keys. Done**, and the measurement in step 1 changed
the design for the better.

**The plan's premise was wrong.** It said the widget "keeps the cursor row at
the same screen position", and that reproducing it might need `ScrollInfo` to
suppress GTK's own scrolling. Instrumenting the adjustment said otherwise: at
a 39-pixel row in a 579-pixel viewport, fourteen rows fit and the widget's
paging settled at offsets **0, 474, 981, 1488** — a *thirteen*-row step with
the ordinary minimal scroll-into-view, which reproduces all four exactly. So
the widget scrolls minimally, like `scroll_to` already does, and keeps one row
of overlap. `ScrollInfo` was not needed at all; the whole difference between
"close enough" and "identical" was one row.

That row is `PAGE_OVERLAP_ROWS`, and it earned a second finding: `Shift+PgDn`
was marking fourteen rows where `PgDn` moved thirteen. Both ask `page_step`
now — one place decides what a page is (skill 44).

Five gate checks failed the moment the actions existed and named exactly what
was missing: the bindings table, the generated action table, the end-to-end
press check, the `ALL` list, and the test asserting the old design. **That is
the first time this repository's documentation checks have been load-bearing
for a change rather than for a review**, and they behaved as advertised.

The old design's test was rewritten rather than deleted, and says what changed
and why (skill 24).

**Phase 3 — the scroll, by eye. Done**, via `scripts/check-page-scroll.sh`,
which is committed rather than left in a scratch directory (skill 68) because
it is the only check on the appearance there will ever be.

It showed the first page moving thirteen rows without scrolling, the second
page opening on the row the cursor had just been on, and a two-down-two-up
round trip returning the pane pixel-identical. The single difference in that
comparison was in the *other* pane, an overlay scrollbar caught mid-fade.

**A probe found what the tests cannot see.** Removing the overlap row left
`page_down_moves_the_cursor_a_screenful_and_page_up_brings_it_back` green: a
round trip cannot see the step size, because both directions use it and
cancel. So the arithmetic was split into a pure `page_step_for` and pinned by
a unit test carrying the measured numbers, and the appearance is recorded in
`keymap.md` as verified by hand — not as covered.

**Phase 4 — refactoring audit** (skill 49). **Done.**

Three phases added 298 lines of Rust and a script. The audit changed one thing
and it was worth the phase.

**Two free functions in `actions.rs` were not free functions for the reason
they claimed.** `extend_by_page` said it existed "because the page size has to
be measured off the pane before the same pane is borrowed mutably to act on
it", and `move_by_page` was written to match. The borrow it blames does not
exist — measuring and acting are sequential uses of one `&mut`, which the
compiler is perfectly happy with, and testing that took one build. Both are
now `PaneView` methods (`move_by_page`, `extend_mark_by_page`) beside the
other cursor moves, and the four dispatch arms read like the arms around them
rather than calling out to a helper. A comment that was wrong about the
compiler is gone with them.

That the new code copied the old code's mistaken reason is the part worth
recording: an explanation in a comment is load-bearing, and copying its
*shape* without testing its *claim* is how a wrong reason outlives the code
that first had it.

**Left alone.** `page_rows` now has exactly one caller (`page_step`) and stays
split from it, because the two answer different questions — how many rows fit,
and how far a key moves — and collapsing them would put the measured overlap
inside a function named for the measurement. `check-page-scroll.sh` duplicates
the scroll-memory script's Xvfb preamble, which is real duplication and left
deliberately: the two scripts are read one at a time by someone debugging, and
a shared preamble would mean opening two files to understand either.

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
