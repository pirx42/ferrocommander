# UI shell — the GTK4 window

← Parent: [CLAUDE.md](CLAUDE.md)

`tc-app` assembles widgets and forwards everything else to `tc-core`. It
contains no filesystem access, no sorting, and no notion of what a directory
holds — those are [vfs.md](vfs.md) and [listing.md](listing.md).

## Widget tree

```
ApplicationWindow
└── Box (vertical)
    ├── Box (horizontal)                   ← the drive bar, one button per mount
    ├── Paned (horizontal, 50/50)
    │   ├── PaneView.root : Box(vertical)  ← left
    │   │   ├── Label            .path-bar
    │   │   ├── Entry            .filter-bar   (hidden until Ctrl+S)
    │   │   ├── ScrolledWindow
    │   │   │   └── ColumnView   Name | Ext | Size | Date | Attr
    │   │   └── Box (horizontal)               ← the status line
    │   │       ├── Label        n of m selected
    │   │       └── Label        free of total
    │   └── PaneView.root : Box(vertical)  ← right
    └── Box (horizontal)                   ← the command line
        ├── Label               the prompt
        └── Entry
```

The window's child is the outer `Box`, not the `Paned`: the drive bar and the
command line are siblings of the panes, so neither is squeezed by the divider
and both span the full width.

A `PaneView` owns a `Listing` and a `gio::ListStore` of `PaneEntry` objects.
`PaneEntry` is a minimal GObject wrapping a rendered [`Row`] — GObject only
because `ColumnView` requires its items to be one, not because the row needs
object semantics.

## The drive bar

A row of buttons above the panes, one per mounted filesystem, each sending
the **active** pane there — the active one, because that is where the keyboard
is and what the user is looking at.

What counts as a mount worth offering is a judgement call, so it is made in
one place and tested against a fixture rather than against whatever the
running machine happens to have mounted: pseudo-filesystems are excluded by
type, and anything under `/proc`, `/sys`, `/dev` or `/run` by path. On Windows
the same function returns the drive list, which `root_entries` already knew
how to find.

## Dialogs

Every dialog is built from one shell in `dialogs/mod.rs`, so ten dialogs do not
become ten layouts. Each takes a callback rather than returning an answer:
GTK4 has no blocking dialog, and the shell must keep running the main loop
while one is open.

| Dialog | Opened by | Answers |
|---|---|---|
| target | F5, F6 | a line of text — see [keymap.md](keymap.md) |
| name | F7, Shift+F4, Alt+F5 | a line of text |
| delete confirmation | F8, Shift+F8 | yes / no |
| conflict | a job that hit an existing target | overwrite / skip / keep both / abort, each with *apply to all* |
| a list to pick from | Alt+F1/F2, Ctrl+↓, Num +/− | one row |
| favourites | Ctrl+D | one row — and it is edited in place, see [keymap.md](keymap.md) |
| failures | a job that could not finish everything | nothing; it reports |
| output | a command that printed something, and the refusals inside an archive | nothing; it reports |
| progress | a job that outlives `PROGRESS_DELAY` | cancel, or Background — which closes the window and leaves the job running |
| viewer | F3 | its own keys — see [viewer.md](viewer.md) |
| search | Alt+F7 | a result to go to — see [search.md](search.md) |
| multi-rename | Ctrl+M | rules, and a preview of them — see [multi-rename.md](multi-rename.md) |

The last four, and the favourites, have a lifetime of their own — a bar being
driven, an offset being paged, a list filling as results arrive, a preview
redrawn on every keystroke, a list whose rows change under it — so each is a
file under `dialogs/` rather than a function that opens a window.

**The favourites list is the one that outlives its own choice.** Adding and
removing happen inside it, so it needs the window to stay and the rows to be
redrawn; that is what separates it from `choose_one`, which hands back one
value and closes. What the two share — the row with a dimmed path beside the
name, and the scroller that only scrolls past a screenful — they share as
code, because two lists meant to look alike and built separately are two
lists that drift.

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

## Marks

Marked rows are drawn in red — colour only, no bold. Bold text is wider, and
it pushed the date out of its fixed-width column so a marked row read
`2026-0... 8 17:20`; Total Commander marks in red alone for the same reason.
Red also survives the row being the cursor at the same time, which a
background colour would not.

