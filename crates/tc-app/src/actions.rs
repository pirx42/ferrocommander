//! What each key does.
//!
//! One function per action, all of them shaped the same way: read what is
//! needed out of the shell, drop the borrow, then open a dialog or submit a
//! job. The borrow has to be dropped before anything that can call back into
//! the shell, which a dialog always can.
//!
//! The dispatch table at the top is the only place that maps an
//! [`Action`](crate::keymap::Action) onto one of them, so a binding that
//! reaches nothing is a compile error rather than a key that does nothing.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use gtk::gdk;
use gtk::glib;
use gtk::prelude::*;

use tc_core::clipboard;
use tc_core::config;
use tc_core::ops::{DeleteMode, Destination, Job};
use tc_core::vfs::{LocalFs, VfsError, VfsPath};

use crate::constants::{
    CLIPBOARD_IN_ARCHIVE, CLIPBOARD_READ_LIMIT, COMMAND_IN_ARCHIVE, EDIT_IN_ARCHIVE,
    FAVOURITE_IN_ARCHIVE, LEFT_PANE, NEW_FILE_DEFAULT, OPEN_IN_ARCHIVE, PASTE_INTO_ARCHIVE,
    PATTERN_DEFAULT, PROMPT_COPY, PROMPT_CREATE_DIR, PROMPT_CREATE_FILE, PROMPT_MOVE, PROMPT_PACK,
    PROMPT_PATTERN, RIGHT_PANE, TITLE_COPY, TITLE_CREATE_DIR, TITLE_CREATE_FILE, TITLE_DELETE,
    TITLE_DRIVES, TITLE_HISTORY, TITLE_MARK_PATTERN, TITLE_MOVE, TITLE_OUTPUT, TITLE_PACK,
    TITLE_UNMARK_PATTERN,
};
use crate::jobs::Packing;
use crate::keymap::Action;
use crate::shell::{await_listing, await_listing_or, remember, resync_watches};
use crate::shell::{submit, submit_then, Shell, Writes};
use crate::{command_line, constants, dialogs, jobs};

