# Compare by content: a side-by-side diff, and the door to a better one

Status: Draft — decisions 1–4 settled by the owner, 2026-08-31

Total Commander's *Compare by Content*, in this project's shape: pick two
files, see their lines side by side with the differences marked — and when
the owner has a real diff tool, hand the two paths to that instead, through
a settings line with placeholder parameters. The internal view is
deliberately *simple*; the external hook is what makes simple acceptable,
because anyone who outgrows it writes one config line.

## 1. What exists to build on

- **The external-tool precedent is `editor`** ([config.md](../config.md) §
  `editor`): a command line in settings, run like a typed one,
  `tc_core::command::open_with` appending one quoted path. The compare tool
  cannot append — two paths must land in caller-chosen positions — so it
  substitutes placeholders instead: `%1` and `%2`, Total Commander's own
  spelling, each replaced by a shell-quoted path. `command.rs` already owns
  `shell_quoted` and the spawn-on-a-thread shape.
- **The window precedent is the viewer** (`dialogs/viewer.rs`): a second
  window opened from an action, driven and closed by keys, already covered
  by the end-to-end suite — so a compare window is a known quantity to test,
  not a first.
- **The engine boundary holds**: the diff itself is text in, rows out — a
  `tc-core` module with no GTK in it, unit-testable headless, reading both
  files through the VFS so a file inside an archive compares like any other.
- **The keymap gate works for us**: a new action and binding force
  keymap.md's generated table and checked bindings table to follow in the
  same commit, or the gate is red.

## 2. Decisions before implementation

1. **The key.** No Total Commander default exists to inherit (there it is a
   menu item). Recommended: **`Ctrl+Shift+C`** — c for compare, one
   modifier above the `Ctrl+C` family, unbound today. Alternatives:
   `Shift+F2` (unbound, but TC users know it as *compare directories*,
   which we may want someday and should not squat), `Shift+F3` (view-ish),
   or no default binding at all.
2. **Which two files.** Recommended: Total Commander's cascade — **two
   marked files in the active pane** compare with each other; otherwise the
   **cursor file in the active pane against the same-named file in the
   other pane** when it exists; otherwise **cursor against cursor**.
   Alternative: always cursor-vs-cursor, simpler to explain, blind to the
   marked-pair gesture TC users reach for first.
3. **What a configured tool replaces.** Recommended: TC's rule — a
   non-empty `compare_tool` **is** the compare command; the internal view
   is what an empty setting means. One key, one meaning, the settings line
   decides. Alternative: two separate actions (internal and external),
   which costs a second binding and a second table row for a distinction
   the setting already expresses.
4. **How simple is the internal view.** The owner chose the fuller one,
   against the plan's line-level recommendation: **intra-line highlights in
   v1**. Side-by-side, synchronized scrolling, changed/added/removed tinted,
   `n`/`p` between difference blocks, `Esc` closes, read-only — and inside
   each `Changed` row pair, the character ranges that actually differ carry
   a stronger mark than the line tint. The cost lands in two places the
   phases below now budget for: the engine's `Changed` rows carry span
   lists, and the window paints ranges rather than whole lines.

## 3. Design points that are technical rather than owner decisions

- **The diff algorithm comes from the `similar` crate** (Myers with
  patience refinement, pure Rust, no further dependencies) rather than
  hand-rolled. The prime directive wants a measurement, not an
  implementation adventure: phase 1 benchmarks it on skill-74 worst cases —
  two large files with nothing in common, and two identical ones — and the
  numbers go into performance.md. If the crate embarrasses itself there,
  the module boundary (`compare.rs`, text in, rows out) is exactly the seam
  a replacement slots into.
- **Binary and huge files get a verdict, not a view.** A NUL byte in either
  file's first block, or either file over a named size ceiling, and the
  answer is a dialog line — *identical* / *differ, first at byte N* — from
  a streaming byte comparison that never loads the whole file. The viewer's
  "never read the file whole" rule stops at the diff's door (a line diff
  needs both files in memory), so the ceiling is where that honesty lives.
- **The external tool needs operating-system paths**, so inside an archive
  it refuses with the same words `F4` uses there ([viewer.md](../viewer.md) —
  an editor takes an OS path, and extracting a temp copy is not v1). The
  internal view reads through the VFS and works in archives and branch
  view.
- **`compare_tool` rides the settings round-trip** that already preserves
  `editor` — including the write-back rule config.md documents, where the
  saved copy is built from what was loaded so an unknown field survives.
  The existing close-and-reread end-to-end test gains the new line.

## 4. Phases