A status line under each pane shows `n of m selected — x of y`, which is the
number a person checks before pressing F5, and at its other end how much room
is left on the disk: `14.4 GiB free of 252.0 GiB`. Both figures, because
"18 GB free" alone says nothing about whether that is a nearly empty disk or a
nearly full one. The two ends grow in opposite directions, so a long selection
summary and a long size cannot push each other off the line.

The figure is asked of the **backend**, not of the path: an archive has no
free space of its own and leaves it empty, rather than reporting the disk the
archive file happens to sit on — an answer to a question nobody asked. A
filesystem that has gone leaves it empty too, because an empty status line is
honest where `0 B free` is a lie. On Windows it is empty for now
([future-improvements.md](future-improvements.md)). It is rendered by the same pure
`jobs` module that decides what an operation acts on, so the count in the
question and the count in the status line cannot disagree.

The dialog that asks for a wildcard is the same text dialog F5 and F7 use.

## Running a job

The shell owns a `JobQueue`. A keystroke opens a dialog, the dialog's answer
builds a `Job`, and the job goes to the queue with both panes' backends. Two
futures on the GLib main loop then follow it — one draining conflict questions
into dialogs, one waiting for the report — because the two arrive on separate
channels and neither should wait for the other. Both end on their own when the
job does.

**A progress window appears only once a job has proved it will take a
moment.** The check is against elapsed time as events arrive rather than on a
timer, so a job that finishes first never opens one and a job that moves no
bytes — `mkdir` — never qualifies. Cancel pulls the same token the engine
checks between tasks and inside the copy loop, and the window closes on the
click rather than waiting for the worker to notice, because a dialog that
lingers after a click looks broken.

**Background sends the job on without watching it.** The button closes the
window and does nothing else: the cancel token is not pulled, the future
draining the job's events keeps draining them, so the copy runs to its end,
its failures are still reported and both panes still reload. It is the
focused button rather than Cancel, for the reason the delete dialog opens on
Cancel — Enter is the key everyone reaches for, and it must never be the one
that stops a copy halfway.

Testing that took two attempts, and the first one turned `main` red. A
progress window exists only while a job outlasts `PROGRESS_DELAY`, so the
first version made the job slow with half a gigabyte of bytes — and the
fixture writes that file immediately before the copy reads it, so it is in
the page cache and the copy runs at memory speed. The runner did it in under
300 ms, no window appeared, and the test failed on a machine faster than the
one the number was chosen on.

Bytes were the wrong lever. The test now uses two that are not
hardware-sensitive: a **collision**, which stops the engine until the test
answers and so guarantees the app's own clock passes the threshold, and
**twenty thousand small files** behind it, so the window is still open when
the test reaches for it — a copy of many small files is bound by syscalls per
file, which varies far less between machines than throughput does. Twenty
files opened the window and closed it again inside a microsecond, which is
how the second lever earned its place.

What is still missing is the way *back*: a window listing what is running, so
a job put in the background can be watched again or cancelled later. Until
then, backgrounding is one-way
([future-improvements.md](future-improvements.md)).

The arithmetic behind the bar is in `progress.rs` and is pure: `Meter` folds
the event stream into a fraction, a caption and the current path. `Advanced`
carries a delta, so summing it is the window's job, not the engine's. **How a
number is written** — byte counts, a time left, the failure lines — is
`format.rs`, because two of its three callers want no meter at all.

**Failures are shown once, at the end**, after the panes have been reloaded —
not one dialog per file while the job is still running. A long list is cut off
with a count, because four hundred identical permission errors are not
information. The list grows with its content and stops at a screenful; a
minimum height instead opened a window mostly full of empty space to report a
single failure, which is what the first version did.

**Both panes reload when a job finishes.** A copy changed the target side, a
move changed both, and a delete may have removed the directory a pane was
standing in — which is why the reload goes through `Listing::load_nearest`
rather than a plain reload. See [listing.md](listing.md).

## Where the code lives

Four phases each added a `start_*` function to one file and a window to
another, which is how `main.rs` came to hold the shell's state, every action
and the window at once. It is three things now, and each file's name says
which:

