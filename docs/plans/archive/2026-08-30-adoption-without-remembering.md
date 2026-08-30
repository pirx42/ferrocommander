# Adoption without remembering — the selection signal

Status: Implemented

`PaneView::adopt_selection` reads the widget's selection back into the model.
Two places call it, and both call it because somebody remembered to. This is
about the shell being told instead.

## 0. Why now, and the honest case against

The page-keys plan deferred this with a condition — *"worth its own plan if a
third instance turns up"* — and **a third has not turned up.** The owner asked
for the plan anyway, which is their call to make; what follows is written so
the decision to build it can be made on what it is rather than on the memory
of two bugs.

**There is no live bug here.** Both routes into a pane adopt today:
`dispatch` for every action, and `typed_into_the_pane` for type-ahead since
this morning. Binding `Page Up`/`Page Down` removed the widget-driven move
that caused the first instance. What is left that moves a selection behind the
model's back is **the mouse click, and nothing else** — and both of the
routes that would then read a stale cursor already adopt before they do.

So this buys **prevention, not a fix**: that the next route into a pane owes
nothing, instead of owing a call somebody has to know about. The two instances
so far both came from adding a route — type-ahead, and before it the paging
the widget owned — and neither author knew there was a call to make. That is
the failure mode, and it is the one a signal removes.

**What would make it not worth building:** if the answer to "how often does a
new route into a pane appear" is "twice, ever, and both are now covered". The
counter is that a rule kept by discipline fails silently and this one already
has, twice, in a week.

## 1. What is true today

```rust
pub fn adopt_selection(&mut self) {
    if let Some(cursor) = adopted_cursor(self.selection.selected()) {
        self.shown.listing.set_cursor(cursor);
    }
}
```

Three callers. `dispatch`, once per action, which is the documented contract.
`typed_into_the_pane`, added this morning because type-ahead is not an action
and the contract's words stayed true while ceasing to be sufficient. And the
pane exchange, which adopts the *other* pane and is a different call.

`refresh` works around the same mechanism from the other side: emptying and
refilling the store makes the widget move its own selection, so the model's
cursor is saved and restored around the rebuild.

## 2. The question that decides it, and its answer

A `SingleSelection` emits `selection-changed` on every move, ours and the
user's alike. Adopting all of them would be wrong — the shell sets the
selection itself in `sync_cursor` and churns it in `refresh`, and reading
those back as intent is how a rebuild would eat the cursor.

**So: can the two be told apart?** Measured, not guessed. A probe on
`connect_selection_changed` reporting whether the shell's `RefCell` is free at
the moment of emission, driven through the end-to-end harness:

| What moved the selection | `shell` free? |
|---|---|
| Startup, both panes | no |
| `Down`, `Page Down` | no |
| `Return` into a directory (store rebuilt) | no |
| `BackSpace` out of it (store rebuilt) | no |
| **A mouse click on a row** | **yes** |

Seven emissions with the borrow held, one without, and the one without is
exactly the one that is the user's. **Every move the shell makes is made from
inside a `borrow_mut`; the only move that is not ours arrives with nothing of
ours on the stack.**

That is not a coincidence to lean on carelessly, but it is not luck either: a
GTK signal emitted while we are inside our own mutation is emitted *because*
of that mutation.

**There is a precedent, shipped and reasoned.** `wire_filter_bar` handles the
identical shape and its comment already says why skipping is right:

> Reentrancy is real here: hiding the field or navigating away sets the text
> from inside a `borrow_mut`, and that emits `changed`. The borrow failing
> means a pane is already applying this very change itself, so skipping is not
> a lost update — it is the same update, once.

## 3. What to build

One handler per pane, wired where `wire_filter_bar` is:

```rust
selection.connect_selection_changed(move |_, _, _| {
    // A move we made is made from inside a borrow; a move the user made is
    // not. So the borrow failing is the signal that this one is ours, and
    // ours is already in the model.
    if let Ok(mut state) = shell.try_borrow_mut() {
        state.panes[index].adopt_selection();
    }
});
```

Then the two explicit calls come out — but **one at a time, each proved
redundant rather than assumed to be.** Removing a call and watching the suite
stay green is the probe: it says the automatic path covers what the explicit
one did.

## 4. The risk worth naming

The table in § 2 is seven emissions, not a proof. What it cannot rule out is a
selection move GTK makes **outside** any of our borrows and which is still not
the user — a deferred adjustment on an idle turn, or a layout pass after a
resize. Such a move would be adopted as intent, and the symptom would be a
cursor that wanders on its own: rare, hard to reproduce, and much worse than
the bug this fixes.

So phase 1 keeps the explicit calls and adds the handler beside them, and the
suite runs unchanged. If anything moves, it moves before a single line of
safety net has been taken away.

## 5. Phases — one phase, one commit

