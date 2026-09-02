# The context menu — the right button, and what the platform puts in it

Status: **In progress**, 2026-09-01 — phases 1–3 done, phase 4 next.

> display the context menu like in windows explorer when click with right
> mouse button on an item (folder or file)

Three answers shaped this plan (2026-09-01): **the platform's native shell
menu**, **Total Commander's right button** (it marks; the menu is a separate
gesture), and **yes to a background menu** on the empty part of a pane.

## 1. Where the program is today

There is no mouse in this program beyond what GTK does on its own. No
`GestureClick` anywhere in `fc-app`; no menu, popover or `gio::Menu` either.
A left click reaches the pane only because `SingleSelection` changes and
`main.rs:396` adopts it (`adopt_selection`). Everything else is the keymap.
So the right button, a menu, and the idea of "the row under the pointer"
are all new — and the pointer-side of the end-to-end harness is not:
`harness::click` drives `xdotool`, and `mousedown`/`mouseup` with button 3
is the same two lines with a different number.

## 2. What "native" means on each platform, honestly

The request is Explorer's menu. Only Windows *has* one.

| | What the platform offers | What this plan does |
|---|---|---|
| Windows | `IShellFolder` → `IContextMenu` — the real Explorer menu: verbs, *Open with*, *Send to*, *Properties*, and every shell extension the machine has (7-Zip, TortoiseGit, …) | **Use it.** Built through COM, shown with `TrackPopupMenuEx`, invoked through `InvokeCommand`. Phase 5 |
| Linux | nothing. Each desktop's file manager draws its own menu and no desktop exposes it. What freedesktop *does* have is the application registry: `gio::AppInfo::all_for_type` lists what is registered for a content type, which is exactly *Open with* | Our own menu from the action table, with an **Open with ▸** submenu from `gio` — the one native piece there is |
| macOS | no public API for Finder's contextual menu either. `gio` on macOS (`GOsxAppInfo`) reads Launch Services, so the same *Open with* list comes from the same call | The same menu as Linux, and the same `gio` submenu. Quick Look, Services and Tags are Finder's and stay there |

So "native" is: Explorer's menu where there is one, and on the other two a
menu of this program's own commands with the platform's own *Open with*
list folded in. That is more native than Total Commander manages on its
one platform, and it is what can be built without inventing a second
Finder.

## 3. Decisions

1. **The right button marks; a long press opens the menu; `Shift+F10` and
   the `Menu` key open it too.** Total Commander's default mouse mode: a
   right click toggles the mark on the row under the pointer *and moves the
   cursor there*; holding the right button (TC: about a second) opens the
   context menu on that row. The owner chose TC's button over Explorer's,
   and the long press is part of what TC's button does — it is how a
   right-button-marks user reaches the menu with the mouse at all. The
   cursor-moves half was confirmed by the owner against a running copy
   (2026-09-01); the long press is from memory.
2. **The menu acts on the marked set when the row is marked, and on the
   row alone when it is not.** Explorer's rule and TC's; it is what makes
   "mark five, right-hold, delete" work, and it means a stray right-hold on
   an unmarked row cannot delete the five you marked elsewhere. Marks are
   never changed by opening the menu.
3. **In an archive there is no shell menu** — an entry in an archive has no
   operating-system path, the rule `Enter` and `F4` already follow
   ([archives.md](../archives.md)). The GTK menu is shown instead, with
   the actions that work there (view, copy out, unpack); on Windows too.
4. **In branch view (`Ctrl+B`) the Windows menu is the row's alone.**
   `IShellFolder::GetUIObjectOf` takes children of *one* folder, and a
   branch view's marked rows live in several. The row under the pointer,
   through its own parent folder, is always answerable; a marked set across
   directories is not, and pretending with desktop-relative PIDLs is
   documented as "works for many verbs", which is not a guarantee this
   program hands a delete to.
5. **The Windows half is a `windows`-crate dependency, not a hand-declared
   extern.** `GetDiskFreeSpaceExW` and `ShellExecuteW` were one entry point
   each; this is `IShellFolder`, `IContextMenu` (and `2`/`3` for the
   owner-drawn *Send to* and *Open with* submenus), `IMalloc`, PIDL walking,
   an `HMENU`, a helper `HWND` with a window procedure, `TrackPopupMenuEx`
   — a vtable hierarchy, not a function. The `windows` crate (0.56) is
   **already in the tree** as `trash`'s dependency, so this costs no second
   copy and no new build; it becomes a direct, Windows-only dependency of
   `fc-app` with the `Win32_UI_Shell`, `Win32_UI_WindowsAndMessaging` and
   `Win32_System_Com` features. `gdk4-win32` (0.11, part of gtk4-rs) gives
   the toplevel's `HWND`, needed to place a keyboard-invoked menu at the
   row and to hand focus back afterwards.
6. **The menu owns its own window on Windows.** A helper `HWND` with a
   window procedure of ours is the popup's owner, so `WM_INITMENUPOPUP`,
   `WM_MEASUREITEM` and `WM_DRAWITEM` can be forwarded to `IContextMenu3`
   without subclassing GTK's window. Standard tray-icon technique; the
   alternative — `SetWindowSubclass` on a window GTK owns — reaches into a
   toolkit's window for the duration of a popup, and this program has one
   rule about GTK's internals already (the anchor, `performance.md`).
