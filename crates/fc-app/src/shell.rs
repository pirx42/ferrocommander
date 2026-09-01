//! The shell: what the window knows, and the machinery every action shares.
//!
//! One object the whole crate cooperates on, which is why its fields are
//! `pub(crate)` rather than private — two panes, a queue, a keymap and the
//! settings are not three objects pretending not to know about each other.
//!
//! What lives here is everything that is *not* about a particular key:
//! remembering the settings, handing a job to the queue and following it,
//! waiting for a listing a worker thread is reading, and keeping the
//! directory watches pointed at the right directories. What a key *means* is
//! [`crate::actions`].

use std::cell::RefCell;
use std::rc::Rc;

use gtk::glib;
use gtk::prelude::*;

use fc_core::config;
use fc_core::listing::Loading;
use fc_core::ops::{Job, JobHandle, JobQueue};
use fc_core::sizes::Sizes;
use fc_core::vfs::VfsPath;

use crate::actions::{preview_for, preview_text, Preview};
use crate::constants::{
    CONFLICT_PROMPT, PANE_COUNT, PREVIEW_FOLDER_COUNTING, PROGRESS_DELAY, SETTINGS_SAVE_DELAY,
    SETTINGS_UNWRITABLE, TITLE_CONFLICT,
};
use crate::keymap::Keymap;
use crate::pane::PaneView;
use crate::{command_line, dialogs, progress};
use fc_core::vfs::LocalFs;

/// The two panes, which of them keystrokes go to, and the jobs they started.
pub(crate) struct Shell {
    pub(crate) panes: [PaneView; PANE_COUNT],
    pub(crate) active: usize,
    /// Whether the inactive pane is a preview rather than a listing
    /// (`Ctrl+Q`, [`docs/viewer.md`]).
    ///
    /// A flag rather than a pane index, and the *inactive* pane rather than a
    /// remembered one: `Tab` then keeps its plain meaning — the preview
    /// simply follows the keyboard to whichever pane it is not in — and no
    /// key needs a special case for a mode. Not persisted: a file manager
    /// that starts with one pane showing the head of a text file has to be
    /// explained, and the key is cheap to press again.
    pub(crate) quick_view: bool,
    /// Every file operation goes through here, so they run one at a time and
    /// off the UI thread.
    queue: JobQueue,
    /// Weak, or the window would own the shell that owns the window. Read
    /// through [`window`](Self::window), which is the only thing anyone wants
    /// from it.
    window: glib::WeakRef<gtk::ApplicationWindow>,
    /// Where the settings file goes, or `None` when the platform offers
    /// nowhere to put one.
    config_root: Option<VfsPath>,
    /// What was last written. Compared against the current state so that
    /// moving the cursor around does not rewrite an identical file.
    pub(crate) saved: config::Settings,
    /// What the last multi-rename did, as `(new, old)` pairs, for `Ctrl+Z`.
    ///
    /// One batch deep, and cleared when it is used.
    pub(crate) renamed: Vec<(VfsPath, VfsPath)>,
    /// The command lines that were run, newest first.
    ///
    /// Beside `saved` rather than in it — see
    /// [`remember_command`](Self::remember_command).
    pub(crate) command_history: Vec<String>,
    /// The column widths as they are right now, shared by both panes.
    ///
    /// A field of its own rather than a write into `saved`, like every other
    /// setting the shell owns: `saved` means "what the file says", and
    /// `remember` compares the two to decide whether there is anything to
    /// write. Mutating `saved` directly makes them equal and the save is
    /// skipped — which is exactly what happened, and looked like a drag that
    /// GTK never reported.
    pub(crate) columns: config::ColumnSettings,
    /// The command line across the bottom, which follows the active pane.
    pub(crate) command_line: command_line::CommandLine,
    /// Where each mount point was last showing, keyed by mount path.
    ///
    /// On the shell rather than on a pane, because it is shared: leaving a
    /// drive in one pane is what the other finds when it arrives there, which
    /// is how Total Commander behaves.
    pub(crate) drives: std::collections::BTreeMap<String, String>,
    /// The directories `Ctrl+D` offers, in the order they are shown.
    ///
    /// Beside `saved` rather than read out of it, for the reason
    /// `command_history` is: the live list is what the dialog changes, and
    /// writing a change straight into the record of what is on disk would
    /// mark it as already saved and it would never reach the file.
    pub(crate) favourites: Vec<config::Favourite>,
    /// The default bindings with the user's own laid over them. Read on every
    /// keystroke and never changed again, so it is built once at startup.
    pub(crate) keymap: Keymap,
    /// Whether a write is already scheduled. One pending write picks up
    /// whatever the settings are when it runs, so a burst of changes costs
    /// one file write rather than one each.
    save_queued: bool,
}