**Phase 0 — coverage pre-check** (skill 43). The seams this touches and
what covers them today: `Action` dispatch (the keymap gate tests plus e2e
per binding), the settings round-trip (the editor-preservation e2e), the
viewer window's open/close (e2e). No characterization tests owed — every
seam is already pinned; the new code brings its own.

**Phase 1 — the engine** (`crates/tc-core/src/compare.rs`). Read both
files via VFS; detect binary/oversize and answer with the verdict variant;
otherwise produce paired rows (`Same`, `Changed`, `LeftOnly`, `RightOnly`)
from `similar` — and for each `Changed` pair, the differing character spans
per side (decision 4), from a second, char-level pass over just that pair,
so the expensive refinement runs only on lines already known to differ.
Spans are byte ranges into the row's own text, aligned to `char`
boundaries, so the window can hand them to a text buffer unexamined. Unit
tests on the skill-52 invariant that matters: **each
side's rows, with the other side's insertions dropped, reconstruct that
side's file exactly** — a diff that loses or invents a line fails loudly — and its intra-line sibling: **a `Changed` row's two texts
with their differing spans deleted are equal**, so a span list that misses
or invents a difference fails the same way. Benchmarks with stated
worst-case layouts (skill 74), now including the intra-line worst case —
many long changed lines differing at their far ends; numbers to
performance.md in the same commit.

**Phase 2 — the setting and the substitution.** `compare_tool` in
settings with load/save preservation and the e2e line added;
`command::run_with_paths(template, left, right)` beside `open_with`,
substituting `%1`/`%2` with shell-quoted paths — unit-tested against
spaces, quotes, and a template that names a placeholder twice or not at
all (a missing `%2` is the user's business; the substitution still quotes
what it does place).

**Phase 3 — the action.** `Action::Compare`, the binding, the selection
cascade as a pure function over (marked names, cursor left, cursor right,
other pane's names) with unit tests per cascade arm; dispatch chooses
external (non-empty setting, both paths local — refusal message otherwise)
or the internal window. Directories refuse the way `F3` does.

**Phase 4 — the window** (`dialogs/compare.rs`). Two read-only text views
in one scrolled pair, one shared vertical adjustment so the sync cannot
drift, row tints from the engine's row kinds, the intra-line spans as
stronger tags over the tint (decision 4), `n`/`p`/`Esc`. End-to-end:
open on two fixture files that differ in a known line, assert the window
title carries both names; press `Esc`, assert it is gone; press the key on
a directory, assert the refusal. The e2e suite grows by ~3 tests ≈ 10 s.

**Phase 5 — docs, in the same commits as their subjects** where the gate
does not already force it: keymap.md (forced by the gate), config.md's
`compare_tool` section beside `editor`, a `docs/compare.md` (skill 29 —
one topic, indexed) holding the cascade, the verdict rule and the
`%1`/`%2` contract, and performance.md's measured numbers from phase 1.

**Phase 6 — refactoring audit** (skill 49) across the whole change, then
the end-of-plan ritual.

## 5. Effort

Corrected per skill 45 (feature plan; the last plans ran at ~0.10):

| Phase | Raw | Corrected |
|---|---|---|
| 0 | 0.25 d | 0.25 h |
| 1 engine | 2 d | 2 h |
| 2 setting | 0.5 d | 0.5 h |
| 3 action | 0.5 d | 0.5 h |
| 4 window | 2.5 d | 2.5 h |
| 5 docs | 0.5 d | 0.5 h |
| 6 audit | 0.5 d | 0.5 h |

About seven hours of work, plus a full-gate run (~14 min) per phase commit.

## 6. What would make this wrong

- **The shared-adjustment scroll sync** is asserted from GTK's documented
  model, not from a run; if two `TextView`s fight over one adjustment, the
  fallback is mirroring scroll positions in a handler, and phase 4 budgets
  for finding that out.
- **`similar`'s worst-case cost** is what phase 1 measures before anything
  is built on it; the plan names the seam a replacement would use.
- **The marked-pair cascade** reads Total Commander behaviour from memory
  and the owner's; decision 2 is where that gets confirmed or corrected.
- The e2e suite cannot see *colors*, so the tint mapping and the span
  tagging get unit tests on the mapping rather than screenshot assertions —
  the same line ui-shell.md already draws for the outline cursor.
- **Intra-line spans meet multi-byte text**: a span cut through a UTF-8
  sequence panics in a GTK buffer. The engine aligns spans to `char`
  boundaries by construction and a unit test feeds it text where every
  interesting boundary is multi-byte.