/// Carries out an action.
///
/// A free function rather than a method, because the file operations open a
/// dialog whose answer arrives later and has to find the shell again.
/// [`Action::Quit`] is handled by the caller, which is the one that has a
/// window to close.
pub(crate) fn dispatch(shell: &Rc<RefCell<Shell>>, action: Action) {
    // The widget may have moved its own selection since the last action —
    // Page Up/Down are not bound here and go straight to the ColumnView.
    // Catch the model up before acting on a stale cursor.
    //
    // **Once, here, for every action.** This used to be repeated inside the
    // handlers and inside `PaneView`'s own methods, where every call after
    // this one was a no-op — no main-loop turn runs in between — and where
    // nobody could tell which of the eleven was the load-bearing one. The
    // contract is on `PaneView::adopt_selection`; the only other caller is
    // the pane exchange, which adopts the pane this line does not.
    shell.borrow_mut().active_pane().adopt_selection();

    match action {
        Action::SwitchPane => {
            let mut shell = shell.borrow_mut();
            shell.active = shell.other();
            shell.update_active();
        }
        Action::CursorUp => shell.borrow_mut().active_pane().move_cursor_by(-1),
        Action::CursorDown => shell.borrow_mut().active_pane().move_cursor_by(1),
        Action::CursorFirst => shell.borrow_mut().active_pane().move_cursor_to_first(),
        Action::CursorLast => shell.borrow_mut().active_pane().move_cursor_to_last(),
        Action::Activate => {
            let index = shell.borrow().active;
            let loading = shell.borrow_mut().panes[index].activate();
            match loading {
                // A directory, the parent row, or an archive walked into.
                Some(loading) => await_listing(shell, index, Some(loading)),
                // Nothing to load means the row is a file, which until now
                // did nothing at all.
                None => open_current_file(shell),
            }
        }
        Action::GoParent => {
            let index = shell.borrow().active;
            let loading = shell.borrow_mut().panes[index].go_parent();
            await_listing(shell, index, loading);
        }
        // Focus and nothing else, with the cursor at the end of whatever is
        // there. In practice the line is empty when this arrives — Escape is
        // the only way back to the rows and it clears — but focusing is not
        // the same as starting fresh, and this key should not be the one that
        // decides that.
        Action::FocusCommandLine => shell.borrow().command_line.grab_focus(),
        Action::ClipboardCopy => put_on_clipboard(shell, false),
        Action::ClipboardCut => put_on_clipboard(shell, true),
        Action::ClipboardPaste => paste_from_clipboard(shell),
        Action::ToggleMark => shell.borrow_mut().active_pane().toggle_mark(0),
        Action::ToggleMarkAndAdvance => shell.borrow_mut().active_pane().toggle_mark(1),
        Action::ToggleMarkAndRetreat => shell.borrow_mut().active_pane().toggle_mark(-1),
        Action::ExtendMarkToFirst => shell.borrow_mut().active_pane().extend_mark_to(0),
        Action::ExtendMarkToLast => {
            let mut state = shell.borrow_mut();
            let last = state.active_pane().last_row();
            state.active_pane().extend_mark_to(last);
        }
        Action::ExtendMarkPageUp => extend_by_page(shell, -1),
        Action::ExtendMarkPageDown => extend_by_page(shell, 1),
        Action::MarkByPattern => start_pattern_marking(shell, true),
        Action::UnmarkByPattern => start_pattern_marking(shell, false),
        Action::InvertMarks => shell.borrow_mut().active_pane().invert_marks(false),
        Action::InvertMarksIncludingFolders => shell.borrow_mut().active_pane().invert_marks(true),
        Action::MarkSameExtension => shell.borrow_mut().active_pane().mark_same_extension(true),
        Action::UnmarkSameExtension => shell.borrow_mut().active_pane().mark_same_extension(false),
        Action::RestoreMarks => shell.borrow_mut().active_pane().restore_marks(),
        Action::MarkAll => shell.borrow_mut().active_pane().mark_all(),
        Action::UnmarkAll => shell.borrow_mut().active_pane().unmark_all(),
        Action::QuickFilter => {
            let state = shell.borrow();
            state.panes[state.active].begin_filter();
        }
        Action::ClearFilter => {
            // Escape stops a branch walk first. It is the only key that can,
            // and a walk of somebody's home directory is the case the whole
            // cancel exists for — clearing a filter can wait a keystroke.
            let mut state = shell.borrow_mut();
            if !state.active_pane().abandon_background() {
                state.active_pane().reset_filter();
            }
        }
        Action::SortBy(key) => shell.borrow_mut().active_pane().sort_by(key),
        Action::CommandHistory => show_command_history(shell),
        Action::InsertName => {
            let mut state = shell.borrow_mut();
            // The row under the cursor, `..` included: `cd ..` is a perfectly
            // good thing to build this way, and there is no harm in the name.
            if let Some(name) = state.active_pane().current_name() {
                state.command_line.append_word(&name);
            }
        }
        Action::Favourites => start_favourites(shell),
        Action::BranchView => start_branch_view(shell),
        Action::FolderSizes => start_folder_sizes(shell),
        Action::SelectDriveLeft => start_drive_selection(shell, LEFT_PANE),
        Action::SelectDriveRight => start_drive_selection(shell, RIGHT_PANE),
        Action::CloneToRight => clone_pane(shell, RIGHT_PANE),
        Action::CloneToLeft => clone_pane(shell, LEFT_PANE),
        Action::ExchangePanes => {
            let mut state = shell.borrow_mut();
            let (left, right) = state.panes.split_at_mut(RIGHT_PANE);
            left[LEFT_PANE].exchange_with(&mut right[0]);
        }
        Action::ToggleHidden => shell.borrow_mut().active_pane().toggle_hidden(),
        Action::Copy => start_transfer(shell, true),
        Action::Move => start_transfer(shell, false),
        Action::Pack => start_pack(shell),
        Action::RenameInline => shell.borrow_mut().active_pane().begin_rename(),
        Action::CreateDir => start_create_dir(shell),
        Action::Search => start_search(shell),
        Action::MultiRename => start_multi_rename(shell),
        Action::UndoRename => undo_rename(shell),
        Action::View => start_viewing(shell),
        Action::Edit => start_editing(shell),
        Action::CreateFile => start_create_file(shell),
        Action::Reread => shell.borrow_mut().active_pane().reread(),
        Action::Delete => start_delete(shell, DeleteMode::Trash),
        Action::DeletePermanently => start_delete(shell, DeleteMode::Permanent),
        Action::Quit => {}
    }
    // One place, rather than at the end of every arm: a keystroke that
    // changed nothing worth saving costs a comparison and no more. The
    // command line's prompt is the same story — navigation moves it, and
    // there is no arm that could not have.
    shell.borrow().follow_active();
    // A pane that moved is a pane watching the wrong directory.
    resync_watches(shell);
    remember(shell);
}

/// `Ctrl+↓` / `Alt+F8`: the command history, to pick a line from.
///
/// Picking puts the line in the entry rather than running it, so it can be
/// edited first — which is most of why anybody opens a history at all, and
/// what Total Commander does.
pub(crate) fn show_command_history(shell: &Rc<RefCell<Shell>>) {
    let (window, history) = {
        let state = shell.borrow();
        let Some(window) = state.window() else {
            return;
        };
        (window, state.command_history.clone())
    };
    // Nothing run yet is not a window worth opening on an empty list.
    if history.is_empty() {
        return;
    }
    // Label and value are the same here: a command line is its own name.
    let rows: Vec<(String, String)> = history
        .into_iter()
        .map(|line| (line, String::new()))
        .map(|(line, _)| (line.clone(), line))
        .collect();

    let shell = shell.clone();
    dialogs::choose_one(&window, TITLE_HISTORY, &rows, move |line| {
        let state = shell.borrow();
        state.command_line.set_text(&line);
        state.command_line.grab_focus();
    });
}

