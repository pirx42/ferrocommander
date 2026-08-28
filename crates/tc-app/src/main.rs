//! GTK4 shell of the file manager.
//!
//! The window is deliberately thin: it assembles widgets, looks each
//! keystroke up in the keymap, and forwards to `tc-core`, which owns every
//! decision about what a directory contains and how it is ordered.

mod constants;
mod keymap;
mod navigation;
mod pane;
mod row;

use std::cell::RefCell;
use std::rc::Rc;

use gtk::gdk;
use gtk::glib;
use gtk::prelude::*;

use tc_core::vfs::{LocalFs, VfsPath};

use constants::{
    APP_ID, APP_NAME, PANE_COUNT, PANE_SPLIT_RATIO, STYLESHEET, WINDOW_HEIGHT, WINDOW_WIDTH,
};
use keymap::Action;
use pane::PaneView;

/// The two panes and which of them keystrokes go to.
struct Shell {
    panes: [PaneView; PANE_COUNT],
    active: usize,
}

impl Shell {
    fn new(panes: [PaneView; PANE_COUNT]) -> Self {
        let mut shell = Shell { panes, active: 0 };
        shell.update_active();
        shell
    }

    fn active_pane(&mut self) -> &mut PaneView {
        &mut self.panes[self.active]
    }

    /// Carries out an action. [`Action::Quit`] is not handled here — closing
    /// the window is the caller's business, since the shell has no window.
    fn dispatch(&mut self, action: Action) {
        // The widget may have moved its own selection since the last action —
        // Page Up/Down are not bound here and go straight to the ColumnView.
        // Catch the model up before acting on a stale cursor.
        self.active_pane().adopt_selection();

        match action {
            Action::SwitchPane => {
                self.active = (self.active + 1) % PANE_COUNT;
                self.update_active();
            }
            Action::CursorUp => self.active_pane().move_cursor_by(-1),
            Action::CursorDown => self.active_pane().move_cursor_by(1),
            Action::CursorFirst => self.active_pane().move_cursor_to_first(),
            Action::CursorLast => self.active_pane().move_cursor_to_last(),
            Action::Activate => self.active_pane().activate(),
            Action::GoParent => self.active_pane().go_parent(),
            Action::Quit => {}
        }
    }

    fn update_active(&mut self) {
        for (index, pane) in self.panes.iter().enumerate() {
            pane.set_active(index == self.active);
        }
        self.panes[self.active].grab_focus();
    }
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

    let left = PaneView::new(Box::new(LocalFs), start.clone());
    let right = PaneView::new(Box::new(LocalFs), start);

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

    let shell = Rc::new(RefCell::new(Shell::new([left, right])));
    window.add_controller(key_controller(&window, shell));
    window.present();
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
    controller.connect_key_pressed(move |_, key, _code, modifiers| {
        let Some(action) = keymap::action_for(key, modifiers) else {
            return glib::Propagation::Proceed;
        };
        if action == Action::Quit {
            if let Some(window) = window.upgrade() {
                window.close();
            }
        } else {
            shell.borrow_mut().dispatch(action);
        }
        glib::Propagation::Stop
    });

    controller.upcast()
}
