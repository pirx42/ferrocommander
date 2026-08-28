//! The modal windows the file operations need.
//!
//! All of them are built from one shell, so three dialogs do not become three
//! layouts. Each takes a callback rather than returning an answer: GTK4 has no
//! blocking dialog, and the shell must keep running the main loop while one
//! is open.
//!
//! Nothing here decides anything. What the typed text means is
//! [`crate::jobs`], and what to do with the answer is the caller's.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use gtk::gdk::Key;
use gtk::glib;
use gtk::prelude::*;

use tc_core::ops::{Answer, CancelToken, Resolution};
use tc_core::rename::{preview, Refusal, Renamed, Rules};
use tc_core::search::{Criteria, Results as SearchResults};
use tc_core::vfs::{VfsPath, VirtualFs};
use tc_core::viewer::{Mode, View};

use crate::constants::{
    BUTTON_ABORT, BUTTON_CANCEL, BUTTON_CLOSE, BUTTON_KEEP_BOTH, BUTTON_OK, BUTTON_OVERWRITE,
    BUTTON_RENAME, BUTTON_SKIP, CHECK_APPLY_TO_ALL, CLASS_DESTRUCTIVE, CLASS_DIM, CLASS_OUTPUT,
    CLASS_SUGGESTED, DIALOG_MARGIN, DIALOG_SPACING, DIALOG_WIDTH, DRIVE_LIST_HEIGHT,
    ENTRY_WIDTH_CHARS, FAILURE_LIST_HEIGHT, OUTPUT_HEIGHT, PROMPT_RENAME_COUNTER,
    PROMPT_RENAME_FIND, PROMPT_RENAME_TEMPLATE, PROMPT_RENAME_WITH, PROMPT_SEARCH_CONTENT,
    PROMPT_SEARCH_NAME, RENAME_COLLIDES, RENAME_EMPTY, RENAME_LIST_HEIGHT, RENAME_ROW,
    RENAME_SEPARATOR, RENAME_UNCHANGED, SEARCH_CAPPED, SEARCH_FOUND, SEARCH_LIST_HEIGHT,
    SEARCH_LIST_LIMIT, SEARCH_NAME_DEFAULT, SEARCH_NOTHING, SEARCH_SEARCHING, SEARCH_START,
    SEARCH_STOP, TITLE_FAILURES, TITLE_PROGRESS, TITLE_RENAME, TITLE_SEARCH, TITLE_VIEWER,
    VIEWER_EMPTY, VIEWER_HEIGHT, VIEWER_WIDTH, XALIGN_LEFT,
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
        // The path beside the label, dimmed: two mounts can share a last
        // component, and then the label alone does not say which is which.
        let path = gtk::Label::builder()
            .label(detail)
            .xalign(XALIGN_LEFT)
            .hexpand(true)
            .ellipsize(gtk::pango::EllipsizeMode::Start)
            .build();
        path.add_css_class(CLASS_DIM);
        row.append(&path);
        list.append(&row);
    }

    let scroller = gtk::ScrolledWindow::builder()
        .child(&list)
        .propagate_natural_height(true)
        .max_content_height(DRIVE_LIST_HEIGHT)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .build();
    content.append(&scroller);

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

/// A window over one file: text or hex, paged by keys.
///
/// Its own window rather than a modal dialog: reading a file is not answering
/// a question, and the shell behind it stays usable.
///
/// Everything about *what* is shown is [`tc_core::viewer`] — this holds the
/// widget, the keys, and the reader the view asks for bytes.
pub struct Viewer {
    window: gtk::Window,
    text: gtk::Label,
    scroller: gtk::ScrolledWindow,
    view: RefCell<View>,
    fs: Arc<dyn VirtualFs>,
}

impl Viewer {
    pub fn open(parent: &impl IsA<gtk::Window>, fs: Arc<dyn VirtualFs>, view: View) -> Rc<Viewer> {
        let text = gtk::Label::builder()
            .xalign(XALIGN_LEFT)
            .yalign(XALIGN_LEFT)
            .selectable(true)
            .build();
        text.add_css_class(CLASS_OUTPUT);

        let scroller = gtk::ScrolledWindow::builder()
            .child(&text)
            .vexpand(true)
            .hexpand(true)
            .build();

        let window = gtk::Window::builder()
            .transient_for(parent)
            .default_width(VIEWER_WIDTH)
            .default_height(VIEWER_HEIGHT)
            .child(&scroller)
            .build();

        let viewer = Rc::new(Viewer {
            window: window.clone(),
            text,
            scroller,
            view: RefCell::new(view),
            fs,
        });

        let keys = gtk::EventControllerKey::new();
        // Capture phase, like every other key handler here: the scrolled
        // window would otherwise take the arrows and page keys for itself and
        // scroll a label that only ever holds one screenful.
        keys.set_propagation_phase(gtk::PropagationPhase::Capture);
        let handling = viewer.clone();
        keys.connect_key_pressed(move |_, key, _, _| handling.on_key(key));
        window.add_controller(keys);

        viewer.redraw();
        window.present();
        viewer
    }