/// Runs whatever is in the command line, in the active pane's directory.
///
/// `cd` never reaches a shell: a `cd` in a child process changes nothing
/// anybody can see, so a command line that spawned one would look broken.
///
/// Everything else runs on a worker thread and the answer arrives back on the
/// GLib loop, like a job. Nothing blocks: a file manager whose prime directive
/// is speed does not get to freeze while `find /` finishes, and a second
/// command may start while the first is still running.
pub(crate) fn run_command(shell: &Rc<RefCell<Shell>>) {
    let (line, directory) = {
        let mut state = shell.borrow_mut();
        let line = state.command_line.text();
        if line.trim().is_empty() {
            return;
        }
        // A directory inside an archive is not a place a process can run.
        // Refused with the reason on screen rather than run in whatever the
        // path happens to mean on the real filesystem, which is how a command
        // meant for an archive ends up acting on somebody's home directory.
        if state.active_pane().in_archive() {
            let window = state.window();
            drop(state);
            if let Some(window) = window {
                dialogs::show_output(&window, TITLE_OUTPUT, COMMAND_IN_ARCHIVE);
            }
            return;
        }
        (line, state.active_pane().target_dir())
    };

    match command_line::read(&line) {
        command_line::Typed::ChangeDirectory(argument) => {
            // Remembered exactly as a spawned command is, and for the same
            // reason: what the history is for is getting a line back, and a
            // `cd` into somewhere long is the line most worth not typing
            // twice. That it moves a pane rather than starting a process is
            // an implementation detail of how this program reads it — the
            // person typed a command either way.
            shell.borrow_mut().remember_command(&line);
            let target = command_line::destination(&directory, &argument, LocalFs::home_dir());
            if let Some(target) = target {
                let index = shell.borrow().active;
                let loading = shell.borrow_mut().panes[index].go_to(target);
                await_listing(shell, index, Some(loading));
            }
            // Scheduled here for the reason the shell arm gives: a command
            // does not come through `dispatch`, so the one save at the end of
            // a keystroke never runs for it.
            remember(shell);
            finish_command(shell);
        }
        command_line::Typed::Shell(line) => {
            // Remembered before it runs, and whatever it does: a command that
            // failed is the one most worth getting back to and correcting.
            shell.borrow_mut().remember_command(&line);
            // Scheduled here because a command does not come through
            // `dispatch`: the entry's own activate handler calls this, and the
            // one save at the end of a keystroke never runs for it.
            remember(shell);
            finish_command(shell);
            // Before it finishes, not after: the keyboard belongs back in the
            // rows the moment the command is away, and a long one would
            // otherwise hold it for as long as it ran.
            spawn_command(shell, directory, line);
        }
    }
}

/// Empties the command line and hands the keyboard back to the rows.
///
/// Running a command is the end of typing one. Leaving the focus in the entry
/// meant the next F7 was typed into it rather than opening a dialog, which is
/// the sort of thing that reads as "the program ignored me".
pub(crate) fn finish_command(shell: &Rc<RefCell<Shell>>) {
    let state = shell.borrow();
    state.command_line.clear();
    state.panes[state.active].grab_focus();
}

/// Runs one shell command off the UI thread and shows what it said.
pub(crate) fn spawn_command(shell: &Rc<RefCell<Shell>>, directory: VfsPath, line: String) {
    let receiver = tc_core::command::spawn(directory, line);
    let shell = shell.clone();
    glib::spawn_future_local(async move {
        let Ok(outcome) = receiver.recv().await else {
            return;
        };
        // Both panes: a command is the one thing here that can change
        // anything, and nothing says which side it touched.
        shell.borrow_mut().reload_all();
        if !outcome.worth_showing() {
            return;
        }
        let Some(window) = shell.borrow().window() else {
            return;
        };
        dialogs::show_output(&window, TITLE_OUTPUT, &outcome.output);
    });
}

/// Sends a pane to a mount point, landing where that mount was last showing.
///
/// Total Commander's behaviour with its default `AlwaysToRoot=0`: a drive
/// remembers the directory you left it in, so switching away and back is not
/// a trip to the root and a walk down again. The memory is shared between the
/// panes, as it is there.
///
/// A remembered directory that has since gone — an unplugged disk, a deleted
/// folder — falls back to the mount itself rather than leaving the pane
/// showing an error about a path the user never asked for by name.
pub(crate) fn go_to_drive(shell: &Rc<RefCell<Shell>>, target: usize, mount: &VfsPath) {
    let mut state = shell.borrow_mut();

    // Where the pane is now belongs to whatever drive it is on, and has to be
    // put away before the pane leaves it.
    let leaving = state.panes[target].target_dir();
    if let Some(from) = tc_core::vfs::mount_for(&leaving, &tc_core::vfs::mount_points()) {
        state
            .drives
            .insert(from.as_str().to_string(), leaving.to_string());
    }

    let remembered = state
        .drives
        .get(mount.as_str())
        .map(|dir| VfsPath::new(dir));
    let arriving = remembered.unwrap_or_else(|| mount.clone());
    let loading = state.panes[target].leave_for(arriving);
    drop(state);
    await_listing_or(shell, target, Some(loading), Some(mount.clone()));
}

