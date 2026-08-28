//! GTK4 shell of the file manager.
//!
//! The window is deliberately thin: it assembles widgets, looks each
//! keystroke up in the keymap, and forwards to `tc-core`, which owns every
//! decision about what a directory contains and how it is ordered.

mod command_line;
mod constants;
mod dialogs;
mod jobs;
mod keymap;
mod navigation;
mod pane;
mod progress;
mod row;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use gtk::gdk;
use gtk::glib;
use gtk::prelude::*;

use tc_core::config;
use tc_core::ops::{DeleteMode, Destination, Job, JobHandle, JobQueue};
use tc_core::vfs::{LocalFs, VfsPath};

use constants::{
    APP_ID, APP_TITLE, CLASS_DRIVE_BAR, CONFLICT_PROMPT, DRIVE_BAR_SPACING, LEFT_PANE,
    NEW_FILE_DEFAULT, PANE_COUNT, PANE_SPACING, PANE_SPLIT_RATIO, PATTERN_DEFAULT, PROGRESS_DELAY,
    PROMPT_COPY, PROMPT_CREATE_DIR, PROMPT_CREATE_FILE, PROMPT_MOVE, PROMPT_PATTERN, RIGHT_PANE,
    SETTINGS_SAVE_DELAY, SETTINGS_UNREADABLE, SETTINGS_UNWRITABLE, STYLESHEET, TITLE_CONFLICT,
    TITLE_COPY, TITLE_CREATE_DIR, TITLE_CREATE_FILE, TITLE_DELETE, TITLE_DRIVES, TITLE_HISTORY,
    TITLE_MARK_PATTERN, TITLE_MOVE, TITLE_OUTPUT, TITLE_UNMARK_PATTERN,
};
use keymap::{Action, Keymap};
use pane::PaneView;

/// The two panes, which of them keystrokes go to, and the jobs they started.
struct Shell {
    panes: [PaneView; PANE_COUNT],
    active: usize,
    /// Every file operation goes through here, so they run one at a time and
    /// off the UI thread.
    queue: JobQueue,
    /// Weak, or the window would own the shell that owns the window.
    window: glib::WeakRef<gtk::ApplicationWindow>,
    /// Where the settings file goes, or `None` when the platform offers
    /// nowhere to put one.
    config_root: Option<VfsPath>,
    /// What was last written. Compared against the current state so that
    /// moving the cursor around does not rewrite an identical file.
    saved: config::Settings,
    /// The command line across the bottom, which follows the active pane.
    command_line: command_line::CommandLine,
    /// Where each mount point was last showing, keyed by mount path.
    ///
    /// On the shell rather than on a pane, because it is shared: leaving a
    /// drive in one pane is what the other finds when it arrives there, which
    /// is how Total Commander behaves.
    drives: std::collections::BTreeMap<String, String>,
    /// The default bindings with the user's own laid over them. Read on every
    /// keystroke and never changed again, so it is built once at startup.
    keymap: Keymap,
    /// Whether a write is already scheduled. One pending write picks up
    /// whatever the settings are when it runs, so a burst of changes costs
    /// one file write rather than one each.
    save_queued: bool,
}

impl Shell {
    fn new(
        panes: [PaneView; PANE_COUNT],
        window: &gtk::ApplicationWindow,
        config_root: Option<VfsPath>,
        saved: config::Settings,
        keymap: Keymap,
        command_line: command_line::CommandLine,
    ) -> Self {
        let mut shell = Shell {
            panes,
            active: 0,
            command_line,
            queue: JobQueue::new(),
            window: window.downgrade(),
            config_root,
            drives: saved.drives.clone(),
            saved,
            keymap,
            save_queued: false,
        };
        shell.update_active();
        shell
    }

    fn active_pane(&mut self) -> &mut PaneView {
        &mut self.panes[self.active]
    }

    /// The pane a copy or move would land in — the other one.
    fn other(&self) -> usize {
        (self.active + 1) % PANE_COUNT
    }

    fn update_active(&mut self) {
        for (index, pane) in self.panes.iter().enumerate() {
            pane.set_active(index == self.active);
        }
        self.panes[self.active].grab_focus();
        self.follow_active();
    }

