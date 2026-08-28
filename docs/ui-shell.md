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

The `gtk4` bindings are built against the **baseline GTK 4.0 API** — no
`v4_x` feature is enabled. The shell needs nothing newer, and a higher floor
would exclude distributions and Windows builds shipping an older libgtk-4.

This has teeth: `CssProvider::load_from_string` needs GTK 4.12 and is
therefore unavailable; `load_from_data` is the baseline equivalent. When an
API is missing at compile time, that is the floor doing its job — reach for
the baseline call, do not raise the floor without deciding to.

## Testing

`row.rs` is unit-tested headlessly, including the date formatter, which is
split into "which time zone" and "how it is formatted" so the format can be
asserted against a fixed instant instead of wherever the machine happens to
be.

Window construction is verified by running the binary — per the design doc,
the shell is kept thin enough that manual testing plus the pure-logic tests
suffice for v1.