/// Ctrl+B: every file below this pane, as one flat list.
///
/// Total Commander's branch view. What comes back is an ordinary listing, so
/// the sort, the filter, the marks and every file operation go on meaning
/// what they meant (`docs/listing.md`).
///
/// Pressing it while already in one walks again, which is the re-read a
/// branch view has instead of `Ctrl+R`. Leaving is a navigation: any step
/// lands an ordinary listing of somewhere.
pub(crate) fn start_branch_view(shell: &Rc<RefCell<Shell>>) {
    let index = shell.borrow().active;
    let loading = shell.borrow_mut().panes[index].branch();
    await_listing(shell, index, Some(loading));
}

/// Alt+Shift+Enter: count what the marked folders hold.
///
/// Total Commander's own key. The answers arrive one folder at a time and
/// each is spliced into its row as it lands, so ten marked folders fill in as
/// they finish rather than all at the pace of the slowest
/// (`docs/listing.md`).
pub(crate) fn start_folder_sizes(shell: &Rc<RefCell<Shell>>) {
    let index = shell.borrow().active;
    let Some(answers) = shell.borrow_mut().panes[index].measure_folders() else {
        return;
    };

    let counting = shell.clone();
    glib::spawn_future_local(async move {
        while let Ok((name, measured)) = answers.recv().await {
            counting.borrow_mut().panes[index].measured(&name, measured.bytes, measured.complete);
        }
    });
}

/// Ctrl+D: offer the favourite directories, and go to the chosen one.
///
/// The **active** pane, which is the ordinary rule here — a key with no
/// direction and no number in it acts on whichever pane has the keyboard.
/// That is the opposite of `Alt+F1`/`Alt+F2`, where the F-key number *is* the
/// pane number, and deliberately so.
///
/// It `leave_for`s rather than navigating, so pressing a favourite while
/// inside an archive comes out of it: a favourite is always a path on the
/// real filesystem, which the archive backend has never heard of.
pub(crate) fn start_favourites(shell: &Rc<RefCell<Shell>>) {
    let (window, favourites) = {
        let state = shell.borrow();
        let Some(window) = state.window() else {
            return;
        };
        (window, state.favourites.clone())
    };

    let going = shell.clone();
    let adding = shell.clone();
    let removing = shell.clone();
    dialogs::open_favourites(
        &window,
        favourites,
        dialogs::FavouriteHooks {
            go: Box::new(move |target| {
                let index = going.borrow().active;
                let loading = going.borrow_mut().panes[index].leave_for(target);
                await_listing(&going, index, Some(loading));
            }),
            add: Box::new(move || {
                let mut state = adding.borrow_mut();
                // A path inside an archive belongs to that archive's own
                // store, where the same spelling means a different file. Kept,
                // it would either fail on the next run or resolve against the
                // real filesystem — which is the worse of the two.
                if state.active_pane().in_archive() {
                    return Err(FAVOURITE_IN_ARCHIVE);
                }
                let directory = state.active_pane().target_dir();
                config::remember_favourite(&mut state.favourites, &directory);
                let list = state.favourites.clone();
                drop(state);
                remember(&adding);
                Ok(list)
            }),
            remove: Box::new(move |target| {
                let mut state = removing.borrow_mut();
                config::forget_favourite(&mut state.favourites, &target);
                let list = state.favourites.clone();
                drop(state);
                remember(&removing);
                list
            }),
        },
    );
}

/// Alt+F1 / Alt+F2: offer `target` a list of places to go.
///
/// The pane is named by the key, not by which one has the keyboard — the F-key
/// number *is* the pane number, exactly as in Total Commander. That is the
/// opposite rule to `Ctrl+←/→` above, and deliberately so: an arrow has a
/// direction to be relative to and a number does not.
pub(crate) fn start_drive_selection(shell: &Rc<RefCell<Shell>>, target: usize) {
    let Some(window) = shell.borrow().window() else {
        return;
    };
    let places: Vec<(String, String)> = tc_core::vfs::mount_points()
        .into_iter()
        .map(|mount| (mount.label, mount.path.as_str().to_string()))
        .collect();

    let shell = shell.clone();
    dialogs::choose_one(&window, TITLE_DRIVES, &places, move |path| {
        // `go_to_drive` remembers for us — and it has to, because the
        // keystroke that opened this dialog returned long before the answer
        // arrived, so the one call at the end of `dispatch` has been and gone.
        go_to_drive(&shell, target, &VfsPath::new(&path));
    });
}

/// Ctrl+← / Ctrl+→: the active pane's directory, shown in `target`.
///
/// Relative to the active pane, as in Total Commander: the arrow points at the
/// pane being *written*. Pressing it toward the pane the keyboard is already
/// in does nothing, rather than guessing which of the two directions was
/// meant.
pub(crate) fn clone_pane(shell: &Rc<RefCell<Shell>>, target: usize) {
    let mut state = shell.borrow_mut();
    if state.active == target {
        return;
    }
    let active = state.active;
    let loading = {
        let (left, right) = state.panes.split_at_mut(RIGHT_PANE);
        let (source, destination) = if active == LEFT_PANE {
            (&left[LEFT_PANE], &mut right[0])
        } else {
            (&right[0], &mut left[LEFT_PANE])
        };
        destination.follow(source)
    };
    drop(state);
    await_listing(shell, target, Some(loading));
}

