//! The window a running job shows: what it is on, how far, how fast.

use gtk::prelude::*;

use fc_core::ops::CancelToken;

use crate::constants::{BUTTON_BACKGROUND, BUTTON_CANCEL, TITLE_PROGRESS};
use crate::progress::Meter;

use super::{button_row, shell};

/// The window a running job puts up.
///
/// Held by the caller for as long as the job runs; dropping it is not enough,
/// [`ProgressView::close`] is, because the window belongs to GTK once it is
/// presented.
pub struct ProgressView {
    window: gtk::Window,
    path: gtk::Label,
    bar: gtk::ProgressBar,
}

impl ProgressView {
    /// Opens the window. Cancel pulls `cancel`, which is the same token the
    /// engine checks between tasks and inside the copy loop.
    pub fn open(parent: &impl IsA<gtk::Window>, cancel: CancelToken) -> Self {
        let (window, content) = shell(parent, TITLE_PROGRESS);
        let path = gtk::Label::builder()
            .xalign(0.0)
            .ellipsize(gtk::pango::EllipsizeMode::Start)
            .build();
        let bar = gtk::ProgressBar::builder().show_text(true).build();

        content.append(&path);
        content.append(&bar);

        let row = button_row();
        // Background first, and focused below, because it is the harmless
        // one: Enter is the key everyone reaches for, and it must never be
        // the one that stops a copy halfway.
        let background = gtk::Button::with_label(BUTTON_BACKGROUND);
        let button = gtk::Button::with_label(BUTTON_CANCEL);
        row.append(&background);
        row.append(&button);
        content.append(&row);

        // Only the window goes. Nothing here touches the cancel token, and
        // the future feeding this view goes on draining the job's events —
        // so the copy runs to its end, its failures are still reported, and
        // both panes still reload when it is done.
        let backgrounding = window.clone();
        background.connect_clicked(move |_| backgrounding.close());

        let closing = window.clone();
        button.connect_clicked(move |_| {
            cancel.cancel();
            // The window goes now rather than when the worker notices: the
            // job stops at its next checkpoint, and a dialog that lingers
            // after a click looks broken.
            closing.close();
        });

        window.present();
        background.grab_focus();
        ProgressView { window, path, bar }
    }

    pub fn update(&self, meter: &Meter) {
        self.path.set_text(meter.current());
        self.bar.set_fraction(meter.fraction());
        self.bar.set_text(Some(&meter.caption()));
    }

    pub fn close(self) {
        self.window.close();
    }
}