    /// Adds a command line to the history, which is part of the settings.
    ///
    /// Written into `saved` rather than a field of its own, because `saved` is
    /// what the next write compares against — a history kept beside it would
    /// look like no change at all and never reach the disk.
    fn remember_command(&mut self, line: &str) {
        self.saved.remember_command(line);
    }

    /// Points the command line's prompt at the active pane.
    ///
    /// Shown rather than left to be remembered: which directory a command
    /// would run in changes under the user with every Tab, and a command line
    /// that did not say so is one you check by running something.
    fn follow_active(&self) {
        self.command_line
            .follow(self.panes[self.active].listing().dir());
    }

    /// What the settings file would say if it were written right now.
    ///
    /// **Built from what was loaded, not from the defaults**, and then
    /// overwritten field by field with what the shell actually owns. That way
    /// round on purpose: a setting the shell does not know about — the
    /// bindings, the editor, whatever is added next — is carried through
    /// untouched, where starting from the defaults would zero it and then
    /// write the zero back over the user's own line.
    ///
    /// This has now gone wrong twice. `[keys]` was defaulted away when it
    /// arrived, and `editor` again a phase later; both were found by a test
    /// rather than by reading. Starting from `saved` makes the failure mode
    /// "a new setting is preserved" instead of "a new setting is destroyed".
    fn current_settings(&self) -> config::Settings {
        let mut settings = config::Settings {
            // A window that has already gone keeps the size last written,
            // rather than reporting zero on the way out.
            window: match self.window.upgrade() {
                Some(window) => config::WindowSettings {
                    width: window.width(),
                    height: window.height(),
                },
                None => self.saved.window,
            },
            drives: self.drives.clone(),
            ..self.saved.clone()
        };
        settings.active_pane = self.active;
        for (index, pane) in self.panes.iter().enumerate() {
            let (directory, sort, show_hidden) = pane.state();
            let mut pane_settings = config::PaneSettings {
                directory: directory.to_string(),
                show_hidden,
                ..config::PaneSettings::default()
            };
            pane_settings.set_sort(sort);
            settings.set_pane(index, pane_settings);
        }
        settings
    }

    /// Writes the settings if they have actually changed.
    ///
    /// Failing to write is reported and otherwise ignored: settings are worth
    /// less than the program continuing to work.
    fn write_settings(&mut self) {
        let Some(root) = self.config_root.clone() else {
            return;
        };
        let current = self.current_settings();
        if current == self.saved {
            return;
        }
        match config::save(&LocalFs, &root, &current) {
            Ok(()) => self.saved = current,
            Err(reason) => eprintln!("{SETTINGS_UNWRITABLE}: {reason}"),
        }
    }

    /// Both panes re-read the filesystem: a copy changed the target side, a
    /// move changed both, and a delete may have removed the directory a pane
    /// was standing in.
    fn reload_all(&mut self) {
        for pane in &mut self.panes {
            pane.reload_after_job();
        }
    }
}

/// Carries out an action.
///
/// A free function rather than a method, because the file operations open a
/// dialog whose answer arrives later and has to find the shell again.
/// [`Action::Quit`] is handled by the caller, which is the one that has a
/// window to close.
fn dispatch(shell: &Rc<RefCell<Shell>>, action: Action) {
    // The widget may have moved its own selection since the last action —
    // Page Up/Down are not bound here and go straight to the ColumnView.
    // Catch the model up before acting on a stale cursor.
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
        Action::Activate => shell.borrow_mut().active_pane().activate(),
        Action::GoParent => shell.borrow_mut().active_pane().go_parent(),
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
        Action::ClearFilter => shell.borrow_mut().active_pane().reset_filter(),
        Action::SortBy(key) => shell.borrow_mut().active_pane().sort_by(key),
        Action::CommandHistory => show_command_history(shell),
        Action::InsertName => {
            let mut state = shell.borrow_mut();
            state.active_pane().adopt_selection();
            // The row under the cursor, `..` included: `cd ..` is a perfectly
            // good thing to build this way, and there is no harm in the name.
            if let Some(name) = state.active_pane().current_name() {
                state.command_line.append_word(&name);
            }
        }
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
        Action::RenameInline => shell.borrow_mut().active_pane().begin_rename(),
        Action::CreateDir => start_create_dir(shell),
        Action::CreateFile => start_create_file(shell),
        Action::Delete => start_delete(shell, DeleteMode::Trash),
        Action::DeletePermanently => start_delete(shell, DeleteMode::Permanent),
        Action::Quit => {}
    }
    // One place, rather than at the end of every arm: a keystroke that
    // changed nothing worth saving costs a comparison and no more. The
    // command line's prompt is the same story — navigation moves it, and
    // there is no arm that could not have.
    shell.borrow().follow_active();
    remember(shell);
}