/// Shift+PgUp / Shift+PgDn: mark across one screenful and land there.
///
/// A free function because the page size has to be measured off the pane
/// before the same pane is borrowed mutably to act on it.
pub(crate) fn extend_by_page(shell: &Rc<RefCell<Shell>>, direction: isize) {
    let mut state = shell.borrow_mut();
    let pane = state.active_pane();
    let page = pane.page_rows() as isize;
    let target = (pane.cursor() as isize + direction * page).max(0) as usize;
    pane.extend_mark_to(target);
}

/// F5 and F6: ask where, then hand it to the queue.
pub(crate) fn start_transfer(shell: &Rc<RefCell<Shell>>, copying: bool) {
    let (window, sources, source_dir, prefill) = {
        let state = shell.borrow();
        let pane = &state.panes[state.active];
        // Everything marked, or the cursor row. Empty means `..` on its own,
        // which is a navigation control rather than something to copy.
        let sources = jobs::sources(pane.listing());
        if sources.is_empty() {
            return;
        }
        let Some(window) = state.window() else {
            return;
        };
        let prefill = jobs::prefilled_target(&state.panes[state.other()].target_dir());
        // Sources from the listing, because that is where the marks are, and
        // a relative destination against the same listing they came from.
        (window, sources, pane.listing().dir().clone(), prefill)
    };

    let (title, prompt) = if copying {
        (TITLE_COPY, PROMPT_COPY)
    } else {
        (TITLE_MOVE, PROMPT_MOVE)
    };
    let shell = shell.clone();
    dialogs::ask_text(&window, title, prompt, &prefill, move |text| {
        let Some(destination) = jobs::parse_destination(&text, &source_dir) else {
            return;
        };
        let sources = sources.clone();
        let job = if copying {
            Job::Copy {
                sources,
                destination,
            }
        } else {
            Job::Move {
                sources,
                destination,
            }
        };
        submit(&shell, job, Writes::IntoOtherPane);
    });
}

/// `Num +` and `Num −`: mark or unmark everything matching a wildcard.
pub(crate) fn start_pattern_marking(shell: &Rc<RefCell<Shell>>, marking: bool) {
    let Some(window) = shell.borrow().window() else {
        return;
    };
    let title = if marking {
        TITLE_MARK_PATTERN
    } else {
        TITLE_UNMARK_PATTERN
    };
    let shell = shell.clone();
    dialogs::ask_text(
        &window,
        title,
        PROMPT_PATTERN,
        PATTERN_DEFAULT,
        move |pattern| {
            let pattern = pattern.trim().to_string();
            if pattern.is_empty() {
                return;
            }
            shell
                .borrow_mut()
                .active_pane()
                .mark_matching(&pattern, marking);
        },
    );
}

/// F7.
pub(crate) fn start_create_dir(shell: &Rc<RefCell<Shell>>) {
    let (window, dir) = {
        let state = shell.borrow();
        let Some(window) = state.window() else {
            return;
        };
        (window, state.panes[state.active].target_dir())
    };

    let shell = shell.clone();
    dialogs::ask_text(
        &window,
        TITLE_CREATE_DIR,
        PROMPT_CREATE_DIR,
        "",
        move |name| {
            let name = name.trim().to_string();
            if name.is_empty() {
                return;
            }
            submit(
                &shell,
                Job::CreateDir {
                    path: dir.child(&name),
                },
                Writes::InThisPane,
            );
        },
    );
}

/// F8 / Del, and their Shift variants.
pub(crate) fn start_delete(shell: &Rc<RefCell<Shell>>, mode: DeleteMode) {
    let (window, paths, message) = {
        let state = shell.borrow();
        let pane = &state.panes[state.active];
        let paths = jobs::sources(pane.listing());
        if paths.is_empty() {
            return;
        }
        let Some(window) = state.window() else {
            return;
        };
        let subject = jobs::subject(pane.listing(), paths.len());
        (window, paths, jobs::delete_prompt(&subject, mode))
    };

    let shell = shell.clone();
    let permanent = mode == DeleteMode::Permanent;
    dialogs::confirm(
        &window,
        TITLE_DELETE,
        &message,
        constants::BUTTON_DELETE,
        permanent,
        move || {
            submit(
                &shell,
                Job::Delete {
                    paths: paths.clone(),
                    mode,
                },
                Writes::InThisPane,
            );
        },
    );
}