**Phase 0 — coverage pre-check** (skill 43). What holds adoption today:
`page_down_then_f5_acts_on_the_row_the_widget_moved_to`,
`page_down_then_space_marks_the_row_the_widget_moved_to` and
`page_down_then_a_letter_searches_from_the_row_on_screen`. All three drive the
*keyboard*. **Nothing covers a mouse click**, which is now the only instance
of the class — so that test comes first, and it is worth having whether or not
the rest is built.

*(The page-keys plan said such a test could not be written, because "placing a
click on a row means knowing where that row is in window coordinates, which
the harness has no way to compute". That was wrong, and the probe in § 2
disproved it by accident: a click at fixed coordinates in the pane landed on a
row, and a test does not need to know **which** row — only that the cursor
ended up somewhere the model had not put it.)*

**Done.** `a_click_then_a_letter_searches_from_the_row_that_was_clicked`, on a
`click` helper added to the harness beside `drag`. It passes today, because
`typed_into_the_pane` adopts — and removing that adoption turns it red with
the message it was written for, which is what makes it the probe phase 2's
first removal will be measured against. The helper's own comment carries the
rule the test has to obey: never assert *which* row a click lands on.

**Phase 1 — the handler, beside the explicit calls. Done.** `wire_selection`
sits where `wire_filter_bar` does, both paths live at once, nothing removed.

**The § 4 risk did not materialise.** All 165 end-to-end tests passed
unchanged with the handler installed — so no selection move GTK makes outside
our borrows is being adopted as intent, at least not on any path the suite
walks. That is the evidence phase 2 needs before it starts taking the safety
net away, and it is worth more than the seven-row table in § 2 because it
exercises every route rather than the five a probe happened to press.

**Phase 2 — take the explicit calls out, one per commit. Done**, both, and
neither needed a comment explaining why it had to stay.

**The first removal exposed a gap the plan had not seen.** The three tests
that pinned `dispatch`'s adoption all page with the keyboard — and paging
became a bound action the day before, so the widget no longer moves the
selection there and those tests no longer exercise what they were written for.
They passed with the line removed because they no longer touch it. The case
that still does is a click followed by an *action*, and nothing covered it, so
`a_click_then_f5_copies_the_row_that_was_clicked` was written to be the probe
before the removal could be believed.

That is worth naming as a hazard of this kind of work: **a test can stop
covering its subject without going red**, when the subject moves out from
under it. Both removals are now held by tests that fail with the handler
disabled — checked, both, in both directions.

**Phase 3 — the contract, rewritten. Done**, in the same commit as the second
removal, since the sentence being replaced was made false by it (skill 28).
`adopt_selection`, `ui-shell.md` and `keymap.md` describe a mechanism now, and
each keeps the epitaph of the rule it replaced: kept perfectly, insufficient
anyway, twice in a week, on paths whose authors had no way to know.

**Phase 4 — refactoring audit** (skill 49). **Done.**

Net **−33 lines** of production code: two explicit calls and their two
paragraphs of contract, replaced by one handler.

**One finding, and it was hiding behind a comment that had stopped being
true.** `refresh` saved the model's cursor across the store rebuild and
restored it afterwards, because "emptying and refilling the store makes the
widget move its own selection, which `adopt_selection` would then read back as
the user's intent". Nothing between those two lines can change the model's
cursor — reaching `refresh` at all means holding the borrow that keeps the
handler out — so the restore wrote back a value that never moved. Checked by
removing it and running all 166 end-to-end tests, not by reasoning alone. The
lines are gone and the comment now says why no protection is needed (skill
19).

That the *old* comment named `adopt_selection` is what made it worth looking
at: it described a hazard in terms of a mechanism this plan replaced, which is
the shape of a comment that has outlived its subject.

**Left alone.** `wire_selection` sits beside `wire_filter_bar` and duplicates
its shape — clone the widget, clone the shell, `try_borrow_mut`, act — which
is real duplication of about four lines. Extracting it would mean a helper
generic over the widget and the signal, for two call sites whose *reasoning*
is shared and already cross-referenced in both comments. The reasoning is the
part worth sharing, and it is.

## 6. Effort

Corrected per skill 45, factor 0.10.

| Phase | Raw | Factor | Corrected |
|---|---|---|---|
| 0 the click test | 0.5 d | 0.10 | 0.5 h |
| 1 the handler | 0.5 d | 0.10 | 0.5 h |
| 2 the two removals | 1 d | 0.10 | 1 h |
| 3 docs | — | — | 0.5 h |
| 4 audit | 0.5 d | 0.10 | 0.5 h |

**About three hours**, of which the gate runs are more than half — five
commits at nine minutes each.

## 7. What is deliberately not in this

- **Making the cursor live in the widget.** The honest end of this road is one
  authority rather than two kept in step, and it is a different and much
  larger change: the `Listing` cursor is what every pure decision in
  `navigation.rs` and `jobs.rs` is written against, and it is what makes them
  testable without a display server. Two authorities with a signal between
  them is the smaller thing, and this plan is only the smaller thing.
- **Adopting in the inactive pane.** The handler is per pane and would fire
  for either, but the pane exchange's explicit call stays: it adopts a pane
  `dispatch` never touches, for a reason that has nothing to do with this.