    /// Acts on a key, and says whether the viewer took it.
    fn on_key(&self, key: Key) -> glib::Propagation {
        match key {
            Key::Escape | Key::F3 => {
                self.window.close();
                return glib::Propagation::Stop;
            }
            Key::Down => self.view.borrow_mut().scroll_lines(self.fs.as_ref(), 1),
            Key::Up => self.view.borrow_mut().scroll_lines(self.fs.as_ref(), -1),
            Key::Page_Down | Key::space => self.view.borrow_mut().scroll_window(1),
            Key::Page_Up => self.view.borrow_mut().scroll_window(-1),
            Key::Home => self.view.borrow_mut().to_start(),
            Key::End => self.view.borrow_mut().to_end(),
            // Total Commander's own numbering for its view modes.
            Key::_1 => self.view.borrow_mut().set_mode(Mode::Text),
            Key::_2 => self.view.borrow_mut().set_mode(Mode::Hex),
            _ => return glib::Propagation::Proceed,
        }
        self.redraw();
        glib::Propagation::Stop
    }

    /// Reads the window the view is looking at and shows it.
    fn redraw(&self) {
        let view = self.view.borrow();
        let shown = view.render(self.fs.as_ref()).unwrap_or_default();
        self.text.set_text(if shown.is_empty() {
            VIEWER_EMPTY
        } else {
            &shown
        });
        // Back to the top of the label on every page: the label holds one
        // windowful, and leaving the scrollbar where it was would skip the
        // first lines of the page just turned to.
        self.scroller.vadjustment().set_value(0.0);

        let name = view.path().file_name().unwrap_or_default();
        let percent = match view.size() {
            0 => 100,
            size => (view.offset() * 100 / size).min(100),
        };
        self.window.set_title(Some(
            &TITLE_VIEWER
                .replace("{name}", name)
                .replace("{percent}", &percent.to_string()),
        ));
    }
}

/// The search window: two fields, a list that fills as results arrive, and a
/// button that starts and stops.
///
/// Its own window rather than a modal dialog, for the reason the viewer is:
/// a search runs for minutes and the shell behind it stays usable.
pub struct Search {
    window: gtk::Window,
    list: gtk::ListBox,
    status: gtk::Label,
    found: RefCell<Vec<VfsPath>>,
    /// How many results were found beyond what the list will show.
    overflowed: std::cell::Cell<usize>,
    running: RefCell<Option<CancelToken>>,
    button: gtk::Button,
}

impl Search {
    /// Opens the window. `start` is asked for a stream of results; `accept` is
    /// handed the path of whichever result is chosen.
    pub fn open(
        parent: &impl IsA<gtk::Window>,
        start: impl Fn(Criteria, CancelToken) -> SearchResults + 'static,
        accept: impl Fn(VfsPath) + 'static,
    ) -> Rc<Search> {
        let (window, content) = shell(parent, TITLE_SEARCH);

        let name = gtk::Entry::builder()
            .text(SEARCH_NAME_DEFAULT)
            .width_chars(ENTRY_WIDTH_CHARS)
            .build();
        let text = gtk::Entry::builder().width_chars(ENTRY_WIDTH_CHARS).build();
        content.append(&label(PROMPT_SEARCH_NAME));
        content.append(&name);
        content.append(&label(PROMPT_SEARCH_CONTENT));
        content.append(&text);

        let list = gtk::ListBox::new();
        list.set_selection_mode(gtk::SelectionMode::Browse);
        let scroller = gtk::ScrolledWindow::builder()
            .child(&list)
            .propagate_natural_height(true)
            .max_content_height(SEARCH_LIST_HEIGHT)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build();
        content.append(&scroller);

        let status = gtk::Label::builder().xalign(XALIGN_LEFT).build();
        status.add_css_class(CLASS_DIM);
        content.append(&status);

        let row = button_row();
        let button = gtk::Button::with_label(SEARCH_START);
        button.add_css_class(CLASS_SUGGESTED);
        row.append(&button);
        content.append(&row);

        let search = Rc::new(Search {
            window: window.clone(),
            list: list.clone(),
            status,
            found: RefCell::new(Vec::new()),
            overflowed: std::cell::Cell::new(0),
            running: RefCell::new(None),
            button: button.clone(),
        });

        let accept = Rc::new(accept);
        let choosing = search.clone();
        list.connect_row_activated(move |_, row| {
            let Some(path) = choosing.found.borrow().get(row.index() as usize).cloned() else {
                return;
            };
            choosing.window.close();
            accept(path);
        });

        let starting = search.clone();
        let start = Rc::new(start);
        let (asking_name, asking_text) = (name.clone(), text.clone());
        button.connect_clicked(move |_| {
            // The same button stops a search that is running: a search and its
            // cancel are the same act from the user's side, and two buttons
            // where one will do is one more thing to look at.
            if starting.stop() {
                return;
            }
            let criteria = Criteria {
                name: asking_name.text().to_string(),
                content: asking_text.text().to_string(),
            };
            starting.begin(&start, criteria);
        });

        // Enter in either field starts the search, so nobody has to reach for
        // the button to do the only thing this window is for.
        for entry in [&name, &text] {
            let button = button.clone();
            entry.connect_activate(move |_| button.emit_clicked());
        }

        window.present();
        name.grab_focus();
        name.select_region(0, -1);
        search
    }