/// Ctrl+M: rename what is marked, by a rule, with a preview first.
///
/// On the marks, falling back to the row under the cursor the way every other
/// operation does — renaming one file by a template is a strange thing to want
/// but a stranger thing to refuse.
///
/// Each rename is a `Move` with an exact destination through the usual queue,
/// so a name that is already taken **outside** the batch asks the conflict
/// question it always asks. The batch's own collisions are refused in the
/// preview before anything runs.
pub(crate) fn start_multi_rename(shell: &Rc<RefCell<Shell>>) {
    let (window, directory, names) = {
        let mut state = shell.borrow_mut();
        let Some(window) = state.window() else {
            return;
        };
        let pane = state.active_pane();
        let names: Vec<String> = jobs::sources(pane.listing())
            .iter()
            .filter_map(|path| path.file_name().map(str::to_string))
            .collect();
        if names.is_empty() {
            return;
        }
        (window, pane.listing().dir().clone(), names)
    };

    let shell = shell.clone();
    dialogs::MultiRename::open(&window, names, move |rows| {
        if rows.is_empty() {
            return;
        }
        // Remembered before anything runs, and as *paths*: undo is a rename
        // back, and a rename back needs to know where the files went.
        shell.borrow_mut().renamed = rows
            .iter()
            .map(|row| (directory.child(&row.to), directory.child(&row.from)))
            .collect();
        for row in rows {
            submit(
                &shell,
                Job::Move {
                    sources: vec![directory.child(&row.from)],
                    destination: Destination::Exact(directory.child(&row.to)),
                },
                Writes::InThisPane,
            );
        }
    });
}

/// Ctrl+Z: rename the last batch back.
///
/// The recorded names are not trusted — each is submitted as an ordinary move,
/// so a file that has since been moved or replaced by something else asks the
/// same conflict question it always would, and one that has gone is reported
/// as a failure rather than silently skipped.
///
/// One batch deep. A rename is undone or it is not; a stack of them would be a
/// history feature, and the honest version of that is a bigger thing than a
/// key.
pub(crate) fn undo_rename(shell: &Rc<RefCell<Shell>>) {
    let batch = std::mem::take(&mut shell.borrow_mut().renamed);
    for (from, to) in batch {
        submit(
            shell,
            Job::Move {
                sources: vec![from],
                destination: Destination::Exact(to),
            },
            Writes::InThisPane,
        );
    }
}

/// Alt+F5: pack what is marked into a new archive.
///
/// The name is asked for the way a copy destination is, prefilled beside the
/// other pane with the cursor row's name and a `.zip` on the end — which is
/// what somebody wanted nine times in ten, and which they can type over. The
/// extension is the whole choice of format: one rule decides what opens as an
/// archive and what packs into one, so a name this writes is a name that opens
/// again (`docs/archives.md`).
pub(crate) fn start_pack(shell: &Rc<RefCell<Shell>>) {
    let (window, sources, into, prefill) = {
        let state = shell.borrow();
        let pane = &state.panes[state.active];
        let sources = jobs::sources(pane.listing());
        if sources.is_empty() {
            return;
        }
        let Some(window) = state.window() else {
            return;
        };
        // Beside the *other* pane, which is where the archive is written and
        // what a bare name resolves against. The two have to be the same
        // directory or a name typed over the prefill lands somewhere the
        // prefill never mentioned.
        let into = state.panes[state.other()].target_dir();
        let prefill = jobs::prefilled_archive(&into, pane.listing(), sources.len());
        (window, sources, into, prefill)
    };

    let shell = shell.clone();
    let refusing = window.clone();
    dialogs::ask_text(&window, TITLE_PACK, PROMPT_PACK, &prefill, move |text| {
        match jobs::packed_at(&text, &into) {
            Packing::Nothing => {}
            // Refused before anything starts, because there is nothing to
            // start: the name names no packer. The same list a job's
            // failures are shown in, so one unreadable answer looks like any
            // other.
            Packing::NoFormat(archive) => {
                dialogs::show_failures(&refusing, &[(archive, VfsError::NotAnArchive)]);
            }
            Packing::Into(archive, format) => submit(
                &shell,
                Job::Pack {
                    sources: sources.clone(),
                    archive,
                    format,
                },
                Writes::IntoOtherPane,
            ),
        }
    });
}

/// Alt+F7: find files below the active pane's directory.
///
/// Results arrive as they are found and the window stays open while they do —
/// the whole point of the search streaming rather than returning a list.
/// Choosing one sends the pane to the file's directory **with the cursor on
/// it**, which is what a search is for: getting to the file. Landing in the
/// right directory and leaving somebody to hunt for the row is half the job.
pub(crate) fn start_search(shell: &Rc<RefCell<Shell>>) {
    let (window, fs, root) = {
        let mut state = shell.borrow_mut();
        let Some(window) = state.window() else {
            return;
        };
        let root = state.active_pane().target_dir();
        let fs = state.active_pane().fs();
        (window, fs, root)
    };

    let searching = Arc::clone(&fs);
    let going = shell.clone();
    dialogs::Search::open(
        &window,
        move |criteria, cancel| {
            tc_core::search::spawn(Arc::clone(&searching), root.clone(), criteria, cancel)
        },
        move |path| {
            let Some(directory) = path.parent() else {
                return;
            };
            let index = going.borrow().active;
            let loading = going.borrow_mut().panes[index].go_to(directory);
            going.borrow_mut().panes[index].focus_on_arrival(&path);
            await_listing(&going, index, Some(loading));
        },
    );
}