7. **The popup blocks the GTK loop while it is up, on purpose.**
   `TrackPopupMenuEx` is modal and pumps its own messages; running it on the
   UI thread means the panes do not repaint for the second the menu is
   open, which is what Explorer's own window does. A second thread would
   need foreground-window games to make the menu dismiss correctly, and
   those are the part of Win32 that differs between Windows versions.
8. **The background menu is small and is where Paste lives:** Paste,
   New folder (`F7`), Refresh (`Ctrl+R`), Show hidden (`Ctrl+H`),
   Sort by ▸. On Windows the *folder's* shell menu (`GetUIObjectOf` on the
   folder itself, or `IShellView`'s background verbs) is what Explorer
   shows there — *New ▸*, *Paste*, *Properties*. Same rule as rows: shell
   menu where there is one.
9. **Menu entries are the action table, not a second list.** Every row of
   the GTK menu is an `Action` from `keymap.rs`, labelled with its
   description and its bound key. So the menu cannot name a command that
   does not exist, its shortcuts follow the user's `[keys]`, and the gate
   that counts which bindings the suite presses covers menu entries for
   free ([keymap.md](../keymap.md)).

## 4. Phases

**Phase 0 — coverage pre-check** (skill 43). What the suite already
asserts about marks after a mouse action (`a_click_then_f5_copies_the_row_
that_was_clicked` and its sibling), which `Action`s have an end-to-end
test, and whether `harness::click` can be given a button. Written into
§ 7 before phase 1 starts.

**Phase 1 — the menu, from the keyboard.** `Action::ContextMenu` on
`Shift+F10` and `Menu`; a `gtk::PopoverMenu` over a `gio::Menu` built from
the action table: Open, View, Edit, —, Copy, Move, Rename, Pack, —, Cut,
Copy, Paste, —, Delete, Delete permanently, —, Folder size (on a
directory). Anchored to the cursor row. Marked-set rule from § 3.2. Tests:
`Shift+F10`, `Down`×n, `Return` reaches the same place as the key it
stands for — one per entry, which is what makes decision 9 checkable.

**Phase 2 — the right button.** A `GestureClick` for button 3 on every
row: cursor to that row, mark toggled — through the same `toggle_mark`
the keyboard uses, so the status line's marked count and the size walk
come with it. A `GestureLongPress` for button 3 opens the menu of phase 1
on that row. Tests drive `xdotool mousedown 3` / `mouseup 3` with a sleep
between for the long press, against a row the harness can name.

**Phase 3 — the background menu.** A gesture on the `ColumnView` itself
for a press that reaches no row. Paste, New folder, Refresh, Show hidden,
Sort by ▸ (Name, Ext, Size, Date). Test: right-hold below the last row of
a short listing, `Down`, `Return` → the F7 dialog.

**Phase 4 — Open with ▸.** `gio::AppInfo::all_for_type` for the row's
content type (`Row::content_type`, already there for the icon), one entry
per application, launched with `AppInfo::launch` — a path handed over as a
`gio::File`, never a line. On Linux this is the registry a double-click
uses; on macOS it is Launch Services. Test: a fake `.desktop` file in a
test `XDG_DATA_HOME` registered for `text/plain` appears in the submenu.

**Phase 5 — the Windows shell menu.** `fc-app/src/shellmenu/` with
`mod.rs` (the one call the actions make: *show the menu for these paths at
this point*) and `windows.rs`: `SHParseDisplayName` → `SHBindToParent` →
`GetUIObjectOf(IContextMenu)` → `CreatePopupMenu` + `QueryContextMenu` →
helper window → `TrackPopupMenuEx(TPM_RETURNCMD)` → `InvokeCommand`, with
`IContextMenu3::HandleMenuMsg2` forwarded from the helper's procedure.
The folder's own menu for the background. The pane refreshes afterwards
the way it does for any external change — the watcher
([watching.md](../watching.md)) — so a shell *Delete* or *New folder*
appears without this code knowing what the verb did.

**What CI can check on Windows, and will:** everything up to the popup.
A test in `fc-app`'s Windows-run set builds the `IContextMenu` for a
temporary file, fills an `HMENU` from it, and asserts the menu holds items
and that a verb named *delete* is among them (`GetCommandString`) — no
window, no pointer, and it runs on the `windows-2025` runner the way the
command tests do. `TrackPopupMenuEx` and `InvokeCommand` are the two calls
only a desktop can run, and they are the two the owner tries.

**Phase 6 — docs + audit.** `keymap.md` (the two keys, the button, the
long press), `ui-shell.md` (the menu, the gestures), `windows.md` (the
shell menu, the helper window, what CI checks), `config.md` if a setting
appears (see § 6), `future-improvements.md` for what was decided against;
then the refactoring audit (skill 49).

## 5. Effort

Corrected per skill 45 (factor 0.10, holding across the last six plans).

