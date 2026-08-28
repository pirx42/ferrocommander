# UI shell — the GTK4 window

← Parent: [CLAUDE.md](CLAUDE.md)

`tc-app` assembles widgets and forwards everything else to `tc-core`. It
contains no filesystem access, no sorting, and no notion of what a directory
holds — those are [vfs.md](vfs.md) and [listing.md](listing.md).

## Widget tree

```
ApplicationWindow
└── Paned (horizontal, 50/50)
    ├── PaneView.root : Box(vertical)      ← left
    │   ├── Label            .path-bar
    │   └── ScrolledWindow
    │       └── ColumnView   Name | Ext | Size | Date
    └── PaneView.root : Box(vertical)      ← right
```

A `PaneView` owns a `Listing` and a `gio::ListStore` of `PaneEntry` objects.
`PaneEntry` is a minimal GObject wrapping a rendered [`Row`] — GObject only
because `ColumnView` requires its items to be one, not because the row needs
object semantics.

## Dialogs

Every dialog is built from one shell in `dialogs.rs`, so three dialogs do not
become three layouts. Each takes a callback rather than returning an answer:
GTK4 has no blocking dialog, and the shell must keep running the main loop
while one is open.

| Dialog | Opened by | Answers |
|---|---|---|
| target | F5, F6 | a line of text — see [keymap.md](keymap.md) |
| name | F7 | a line of text |
| delete confirmation | F8, Shift+F8 | yes / no |
| conflict | a job that hit an existing target | overwrite / skip / keep both / abort, each with *apply to all* |

**Escape closes all of them.** A modal `gtk::Window` does not do this on its
own, and a dialog with no way out but the mouse is a trap in a keyboard-first
program.

**The safe button starts focused.** Permanent delete opens with Cancel under
the finger and a red Delete beside it; the conflict dialog opens on Skip, the
one answer that loses nothing. Enter is the key everyone reaches for, so it
must never be the one that overwrites a file. The conflict dialog originally
opened with *no* focus at all, which made it mouse-only — found by the smoke
run, not by a test.

**Closing the conflict dialog without choosing answers nothing**, and the
engine reads that silence as abort. See [ops.md](ops.md).

## Running a job

The shell owns a `JobQueue`. A keystroke opens a dialog, the dialog's answer
builds a `Job`, and the job goes to the queue with both panes' backends. Two
futures on the GLib main loop then follow it — one draining conflict questions
into dialogs, one waiting for the report — because the two arrive on separate
channels and neither should wait for the other. Both end on their own when the
job does.

**Both panes reload when a job finishes.** A copy changed the target side, a
move changed both, and a delete may have removed the directory a pane was
standing in — which is why the reload goes through `Listing::load_nearest`
rather than a plain reload. See [listing.md](listing.md).

## Where the logic lives

Only one thing in this crate is real logic — turning an `Entry` into the four
column strings — so that is the part that is pure, GTK-widget-free, and
tested: `row.rs`. Everything else is widget assembly, verified by running the
program.

Rendering rules:

- **A directory keeps its whole name.** A folder called `archive.tar.gz` is
  not a `.gz` file, so only files are split across the name and ext columns.
- **Directories show `<DIR>`** instead of a byte count, symlinks to
  directories included.
- **Sizes are grouped in threes** and right-aligned, so digits line up by
  magnitude.
- **The `..` row shows no timestamp.** It is a navigation control, not a file;
  printing the epoch would be a lie.

## Columns

Titles, widths, alignment and value extraction all hang off the `Column` enum,
and the column set is built by iterating `Column::ALL`. Adding a column is one
variant, not four scattered edits (skill
[53](skills/53-generate-instead-of-duplicating.md)). The name column expands
into leftover width; the rest stay fixed so the two panes line up with each
other.

## Cursor and selection

The `Listing` cursor is authoritative and the widget's selection mirrors it —
but only in one direction, and only while the widget cooperates. Keys the
shell does not bind reach the `ColumnView`, which moves its selection without
asking. So the traffic runs both ways:

- **Model → widget** after anything that changes the listing (`refresh`,
  cursor moves, navigation), via `sync_cursor`.
- **Widget → model** at the start of every dispatched action, via
  `adopt_selection`.

`sync_cursor` does three things in one `ColumnView::scroll_to` call —
**select**, **focus**, and **scroll into view**. All three are needed.
Selecting alone moves nothing: only the widget's own key handling scrolls,
and this shell binds the cursor keys itself, so the selection would silently
walk off the bottom of the viewport. Focus matters just as much: Page Up/Down
are handled by the widget and page from *its* focus, so unless focus follows
our cursor, paging resumes from wherever the widget last was.

`refresh` saves and restores the cursor around the store rebuild, because
emptying and refilling the store makes the widget move its selection on its
own — which the next adoption would otherwise read back as the user's intent.

## Active pane

Exactly one pane is active. It is marked by a style class on its **path bar**,
not by dimming the inactive pane: in a dual-pane manager the inactive side
must stay fully readable, since at that moment its whole job is to show you
where a copy would land.

## Startup

Both panes open at the user's home directory (`LocalFs::home_dir()`).
Remembering the last directory is config persistence — phase 3.

A directory that cannot be read yields an **empty pane** rather than a failed
window: one broken pane still leaves a usable program. Phase D puts the reason
in the path bar.

## GTK version floor

The `gtk4` bindings are pinned to the **GTK 4.12 API** (`features =
["v4_12"]`), so the binary requires libgtk-4 ≥ 4.12. That excludes Debian 12
(4.8) and Ubuntu 22.04 (4.6); 4.12 is from March 2024.

The floor started at the 4.0 baseline, on the assumption that the shell
needed nothing newer. That assumption was wrong, and it is worth recording
why rather than just the outcome: **scrolling the cursor row into view has no
supported baseline API.** `gtk_column_view_scroll_to` arrived in 4.12, and
the `list.scroll-to-item` action that predates it lives on `GtkListBase` —
which `ColumnView` is not; it wraps an internal list view. Reaching that
action means poking at GTK's private widget tree.

The cost of raising the floor is a list of old distributions. The cost of not
raising it was either a broken cursor or a hack on undocumented internals.

Raising it is not free elsewhere: from 4.12 a `SignalListItemFactory` hands
its callbacks a plain `Object` rather than a `ListItem`, because a factory
can also produce header and cell items, so the column factory downcasts.

## Testing

`row.rs` is unit-tested headlessly, including the date formatter, which is
split into "which time zone" and "how it is formatted" so the format can be
asserted against a fixed instant instead of wherever the machine happens to
be.

Window construction is verified by running the binary — per the design doc,
the shell is kept thin enough that manual testing plus the pure-logic tests
suffice for v1.

**Running it while another instance is open.** `gtk::Application` is
single-instance: launching the binary again hands off to the running process,
which opens a second window *there* and exits 0 here. A smoke run that exits
0 immediately has not tested anything. To start an isolated instance
alongside one that is already running:

```bash
DBUS_SESSION_BUS_ADDRESS="unix:path=/nope" ./target/release/tc-app
```

Registration fails, the app falls back to a private instance, and the only
output is one harmless "Unable to acquire session bus" warning.

**A quiet run is not a passing run.** Where the code ignores a `Result` —
`activate_action` and friends — a failure prints nothing at all. A first
attempt at scroll-to-cursor was a silent no-op for exactly this reason, and
the clean smoke run said nothing. When verifying a GTK call that returns a
`Result` the code discards, print the result once and look at it.