/// F3: look inside the file under the cursor.
///
/// Nothing on a directory or on `..`: there is nothing to read, and Total
/// Commander does not offer either.
pub(crate) fn start_viewing(shell: &Rc<RefCell<Shell>>) {
    let (window, fs, path) = {
        let mut state = shell.borrow_mut();
        let Some(window) = state.window() else {
            return;
        };
        let Some(path) = state.active_pane().current_file() else {
            return;
        };
        let fs = state.active_pane().fs();
        (window, fs, path)
    };
    // Opening reads the size and nothing else, so this is instant however big
    // the file is (`docs/viewer.md`).
    let Ok(view) = tc_core::viewer::View::open(fs.as_ref(), path) else {
        return;
    };
    dialogs::Viewer::open(&window, fs, view);
}

/// `Ctrl+C` and `Ctrl+X`: put what is marked on the system clipboard.
///
/// The *system* clipboard rather than a buffer of our own, and that is the
/// whole point: the same keystroke has to reach Nautilus, and a second
/// FerroCommander window is just another program as far as this is
/// concerned. It costs nothing extra — writing the format other file managers
/// read is what makes both work.
///
/// Three formats, because they answer three different readers: the GNOME one
/// carries the verb, `text/uri-list` is what everything else understands, and
/// plain text is what a terminal or an editor will paste as a path.
///
/// Inside an archive this is refused with a reason, for the reason `F4` and
/// Enter already are: the entries have no operating-system path, and a URI
/// naming one would point at a file on the disk that merely shares its name.
fn put_on_clipboard(shell: &Rc<RefCell<Shell>>, cut: bool) {
    let state = shell.borrow();
    let pane = &state.panes[state.active];
    let sources = jobs::sources(pane.listing());
    if sources.is_empty() {
        return;
    }
    if pane.in_archive() {
        let window = state.window();
        drop(state);
        if let Some(window) = window {
            dialogs::show_output(&window, TITLE_OUTPUT, CLIPBOARD_IN_ARCHIVE);
        }
        return;
    }
    let Some(window) = state.window() else {
        return;
    };
    let clipped = clipboard::Clipped {
        paths: sources,
        cut,
    };
    let gnome = gdk::ContentProvider::for_bytes(
        clipboard::GNOME_COPIED_FILES,
        &glib::Bytes::from_owned(clipboard::encode_gnome(&clipped).into_bytes()),
    );
    let uris = gdk::ContentProvider::for_bytes(
        clipboard::URI_LIST,
        &glib::Bytes::from_owned(clipboard::encode_uri_list(&clipped.paths).into_bytes()),
    );
    let text = gdk::ContentProvider::for_value(&clipboard::encode_text(&clipped.paths).to_value());
    let union = gdk::ContentProvider::new_union(&[gnome, uris, text]);
    window.clipboard().set_content(Some(&union)).ok();
}

/// `Ctrl+V`: copy or move what is on the clipboard into the active pane.
///
/// Reading is asynchronous — the clipboard's owner is another process and may
/// take its time — so this hands off to a callback and returns. Nothing is
/// blocked meanwhile, which is the same reason every other long thing in this
/// program is a callback.
fn paste_from_clipboard(shell: &Rc<RefCell<Shell>>) {
    let (window, into, read_only) = {
        let state = shell.borrow();
        let pane = &state.panes[state.active];
        let Some(window) = state.window() else {
            return;
        };
        (window, pane.target_dir(), pane.fs().read_only())
    };
    // An archive is read-only, so the paste is refused before anything is
    // read rather than after the engine has scanned it.
    if read_only {
        dialogs::show_output(&window, TITLE_OUTPUT, PASTE_INTO_ARCHIVE);
        return;
    }

    let shell = shell.clone();
    // Both formats, best first. GDK picks whichever the clipboard's owner
    // actually offers and says which it chose, so one read covers a file
    // manager that speaks the GNOME format and a program that only publishes
    // a list of URIs.
    window.clipboard().read_async(
        &[clipboard::GNOME_COPIED_FILES, clipboard::URI_LIST],
        glib::Priority::DEFAULT,
        gtk::gio::Cancellable::NONE,
        move |result| {
            let Ok((stream, mime)) = result else {
                return;
            };
            let pasting = shell.clone();
            let into = into.clone();
            let gnome = mime == clipboard::GNOME_COPIED_FILES;
            read_payload(stream, move |payload| {
                let clipped = match gnome {
                    true => clipboard::decode_gnome(&payload),
                    // A list of URIs cannot say "cut", so it is a copy. Not a
                    // guess: the format has nowhere to put the verb, and
                    // assuming the other direction would delete somebody
                    // else's files on no evidence at all.
                    false => Some(clipboard::Clipped {
                        paths: clipboard::decode_uri_list(&payload),
                        cut: false,
                    }),
                };
                let Some(clipped) = clipped else {
                    return;
                };
                submit_paste(&pasting, clipped, into);
            });
        },
    );
}