| Phase | Raw | Corrected |
|---|---|---|
| 0 pre-check | 0.25 d | 0.25 h |
| 1 the menu, keyboard | 2 d | 2 h |
| 2 the right button | 1.5 d | 1.5 h |
| 3 background menu | 0.75 d | 0.75 h |
| 4 Open with | 1 d | 1 h |
| 5 Windows shell menu | 3 d | 3 h |
| 6 docs + audit | 1 d | 1 h |

About nine and a half hours, plus a full-gate run (~15 min) per phase
commit — **plus rounds on the owner's Windows machine for phase 5** that
no estimate here can size. Phases 1–4 are complete and useful without
phase 5; phase 5 lands last on purpose, so a Windows round that goes long
holds nothing else back.

## 6. What would make this wrong

- **The long press is an inference.** The owner chose "right-click marks;
  the menu from `Shift+F10`". TC also opens the menu on a *held* right
  button, and a plan that dropped that would leave the mouse with no way
  to the menu at all — but it was not asked for in those words. One line
  to strike if unwanted.
- **The Windows half is the largest block of platform code this program
  has, and the popup cannot be tested here.** The last round's lesson is
  applied structurally: the COM plumbing is a Windows CI test, the
  dependency is one already built, the popup is the only untested part and
  it is named as such. It will still take a round or two on a real machine.
- **The GTK loop stops while the Windows menu is up.** A job's progress
  bar freezes for that second. Acceptable for a popup; noted so nobody
  files it as the indicator hanging.
- **`gio::AppInfo` on macOS is less complete than Finder's list** — it
  reads Launch Services' registered handlers, not *Open with → App Store*.
  Good enough for the phrase "the applications that can open this".
- **Marks and the menu interact.** Right-hold on a marked row acts on the
  set; a user who right-clicks (marks) and then right-holds the same row
  has just *unmarked* it and gets a single-row menu. That is TC's
  behaviour too, and the tests write it down so it reads as a decision
  rather than a bug.
- **Speed.** A menu built from the action table is a dozen `gio::MenuItem`s,
  built once per popup; the shell menu is whatever the machine's
  extensions cost, which Explorer pays too. Nothing here is on the
  keystroke path of a pane ([performance.md](../performance.md)).

## 7. Outcome

**Phase 0 — what was there.** Two end-to-end tests already reach a row
by mouse (`a_click_then_f5_copies_the_row_that_was_clicked` and its
type-ahead sibling), both through `harness::click`, which presses button 1
and nothing else. Every default binding is pressed somewhere in the suite
or excused by name (`every_binding_is_pressed_end_to_end_or_says_why_not`),
and `docs/keymap.md`'s table is parsed against `BINDINGS` — so a new key
fails two gates until it has a test and a row. No menu, popover or gesture
existed anywhere in `fc-app`; the mouse reached the panes only through
`SingleSelection`'s own click handling.

**Phase 1 — the menu, from the keyboard.** `Shift+F10` and `Menu` open a
`PopoverMenu` on the cursor row. Entries are `(label, action name)` pairs
in `constants::ROW_MENU`, resolved through `keymap::action_named` and run
by `dispatch` — one `menu.run` action with the name as its parameter,
rather than one action per entry. The shortcut beside each entry comes
from `Keymap::accelerator_for`, so a user's `[keys]` shows through.
Three things found by running it under Xvfb rather than by reading:
the popover opens with its first entry highlighted (so `Down`×n reaches
entry n); GTK caps the popover to the room below the row and scrolls the
rest; and the window's capture-phase key controller has to stand down
while a popover has the focus, or `Down` moves the pane's cursor behind
the menu. Tests: the model's shape and shortcut headlessly; the keymap's
choice of shortcut under an override; and three end-to-end — an entry runs
like its key, the menu acts on the marks rather than the row it points at,
and `Escape` gives the rows the keyboard back.

**Phase 2 — the right button.** A `GestureClick` and a `GestureLongPress`
for button 3 on every cell, reporting a `RowGesture` through a hook the
shell fills — the inline rename's pattern. A click puts the cursor on the
row and toggles its mark through `marking`, so the count and the scroll
follow `Space`'s rule; a hold puts the cursor there and opens phase 1's
menu. Both make the pressed pane active first, which a left click does
not. GTK claiming the sequence on the long press is what stops a hold from
marking, and the test that holds twice then presses F5 pins it. Four
end-to-end tests, all green on the first run; the harness grew
`right_click` and `right_hold`, the latter holding 900 ms against GTK's
500 ms threshold so the two gestures are never a coin toss.

**Phase 3 — the background menu.** A second table, `BACKGROUND_MENU`, in
the row menu's shape and under its test — the four sorts as a flat section
rather than a submenu, so one shape holds. Two more button-3 gestures, on
the `ColumnView` itself; both fire for a press on a row too, and rather
than claiming the sequence (which would have cost the cell gestures their
release, or the long press its cancel of the click) the handler picks the
widget under the pointer and reads its ancestry: a `row` by CSS name means
a cell had it, no `ListView` above means the header, else empty space. A
click and a hold both open it, since there is nothing to mark. Two
end-to-end tests, choosing New folder from each gesture: the dialog that
opens says which menu it was.
