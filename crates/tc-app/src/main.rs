//! GTK4 shell of the file manager.
//!
//! The window is deliberately thin: it assembles widgets, looks each
//! keystroke up in the keymap, and forwards to `tc-core`, which owns every
//! decision about what a directory contains and how it is ordered.

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

use tc_core::ops::{DeleteMode, Job, JobHandle, JobQueue};
use tc_core::vfs::{LocalFs, VfsPath};

use constants::{
    APP_ID, APP_NAME, CONFLICT_PROMPT, PANE_COUNT, PANE_SPLIT_RATIO, PATTERN_DEFAULT,
    PROGRESS_DELAY, PROMPT_COPY, PROMPT_CREATE_DIR, PROMPT_MOVE, PROMPT_PATTERN, STYLESHEET,
    TITLE_CONFLICT, TITLE_COPY, TITLE_CREATE_DIR, TITLE_DELETE, TITLE_MARK_PATTERN, TITLE_MOVE,
    TITLE_UNMARK_PATTERN, WINDOW_HEIGHT, WINDOW_WIDTH,
};
use keymap::Action;
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
}

impl Shell {
    fn new(panes: [PaneView; PANE_COUNT], window: &gtk::ApplicationWindow) -> Self {
        let mut shell = Shell {
            panes,
            active: 0,
            queue: JobQueue::new(),
            window: window.downgrade(),
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
        Action::ToggleMark => shell.borrow_mut().active_pane().toggle_mark(false),
        Action::ToggleMarkAndAdvance => shell.borrow_mut().active_pane().toggle_mark(true),
        Action::MarkByPattern => start_pattern_marking(shell, true),
        Action::UnmarkByPattern => start_pattern_marking(shell, false),
        Action::InvertMarks => shell.borrow_mut().active_pane().invert_marks(),
        Action::MarkAll => shell.borrow_mut().active_pane().mark_all(),
        Action::QuickFilter => {
            let state = shell.borrow();
            state.panes[state.active].begin_filter();
        }
        Action::ClearFilter => shell.borrow_mut().active_pane().reset_filter(),
        Action::Copy => start_transfer(shell, true),
        Action::Move => start_transfer(shell, false),
        Action::CreateDir => start_create_dir(shell),
        Action::Delete => start_delete(shell, DeleteMode::Trash),
        Action::DeletePermanently => start_delete(shell, DeleteMode::Permanent),
        Action::Quit => {}
    }
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
    let handle = {
        let state = shell.borrow();
        let source_fs = state.panes[state.active].fs();
        let target_fs = state.panes[state.other()].fs();
        state.queue.submit(job, source_fs, target_fs)
    };
    watch(shell, handle);
}

/// Follows a running job on the main loop.
///
/// Three futures rather than one, because progress, conflicts and the report
/// arrive on separate channels and none should have to wait for another. Each
/// ends on its own when the job does and its senders drop.
fn watch(shell: &Rc<RefCell<Shell>>, handle: JobHandle) {
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
        // Everything that went wrong, once, after the panes show the truth —
        // not one dialog per file while the job is still running.
        if !report.failures.is_empty() {
            if let Some(window) = finishing.borrow().window.upgrade() {
                dialogs::show_failures(&window, &report.failures);
            }
        }
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
    // Both panes open at the user's home directory. Remembering the last
    // directory is config persistence, which is phase 3.
    let start = LocalFs::home_dir().unwrap_or_else(VfsPath::root);

    let backend: Arc<dyn tc_core::vfs::VirtualFs> = Arc::new(LocalFs);
    let left = PaneView::new(Arc::clone(&backend), start.clone());
    let right = PaneView::new(backend, start);

    let panes = gtk::Paned::builder()
        .orientation(gtk::Orientation::Horizontal)
        .start_child(left.widget())
        .end_child(right.widget())
        .position((WINDOW_WIDTH as f32 * PANE_SPLIT_RATIO) as i32)
        // Neither side may be squeezed to nothing by dragging the divider.
        .resize_start_child(true)
        .resize_end_child(true)
        .build();

    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title(APP_NAME)
        .default_width(WINDOW_WIDTH)
        .default_height(WINDOW_HEIGHT)
        .child(&panes)
        .build();

    let shell = Rc::new(RefCell::new(Shell::new([left, right], &window)));
    for index in 0..PANE_COUNT {
        wire_filter_bar(&shell, index);
    }
    window.add_controller(key_controller(&window, shell));
    window.present();
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

/// Whether the keyboard focus is inside a text field.
///
/// `gtk::Text` is the widget inside a `gtk::Entry` that actually holds the
/// focus, so that is what this looks for rather than the entry itself.
fn typing(controller: &gtk::EventControllerKey) -> bool {
    controller
        .widget()
        .and_downcast::<gtk::Window>()
        .and_then(|window| gtk::prelude::GtkWindowExt::focus(&window))
        .is_some_and(|focused| focused.is::<gtk::Text>())
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
        if typing(controller) {
            return glib::Propagation::Proceed;
        }
        let Some(action) = keymap::action_for(key, modifiers) else {
            return glib::Propagation::Proceed;
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