/// Builds the job a pasted clipboard asks for and hands it to the queue.
fn submit_paste(shell: &Rc<RefCell<Shell>>, clipped: clipboard::Clipped, into: VfsPath) {
    if clipped.paths.is_empty() {
        return;
    }
    // A cut is a move, which is the engine's own operation: it copies, checks,
    // and only then removes the source. Nothing here deletes anything.
    let cut = clipped.cut;
    let job = match cut {
        true => Job::Move {
            sources: clipped.paths,
            destination: Destination::Into(into),
        },
        false => Job::Copy {
            sources: clipped.paths,
            destination: Destination::Into(into),
        },
    };
    if !cut {
        submit(shell, job, Writes::InThisPane);
        return;
    }
    // A cut is spent by being pasted. Leaving it there invites a second paste
    // that can only fail, over files the first one already moved — every
    // desktop file manager clears it for the same reason.
    let clearing = shell.clone();
    submit_then(shell, job, Writes::InThisPane, move || {
        if let Some(window) = clearing.borrow().window() {
            window
                .clipboard()
                .set_content(gdk::ContentProvider::NONE)
                .ok();
        }
    });
}

/// Reads a clipboard payload and hands over what it said.
///
/// A read that comes back exactly full is **refused**, not used. The call
/// reads *up to* the limit, so a full buffer and a truncated one look the
/// same from here — and half a list is the worst thing to act on: a copy
/// would silently miss files, and a cut would move a subset and then clear
/// the clipboard that held the rest.
fn read_payload(stream: gtk::gio::InputStream, done: impl FnOnce(String) + 'static) {
    stream.read_bytes_async(
        CLIPBOARD_READ_LIMIT,
        glib::Priority::DEFAULT,
        gtk::gio::Cancellable::NONE,
        move |result| {
            let Ok(bytes) = result else {
                return;
            };
            if bytes.len() >= CLIPBOARD_READ_LIMIT {
                return;
            }
            done(String::from_utf8_lossy(&bytes).into_owned());
        },
    );
}

/// Enter on a file: hand it to whatever the desktop opens that kind with.
///
/// The desktop's handler, deliberately, and never the editor from the
/// settings: `F4` is "edit this", Enter is "open this", and the two are
/// different questions even when one program answers both.
///
/// It never executes the file either, whatever its permission bits say.
/// Enter is how somebody walks a directory tree, the cursor lands on every
/// row on the way past, and a file manager that started programs when the
/// cursor stopped on one would be a file manager nobody could trust to
/// browse.
pub(crate) fn open_current_file(shell: &Rc<RefCell<Shell>>) {
    let mut state = shell.borrow_mut();
    let Some(path) = state.active_pane().current_file() else {
        return;
    };
    // The same reason F4 is refused here: a handler is given an
    // operating-system path, and a file inside an archive has none. Handing
    // over what the archive calls it would open something of that name **on
    // the disk** (`docs/archives.md`).
    if state.active_pane().in_archive() {
        let window = state.window();
        drop(state);
        if let Some(window) = window {
            dialogs::show_output(&window, TITLE_OUTPUT, OPEN_IN_ARCHIVE);
        }
        return;
    }
    drop(state);
    tc_core::command::open_with(tc_core::config::DEFAULT_EDITOR, &path);
}

/// F4: hand the file under the cursor to the editor from the settings.
pub(crate) fn start_editing(shell: &Rc<RefCell<Shell>>) {
    let mut state = shell.borrow_mut();
    let Some(path) = state.active_pane().current_file() else {
        return;
    };
    // An editor is given an operating-system path, and a file inside an
    // archive has none: handing over what the archive calls it would open the
    // editor on a path of the same name **on the disk**, and create it there
    // when saved. F3 reads through the backend and works; F4 hands the file
    // over and cannot (`docs/archives.md`).
    if state.active_pane().in_archive() {
        let window = state.window();
        drop(state);
        if let Some(window) = window {
            dialogs::show_output(&window, TITLE_OUTPUT, EDIT_IN_ARCHIVE);
        }
        return;
    }
    let editor = state.saved.editor().to_string();
    tc_core::command::open_with(&editor, &path);
}

/// Shift+F4: ask for a name, create an empty file, open it in the editor.
///
/// Asked for rather than assumed, unlike Total Commander's fixed `new.txt`:
/// the name is the first thing anybody changes, and a dialog they can accept
/// with Enter costs them nothing.
///
/// The editor is launched when the job reports success and not before — an
/// editor opened on a file that was never created shows an empty buffer that
/// silently recreates it on save, which is a worse answer than nothing.
pub(crate) fn start_create_file(shell: &Rc<RefCell<Shell>>) {
    let (window, directory) = {
        let mut state = shell.borrow_mut();
        let Some(window) = state.window() else {
            return;
        };
        let directory = state.active_pane().target_dir();
        (window, directory)
    };

    let shell = shell.clone();
    dialogs::ask_text(
        &window,
        TITLE_CREATE_FILE,
        PROMPT_CREATE_FILE,
        NEW_FILE_DEFAULT,
        move |name| {
            let name = name.trim();
            if name.is_empty() {
                return;
            }
            let path = directory.child(name);
            let opening = shell.clone();
            let target = path.clone();
            submit_then(
                &shell,
                Job::CreateFile { path },
                Writes::InThisPane,
                move || {
                    let editor = opening.borrow().saved.editor().to_string();
                    tc_core::command::open_with(&editor, &target);
                },
            );
        },
    );
}
