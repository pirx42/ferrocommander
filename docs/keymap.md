# Keymap — what each key does

← Parent: [CLAUDE.md](CLAUDE.md)

## Bindings

| Key | Action |
|---|---|
| `Tab` | Switch to the other pane |
| `↑` / `↓` | Move the cursor one row |
| `Home` / `End` | Move the cursor to the first / last row |
| `PgUp` / `PgDn` | Move the cursor one screenful, less a row of overlap |
| `Enter`, `Num Enter` | Enter the directory under the cursor, or open the file with the desktop's handler |
| `Backspace` | Leave the current directory |
| `F3` | Look inside the file under the cursor — see [viewer.md](viewer.md) |
| `F4` | Hand it to the editor |
| `F5` | Copy what is marked |
| `F6` | Move what is marked, or rename it |
| `Shift+F6` | Rename the row under the cursor, in the list itself |
| `F7` | Create a directory |
| `Shift+F4` | Create a file and open it in the editor |
| `F8`, `Delete` | Delete what is marked, to the trash |
| `Shift+F8`, `Shift+Delete` | The same, permanently |
| `Space` | Mark the row under the cursor — and count it, if it is a folder |
| `Insert`, `Shift+↓` | Mark it and step down |
| `Shift+↑` | Mark it and step up |
| `Shift+Home` / `Shift+End` | Mark from the cursor to the first / last row |
| `Shift+PgUp` / `Shift+PgDn` | The same, over one page |
| `Num +` | Mark everything matching a wildcard |
| `Num −` | Unmark everything matching a wildcard |
| `Num *` | Invert the marks on the **files** |
| `Shift+Num *` | Invert them on the directories too |
| `Alt+Num +` / `Alt+Num −` | Mark / unmark every file with the cursor row's extension |
| `Num /` | The selection from before the last operation |
| `Ctrl+A`, `Ctrl+Num +` | Mark everything visible |
| `Ctrl+Num −` | Unmark everything visible |
| `Ctrl+↓`, `Alt+F8` | The command history — see [command-line.md](command-line.md) |
| a letter, digit or symbol | Move the cursor to the next row whose name contains it |
| `→` | Put the keyboard in the command line |
| `Ctrl+C` / `Ctrl+X` | Put what is marked on the system clipboard, to copy or to move |
| `Ctrl+V` | Copy or move what is on the clipboard into this pane |
| `Ctrl+Enter`, `Ctrl+Num Enter` | Put the name under the cursor into the command line |
| `Alt+F7` | Find files below the active pane — see [search.md](search.md) |
| `Alt+F5` | Pack what is marked into a new archive, beside the other pane — see [archives.md](archives.md) |
| `Ctrl+M` | Rename what is marked by a rule — see [multi-rename.md](multi-rename.md) |
| `Ctrl+Z` | Put the last multi-rename back |
| `Alt+Shift+Enter`, `Alt+Shift+Num Enter` | Count what the marked folders hold, recursively |
| `Ctrl+B` | Every file below this pane, as one flat list |
| `Ctrl+D` | The favourite directories, to pick one and go there |
| `Alt+F1` / `Alt+F2` | Send the left / right pane to a drive |
| `Ctrl+→` / `Ctrl+←` | Show the active pane's directory in the right / left pane |
| `Ctrl+U` | Exchange the two panes |
| `Ctrl+R` | Re-read the directory — see [watching.md](watching.md) |
| `Ctrl+F3` / `Ctrl+F4` / `Ctrl+F5` / `Ctrl+F6` | Sort by name / ext / date / size |
| `Ctrl+H` | Show or hide the dot-files |
| `Ctrl+Shift+C` | Compare two files by content — see [compare.md](compare.md) |
| `Ctrl+Q` | Quick view: the other pane shows what the cursor is on — see [viewer.md](viewer.md) |
| `Ctrl+S` | Narrow the pane as you type |
| `Esc` | Stop a running branch walk; otherwise stop narrowing |
| `Alt+F4` | Quit — on macOS `Cmd+Q` does it too |

**This table is checked against the code, and its punctuation is what does
the checking.** A comma joins keys that are **one command** reachable more
than one way — `` `F8`, `Delete` ``; a slash separates the **two things a row
is about** — `` `↑` / `↓` ``. Two tests read that:
`the_bindings_table_names_exactly_the_keys_that_are_bound` fails when a
binding is added, removed or moved without this table following, and
`keys_a_row_joins_with_a_comma_are_the_same_command` fails when a comma stops
being true. So the separator is not typography, and a new row picks the one
that says what it means.