| | |
|---|---|
| `main.rs` | the window: what is built at startup, and how a keystroke reaches an action |
| `shell.rs` | what the program knows while it runs — the settings, the queue, the listings in flight, the watches — and the machinery every action shares |
| `actions.rs` | what each key *does*, and the one table mapping an `Action` onto it |
| `dialogs/` | the shell every window is built from, plus a file each for the five with state of their own: the progress bar, the viewer, the search, the multi-rename, the favourites |

`Shell`'s fields are `pub(crate)` rather than private, which is the honest
shape: two panes, a queue, a keymap and the settings are one object the crate
cooperates on, not four pretending not to know about each other.

## Where the logic lives

Three things in this crate are real logic, and all three are pure,
GTK-widget-free and unit-tested: turning an `Entry` into the four column
strings (`row.rs`), where a navigation keystroke leads (`navigation.rs`), and
what the text typed into a dialog asks for (`jobs.rs`). Everything else is
widget assembly, verified by running the program.

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

**Exactly once, in `dispatch`.** The adoption used to be written out again in
four handlers and six `PaneView` methods, all of them reached only *through*
`dispatch` and therefore all of them no-ops — no main-loop turn runs in
between — which left nobody able to say which call was the load-bearing one.
The rule now lives with the method: the active pane's selection is adopted
once per dispatched action, before the action runs. The one other caller is
the pane exchange, and it adopts the **other** pane — a click gives a pane's
widget the focus and a selection of its own without making it active, so that
one is not the same call.

Every mark operation goes through `PaneView::marking`, which repaints the rows
that changed. That is structure rather than discipline: a mark operation that
forgets the repaint does not fail, it silently changes nothing on screen —
which this project has already shipped once.

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

Both panes open where they were left, from the `[[panes]]` entries in
[config.md](config.md); a pane with nothing remembered — a first run — opens
at the user's home directory (`LocalFs::home_dir()`).

A remembered directory that cannot be read opens the **nearest ancestor that
can** rather than failing the window — `Listing::load_nearest`, the same
fallback a finished job uses ([listing.md](listing.md)). A pane that opened
showing an error would strand the user on the one screen where they have not
done anything yet. That is startup only: a directory that cannot be entered
*while navigating* leaves the pane where it is and puts the reason in the path
bar ([keymap.md](keymap.md)), because there the pane has somewhere to stay.

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

## The title names the build

The window is called `FerroCommander #527 (ef8b326)` — the product, the build
number, and the commit it was built from. The title bar is the one part of the
window that survives into a screenshot or a bug report, and "which build were
you running" is the first question either raises.

Both values are stamped in at **build time** by `crates/tc-app/build.rs`,
which asks git for them: the build number is the commit count, so it goes up
and two builds can be told apart at a glance, and the hash is short enough to
read off a title bar and long enough to find the commit. Read from git rather
than kept in a file, because a number somebody has to remember to bump is a
number that stops describing the binary.

**`APP_TITLE` is a `&'static str` constant**, not a function. The build script
hands over the finished tail (` #527 (ef8b326)`) through one environment
variable and `concat!` joins it to the product name at compile time. The
complete string lands in the binary's rodata as one literal, and nothing
assembles it when the app runs.

The name is a `macro_rules!` returning a literal, because `concat!` takes
literals — that keeps its spelling in one place rather than once as a constant
and once inside the concatenation.

Nothing is written into the source tree. A build script that edited
`constants.rs` would leave the working tree dirty after every build and put a
generated value under version control, where the next commit either carries a
stale number or a real one that is wrong the moment anything else is
committed.

**A missing git is not a build failure.** Building from a source tarball, or
in an image without git, leaves the tail empty and the title is the bare
product name — not a placeholder like `#0 (unknown)`, which looks like a build
that exists when the point of the title is that it names one that does.

**That case costs no code.** Both git calls need `HEAD` to resolve, so they
answer or fail together: there is no half-stamped build to describe, and when
they fail `concat!` joins an empty tail and yields the product name on its
own. An earlier version spelled the rule out in a shared file that `build.rs`
pulled in with `include!` so the crate could unit-test it — for a branch that
cannot be reached and one that happens by itself. It was deleted.

