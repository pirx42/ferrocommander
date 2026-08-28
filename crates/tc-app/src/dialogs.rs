//! The modal windows the file operations need.
//!
//! All of them are built from one shell, so three dialogs do not become three
//! layouts. Each takes a callback rather than returning an answer: GTK4 has no
//! blocking dialog, and the shell must keep running the main loop while one
//! is open.
//!
//! Nothing here decides anything. What the typed text means is
//! [`crate::jobs`], and what to do with the answer is the caller's.

use std::rc::Rc;

use gtk::gdk::Key;
use gtk::glib;
use gtk::prelude::*;

use tc_core::ops::{Answer, CancelToken, Resolution};

use crate::constants::{
    BUTTON_ABORT, BUTTON_CANCEL, BUTTON_CLOSE, BUTTON_KEEP_BOTH, BUTTON_OK, BUTTON_OVERWRITE,
    BUTTON_SKIP, CHECK_APPLY_TO_ALL, CLASS_DESTRUCTIVE, CLASS_SUGGESTED, DIALOG_MARGIN,
    DIALOG_SPACING, DIALOG_WIDTH, ENTRY_WIDTH_CHARS, FAILURE_LIST_HEIGHT, TITLE_FAILURES,
    TITLE_PROGRESS,
};
use crate::progress::{failure_lines, Meter};

/// A modal window with a vertical content box, parented so the window manager
/// keeps it above the shell.
fn shell(parent: &impl IsA<gtk::Window>, title: &str) -> (gtk::Window, gtk::Box) {
    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(DIALOG_SPACING)
        .margin_top(DIALOG_MARGIN)
        .margin_bottom(DIALOG_MARGIN)
        .margin_start(DIALOG_MARGIN)
        .margin_end(DIALOG_MARGIN)
        .build();

    let window = gtk::Window::builder()
        .title(title)
        .transient_for(parent)
        .modal(true)
        .resizable(false)
        .default_width(DIALOG_WIDTH)
        .child(&content)
        .build();

    // Escape closes it. A modal `gtk::Window` does not do this on its own,
    // and a dialog with no way out but the mouse is a trap in a
    // keyboard-first program.
    //
    // The default bubble phase is enough, unlike on the main window where the
    // column view fights for the arrow keys: nothing inside a dialog consumes
    // Escape before the window sees it, not even a focused entry that has
    // just been typed into. A UI test dismisses a dialog in exactly that
    // state, so the claim is checked rather than assumed.
    let controller = gtk::EventControllerKey::new();
    let closing = window.clone();
    controller.connect_key_pressed(move |_, key, _, _| {
        if key == Key::Escape {
            closing.close();
            return glib::Propagation::Stop;
        }
        glib::Propagation::Proceed
    });
    window.add_controller(controller);

    (window, content)
}

/// A right-aligned row of buttons, in the order they are given.
fn button_row() -> gtk::Box {
    gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(DIALOG_SPACING)
        .halign(gtk::Align::End)
        .build()
}

/// Asks for a line of text.
///
/// `accept` runs with what the entry held, and only when the user accepted —
/// closing or cancelling calls nothing, so a caller never has to distinguish
/// "empty" from "declined".
pub fn ask_text(
    parent: &impl IsA<gtk::Window>,
    title: &str,
    prompt: &str,
    prefill: &str,
    accept: impl Fn(String) + 'static,
) {
    let (window, content) = shell(parent, title);
    let label = gtk::Label::builder().label(prompt).xalign(0.0).build();
    let entry = gtk::Entry::builder()
        .text(prefill)
        .width_chars(ENTRY_WIDTH_CHARS)
        .build();

    content.append(&label);
    content.append(&entry);

    let accept = Rc::new(accept);
    let confirm = {
        let window = window.clone();
        let entry = entry.clone();
        let accept = accept.clone();
        move || {
            let text = entry.text().to_string();
            window.close();
            accept(text);
        }
    };

    let row = button_row();
    let cancel = gtk::Button::with_label(BUTTON_CANCEL);
    let ok = gtk::Button::with_label(BUTTON_OK);
    ok.add_css_class(CLASS_SUGGESTED);
    row.append(&cancel);
    row.append(&ok);
    content.append(&row);

    let closing = window.clone();
    cancel.connect_clicked(move |_| closing.close());
    let on_ok = confirm.clone();
    ok.connect_clicked(move |_| on_ok());
    // Enter in the entry means the same as pressing OK, which is how anyone
    // types a path and moves on without reaching for the mouse.
    entry.connect_activate(move |_| confirm());

    window.present();
    entry.grab_focus();
    // The prefill arrives selected, so typing replaces it and there is no
    // select-all to reach for first. Every one of these dialogs offers a
    // starting point the user is as likely to overwrite as to accept.
    entry.select_region(0, -1);
}