/// `Ctrl+↓` / `Alt+F8`: the command history, to pick a line from.
///
/// Picking puts the line in the entry rather than running it, so it can be
/// edited first — which is most of why anybody opens a history at all, and
/// what Total Commander does.
fn show_command_history(shell: &Rc<RefCell<Shell>>) {
    let (window, history) = {
        let state = shell.borrow();
        let Some(window) = state.window.upgrade() else {
            return;
        };
        (window, state.saved.command_history.clone())
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
fn run_command(shell: &Rc<RefCell<Shell>>) {
    let (line, directory) = {
        let mut state = shell.borrow_mut();
        let line = state.command_line.text();
        if line.trim().is_empty() {
            return;
        }
        (line, state.active_pane().listing().dir().clone())
    };

    match command_line::read(&line) {
        command_line::Typed::ChangeDirectory(argument) => {
            let target = command_line::destination(&directory, &argument, LocalFs::home_dir());
            if let Some(target) = target {
                shell.borrow_mut().active_pane().go_to(target);
            }
            finish_command(shell);
            remember(shell);
        }
        command_line::Typed::Shell(line) => {
            // Remembered before it runs, and whatever it does: a command that
            // failed is the one most worth getting back to and correcting.
            shell.borrow_mut().remember_command(&line);
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
fn finish_command(shell: &Rc<RefCell<Shell>>) {
    let state = shell.borrow();
    state.command_line.clear();
    state.panes[state.active].grab_focus();
}

/// Runs one shell command off the UI thread and shows what it said.
fn spawn_command(shell: &Rc<RefCell<Shell>>, directory: VfsPath, line: String) {
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
        let Some(window) = shell.borrow().window.upgrade() else {
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
fn go_to_drive(shell: &Rc<RefCell<Shell>>, target: usize, mount: &VfsPath) {
    let mut state = shell.borrow_mut();

    // Where the pane is now belongs to whatever drive it is on, and has to be
    // put away before the pane leaves it.
    let leaving = state.panes[target].listing().dir().clone();
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
    state.panes[target].go_to(arriving);
    if state.panes[target].went_wrong() {
        state.panes[target].go_to(mount.clone());
    }
    drop(state);
    remember(shell);
}

/// Alt+F1 / Alt+F2: offer `target` a list of places to go.
///
/// The pane is named by the key, not by which one has the keyboard — the F-key
/// number *is* the pane number, exactly as in Total Commander. That is the
/// opposite rule to `Ctrl+←/→` above, and deliberately so: an arrow has a
/// direction to be relative to and a number does not.
fn start_drive_selection(shell: &Rc<RefCell<Shell>>, target: usize) {
    let Some(window) = shell.borrow().window.upgrade() else {
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
fn clone_pane(shell: &Rc<RefCell<Shell>>, target: usize) {
    let mut state = shell.borrow_mut();
    if state.active == target {
        return;
    }
    let dir = state.active_pane().listing().dir().clone();
    state.panes[target].go_to(dir);
}

/// Shift+PgUp / Shift+PgDn: mark across one screenful and land there.
///
/// A free function because the page size has to be measured off the pane
/// before the same pane is borrowed mutably to act on it.
fn extend_by_page(shell: &Rc<RefCell<Shell>>, direction: isize) {
    let mut state = shell.borrow_mut();
    let pane = state.active_pane();
    let page = pane.page_rows() as isize;
    let target = (pane.cursor() as isize + direction * page).max(0) as usize;
    pane.extend_mark_to(target);
}

/// F5 and F6: ask where, then hand it to the queue.
fn start_transfer(shell: &Rc<RefCell<Shell>>, copying: bool) {
    let (window, sources, source_dir, prefill) = {
        let state = shell.borrow();
        let pane = &state.panes[state.active];
        // Everything marked, or the cursor row. Empty means `..` on its own,
        // which is a navigation control rather than something to copy.
        let sources = jobs::sources(pane.listing());
        if sources.is_empty() {
            return;
        }
        let Some(window) = state.window.upgrade() else {
            return;
        };
        let prefill = jobs::prefilled_target(state.panes[state.other()].listing().dir());
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
        submit(&shell, job);
    });
}

/// `Num +` and `Num −`: mark or unmark everything matching a wildcard.
fn start_pattern_marking(shell: &Rc<RefCell<Shell>>, marking: bool) {
    let Some(window) = shell.borrow().window.upgrade() else {
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
fn start_create_dir(shell: &Rc<RefCell<Shell>>) {
    let (window, dir) = {
        let state = shell.borrow();
        let Some(window) = state.window.upgrade() else {
            return;
        };
        (window, state.panes[state.active].listing().dir().clone())
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
            );
        },
    );
}

/// F8 / Del, and their Shift variants.
fn start_delete(shell: &Rc<RefCell<Shell>>, mode: DeleteMode) {
    let (window, paths, message) = {
        let state = shell.borrow();
        let pane = &state.panes[state.active];
        let paths = jobs::sources(pane.listing());
        if paths.is_empty() {
            return;
        }
        let Some(window) = state.window.upgrade() else {
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
            );
        },
    );
}

/// Hands a job to the queue and starts watching it.
fn submit(shell: &Rc<RefCell<Shell>>, job: Job) {
    submit_then(shell, job, || {});
}

/// Submits a job and runs `done` if it finished without a single failure.
///
/// What `Shift+F4` needs: an editor opened on a file that was never created
/// shows an empty buffer that silently recreates it on save, which is a worse
/// answer than nothing at all.
fn submit_then(shell: &Rc<RefCell<Shell>>, job: Job, done: impl FnOnce() + 'static) {
    let handle = {
        let mut state = shell.borrow_mut();
        // Put the marks away before the job spends them: it ends with a fresh
        // listing, and by then there is nothing left for `Num /` to restore.
        state.active_pane().remember_marks();
        let source_fs = state.panes[state.active].fs();
        let target_fs = state.panes[state.other()].fs();
        state.queue.submit(job, source_fs, target_fs)
    };
    watch(shell, handle, done);
}

/// Follows a running job on the main loop.
///
/// Three futures rather than one, because progress, conflicts and the report
/// arrive on separate channels and none should have to wait for another. Each
/// ends on its own when the job does and its senders drop.
fn watch(shell: &Rc<RefCell<Shell>>, handle: JobHandle, done: impl FnOnce() + 'static) {
    let JobHandle {
        progress,
        conflicts,
        report,
        cancel,
    } = handle;

    let showing = shell.clone();
    glib::spawn_future_local(async move {
        let started = std::time::Instant::now();
        let mut meter = progress::Meter::default();
        let mut view: Option<dialogs::ProgressView> = None;
        while let Ok(event) = progress.recv().await {
            meter.apply(&event);
            // The window appears only once a job has proved it is going to
            // take a moment. Checked as events arrive rather than on a timer:
            // a job that finishes first simply never opens one, and a job
            // that moves no bytes at all never qualifies.
            if view.is_none() && meter.has_work() && started.elapsed() >= PROGRESS_DELAY {
                let Some(window) = showing.borrow().window.upgrade() else {
                    return;
                };
                view = Some(dialogs::ProgressView::open(&window, cancel.clone()));
            }
            if let Some(view) = &view {
                view.update(&meter);
            }
        }
        if let Some(view) = view {
            view.close();
        }
    });

    let asking = shell.clone();
    glib::spawn_future_local(async move {
        while let Ok(request) = conflicts.recv().await {
            let Some(window) = asking.borrow().window.upgrade() else {
                // Nothing left to ask with. Dropping the question is read as
                // abort, which is what the engine should do here.
                return;
            };
            let message = CONFLICT_PROMPT.replace("{name}", &request.conflict.target.to_string());
            // Answering consumes the request, but a button handler may be
            // called more than once, so the request is taken out on the
            // first answer and the rest find nothing.
            let pending = Rc::new(RefCell::new(Some(request)));
            dialogs::ask_conflict(&window, TITLE_CONFLICT, &message, move |answer| {
                if let Some(request) = pending.borrow_mut().take() {
                    request.answer(answer);
                }
            });
        }
    });

    let finishing = shell.clone();
    glib::spawn_future_local(async move {
        let Ok(report) = report.recv().await else {
            return;
        };
        finishing.borrow_mut().reload_all();
        // A job can move the ground a pane is standing on, so where it ends
        // up is worth remembering too.
        remember(&finishing);
        // Everything that went wrong, once, after the panes show the truth —
        // not one dialog per file while the job is still running.
        if !report.failures.is_empty() {
            if let Some(window) = finishing.borrow().window.upgrade() {
                dialogs::show_failures(&window, &report.failures);
            }
            return;
        }
        done();
    });
}

fn main() -> glib::ExitCode {
    let app = gtk::Application::builder().application_id(APP_ID).build();
    app.connect_startup(|_| load_stylesheet());
    app.connect_activate(build_window);
    app.run()
}

fn load_stylesheet() {
    let provider = gtk::CssProvider::new();
    provider.load_from_string(STYLESHEET);
    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

fn build_window(app: &gtk::Application) {
    let config_root = LocalFs::config_dir();
    let (settings, complaint) = match &config_root {
        Some(root) => config::load(&LocalFs, root),
        // Nowhere to keep settings is a first run that never ends, not a
        // reason to refuse to start.
        None => (config::Settings::default(), None),
    };
    if let Some(reason) = complaint {
        eprintln!("{SETTINGS_UNREADABLE}: {reason}");
    }

    // A binding nobody can make sense of is named and skipped, never fatal: a
    // misspelling in one line must not cost the other nineteen, and nothing
    // about the settings may stop the program starting.
    let (keymap, complaints) = Keymap::with_overrides(&settings.keys);
    for complaint in complaints {
        eprintln!("{complaint}");
    }

    // Panes open where they were, or at the home directory on a first run.
    let start = LocalFs::home_dir().unwrap_or_else(VfsPath::root);

    let backend: Arc<dyn tc_core::vfs::VirtualFs> = Arc::new(LocalFs);
    let mut left = PaneView::new(
        Arc::clone(&backend),
        settings
            .pane(0)
            .directory()
            .unwrap_or_else(|| start.clone()),
    );
    let mut right = PaneView::new(backend, settings.pane(1).directory().unwrap_or(start));
    for (index, pane) in [&mut left, &mut right].into_iter().enumerate() {
        let remembered = settings.pane(index);
        pane.restore(remembered.sort(), remembered.show_hidden);
    }

    let drives = drive_bar();

    let panes = gtk::Paned::builder()
        .orientation(gtk::Orientation::Horizontal)
        .start_child(left.widget())
        .end_child(right.widget())
        .position((settings.window.width as f32 * PANE_SPLIT_RATIO) as i32)
        // Neither side may be squeezed to nothing by dragging the divider.
        .resize_start_child(true)
        .resize_end_child(true)
        .build();

    let command_line = command_line::CommandLine::new();

    let layout = gtk::Box::new(gtk::Orientation::Vertical, PANE_SPACING);
    layout.append(&drives);
    layout.append(&panes);
    layout.append(command_line.widget());

    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title(APP_TITLE)
        .default_width(settings.window.width)
        .default_height(settings.window.height)
        .child(&layout)
        .build();

    let shell = Rc::new(RefCell::new(Shell::new(
        [left, right],
        &window,
        config_root,
        settings.clone(),
        keymap,
        command_line,
    )));
    shell.borrow_mut().active = settings.active_pane.min(PANE_COUNT - 1);
    shell.borrow_mut().update_active();
    for index in 0..PANE_COUNT {
        wire_filter_bar(&shell, index);
    }
    fill_drive_bar(&drives, &shell);
    wire_command_line(&shell);
    for index in 0..PANE_COUNT {
        wire_inline_rename(&shell, index);
    }
    remember_on_close(&window, &shell);
    remember_window_size(&window, &shell);
    window.add_controller(key_controller(&window, shell));
    window.present();
}

/// One button per mounted filesystem, sending the *active* pane there.
///
/// The active one, because that is the pane the keyboard is in and the one
/// the user is looking at — sending the other would be a surprise.
fn drive_bar() -> gtk::Box {
    let bar = gtk::Box::new(gtk::Orientation::Horizontal, DRIVE_BAR_SPACING);
    bar.add_css_class(CLASS_DRIVE_BAR);
    bar
}

/// Fills the drive bar, once there is a shell for its buttons to talk to.
fn fill_drive_bar(bar: &gtk::Box, shell: &Rc<RefCell<Shell>>) {
    for mount in tc_core::vfs::mount_points() {
        let button = gtk::Button::with_label(&mount.label);
        button.set_tooltip_text(Some(mount.path.as_str()));
        let shell = shell.clone();
        let path = mount.path.clone();
        button.connect_clicked(move |_| {
            // The active pane, because that is the one the keyboard is in and
            // the one the user is looking at. Through the same helper as
            // Alt+F1, so a drive remembers where it was left however it was
            // reached.
            let target = shell.borrow().active;
            go_to_drive(&shell, target, &path);
        });
        bar.append(&button);
    }
}

/// Notes that something worth remembering may have changed.
///
/// Saving on change rather than only on exit is what makes the settings
/// survive a kill, a crash or a lost session — none of which run a close
/// handler. The write is delayed slightly so that a burst of changes (a
/// window being dragged to a new size) costs one file write rather than one
/// per step, and skipped entirely when nothing actually differs, so moving
/// the cursor around never touches the disk.
fn remember(shell: &Rc<RefCell<Shell>>) {
    {
        let mut state = shell.borrow_mut();
        if state.save_queued || state.config_root.is_none() {
            return;
        }
        if state.current_settings() == state.saved {
            return;
        }
        state.save_queued = true;
    }

    let shell = shell.clone();
    glib::timeout_add_local_once(SETTINGS_SAVE_DELAY, move || {
        let mut state = shell.borrow_mut();
        state.save_queued = false;
        state.write_settings();
    });
}

/// Writes the settings out when the window closes, so a change made in the
/// last half-second is not lost to the delay.
fn remember_on_close(window: &gtk::ApplicationWindow, shell: &Rc<RefCell<Shell>>) {
    let shell = shell.clone();
    window.connect_close_request(move |_| {
        shell.borrow_mut().write_settings();
        glib::Propagation::Proceed
    });
}

/// Watches the things that change outside the keymap: the window's own size.
fn remember_window_size(window: &gtk::ApplicationWindow, shell: &Rc<RefCell<Shell>>) {
    let width = shell.clone();
    window.connect_default_width_notify(move |_| remember(&width));
    let height = shell.clone();
    window.connect_default_height_notify(move |_| remember(&height));
}

/// A key no binding claimed: if it is a character, it starts a command.
///
/// Total Commander's feel, and the only way into the command line from the
/// keyboard — without it a keyboard-first program has a command line nobody
/// can reach. Plain `a` did nothing before this; now it types.
///
/// Only an unmodified character, though. `Ctrl+X` and `Alt+X` still report a
/// letter, and a user reaching for a shortcut this program does not have
/// meant a shortcut, not the letter — silently typing it would be a wrong
/// answer rather than a missing one.
fn typed_into_command_line(
    shell: &Rc<RefCell<Shell>>,
    key: gdk::Key,
    modifiers: gdk::ModifierType,
) -> glib::Propagation {
    if modifiers.intersects(gdk::ModifierType::CONTROL_MASK | gdk::ModifierType::ALT_MASK) {
        return glib::Propagation::Proceed;
    }
    // Control characters are keys, not text: Backspace and the arrows all
    // report one, and typing them into the line would be nonsense.
    let Some(character) = key.to_unicode().filter(|typed| !typed.is_control()) else {
        return glib::Propagation::Proceed;
    };
    shell.borrow().command_line.accept(character);
    glib::Propagation::Stop
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
fn start_create_file(shell: &Rc<RefCell<Shell>>) {
    let (window, directory) = {
        let mut state = shell.borrow_mut();
        let Some(window) = state.window.upgrade() else {
            return;
        };
        let directory = state.active_pane().listing().dir().clone();
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
            submit_then(&shell, Job::CreateFile { path }, move || {
                let editor = opening.borrow().saved.editor().to_string();
                tc_core::command::open_in_editor(&editor, &target);
            });
        },
    );
}

/// Connects one pane's inline rename to the job that carries it out.
///
/// A rename **is** a move whose destination is exact — the same rule F6's
/// dialog follows — so it goes through the same queue and gets the same
/// conflict question when something is already called that.
fn wire_inline_rename(shell: &Rc<RefCell<Shell>>, index: usize) {
    let hooked = shell.clone();
    let accept = move |outcome| {
        let renamed = match outcome {
            pane::Renamed::To(name) => name,
            pane::Renamed::Abandoned => {
                hooked.borrow_mut().panes[index].end_rename();
                return;
            }
        };

        let source = {
            let mut state = hooked.borrow_mut();
            let pane = &mut state.panes[index];
            let from = pane.renaming().map(str::to_string);
            pane.end_rename();
            from.map(|from| pane.listing().dir().child(&from))
        };
        let Some(source) = source else {
            return;
        };
        // An unchanged or empty name is not a rename to refuse loudly; it is
        // somebody deciding not to. The row is already back to a label.
        let trimmed = renamed.trim();
        if trimmed.is_empty() || Some(trimmed) == source.file_name() {
            return;
        }
        let target = hooked.borrow().panes[index].listing().dir().child(trimmed);
        submit(
            &hooked,
            Job::Move {
                sources: vec![source],
                destination: Destination::Exact(target),
            },
        );
    };
    shell.borrow().panes[index].on_rename(accept);
}

/// Connects the command line: Enter runs, Escape hands the keyboard back.
fn wire_command_line(shell: &Rc<RefCell<Shell>>) {
    let entry = shell.borrow().command_line.entry().clone();

    let running = shell.clone();
    entry.connect_activate(move |_| run_command(&running));

    // Capture phase, for the reason the filter bar's handler is: `GtkText`
    // consumes Escape and Return itself, so a bubble-phase handler never sees
    // them and the keyboard stays trapped in the field.
    let controller = gtk::EventControllerKey::new();
    controller.set_propagation_phase(gtk::PropagationPhase::Capture);
    let leaving = shell.clone();
    controller.connect_key_pressed(move |_, key, _, _| {
        if key != gdk::Key::Escape {
            return glib::Propagation::Proceed;
        }
        // Escape clears what is typed and gives the rows the keyboard back —
        // one key for "never mind", rather than select-all-and-delete and
        // then a reach for Tab.
        let state = leaving.borrow();
        state.command_line.clear();
        state.panes[state.active].grab_focus();
        glib::Propagation::Stop
    });
    entry.add_controller(controller);
}

/// Connects one pane's quick-filter field: typing narrows, Escape stops,
/// Enter keeps the narrowed view and hands the keyboard back to the rows.
fn wire_filter_bar(shell: &Rc<RefCell<Shell>>, index: usize) {
    let entry = shell.borrow().panes[index].filter_bar().clone();

    let typing = shell.clone();
    entry.connect_changed(move |_| {
        // Reentrancy is real here: hiding the field or navigating away sets
        // the text from inside a `borrow_mut`, and that emits `changed`. The
        // borrow failing means a pane is already applying this very change
        // itself, so skipping is not a lost update — it is the same update,
        // once.
        if let Ok(mut state) = typing.try_borrow_mut() {
            state.panes[index].apply_filter();
        }
    });

    let controller = gtk::EventControllerKey::new();
    // Capture phase, unlike the dialogs': `GtkText` inside the entry consumes
    // Return to emit its own activate signal, so in the bubble phase this
    // handler never sees it and the keyboard stays trapped in the field.
    // Escape does arrive either way, which is why only half of it looked
    // wired up.
    controller.set_propagation_phase(gtk::PropagationPhase::Capture);
    let keys = shell.clone();
    controller.connect_key_pressed(move |_, key, _, _| match key {
        gdk::Key::Escape => {
            keys.borrow_mut().panes[index].reset_filter();
            glib::Propagation::Stop
        }
        gdk::Key::Return | gdk::Key::KP_Enter => {
            keys.borrow().panes[index].leave_filter();
            glib::Propagation::Stop
        }
        _ => glib::Propagation::Proceed,
    });
    entry.add_controller(controller);
}

/// The widget that currently holds the keyboard focus, if any.
fn focused(controller: &gtk::EventControllerKey) -> Option<gtk::Widget> {
    controller
        .widget()
        .and_downcast::<gtk::Window>()
        .and_then(|window| gtk::prelude::GtkWindowExt::focus(&window))
}

/// Whether the keyboard focus is inside a text field.
///
/// `gtk::Text` is the widget inside a `gtk::Entry` that actually holds the
/// focus, so that is what this looks for rather than the entry itself.
fn typing(controller: &gtk::EventControllerKey) -> bool {
    focused(controller).is_some_and(|focused| focused.is::<gtk::Text>())
}

/// Whether the focus is in the command line rather than some other field.
fn typing_a_command(controller: &gtk::EventControllerKey, shell: &Rc<RefCell<Shell>>) -> bool {
    let Some(focused) = focused(controller) else {
        return false;
    };
    // The `gtk::Text` inside the entry is what holds the focus, so the entry
    // is its parent.
    focused.parent().is_some_and(|entry| {
        entry
            == *shell
                .borrow()
                .command_line
                .entry()
                .upcast_ref::<gtk::Widget>()
    })
}

/// Routes keystrokes through the keymap.
fn key_controller(
    window: &gtk::ApplicationWindow,
    shell: Rc<RefCell<Shell>>,
) -> gtk::EventController {
    let controller = gtk::EventControllerKey::new();
    // Capture phase: the window must see Tab and the arrow keys before the
    // ColumnView applies its own built-in focus and selection handling, which
    // would otherwise fight the pane cursor.
    controller.set_propagation_phase(gtk::PropagationPhase::Capture);

    // Weak, or the window would own a controller that owns the window.
    let window = window.downgrade();
    controller.connect_key_pressed(move |controller, key, _code, modifiers| {
        // The controller sits in the capture phase so the column view cannot
        // swallow Tab and the arrows. That also puts it ahead of the quick
        // filter's entry, where every letter would become a shell command and
        // Enter would open a directory instead of accepting the filter — so
        // while a text field has the focus, the shell keeps its hands off.
        // While a text field has the focus the shell keeps its hands off — with
        // one exception, and it earns itself. The command line's own
        // shortcuts (`Ctrl+Enter` to insert the name under the cursor,
        // `Ctrl+↓` for the history) are for use *while typing a command*,
        // which is exactly when the entry has the focus. Standing down there
        // would make them unreachable at the only moment they are wanted.
        //
        // Only modified keys, and only ones the keymap claims: a plain letter
        // is text, and `Ctrl+C` is the entry's own and stays hers.
        let commanding = typing_a_command(controller, &shell)
            && modifiers.intersects(gdk::ModifierType::CONTROL_MASK | gdk::ModifierType::ALT_MASK);
        if typing(controller) && !commanding {
            return glib::Propagation::Proceed;
        }
        let Some(action) = shell.borrow().keymap.action_for(key, modifiers) else {
            // No guard for `commanding` here: it is only ever true for a Ctrl
            // or Alt key, and those are the first thing the call below turns
            // away. A probe deleting the guard changed nothing, which is how
            // it was found.
            return typed_into_command_line(&shell, key, modifiers);
        };
        if action == Action::Quit {
            if let Some(window) = window.upgrade() {
                window.close();
            }
        } else {
            dispatch(&shell, action);
        }
        glib::Propagation::Stop
    });

    controller.upcast()
}
