//! The window: what is built at startup, and how a keystroke reaches an
//! action.
//!
//! Deliberately thin. It assembles widgets, wires the few that talk back —
//! the filter bars, the command line, the inline rename — and looks each
//! keystroke up in the keymap. What a key *means* is [`actions`]; what the
//! program knows while it runs is [`shell`]; every decision about what a
//! directory contains and how it is ordered belongs to `tc-core`.

mod actions;
mod command_line;
mod constants;
mod dialogs;
mod format;
mod jobs;
mod keymap;
mod navigation;
mod pane;
mod progress;
mod row;
mod shell;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use gtk::gdk;
use gtk::glib;
use gtk::prelude::*;

use tc_core::config;
use tc_core::ops::{Destination, Job};
use tc_core::vfs::{LocalFs, VfsPath};

use actions::{dispatch, go_to_drive, run_command};
use constants::{
    APP_ID, APP_TITLE, CLASS_DRIVE_BAR, DRIVE_BAR_SPACING, PANE_COUNT, PANE_SPACING,
    PANE_SPLIT_RATIO, SETTINGS_UNREADABLE, STYLESHEET, STYLESHEET_REJECTED,
};
use keymap::{Action, Keymap};
use pane::PaneView;
use shell::{remember, remember_on_close, remember_window_size, submit, watch_pane, Shell, Writes};

fn main() -> glib::ExitCode {
    let app = gtk::Application::builder().application_id(APP_ID).build();
    app.connect_startup(|_| load_stylesheet());
    app.connect_activate(build_window);
    app.run()
}

fn load_stylesheet() {
    let provider = gtk::CssProvider::new();
    // GTK drops a rule it cannot parse and says nothing: the signal is the
    // only report there is, and with nobody connected to it a selector with a
    // typo costs nothing at startup and turns up much later as "that colour
    // never worked". Said once, on stderr, like every other thing this
    // program cannot do but survives.
    provider.connect_parsing_error(|_, section, error| {
        eprintln!("{STYLESHEET_REJECTED}: {} ({error})", section.to_str());
    });
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
        wire_column_widths(&shell, index);
        watch_pane(&shell, index);
    }
    remember_on_close(&window, &shell);
    remember_window_size(&window, &shell);
    // A remembered directory that has gone — deleted, unmounted, or a path
    // inside an archive that is not open any more — puts the pane at the
    // nearest ancestor instead. The file still says otherwise until something
    // writes it, so a session that ends without a keystroke would restore to
    // the same missing place again. A no-op when nothing fell back.
    remember(&shell);
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
        // The cursor follows the file to its new name. A rename ends in a
        // fresh listing, and the entry the cursor was kept on does not exist
        // in it any more — so without this the cursor falls back to the top
        // row, and the file somebody just named is somewhere off screen under
        // a name they have to go and find.
        hooked.borrow_mut().panes[index].focus_on_arrival(&target);
        submit(
            &hooked,
            Job::Move {
                sources: vec![source],
                destination: Destination::Exact(target),
            },
            Writes::InThisPane,
        );
    };
    shell.borrow().panes[index].on_rename(accept);
}

/// Keeps the two panes' column widths equal to each other and to the file.
///
/// Shared rather than per pane, which is the whole reason the widths were
/// constants before they were settings: the panes are meant to line up, and a
/// dual-pane manager whose two halves disagree about where the Size column
/// starts is harder to read than one that cannot be adjusted at all.
///
/// So a drag in either pane is applied to the other and written down. The
/// write is the same debounced save every other setting uses, which matters
/// here more than elsewhere: dragging a column emits a notification per pixel.
fn wire_column_widths(shell: &Rc<RefCell<Shell>>, index: usize) {
    // What the last run left, before anybody can drag anything.
    let widths = shell.borrow().columns;
    shell.borrow().panes[index].set_column_widths(&widths);

    let hooked = shell.clone();
    shell.borrow().panes[index].on_column_resized(move || {
        // Guarded against the loop this would otherwise be: setting the other
        // pane's width notifies its columns too, which would set this one's
        // back, and neither drag would ever settle.
        let Ok(mut state) = hooked.try_borrow_mut() else {
            return;
        };
        let widths = state.panes[index].column_widths();
        if widths == state.columns {
            return;
        }
        state.columns = widths;
        drop(state);
        for other in 0..PANE_COUNT {
            if other != index {
                hooked.borrow().panes[other].set_column_widths(&widths);
            }
        }
        remember(&hooked);
    });
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