/// Asks a yes/no question.
///
/// `destructive` marks the affirmative button, and decides which button
/// starts focused: a question about losing data opens with Cancel under the
/// finger.
pub fn confirm(
    parent: &impl IsA<gtk::Window>,
    title: &str,
    message: &str,
    accept_label: &str,
    destructive: bool,
    accept: impl Fn() + 'static,
) {
    let (window, content) = shell(parent, title);
    let label = gtk::Label::builder()
        .label(message)
        .xalign(0.0)
        .wrap(true)
        .build();
    content.append(&label);

    let row = button_row();
    let cancel = gtk::Button::with_label(BUTTON_CANCEL);
    let ok = gtk::Button::with_label(accept_label);
    ok.add_css_class(if destructive {
        CLASS_DESTRUCTIVE
    } else {
        CLASS_SUGGESTED
    });
    row.append(&cancel);
    row.append(&ok);
    content.append(&row);

    let closing = window.clone();
    cancel.connect_clicked(move |_| closing.close());
    let closing = window.clone();
    ok.connect_clicked(move |_| {
        closing.close();
        accept();
    });

    window.present();
    if destructive {
        cancel.grab_focus();
    } else {
        ok.grab_focus();
    }
}

/// Asks what to do about something already at the destination.
///
/// The four answers are buttons rather than a list, because each is one
/// click, and *apply to all* is a checkbox beside them rather than a fifth
/// answer: it modifies whichever answer is chosen instead of being one.
///
/// Closing the window without choosing calls nothing. The engine reads that
/// silence as abort, which is the safe reading — see `docs/ops.md`.
pub fn ask_conflict(
    parent: &impl IsA<gtk::Window>,
    title: &str,
    message: &str,
    answer: impl Fn(Answer) + 'static,
) {
    let (window, content) = shell(parent, title);
    let label = gtk::Label::builder()
        .label(message)
        .xalign(0.0)
        .wrap(true)
        .build();
    let apply_to_all = gtk::CheckButton::with_label(CHECK_APPLY_TO_ALL);
    content.append(&label);
    content.append(&apply_to_all);

    let row = button_row();
    let answer = Rc::new(answer);
    let mut safe_default: Option<gtk::Button> = None;
    for (caption, resolution) in [
        (BUTTON_OVERWRITE, Resolution::Overwrite),
        (BUTTON_SKIP, Resolution::Skip),
        (BUTTON_KEEP_BOTH, Resolution::KeepBoth),
        (BUTTON_ABORT, Resolution::Abort),
    ] {
        let button = gtk::Button::with_label(caption);
        if resolution == Resolution::Overwrite {
            button.add_css_class(CLASS_DESTRUCTIVE);
        }
        let window = window.clone();
        let apply_to_all = apply_to_all.clone();
        let answer = answer.clone();
        button.connect_clicked(move |_| {
            window.close();
            answer(Answer {
                resolution,
                apply_to_all: apply_to_all.is_active(),
            });
        });
        row.append(&button);
        if resolution == Resolution::Skip {
            safe_default = Some(button);
        }
    }
    content.append(&row);

    window.present();
    // Skip starts focused, so Enter answers with the one choice that loses
    // nothing. In a keyboard-first program a dialog that opens with no focus
    // at all can only be answered with the mouse, and the obvious key to
    // reach for must not be the one that overwrites a file.
    if let Some(button) = safe_default {
        button.grab_focus();
    }
}

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
        let button = gtk::Button::with_label(BUTTON_CANCEL);
        row.append(&button);
        content.append(&row);

        let closing = window.clone();
        button.connect_clicked(move |_| {
            cancel.cancel();
            // The window goes now rather than when the worker notices: the
            // job stops at its next checkpoint, and a dialog that lingers
            // after a click looks broken.
            closing.close();
        });

        window.present();
        button.grab_focus();
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

/// Lists what a finished job could not do.
///
/// One window at the end rather than one dialog per file: a batch that hit
/// six unreadable files should cost one acknowledgement, not six.
pub fn show_failures(
    parent: &impl IsA<gtk::Window>,
    failures: &[(tc_core::vfs::VfsPath, tc_core::vfs::VfsError)],
) {
    let (window, content) = shell(parent, TITLE_FAILURES);
    let list = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(DIALOG_SPACING / 2)
        .build();
    for line in failure_lines(failures) {
        list.append(
            &gtk::Label::builder()
                .label(line)
                .xalign(0.0)
                .selectable(true)
                // Paths are long and the reason is at the end of the line,
                // which is the half worth reading.
                .wrap(true)
                .wrap_mode(gtk::pango::WrapMode::WordChar)
                .build(),
        );
    }
    let scroller = gtk::ScrolledWindow::builder()
        .child(&list)
        // Grows with the list and stops at a screenful. A minimum height
        // instead would open a window mostly full of empty space to report a
        // single failure, which is what the first version did.
        .propagate_natural_height(true)
        .max_content_height(FAILURE_LIST_HEIGHT)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .build();
    content.append(&scroller);

    let row = button_row();
    let close = gtk::Button::with_label(BUTTON_CLOSE);
    close.add_css_class(CLASS_SUGGESTED);
    row.append(&close);
    content.append(&row);

    let closing = window.clone();
    close.connect_clicked(move |_| closing.close());

    window.present();
    close.grab_focus();
}
