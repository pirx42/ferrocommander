//! The modal windows the file operations need.
//!
//! All of them are built from one shell, so a new dialog is a layout rather
//! than a window: [`shell`] makes the window and its content box, and
//! [`button_row`] the row at the bottom. Each takes a callback rather than
//! returning an answer — GTK4 has no blocking dialog, and the shell must keep
//! running the main loop while one is open.
//!
//! Nothing here decides anything. What the typed text means is
//! [`crate::jobs`], and what to do with the answer is the caller's.
//!
//! The small ones live here. The four with state of their own — a progress
//! bar being driven, a viewer holding an offset, a search filling a list, a
//! rename redrawing a preview — are each a file, because each is a thing with
//! a lifetime rather than a function that opens a window.

mod compare;
mod favourites;
mod multi_rename;
mod progress_view;
mod search;
mod viewer;

use std::rc::Rc;

use gtk::gdk::Key;
use gtk::glib;
use gtk::prelude::*;

use fc_core::ops::{Answer, Resolution};

use crate::constants::{
    BUTTON_ABORT, BUTTON_CANCEL, BUTTON_CLOSE, BUTTON_KEEP_BOTH, BUTTON_OK, BUTTON_OVERWRITE,
    BUTTON_SKIP, CHECK_APPLY_TO_ALL, CLASS_DESTRUCTIVE, CLASS_DIM, CLASS_OUTPUT, CLASS_SUGGESTED,
    DIALOG_MARGIN, DIALOG_SPACING, DIALOG_WIDTH, DRIVE_LIST_HEIGHT, ENTRY_WIDTH_CHARS,
    FAILURE_LIST_HEIGHT, OUTPUT_HEIGHT, TITLE_FAILURES, XALIGN_LEFT,
};
use crate::format::failure_lines;

pub use compare::open_compare;
pub use favourites::{open as open_favourites, Hooks as FavouriteHooks};
pub use multi_rename::MultiRename;
pub use progress_view::ProgressView;
pub use search::Search;
pub use viewer::Viewer;

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

/// Offers a list of rows and calls back with the value of the one chosen.
///
/// Each row is `(label, detail)`; the detail is shown dimmed beside the label
/// and is also what comes back, because a label may repeat and the value is
/// what actually identifies the choice.
///
/// Serves the drive selector (`Alt+F1`/`Alt+F2`) and the command history
/// (`Ctrl+↓`). A modal window rather than the dropdown Total Commander uses,
/// for a reason worth writing down: a GTK popover is not a window the
/// end-to-end suite can find or send keys to, and a chooser that cannot be
/// tested through a real key press is exactly the kind of thing that ships
/// broken ([`docs/ui-shell.md`]).
///
/// Keyboard-first, since that is the whole point of having the key at all:
/// the list opens focused with the first row selected, the arrows walk it,
/// Enter takes it and Escape leaves without choosing.
pub fn choose_one(
    parent: &impl IsA<gtk::Window>,
    title: &str,
    rows: &[(String, String)],
    accept: impl Fn(String) + 'static,
) {
    let (window, content) = shell(parent, title);

    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::Browse);
    for (label, detail) in rows {
        list.append(&labelled_row(label, detail));
    }
    content.append(&list_scroller(&list));

    // By index rather than by widget: two rows may carry the same label, and
    // the index is what actually identifies the choice.
    let chosen: Vec<String> = rows.iter().map(|(_, value)| value.clone()).collect();
    let closing = window.clone();
    list.connect_row_activated(move |_, row| {
        let Some(value) = chosen.get(row.index() as usize).cloned() else {
            return;
        };
        closing.close();
        accept(value);
    });

    window.present();
    // Nothing selects or focuses the first row here, because GTK already
    // does: the list is the window's first focusable child, and
    // `SelectionMode::Browse` selects whatever the focus lands on. Code to
    // repeat that was written first and removed when a probe showed the
    // tests could not tell the difference. A UI test presses Down and Enter
    // and has to reach the *second* place, so if a GTK release ever stops
    // doing it, that fails rather than the first Enter quietly dying.
}

/// One row of a chooser: a name, and the path it stands for beside it.
///
/// Dimmed and ellipsised from the *start*, because what tells two rows apart
/// is the end of the path and not the beginning — two mounts can share a last
/// component, and two favourites can share a name, and in both cases the
/// label alone does not say which is which.
///
/// Shared by the drive list and the favourites list rather than written twice:
/// two lists that are meant to look the same and are built separately are two
/// lists that drift.
pub(super) fn labelled_row(label: &str, detail: &str) -> gtk::Box {
    let row = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(DIALOG_SPACING)
        .build();
    row.append(
        &gtk::Label::builder()
            .label(label)
            .xalign(XALIGN_LEFT)
            .build(),
    );
    let path = gtk::Label::builder()
        .label(detail)
        .xalign(XALIGN_LEFT)
        .hexpand(true)
        .ellipsize(gtk::pango::EllipsizeMode::Start)
        .build();
    path.add_css_class(CLASS_DIM);
    row.append(&path);
    row
}

/// A chooser's list, scrolling only once it is taller than a screenful.
pub(super) fn list_scroller(list: &gtk::ListBox) -> gtk::ScrolledWindow {
    gtk::ScrolledWindow::builder()
        .child(list)
        .propagate_natural_height(true)
        .max_content_height(DRIVE_LIST_HEIGHT)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .build()
}

/// Shows what a command printed, in a window that can be scrolled and copied.
///
/// Monospaced and unwrapped: this is program output, where the columns mean
/// something and a wrapped line stops lining up. It scrolls sideways instead,
/// which is what a terminal does.
pub fn show_output(parent: &impl IsA<gtk::Window>, title: &str, output: &str) {
    let (window, content) = shell(parent, title);

    let text = gtk::Label::builder()
        .label(output)
        .xalign(XALIGN_LEFT)
        .yalign(XALIGN_LEFT)
        .selectable(true)
        .build();
    text.add_css_class(CLASS_OUTPUT);

    let scroller = gtk::ScrolledWindow::builder()
        .child(&text)
        .propagate_natural_height(true)
        .max_content_height(OUTPUT_HEIGHT)
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
    // Focused, so Space or Enter closes it without reaching for the mouse.
    close.grab_focus();
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

/// Lists what a finished job could not do.
///
/// One window at the end rather than one dialog per file: a batch that hit
/// six unreadable files should cost one acknowledgement, not six.
pub fn show_failures(
    parent: &impl IsA<gtk::Window>,
    failures: &[(fc_core::vfs::VfsPath, fc_core::vfs::VfsError)],
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

/// A left-aligned prompt above a field.
fn label(text: &str) -> gtk::Label {
    gtk::Label::builder()
        .label(text)
        .xalign(XALIGN_LEFT)
        .build()
}