What is left is a test asserting *this* binary was stamped, which is the half
that actually breaks. The empty case is checked by hand instead, and more
honestly than a unit test could: building with `git` replaced by a failing
stub emits an empty `TC_BUILD_STAMP` and produces a binary whose title carries
no `#`.

The build script rebuilds on a commit and not otherwise: it watches
`.git/HEAD` and the branch file HEAD points at. Watching `.git/index` instead
would rebuild after every `git add`, once per staged file, for a value that
has not moved.

**The application id is unchanged** (`st.rose.Ferrocommander`). It is the
D-Bus identity GTK uses for single-instance handoff and desktop integration,
nobody sees it, and renaming identity strings costs more than the tidiness is
worth. The settings directory (`ferrocommander`, lower case) is likewise its
own name and stays put.

## The drive selector is a window, not a popover

`Alt+F1`/`Alt+F2` open a modal window listing the mount points, where Total
Commander drops a list down under its drive button. The reason is testability:
a GTK popover is not an X window the end-to-end harness can find by name or
send key events to, and a chooser that can only be driven by hand is one that
ships broken. Every other chooser in this shell is already a modal window, so
this costs no new pattern either.

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
DBUS_SESSION_BUS_ADDRESS="unix:path=/nope" ./target/release/ferrocommander
```

Registration fails, the app falls back to a private instance, and the only
output is one harmless "Unable to acquire session bus" warning.

**A quiet run is not a passing run.** Where the code ignores a `Result` —
`activate_action` and friends — a failure prints nothing at all. A first
attempt at scroll-to-cursor was a silent no-op for exactly this reason, and
the clean smoke run said nothing. When verifying a GTK call that returns a
`Result` the code discards, print the result once and look at it.

## End-to-end tests

`crates/tc-app/tests/ui.rs` starts a private X server, launches the built
binary on it, sends real key presses through the X server, and then asserts on
the filesystem. Nothing is mocked; the only thing standing in for a person is
`xdotool`.

They exist because this is the layer where every bug in this project has
lived. Phase 1 shipped three past a green suite — the cursor landing on `..`
after stepping up, a stale selection after Page Down, a cursor that walked off
screen — and phase 2 added two more that only appeared when the program ran: a
conflict dialog with no focused button, and a symlink that aborted a whole
copy. None of those were reachable from a unit test.

**Needs `xvfb` and `xdotool`**, and a missing tool fails the test with a
message naming the package rather than skipping. A test that quietly does not
run is worse than no test.

**The GTK backend is pinned to X11, not just the display.** GTK picks its
backend from the environment and prefers Wayland whenever `WAYLAND_DISPLAY` is
set — so on any Wayland desktop the app under test connected to the
developer's *real compositor* instead of the `Xvfb` the harness had just
started. The window appeared, on a display `xdotool` cannot see, and all 136
tests waited out their full thirty-second timeout: an hour-long run failing
for a reason that has nothing to do with the program. Reported from a stock
Ubuntu 24.04 desktop, where it is 100% reproducible and looks like a hang.

Choosing the display and leaving the backend to the ambient environment was
never coherent. `Xvfb` and `xdotool` are X11-only, so this suite has no
meaning under Wayland at all, and `spawn_app` now pins `GDK_BACKEND` beside
`DISPLAY`. It is checkable on an X11 machine too: run the suite with
`GDK_BACKEND=wayland` in the environment and, without the pin, every test
fails the same way.

**A scroll offset is the one thing this suite cannot see.** It is not a
window title, a file on disk or a key press, so `xdotool` has no way to read
it — which is why the per-directory scroll memory is checked by
`scripts/check-scroll-memory.sh` instead: it drives the real app and leaves
screenshots. The case it sets up is the one the *cursor* cannot explain, with
the cursor left on the first row and the view wheeled far down, because
returning to a cursor at the top would show the top whether the memory works
or not.

**The per-user directories are pinned too, not just `HOME`.** The app follows
the freedesktop rule — `$XDG_CONFIG_HOME`, or `~/.config` when it is unset —
so an ambient `XDG_CONFIG_HOME` beats the private home and the settings file
is written on the machine running the tests. Nothing in the harness notices:
the app starts, the window appears, keys arrive, and only the *assertions*
fail, because the file the tests poll for a pane's directory never appears.
Every such test then waits out its ten seconds and reports the panes as
`["", ""]`.

That is what a GitHub runner did to all 138 tests at once, at 11.3 seconds
each — half an hour of red saying nothing about the program. It is
reproducible anywhere in one command: with `XDG_CONFIG_HOME` set, a test that
passes in 3.0 s fails in 11.4 s with exactly that message.

`XDG_DATA_HOME` is pinned beside it, and that one is worse than a failing
test: the `trash` crate files deletions under it, so an `F8` test on an
unpinned machine puts fixture files in the real user's wastebasket.

Deciding the home and then leaving the variables that *override* the home to
the ambient environment was never coherent — the same mistake as `DISPLAY`
without `GDK_BACKEND`, and found the same way.

**The drive tests ask which row they want; they never count on one.** They
used to index the list positionally — slot 0 for "the drive the pane is
already on", slot 1 for "somewhere else" — which is true of one machine and
of nothing else: the list is `/proc/self/mounts` order. A report from a stock
Ubuntu desktop had a different mount first, and five tests went to the drive
the pane was already on, which a drive that remembers where it was left makes
invisible. They now find the mount the temp home actually sits on
(`mount_for`) and pick a different one. Checkable anywhere, by reversing what
`mount_points` returns and re-running them.

**One app at a time.** Cargo would run them in parallel, and sixteen X servers
with sixteen GTK apps between them do not fit comfortably in a container: the
suite went from all-green to eight failures and back between runs, always with
apps dying at startup on a display that had just answered. A suite that fails
randomly teaches people to ignore red, so a mutex makes them queue.

The cost is the suite's whole runtime: **160 tests × about 3.3 s each,
measured at 527 s here on 2026-08-30**. It was "about half a minute" when that
sentence was written and the suite had a handful of tests; nobody updated it
as the suite grew, and an outside reader measured 530 s for 138 tests before
we did.

**Dating the measurement did not keep it true.** The 138 above stood while the
suite reached 160, in this file and in two others, each with a different
number — which is the whole of what a documentation review is for. The
per-test cost is the figure that holds across all of them: 3.20 s at 138,
3.30 s at 160. Multiply it by whatever `cargo test -p tc-app --test ui`
reports today rather than trusting the total here.

Four things the harness learned the hard way, each now a check rather than a
sleep:

- **Wait for the display to be *usable*, not just present.** Xvfb accepts a
  connection slightly before its screen is ready, and an app that connects in
  that window dies with "Failed to open display". The probe requires the
  geometry to come back at the size that was asked for, which cannot pass
  early — and, because that probe is *still* not quite enough, a launch that
  dies with exactly that message is retried up to three times. Four of 104
  tests failed that way in one run and none in the next, which is the shape of
  an environment problem and not of anything this suite is about. The retry is
  bounded and matched on the message, so a real crash at startup still fails
  on the first attempt with its own log attached; a sleep long enough to
  always work would be a minute added to every run.
- **Asking for the focus is not getting it.** Without confirming the focus
  landed, a key press reaches the window that *used* to have it — which is how
  a directory name typed into a dialog ended up in the main window, where
  every letter is unbound and silently does nothing.
- **A resize request can be swallowed.** Under a bare Xvfb there is no window
  manager to hold one, and about one launch in five lost it outright — the app
  then never saw a size change, and the test spent both its timeouts waiting
  for one. `resize` re-sends until X agrees the window is the size that was
  asked for.
- **A window closes on GTK's schedule, not on the keystroke's.** Checking that
  a dialog is gone the instant after dismissing it is a race the test loses
  about a third of the time; `await_dialog_closed` polls instead.

Each test was checked against its own bug: unbinding F7, giving the conflict
dialog no focused button, and letting operations act on the `..` row each turn
the matching test red.

*A caution from writing them.* The claim that a dialog needed a capture-phase
key controller for Escape to work turned out to be false — removing the phase
left every test green, so the line went and the comment with it. A test suite
is also how you find out which of your explanations were guesses.