The descriptions stay hand-written on purpose: what `Insert` *means* is not
derivable from the code, and a generated table would lose the rows that pair
two keys. The facts are checked; the prose is written. What **is** generated
is the [action-name table](#action-names--what-a-keys-line-may-say-on-the-right)
further down, which is a list rather than an explanation.

Activating a *file* hands it to the desktop's own handler — `xdg-open`, `open`
or, on Windows, the call Explorer makes for a double-click
([windows.md](windows.md)). The path is handed over as one argument and no
command line is built, which is the whole of why there is nothing to quote
wrong. `F3` still views it and `F4` still edits it with the editor from the
settings ([viewer.md](viewer.md)): Enter is "open this", `F4` is "edit this",
and they stay different questions even when one program answers both.

**Enter never executes the file itself**, whatever its permission bits say —
it hands it over, and what the handler makes of it is the handler's answer.
On Windows that answer starts a `.exe`, because that is what a double-click
does there and what was asked for; `xdg-open` opens a script in an editor.
The rule kept here is only that the decision belongs to the same handler the
rest of the system uses, and not to a rule invented in a file manager.

**An archive** is walked into as if it were a directory, and `..` or Backspace
comes back out ([archives.md](archives.md)). Inside an archive `F3` still
works, while `F4` and Enter both say why they cannot: a handler is given an
operating-system path and an entry in an archive has none, so handing over
what the archive calls it would open something of that name on the disk.

**Two keys for each delete**, because Total Commander has both and muscle
memory splits evenly between them. Shift is the only place in the keymap where
a modifier changes what survives, so it has a test of its own rather than
riding on the binding table.

## Marks decide what an operation acts on

F5–F8 act on **everything marked**, and fall back to the row under the cursor
when nothing is marked. That fallback is what keeps marks optional rather than
a mode: press F5 on a file and it copies, mark ten and it copies ten, and
there is no third thing to learn.

`Insert` steps down after marking so it can be held, which is how a run of
files gets selected; `Space` leaves the cursor where it is, for picking one
out of a list.

**`Space` also counts a folder as it marks it**, and the marking keys do not.
That split is Total Commander's and it follows from the one above: `Insert` is
the key you *hold*, and a walk started per row while somebody sweeps a list of
folders is work nobody asked for. `Space` is the deliberate one.

The reason it counts at all is the status line. Its marked-bytes half is the
number a person checks before pressing F5, and a directory's size is zero
until something counts it ([vfs.md](vfs.md)) — so marking three folders used
to report `0 B` for them, and the one question the line exists to answer was
the one it got wrong. Counting on the way in is what makes the total true.

It is the same scan `Alt+Shift+Enter` runs, with the same worker, the same
streaming answer per folder, the same `+` for a count that was cut short and
the same `Escape` to stop the rest. What differs is only which folders are
asked for: `Alt+Shift+Enter` re-counts everything marked, and `Space` asks for
the folders *it* marked that have not answered yet. A folder already counted
is left alone — its number stands until a re-read forgets it, and
`Alt+Shift+Enter` is still how you ask for a fresh one.

**`Shift`+cursor is the same behaviour under another key.** Total Commander
marks the row being *left* and then moves, so `Shift+↓` is `Insert` by another
name — and running back over a row toggles it a second time and takes its mark
off again. That is TC's own quirk: it does not mark the row you land on, so
someone coming from Explorer overshoots by one. Reproducing it is the point.

**A jump marks a range instead of toggling.** `Shift+Home`/`End`/`PgUp`/`PgDn`
have no direction to run back over, and "to the end" does not mean "flip
everything I passed". `..` still cannot be marked, so a range running over it
simply does not pick it up.

**`Num *` inverts the files only; `Shift+Num *` takes the directories too.**
TC's split, and the useful default — a person inverting a selection is nearly
always thinking about files. `Alt+Num ±` follows the same rule: a directory
called `photos.backup` is not one of "the `.backup` files".

**`Num /` restores the selection from before the last operation.** The marks
are put away when a job is *submitted*, not when it finishes: the job ends by
building a fresh listing, and by then the marks it consumed are gone.

### Ordinary keys stand in for the keypad

`+ − * /` reach their keypad bindings through an alias table rather than
through bindings of their own, so a keyboard with no numeric block runs every
marking command, modifiers included: `Ctrl+−` is `Ctrl+Num −`, `Alt++` is
`Alt+Num +`.

**The Shift that produced the character is dropped.** On most layouts `+` is
`Shift+=` and `*` is `Shift+8`, so the keystroke arrives as `plus` or
`asterisk` *carrying* `SHIFT_MASK` and matched nothing at all — the `+` twin
shipped in phase 3 was dead on arrival, and only a real key press could say
so. A modifier needed to type a character is a fact about the keyboard, not
something the user meant.

The cost: the ordinary `*` cannot also carry a *deliberate* Shift, so
`Shift+Num *` is reachable from the keypad only. That is a limitation of
layouts on which `*` cannot be typed without Shift, not a choice, and a plain
`*` that did nothing would be worse.

**Both are a toggle, not a set**: pressing either again on a marked row takes
the mark back, which is the only way to unmark one row out of many. Tested at
both layers — `toggle_selected` twice in `fc-core`, and both keys twice
through the real binary.

Marking acts on what is **visible**: what the hidden-file flag or a filter is
holding back is not something the user can see to have meant. Details of the
model in [listing.md](listing.md).

**`..` is not something to operate on.** It is a navigation control, not an
entry, and copying or deleting "the parent directory" from inside it is never
what the user means. It cannot be marked, and with nothing else marked, F5–F8
on it do nothing. A mark elsewhere in the pane still counts — the `..` rule is
about the fallback, not about the marks.

## Counting what a folder holds

A directory row says `<DIR>` because nobody has counted it.
`Alt+Shift+Enter` counts it — the marked folders, or the one under the cursor
when nothing is marked, the rule every operation here follows. Marked **files**
need no counting: they already know their size, and they still count towards
the total.

**The answer goes in the size column**, which is Total Commander's behaviour
and is worth more than it looks. The number is the entry's own size from then
on, so the status line's marked-bytes total includes it — that total *is* the
accumulated answer — and sorting by size orders the folders properly, which
is the question that always follows.

**One folder at a time**, each appearing as its scan finishes rather than all
at the pace of the slowest, and `Escape` stops the rest. The sizes already
found stay: unlike a half-finished branch walk, each folder's number is its
own and complete.

**A partial count says so**, with a `+` after the number. It means the figure
is a **lower bound**: a subdirectory refused to be read, so its contents are
missing from the total. A size nobody can trust looking exactly like one they
can is the failure worth one character to avoid.

**A cancelled scan is not an answer**, and says nothing at all. It has a
number — every byte it managed to add up before the next keypress stopped it —
and that number is discarded rather than shown, in two places: the walk drops
it, and a listing refuses a lower bound for a folder whose size is already
known. The two guards are separate because they fail separately, and both are
needed: a walk cancelled a keystroke ago can already be past its own check and
on its way back with a number for a question nobody is asking.

Without them a folder's size changed every time it was asked for — the second
testing round's report, and the one place the `+` was appearing without an
unreadable subdirectory behind it.

**Nothing is re-sorted while the answers arrive**, and **a re-read forgets
them** ([listing.md](listing.md)). Pressing the key again counts afresh, which
is how a stale number is refreshed.

## The branch view

`Ctrl+B` fills the pane with every file below where it is — Total Commander's
branch view. What comes back is an **ordinary listing**, so the sort, the
quick filter, the marks and every file operation go on meaning what they
meant; the rows are files from several directories, each named by its path
relative to the root ([listing.md](listing.md)).

**The walk runs on a worker** and the pane keeps showing what it has until it
lands, exactly as it does for a slow directory read. **`Escape` stops it**,
and drops the partial answer with it — a tree half shown looks like a tree,
which is worse than not flattening at all. That is Escape's first job now; it
goes on clearing a filter when there is no walk to stop.

**Pressing `Ctrl+B` again walks again**, which is the re-read a branch view
has instead of `Ctrl+R`. **Leaving is a navigation**: any step — Enter,
Backspace, a favourite, a drive — lands an ordinary listing of somewhere, and
the flat rows go with it.

**The path bar says so**, with `/**` after the root. A pane showing a whole
tree while its path bar reads like one directory is a pane lying about what is
in it.

**Two keys mean something narrower here.** `Alt+F5` offers an archive named
after the file's own name rather than after its row — `inner.zip`, not
`nested/inner.zip`, which would name a directory the other pane need not
have. `Shift+F6` refuses a row whose name carries a directory, since an
inline rename edits a name and that row's name is a path; a row at the walk's
own root renames as it always did.

**A job done from a branch view leaves a branch view.** The panes re-read
after every job, and for this one that means walking again rather than
re-reading the root — otherwise a copy would silently flatten the view back
into one directory.

## The favourite directories

`Ctrl+D` opens the list of directories worth keeping and sends a pane to the
one chosen — Total Commander's directory hotlist. The list, and the order it
is in, live in `[[favourites]]` in the settings file
([config.md](config.md)); it is a menu, so the order is the one somebody put
things in rather than the alphabet's.

**The pane it moves is the one with the keyboard.** A key with no direction
and no number in it acts on the active pane, which is the ordinary rule here
and the opposite of `Alt+F1`/`Alt+F2` below. Both halves are pressed for real:
a test from the left pane and a test from the right, because an
implementation that always moved pane 0 would pass the first on its own.

**A favourite is always a path on the real filesystem**, so choosing one from
inside an archive comes *out* of it — the archive backend has never heard of
`/home/…`, and navigating on it would leave the pane inside showing an error.
That is `leave_for`, the same route `Alt+F1` takes out of an archive.

**The list is maintained from inside itself**, which is why `Ctrl+D` is the
only key this feature has. The last row keeps where the active pane is,
`Delete` removes the row under the cursor, and the window stays open through
both — so adding a directory and then going somewhere is one visit to the
list. Adding a directory that is already there does nothing, which is what
bounds the list instead of a cap.

**Where a pane is standing inside an archive is refused**, with the reason in
the window rather than in a second modal. A path in there belongs to that
archive's own store, where the same spelling means a completely different
file; the probe that removed this check kept `/` — the archive's own root — as
a favourite that would have sent the pane to the root of the disk on the next
run.

**A favourite whose directory is gone leaves the pane where it was**, with the
reason next to the path. Unlike a drive, a favourite has no mount to fall back
to, and sending the pane somewhere nobody named would be a worse answer than
staying.

## The drive selector

`Alt+F1` and `Alt+F2` open a list of mount points and send a pane to the one
chosen. The list opens focused on its first row, so the arrows walk it, Enter
takes it and Escape leaves without going anywhere — there is no Cancel button
to fall back on, which makes Escape load-bearing and it has a test.

**The F-key number *is* the pane number**, exactly as in Total Commander:
`Alt+F1` is the left pane whichever one has the keyboard. That is the opposite
rule to `Ctrl+←/→` below, and deliberately — an arrow has a direction to be
relative to, and a number does not. The test presses `Alt+F2` from the left
pane, since that is the only way to tell the two rules apart.

**A drive remembers the directory it was left in.** Switching away and back
is not a trip to the root and a walk down again — Total Commander's behaviour
with its default `AlwaysToRoot=0`. The memory is **shared between the panes**,
as it is there: leaving a drive in one pane is what the other finds when it
arrives, and whichever pane left a drive most recently is the one that decides.
It survives a restart, in the `[drives]` table of [config.md](config.md), and a
remembered directory that has since gone falls back to the drive itself rather
than leaving the pane showing an error about a path nobody asked for by name.

The drive-bar buttons go through the same code, so a drive remembers where it
was left however it was reached.

**What is *not* carried across**, because TC does not carry it either: the
marks are cleared and the cursor starts at the top. Leaving a directory drops
its selection, and coming back lands on the first row — the same in TC, which
is why this is a matching behaviour rather than a missing one.

**It is a modal window, where TC has a dropdown.** A GTK popover is not a
window the end-to-end suite can find or send keys to, and a drive selector
that cannot be tested through a real key press is the kind of thing that ships
broken. The drive bar's buttons still do the same job with the mouse.

## The two-pane commands

`Ctrl+←/→` are **relative to the active pane**, as in Total Commander: the
arrow points at the pane being *written*, and the active one supplies the
directory. Pressing an arrow toward the pane the keyboard is already in does
nothing, rather than guessing which of the two directions was meant.

"Nothing" is literal, and that is the part worth a test: sending a pane to the
directory it is already in looks identical from outside until you notice it
re-read it, which drops the marks and puts the cursor back at the top. The
end-to-end test marks a file, presses the self-pointing arrow, and then spends
the mark.

`Ctrl+U` exchanges the panes' **contents**, not their widgets — both are
children of a `Paned`, and reparenting them would be work for no reason. The
whole `Listing` is swapped, which is what makes the directory, the cursor and
the marks travel together; anything rebuilt field by field would quietly drop
one of them, and the test for it marks a file on one side and spends it on the
other. The keyboard stays in the same physical pane, now showing the other
side.

## Creating a file to edit

`Shift+F4` asks for a name, creates an **empty** file and hands it to the
editor — Total Commander's behaviour, including leaving the file empty: what an
editor makes of a zero-byte file is the editor's business.

Asked for rather than assumed, unlike TC's fixed `new.txt`: the name is the
first thing anybody changes, and a dialog they can accept with Enter costs
nothing. Which program opens it is the `editor` line in
[config.md](config.md).

**A name that is already taken is refused**, and that is the whole reason this
is a job rather than a bare VFS call: `create_file` truncates, so `Shift+F4` on
an existing name would empty the very file the user meant to open — and then
hand it to an editor, which would save the emptiness back.

**The editor starts only once the job reports success.** One opened on a file
that was never created shows an empty buffer that silently recreates it on
save, which is a worse answer than nothing.

## Renaming in the list

`Shift+F6` turns the name under the cursor into a field in the list itself,
rather than a dialog that covers the thing being renamed — Total Commander's
behaviour. Enter accepts, Escape abandons and hands the keyboard back.

**The field carries the whole filename**, extension and all. The name and ext
columns are a presentation split; renaming `notes` to `todo` while silently
keeping `.txt` in another column is not something the user can see to have
agreed to. The **stem is selected** when it opens, so typing replaces the name
and leaves the extension — the ordinary case, and retyping `.txt` every time
is the annoying one.

**A rename is a move whose destination is exact**, which is the same rule F6's
dialog follows, so it goes through the same queue and gets the same conflict
question when something is already called that. Accepting an unchanged name
does nothing at all: pressing Enter straight away is somebody deciding not to
rename, not a job to run.

The editable cell is a `Stack` per *recycled* cell, not per entry: a
`ColumnView` keeps widgets only for the rows on screen, so it is some forty
deep rather than fifty thousand ([performance.md](performance.md)).

**While a rename is open the cursor sync stops asking for the focus.** It
belongs to the editor, and the column view taking it back would close the
field under the user's fingers. That the rename worked at all before that was
ordering luck — a cell further down happens to bind and grab the focus after
the scroll, and the first row did not.

## What the target field means

F5 and F6 open the same dialog, prefilled with the *other* pane's directory
**and a trailing separator**. That slash is the rule, shown rather than
hidden:

| What is in the field | Where the entry lands |
|---|---|
| `/mnt/backup/` | into that directory, keeping its name |
| `/mnt/backup/holiday.txt` | at exactly that path |
| `holiday.txt` | beside the source, under that name |

The last row is the whole of "F6 renames" and "F5 duplicates a file" — no
separate rename command, and no dialog that has to guess. A rename *is* a move
whose destination is exact, which is why `ops` has no `Rename` job.

**That is not `Shift+F6`.** Both rename, and they are different keys for
different moments: `F6` with a bare name in its field is a rename *through the
dialog*, and `Shift+F6` edits the name **in place**, in the row itself, with no
dialog over the thing being renamed. The phrase "in place" belongs to
`Shift+F6`, and the table above used to lend it to `F6`.

## Page Up / Page Down were the widget's job until they were measured

They are in the table now. They were not, and the reason given was that
"paging depends on how many rows fit on screen, and the model has no idea how
tall the viewport is — the `ColumnView` does". That was true when it was
written and stopped being true the moment `Shift+PgUp`/`PgDn` needed the same
number: `PaneView::page_rows` measures the viewport off the scroll adjustment,
and the measurement the argument called impossible had been a method on the
type ever since. A decision whose reason had expired, left standing because
nobody went back to it.

What forced the revisit was not tidiness. The widget moves its *own* selection
and the model hears nothing until the next dispatched action adopts it — which
is fine for an action and wrong for type-ahead, the one route into a pane that
is not one. Paging and then typing searched from a row off the top of the
screen ([listing.md](listing.md)).

**The step is a screenful less one row, and that row was measured, not
chosen.** The `ColumnView`'s own paging scrolled thirteen rows where fourteen
fit, leaving the last row of one page as the first of the next — which is what
lets a reader place themselves after a jump. Reproducing it was the
requirement, so the numbers were taken from the running app before anything
was written: at a 39-pixel row and a 579-pixel viewport the widget settled at
offsets 0, 474, 981, 1488, and a thirteen-row step with the ordinary
scroll-into-view reproduces all four exactly. `PAGE_OVERLAP_ROWS` is that one
row.

It also settled a disagreement nobody had noticed: `Shift+PgDn` marked
fourteen rows where `PgDn` moved thirteen. Both now ask `page_step`, so one
place decides what a page is.

**Checked by eye, because nothing else can see it.**
`scripts/check-page-scroll.sh` drives the real app and leaves five
screenshots. What it showed: the first page moves the cursor thirteen rows and
does **not** scroll, because the row it lands on is already visible — which is
what the widget did too; the second page shows `row-013` at the top and
`row-026` at the bottom, so the row the cursor sat on becomes the first row of
the next page; and two pages down followed by two up returns the pane
**pixel-identical** to where it started. The only difference anywhere in that
last comparison is in the *other* pane, where GTK's overlay scrollbar was
caught mid-fade — a screenshot-timing artifact, not a behaviour.

The arithmetic is pinned by
`a_page_is_a_screenful_less_the_row_that_carries_over`, which carries the
measured numbers. The **appearance** is not pinned by anything: the end-to-end
suite asserts on the filesystem, and a down-and-back-up test cannot see the
overlap either, since both directions use the same step and cancel out. Said
here rather than left looking like coverage it is not.

**The pane measures a page itself.** The scrolled window's adjustment knows
the content height and the viewport height, and the rows are uniform, so the
row count falls out of the ratio; before the first layout there is no height
to divide by and a fallback constant stands in. That measurement is the one
part of this that could quietly be wrong, so it is pinned twice end to end: a
`Shift+PgDn` on a screen tall enough to hold every row has to mark all of
them, and a plain `PgDn` has to land among the two hundred rows without
running to the last of them — a step of the whole listing would pass a
down-and-back-up test, because both directions would clamp.

**The widget still moves a selection of its own**, on a mouse click — the one
move that is genuinely the user's and goes through no binding. The model hears
about it through the widget's own signal rather than through a call anybody
has to remember ([ui-shell.md](ui-shell.md)); without that the two drift apart
and the next Enter opens whatever row the cursor was on before the click.

The traffic runs the other way too: when the shell moves the cursor it also
moves the widget's *focus*, because anything the widget does handle on its own
starts from its focus. If focus did not follow, a click after some arrow keys
would leave the widget starting from wherever it last was rather than from the
cursor.

## The macOS layer

On macOS a second table sits between the defaults and the user's `[keys]`:
`MACOS_LAYER` in `keymap.rs`, **data in exactly the `[keys]` shape**, run
through the same parser — so the platform layer costs no second mechanism and
"what a line may say" cannot drift between the two. Precedence is platform
under person: a user's `[keys]` line beats the layer the way it beats any
default.

It is **additive and unjudged**: `Cmd+C/X/V/A/Z/Q/R` gain their universal
macOS meanings and every `Ctrl` binding keeps working. Whether parts of the
`Ctrl` table should *move* to Cmd is a taste question for somebody at a real
Mac — the layer being data is what makes that tuning an edit, not a build.
Tests apply the layer explicitly on Linux, so it is reviewable here: it must
parse, it must not take the Ctrl twins away, and a user override must beat it.

The bindings table above documents `BINDINGS` — the defaults every platform
shares — and its check counts exactly those; the layer is deliberately not in
it, being no platform's default but macOS's addition.

## Action names — what a `[keys]` line may say on the right

Rebinding a key is a line in the `[keys]` table of [config.md](config.md), and
the right-hand side has to be one of these. Until this table existed the only
way to find one was to read `keymap.rs`.

**How many there are is not written here**, which is deliberate: the plan that
produced this table carried the number twice and got it wrong once, and a
count beside a generated list is the one part of it that is not generated.

**The table below is generated** from `ACTION_NAMES` and `BINDINGS`, and a
test fails when it and the code disagree
(`the_action_name_table_in_the_docs_is_the_one_the_code_generates`). Editing
it by hand is editing something that will be overwritten with the truth — the
markers around it are load-bearing (skill
[53](skills/53-generate-instead-of-duplicating.md)).

The key spellings in the right-hand column are the ones a settings file may
use verbatim, and a second test presses that claim: every one of them is read
back through the same parser a `[keys]` line goes through, and has to yield
the keystroke it was written from. Case and modifier order do not matter when
*you* write one — `Ctrl+Shift+F5` and `shift+ctrl+f5` are the same line.

<!-- generated: action names -->
| Action name | Default keys |
|---|---|
| `switch_pane` | `Tab` |
| `cursor_up` | `Up` |
| `cursor_down` | `Down` |
| `cursor_first` | `Home` |
| `cursor_last` | `End` |
| `cursor_page_up` | `Page_Up` |
| `cursor_page_down` | `Page_Down` |
| `activate` | `Return`, `KP_Enter` |
| `go_parent` | `BackSpace` |
| `copy` | `F5` |
| `pack` | `alt+F5` |
| `move` | `F6` |
| `reread` | `ctrl+r` |
| `search` | `alt+F7` |
| `multi_rename` | `ctrl+m` |
| `undo_rename` | `ctrl+z` |
| `view` | `F3` |
| `edit` | `F4` |
| `create_file` | `shift+F4` |
| `rename_inline` | `shift+F6` |
| `create_dir` | `F7` |
| `delete` | `F8`, `Delete` |
| `delete_permanently` | `shift+F8`, `shift+Delete` |
| `toggle_mark` | `space` |
| `toggle_mark_and_advance` | `Insert`, `shift+Down` |
| `toggle_mark_and_retreat` | `shift+Up` |
| `extend_mark_to_first` | `shift+Home` |
| `extend_mark_to_last` | `shift+End` |
| `extend_mark_page_up` | `shift+Page_Up` |
| `extend_mark_page_down` | `shift+Page_Down` |
| `mark_by_pattern` | `KP_Add` |
| `unmark_by_pattern` | `KP_Subtract` |
| `invert_marks` | `KP_Multiply` |
| `invert_marks_including_folders` | `shift+KP_Multiply` |
| `mark_same_extension` | `alt+KP_Add` |
| `unmark_same_extension` | `alt+KP_Subtract` |
| `restore_marks` | `KP_Divide` |
| `mark_all` | `ctrl+KP_Add`, `ctrl+a` |
| `unmark_all` | `ctrl+KP_Subtract` |
| `quick_filter` | `ctrl+s` |
| `clear_filter` | `Escape` |
| `sort_by_name` | `ctrl+F3` |
| `sort_by_ext` | `ctrl+F4` |
| `sort_by_size` | `ctrl+F6` |
| `sort_by_date` | `ctrl+F5` |
| `command_history` | `ctrl+Down`, `alt+F8` |
| `focus_command_line` | `Right` |
| `clipboard_copy` | `ctrl+c` |
| `clipboard_cut` | `ctrl+x` |
| `clipboard_paste` | `ctrl+v` |
| `insert_name` | `ctrl+Return`, `ctrl+KP_Enter` |
| `favourites` | `ctrl+d` |
| `branch_view` | `ctrl+b` |
| `folder_sizes` | `shift+alt+Return`, `shift+alt+KP_Enter` |
| `select_drive_left` | `alt+F1` |
| `select_drive_right` | `alt+F2` |
| `clone_to_right` | `ctrl+Right` |
| `clone_to_left` | `ctrl+Left` |
| `exchange_panes` | `ctrl+u` |
| `toggle_hidden` | `ctrl+h` |
| `compare` | `ctrl+shift+c` |
| `quick_view` | `ctrl+q` |
| `quit` | `alt+F4` |

<!-- /generated: action names -->

An action with `—` has no default binding. It is still bindable, which is what
this table is for.

## One table, no key names in the widgets

All default bindings live in a single `BINDINGS` table in `keymap.rs`, and the
GTK controller knows no key names at all: it looks up an `Action` and
dispatches it. That is what makes the bindings testable without a display, and
it is what lets the user's own bindings be laid over the defaults in one place.

**Every binding above can be changed** — see the `[keys]` table in
[config.md](config.md). The overlay is built once at startup into a hash map,
so a lookup stays a hash of one key rather than a walk down a table a
configurable keymap would otherwise make arbitrarily long
([performance.md](performance.md)).

**Unlisted modifiers are masked out before the lookup.** GTK reports Caps
Lock, Num Lock and held mouse buttons alongside the real modifiers; without
masking, a user with Caps Lock on would find every key unbound. Ctrl, Shift,
Alt and — written `cmd` in a `[keys]` line — the Command/META key take part in
a binding; `cmd` exists for the macOS layer, and nothing binds it by default
anywhere.

**A bound key with the wrong modifier does nothing.** `Ctrl+↓` does not fall
through to plain `↓`: it means the command history
([command-line.md](command-line.md)), which a binding that silently ignored
its modifiers would have made impossible.

## Sorting

`Ctrl+F3`…`Ctrl+F6` are Total Commander's sort keys, in its order: name,
extension, date, size. **The same key again flips the direction**; a different
one starts ascending. That rule is what makes one key mean both "sort by this"
and "the other way round", and it is a pure function on `Sort` with its own
test rather than something the widget layer decides.

Note that `F5` copies and `Ctrl+F5` sorts by date. The keymap's rule that a
bound key with the wrong modifier does nothing is what keeps both possible;
there is a test for exactly that pair.

The header of the active column carries ▲ or ▼. The marker is put into the
header *text*, because this shell sorts in the model and there is no GTK
sorter whose arrow GTK would draw for us.

**The ordering belongs to the pane, not to the directory.** Every navigation
and every finished job builds a fresh listing, so the pane re-applies its own
sort, hidden-file flag and filter to whatever it just read. Without that,
sorting by size and then copying one file quietly put the order back to name.

## The quick filter takes the keyboard back

`Ctrl+S` opens a field above the rows; typing narrows the pane live; `Esc`
clears it and hides the field; `Enter` keeps the narrowed view and hands the
keyboard back to the rows.

While that field has the focus the shell **stands down**. Its controller sits
in the capture phase so the column view cannot swallow Tab and the arrows, and
that puts it ahead of the field too — every letter would become a command and
`Enter` would open a directory instead of accepting the filter. So the
controller checks whether the focus is inside a text widget and stands down if
it is.

The rule has two exceptions and they are the command line's, not this field's:
the shortcuts meant for use *while typing a command*, and — the other way
round — the shortcuts a text field owns, which the keymap may not take however
it binds them. Both are in [command-line.md](command-line.md), which is the
one place that rule is stated in full.

The field's own handler is *also* in the capture phase, and for a different
reason: `GtkText` consumes `Return` to emit its own activate signal, so a
bubble-phase handler never sees it and the keyboard stays trapped in the
field. `Esc` arrives either way, which is why only half of it looked wired up
until a test pressed `Enter`.

**A filter belongs to the directory it was typed in** and is dropped on
navigation. Carrying it into the next directory would show an empty pane and
no reason why.

**`..` survives every filter**, so a filter that matches nothing can still be
left.

## Capture phase

The controller sits on the window in the **capture** phase, so the window sees
a keystroke before the `ColumnView` does. Otherwise the column view's built-in
focus and selection handling would consume `Tab` and the arrow keys and fight
the pane cursor.

## Where a keystroke leads

`navigation.rs` answers that as pure functions over a `Listing`, so the
decisions are testable without a window:

- `activation_step` — what Enter on the cursor row means, or `None` when it
  means nothing. Three answers rather than a path: `Into` a directory on the
  backend the pane already has, `Enter` an archive, which needs a backend of
  its own opened over it, or `Out` of one ([archives.md](archives.md)). An
  ordinary file is `None` — opening one is F3 or F4. The `..` row needs no
  special case: it is a directory like any other, and `VfsPath` normalization
  makes its target the parent.
- `parent_target` — where `Backspace` leads, `None` at the root.
- `focus_after_move` — which entry the cursor lands on afterwards.

**Stepping up lands on the directory you just left**, not on `..`. Someone
who pressed Backspace is looking for where they were; dropping them at the
top of the list makes them hunt for it again, and makes walking a tree
several levels deep genuinely tedious. Every other move — descending, or
jumping somewhere unrelated — starts at the top, because there is no previous
position to restore.

If the directory just left is hidden and hidden entries are not shown, the
cursor stays at the top: the cursor cannot sit on a row that is not there.

## When a directory cannot be entered

The pane **stays where it is** and shows the reason next to the path:

```
/home/pirx   ⚠ locked: permission denied
```

A pane that cannot read a directory must not display it as empty — that would
strand the user somewhere that looks browsable but is not. Leaving them where
they were, with the reason visible, keeps the pane navigable. The notice
clears on the next successful navigation.

## Testing

The keymap table and the navigation targets are unit-tested headlessly,
including the cases that are easy to get wrong: an unbound key, a bound key
with the wrong modifier, and irrelevant modifiers that must not break a
binding.

What the text in the target field means is unit-tested too, in `jobs.rs`,
including that the prefilled value round-trips back to "into that directory".

The wiring *between* a physical keypress and those functions used to have no
automated coverage at all. It is now covered by the end-to-end suite in
`crates/fc-app/tests/ui.rs`, where a real X server delivers real key events to
the real binary and the checks are on the filesystem afterwards. The cursor
keys and Tab are used by every test to get anywhere at all, and `Alt+F4` is
how the harness closes the app — which is also the only coverage the quit
binding has, and the reason moving that key moved a line eleven tests lean
on.

**What is deliberately not exercised end to end** is no longer a paragraph.
It is `UI_UNPRESSED` in `constants.rs`, eight entries with a reason each, and
`every_binding_is_pressed_end_to_end_or_says_why_not` holds it: a binding
nothing presses and nothing excuses fails the gate, and so does an excuse for
a key that has since been given a test. Two reasons stand behind the eight:

- `Ctrl+F3`/`F4`/`F5` — sorting is covered headlessly, and `Ctrl+F6` is
  pressed for the wiring, so a real window would say nothing the unit tests do
  not.
- The **aliases** — `Num Enter`, `Ctrl+Num Enter`, `Alt+Shift+Num Enter`,
  `Ctrl+Num +`, `Shift+F8`. Each resolves to the same action as a key that
  *is* pressed for real, so pressing it too would only test the lookup, which
  the keymap's own tests cover exhaustively.

**It was a paragraph until 2026-08-30, and the paragraph was wrong.** It
listed `Backspace`, which three tests press; it claimed the second drive key
was skipped when `Alt+F2` is the one pressed; and it did not mention `Num −`,
which no test pressed at all. That one read as covered because its twin
`Num +` has a test and `Ctrl+Num −` has three — and nothing was counting.
`Num −` has a test now; the counting is the part that lasts.

**A pressed key is not a proven key.** The check says a binding reaches the
program in some test, not that its own behaviour is asserted there — `Tab` and
the cursor keys are pressed by nearly every test just to get somewhere. It is
the half that can be checked mechanically, and the half whose absence is
otherwise silent.

Everything a physical press could get wrong on its own — a binding that
matches nothing, a modifier that arrives uninvited, a dialog with no focused
button, an action reaching the wrong pane — is pressed for real. That is not
theoretical: the `+` binding shipped in phase 3 was dead on arrival, and only
a real key press found it.

It is part of `cargo test --workspace`, so touching the controller, the keymap
or a dialog cannot silently break them. How it works and what it needs
installed: [ui-shell.md](ui-shell.md).
