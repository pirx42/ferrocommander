//! GTK4 shell of the file manager.
//!
//! The window is deliberately thin: it assembles widgets and forwards to
//! `tc-core`, which owns every decision about what a directory contains and
//! how it is ordered.

mod constants;
mod pane;
mod row;

use gtk::gdk;
use gtk::glib;
use gtk::prelude::*;

use tc_core::vfs::{LocalFs, VfsPath};

use constants::{APP_ID, APP_NAME, PANE_SPLIT_RATIO, STYLESHEET, WINDOW_HEIGHT, WINDOW_WIDTH};
use pane::PaneView;

fn main() -> glib::ExitCode {
    let app = gtk::Application::builder().application_id(APP_ID).build();
    app.connect_startup(|_| load_stylesheet());
    app.connect_activate(build_window);
    app.run()
}

fn load_stylesheet() {
    let provider = gtk::CssProvider::new();
    // `load_from_string` needs GTK 4.12; `load_from_data` is baseline API and
    // keeps the 4.0 floor this crate builds against.
    provider.load_from_data(STYLESHEET);
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

    let left = PaneView::new(&LocalFs, start.clone());
    let right = PaneView::new(&LocalFs, start);
    left.set_active(true);
    right.set_active(false);

    let panes = gtk::Paned::builder()
        .orientation(gtk::Orientation::Horizontal)
        .start_child(left.widget())
        .end_child(right.widget())
        .position((WINDOW_WIDTH as f32 * PANE_SPLIT_RATIO) as i32)
        // Neither side may be squeezed to nothing by dragging the divider.
        .resize_start_child(true)
        .resize_end_child(true)
        .build();

    gtk::ApplicationWindow::builder()
        .application(app)
        .title(APP_NAME)
        .default_width(WINDOW_WIDTH)
        .default_height(WINDOW_HEIGHT)
        .child(&panes)
        .build()
        .present();
}