    /// Starts a search, replacing whatever the list held.
    fn begin(
        self: &Rc<Self>,
        start: &Rc<impl Fn(Criteria, CancelToken) -> SearchResults>,
        criteria: Criteria,
    ) {
        self.list.remove_all();
        self.found.borrow_mut().clear();
        self.overflowed.set(0);
        let cancel = CancelToken::new();
        *self.running.borrow_mut() = Some(cancel.clone());
        self.button.set_label(SEARCH_STOP);
        self.update_status(true);

        let results = start(criteria, cancel);
        let filling = self.clone();
        glib::spawn_future_local(async move {
            // The channel closes when the walk ends, however it ended.
            while let Ok(path) = results.recv().await {
                filling.push(path);
                // And whatever else is already waiting, before going back to
                // the main loop. A search over a source tree finds thousands,
                // and one turn of the loop each would spend more time being
                // scheduled than searching.
                while let Ok(path) = results.try_recv() {
                    filling.push(path);
                }
                filling.update_status(true);
            }
            filling.finish();
        });
    }

    /// Stops a running search, and says whether there was one.
    fn stop(&self) -> bool {
        let Some(cancel) = self.running.borrow_mut().take() else {
            return false;
        };
        cancel.cancel();
        self.button.set_label(SEARCH_START);
        self.update_status(false);
        true
    }

    fn push(&self, path: VfsPath) {
        let shown = self.found.borrow().len();
        // Past the cap the count still climbs but the list does not: a
        // `ListBox` is not virtualised, so a hundred thousand results would be
        // a hundred thousand widgets and would take the window down with them.
        if shown >= SEARCH_LIST_LIMIT {
            self.overflowed.set(self.overflowed.get() + 1);
            return;
        }
        self.list.append(
            &gtk::Label::builder()
                .label(path.as_str())
                .xalign(XALIGN_LEFT)
                .ellipsize(gtk::pango::EllipsizeMode::Start)
                .build(),
        );
        self.found.borrow_mut().push(path);
        // The first result takes the keyboard, so Enter goes to it rather than
        // back to the field it was typed in — which would start the search
        // again and look like the window ignoring you.
        if shown == 0 {
            if let Some(first) = self.list.row_at_index(0) {
                self.list.select_row(Some(&first));
                first.grab_focus();
            }
        }
    }

    fn finish(&self) {
        *self.running.borrow_mut() = None;
        self.button.set_label(SEARCH_START);
        self.update_status(false);
    }

    fn update_status(&self, searching: bool) {
        let shown = self.found.borrow().len();
        let count = shown + self.overflowed.get();
        let text = if count > shown {
            SEARCH_CAPPED
                .replace("{count}", &count.to_string())
                .replace("{shown}", &shown.to_string())
        } else {
            match (searching, count) {
                (false, 0) => SEARCH_NOTHING.to_string(),
                (true, _) => SEARCH_SEARCHING.replace("{count}", &count.to_string()),
                (false, _) => SEARCH_FOUND.replace("{count}", &count.to_string()),
            }
        };
        self.status.set_text(&text);
    }
}

/// A left-aligned prompt above a field.
fn label(text: &str) -> gtk::Label {
    gtk::Label::builder()
        .label(text)
        .xalign(XALIGN_LEFT)
        .build()
}

/// The multi-rename tool: rules above, a live preview below.
///
/// The preview is [`tc_core::rename::preview`] and so is the rename — the same
/// call, so what is read and what runs cannot drift apart
/// (`docs/multi-rename.md`).
pub struct MultiRename {
    window: gtk::Window,
    list: gtk::Box,
    names: Vec<String>,
    rules: RefCell<Rules>,
}