impl Shell {
    pub(crate) fn new(
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
            quick_view: false,
            command_line,
            queue: JobQueue::new(),
            window: window.downgrade(),
            config_root,
            drives: saved.drives.clone(),
            favourites: saved.favourites.clone(),
            command_history: saved.command_history.clone(),
            columns: saved.columns,
            renamed: Vec::new(),
            saved,
            keymap,
            save_queued: false,
        };
        shell.update_active();
        shell
    }

    /// The window, while there still is one.
    ///
    /// `None` once it has been closed: a dialog opened from a handler that
    /// outlived the window would have nothing to be modal to. Every handler
    /// that opens one starts here, which is why it is a method rather than
    /// nine copies of the same `upgrade()`.
    pub(crate) fn window(&self) -> Option<gtk::ApplicationWindow> {
        self.window.upgrade()
    }

    pub(crate) fn active_pane(&mut self) -> &mut PaneView {
        &mut self.panes[self.active]
    }

    /// The pane a copy or move would land in — the other one.
    pub(crate) fn other(&self) -> usize {
        (self.active + 1) % PANE_COUNT
    }

    pub(crate) fn update_active(&mut self) {
        for (index, pane) in self.panes.iter().enumerate() {
            pane.set_active(index == self.active);
        }
        self.panes[self.active].grab_focus();
        self.follow_active();
    }

    /// Adds a command line to the history.
    ///
    /// Kept beside `saved` rather than inside it, and that is the whole point:
    /// `saved` is the record of what is *on disk*, and the next write happens
    /// only where the two differ. Writing a new command straight into `saved`
    /// marked it as already saved, so it never reached the file — the history
    /// survived a restart only by accident, when some other setting happened
    /// to have changed too, and stopped surviving the moment one stopped.
    pub(crate) fn remember_command(&mut self, line: &str) {
        config::remember_command(&mut self.command_history, line);
    }

    /// Points the command line's prompt at the active pane.
    ///
    /// Shown rather than left to be remembered: which directory a command
    /// would run in changes under the user with every Tab, and a command line
    /// that did not say so is one you check by running something.
    pub(crate) fn follow_active(&self) {
        // The path a person reads, not the one a job addresses: inside an
        // archive the backend calls the directory `/`, and a prompt reading
        // `/ $` beside a path bar reading `…/bundle.zip` is two answers to
        // one question.
        self.command_line
            .follow(&self.panes[self.active].shown_dir());
    }

    /// Puts what the active pane's cursor is on into the other pane, when
    /// quick view is on.
    ///
    /// Called from the two places a cursor can move — the end of `dispatch`,
    /// beside [`follow_active`](Self::follow_active), and the selection
    /// signal that hears a click — rather than from every action that moves
    /// it. The last plan in this repository paid for the other arrangement:
    /// a rule everybody has to remember is a rule that fails the week nobody
    /// does.
    pub(crate) fn refresh_quick_view(&mut self) -> Option<Sizes> {
        // The pane with the keyboard is always a listing: `Tab` moves the
        // preview across rather than leaving one behind, which is what makes
        // the flag enough and a remembered pane index unnecessary.
        let showing = self.other();
        self.panes[self.active].show_listing();
        if !self.quick_view {
            self.panes[showing].show_listing();
            return None;
        }
        match preview_for(self.panes[self.active].listing()) {
            Preview::Nothing(line) => {
                self.panes[showing].show_preview(line);
                None
            }
            Preview::File(path) => {
                let fs = self.panes[self.active].fs();
                self.panes[showing].show_preview(&preview_text(fs.as_ref(), &path));
                None
            }
            // The name first and the figures when they land: a walk is asked
            // for, never waited for (`docs/viewer.md`, decision 3). The text
            // is set only when a walk actually starts, so a count already on
            // the screen survives the keystrokes that did not move the cursor.
            Preview::Folder(name) => {
                let dir = self.panes[self.active].listing().dir().clone();
                let answers = self.panes[showing].preview_folder(dir, name.clone())?;
                self.panes[showing].show_preview(&PREVIEW_FOLDER_COUNTING.replace("{name}", &name));
                Some(answers)
            }
        }
    }

    /// The folder the preview is showing, if it is showing one.
    ///
    /// What a walk's answer is checked against before it is drawn: the walk
    /// that produced it may belong to a row the cursor has since left.
    pub(crate) fn previewed_folder(&self) -> Option<String> {
        if !self.quick_view {
            return None;
        }
        match preview_for(self.panes[self.active].listing()) {
            Preview::Folder(name) => Some(name),
            _ => None,
        }
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
    pub(crate) fn current_settings(&self) -> config::Settings {
        let mut settings = config::Settings {
            // A window that has already gone keeps the size last written,
            // rather than reporting zero on the way out.
            //
            // **`default_width`, not `width`** — the same property
            // `remember_window_size` listens to. They are two different
            // things: `width()` is the current allocation, and nothing
            // notifies about it here, so a change it saw and the property
            // that woke the save could disagree. A test report has one
            // instance of exactly that shape: a resize whose *height* reached
            // the settings file and whose width did not.
            window: match self.window() {
                Some(window) => config::WindowSettings {
                    width: window.default_width(),
                    height: window.default_height(),
                },
                None => self.saved.window,
            },
            drives: self.drives.clone(),
            favourites: self.favourites.clone(),
            command_history: self.command_history.clone(),
            columns: self.columns,
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
    pub(crate) fn write_settings(&mut self) {
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
    pub(crate) fn reload_all(&mut self) {
        for pane in &mut self.panes {
            pane.reload_after_job();
        }
    }
}

/// Hands a job to the queue and starts watching it.
/// Which pane a job writes into.
///
/// It has to be said rather than assumed. A job's two backends used to be the
/// active pane's and the other pane's, always — which was invisibly right
/// while there was one backend and both were the same object, and wrong the
/// moment a pane could hold an archive. `F7` in a pane inside an archive would
/// then have created a directory *on the disk*, at the path the archive calls
/// it: not a failure, a real directory in the wrong place
/// (`docs/archives.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Writes {
    /// F5, F6, Alt+F5 — across the panes, which is what two panes are for.
    IntoOtherPane,
    /// F7, F8, Shift+F4, a rename — inside the pane the user is looking at.
    InThisPane,
}

pub(crate) fn submit(shell: &Rc<RefCell<Shell>>, job: Job, writes: Writes) {
    submit_then(shell, job, writes, || {});
}

/// Hands a job to the queue and follows it, then runs `done`.
pub(crate) fn submit_then(
    shell: &Rc<RefCell<Shell>>,
    job: Job,
    writes: Writes,
    done: impl FnOnce() + 'static,
) {
    let handle = {
        let mut state = shell.borrow_mut();
        // Put the marks away before the job spends them: it ends with a fresh
        // listing, and by then there is nothing left for `Num /` to restore.
        state.active_pane().remember_marks();
        let source_fs = state.panes[state.active].fs();
        let target_fs = match writes {
            Writes::IntoOtherPane => state.panes[state.other()].fs(),
            Writes::InThisPane => source_fs.clone(),
        };
        state.queue.submit(job, source_fs, target_fs)
    };
    watch(shell, handle, done);
}

/// Follows a running job on the main loop.
///
/// Three futures rather than one, because progress, conflicts and the report
/// arrive on separate channels and none should have to wait for another. Each
/// ends on its own when the job does and its senders drop.
pub(crate) fn watch(shell: &Rc<RefCell<Shell>>, handle: JobHandle, done: impl FnOnce() + 'static) {
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
            // The clock is the shell's to read, not the meter's: what the
            // events add up to is arithmetic, and how fast they arrived is an
            // observation.
            meter.observe(started.elapsed());
            // The window appears only once a job has proved it is going to
            // take a moment. Checked as events arrive rather than on a timer:
            // a job that finishes first simply never opens one, and a job
            // that moves no bytes at all never qualifies.
            if view.is_none() && meter.has_work() && started.elapsed() >= PROGRESS_DELAY {
                let Some(window) = showing.borrow().window() else {
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
            let Some(window) = asking.borrow().window() else {
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
        // up is worth remembering — and worth watching.
        resync_watches(&finishing);
        remember(&finishing);
        // Everything that went wrong, once, after the panes show the truth —
        // not one dialog per file while the job is still running.
        if !report.failures.is_empty() {
            if let Some(window) = finishing.borrow().window() {
                dialogs::show_failures(&window, &report.failures);
            }
            return;
        }
        done();
    });
}

/// Notes that something worth remembering may have changed.
///
/// Saving on change rather than only on exit is what makes the settings
/// survive a kill, a crash or a lost session — none of which run a close
/// handler. The write is delayed slightly so that a burst of changes (a
/// window being dragged to a new size) costs one file write rather than one
/// per step, and skipped entirely when nothing actually differs, so moving
/// the cursor around never touches the disk.
pub(crate) fn remember(shell: &Rc<RefCell<Shell>>) {
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
pub(crate) fn remember_on_close(window: &gtk::ApplicationWindow, shell: &Rc<RefCell<Shell>>) {
    let shell = shell.clone();
    window.connect_close_request(move |_| {
        shell.borrow_mut().write_settings();
        glib::Propagation::Proceed
    });
}

/// Watches the things that change outside the keymap: the window's own size.
pub(crate) fn remember_window_size(window: &gtk::ApplicationWindow, shell: &Rc<RefCell<Shell>>) {
    let width = shell.clone();
    window.connect_default_width_notify(move |_| remember(&width));
    let height = shell.clone();
    window.connect_default_height_notify(move |_| remember(&height));
}

/// Hands a pane the listing a worker thread is reading for it.
///
/// `None` is a navigation that was never started — activating a file, or `..`
/// at the root — and there is nothing to wait for.
///
/// Nothing here blocks: the pane keeps showing what it has until the read
/// lands. Every navigation goes through this, which is what took the directory
/// read off the UI thread — it is the one thing the shell does with no bound
/// on how long it takes ([`docs/listing.md`]).
pub(crate) fn await_listing(shell: &Rc<RefCell<Shell>>, index: usize, loading: Option<Loading>) {
    await_listing_or(shell, index, loading, None);
}

/// The same, with somewhere to go if the directory could not be read.
///
/// Only the drive selector needs it: a remembered directory that has since
/// gone falls back to the drive itself rather than leaving the pane showing an
/// error about a path nobody asked for by name. That check used to happen
/// right after a synchronous read; now it waits for the answer like everything
/// else.
pub(crate) fn await_listing_or(
    shell: &Rc<RefCell<Shell>>,
    index: usize,
    loading: Option<Loading>,
    fallback: Option<VfsPath>,
) {
    let Some(loading) = loading else {
        return;
    };
    let wanted = match shell.borrow().panes[index].awaiting() {
        Some(dir) => dir,
        None => return,
    };
    let shell = shell.clone();
    glib::spawn_future_local(async move {
        let Ok(listing) = loading.recv().await else {
            return;
        };
        shell.borrow_mut().panes[index].arrived(&wanted, listing);

        // Somewhere else to try, and a reason to.
        if let Some(fallback) = fallback {
            let mut state = shell.borrow_mut();
            if state.panes[index].went_wrong() {
                let retry = state.panes[index].go_to(fallback);
                drop(state);
                await_listing(&shell, index, Some(retry));
                return;
            }
        }

        // The pane is somewhere else now, which moves the prompt, the watch
        // and what is worth saving.
        shell.borrow().follow_active();
        resync_watches(&shell);
        remember(&shell);
    });
}

/// Points every pane's watch at the directory it is showing now.
///
/// Called wherever a pane may have moved. Cheap when nothing did: it is two
/// path comparisons, and a watch is only torn down and rebuilt when the pane
/// really has gone somewhere else.
pub(crate) fn resync_watches(shell: &Rc<RefCell<Shell>>) {
    for index in 0..PANE_COUNT {
        if shell.borrow().panes[index].watch_is_stale() {
            watch_pane(shell, index);
        }
    }
}

/// Starts watching a pane's directory, and keeps re-reading it as it changes.
///
/// Restarted on every navigation: the old watch is about a directory nobody is
/// looking at any more. Each nudge is checked against the directory the pane
/// is showing *now*, because a navigation and a nudge can cross.
pub(crate) fn watch_pane(shell: &Rc<RefCell<Shell>>, index: usize) {
    let (changes, watched) = {
        let mut state = shell.borrow_mut();
        let pane = &mut state.panes[index];
        (pane.rewatch(), pane.directory())
    };
    // A directory that cannot be watched is a pane that does not refresh
    // itself. `Ctrl+R` still works, which is most of why it exists.
    let Some(changes) = changes else {
        return;
    };

    let shell = shell.clone();
    glib::spawn_future_local(async move {
        while changes.recv().await.is_ok() {
            let mut state = shell.borrow_mut();
            // The pane has navigated since; its new watch is the live one and
            // this nudge is about somewhere else.
            if state.panes[index].directory() != watched {
                return;
            }
            // Not while a name is being typed into the list: rebuilding the
            // rows would take the editor away mid-word.
            if state.panes[index].renaming().is_some() {
                continue;
            }
            state.panes[index].reread();
        }
    });
}