impl MultiRename {
    /// Opens the tool on `names`. `apply` is handed the rows that would change
    /// something, once the user says so.
    pub fn open(
        parent: &impl IsA<gtk::Window>,
        names: Vec<String>,
        apply: impl Fn(Vec<Renamed>) + 'static,
    ) -> Rc<MultiRename> {
        let (window, content) = shell(parent, TITLE_RENAME);

        let template = gtk::Entry::builder()
            .text(&Rules::default().template)
            .width_chars(ENTRY_WIDTH_CHARS)
            .build();
        let counter = gtk::Entry::builder()
            .text(Rules::default().counter_start.to_string())
            .build();
        let find = gtk::Entry::builder().width_chars(ENTRY_WIDTH_CHARS).build();
        let with = gtk::Entry::builder().width_chars(ENTRY_WIDTH_CHARS).build();

        content.append(&label(PROMPT_RENAME_TEMPLATE));
        content.append(&template);
        content.append(&label(PROMPT_RENAME_COUNTER));
        content.append(&counter);
        content.append(&label(PROMPT_RENAME_FIND));
        content.append(&find);
        content.append(&label(PROMPT_RENAME_WITH));
        content.append(&with);

        let list = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();
        let scroller = gtk::ScrolledWindow::builder()
            .child(&list)
            .propagate_natural_height(true)
            .max_content_height(RENAME_LIST_HEIGHT)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build();
        content.append(&scroller);

        let row = button_row();
        let cancel = gtk::Button::with_label(BUTTON_CANCEL);
        let rename = gtk::Button::with_label(BUTTON_RENAME);
        rename.add_css_class(CLASS_SUGGESTED);
        row.append(&cancel);
        row.append(&rename);
        content.append(&row);

        let tool = Rc::new(MultiRename {
            window: window.clone(),
            list,
            names,
            rules: RefCell::new(Rules::default()),
        });

        // Every field redraws the preview as it is typed: seeing the answer
        // while writing the question is the whole point of the tool.
        let fields = [&template, &counter, &find, &with];
        for entry in fields {
            let watching = tool.clone();
            let (template, counter, find, with) = (
                template.clone(),
                counter.clone(),
                find.clone(),
                with.clone(),
            );
            entry.connect_changed(move |_| {
                watching.reread(&template, &counter, &find, &with);
            });
        }

        let closing = window.clone();
        cancel.connect_clicked(move |_| closing.close());

        let applying = tool.clone();
        let apply = Rc::new(apply);
        rename.connect_clicked(move |_| {
            let rows = applying.rows();
            applying.window.close();
            apply(rows.into_iter().filter(Renamed::changes_anything).collect());
        });

        // Enter anywhere in the rules means Rename, the way Enter means OK in
        // every other dialog here. The preview is already on screen by the
        // time anybody can press it, so this commits to something read rather
        // than to something guessed.
        for entry in [&template, &counter, &find, &with] {
            let button = rename.clone();
            entry.connect_activate(move |_| button.emit_clicked());
        }

        tool.redraw();
        window.present();
        template.grab_focus();
        // Selected, so typing a template replaces the default instead of
        // appending to it — as in every other prefilled dialog.
        template.select_region(0, -1);
        tool
    }

    /// Takes the rules out of the fields and redraws.
    ///
    /// A counter start that is not a number keeps the last one that was: a
    /// preview that emptied itself while somebody was halfway through typing
    /// `10` would be unusable.
    fn reread(
        &self,
        template: &gtk::Entry,
        counter: &gtk::Entry,
        find: &gtk::Entry,
        with: &gtk::Entry,
    ) {
        let previous = self.rules.borrow().counter_start;
        *self.rules.borrow_mut() = Rules {
            template: template.text().to_string(),
            find: find.text().to_string(),
            replace: with.text().to_string(),
            counter_start: counter.text().parse().unwrap_or(previous),
        };
        self.redraw();
    }

    fn rows(&self) -> Vec<Renamed> {
        preview(&self.rules.borrow(), &self.names)
    }

    fn redraw(&self) {
        while let Some(child) = self.list.first_child() {
            self.list.remove(&child);
        }
        for row in self.rows() {
            let text = match row.refused {
                Some(Refusal::Empty) => RENAME_EMPTY.to_string(),
                Some(Refusal::Collides) => RENAME_COLLIDES.to_string(),
                Some(Refusal::HasSeparator) => RENAME_SEPARATOR.to_string(),
                None if row.from == row.to => RENAME_UNCHANGED.to_string(),
                None => row.to.clone(),
            };
            let line = RENAME_ROW
                .replace("{from}", &row.from)
                .replace("{to}", &text);
            let widget = gtk::Label::builder()
                .label(line)
                .xalign(XALIGN_LEFT)
                .ellipsize(gtk::pango::EllipsizeMode::Middle)
                .build();
            // A row that will not be renamed is dimmed, so the eye finds the
            // ones that will without reading every line.
            if !row.changes_anything() {
                widget.add_css_class(CLASS_DIM);
            }
            self.list.append(&widget);
        }
    }
}
