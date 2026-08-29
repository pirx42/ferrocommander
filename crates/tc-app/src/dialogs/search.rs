//! The Alt+F7 window: results arriving as they are found, and one button to
//! start and stop.

use std::cell::RefCell;
use std::rc::Rc;

use gtk::glib;
use gtk::prelude::*;

use tc_core::ops::CancelToken;
use tc_core::search::{Criteria, Results as SearchResults};
use tc_core::vfs::VfsPath;

use crate::constants::{
    CLASS_DIM, CLASS_SUGGESTED, ENTRY_WIDTH_CHARS, PROMPT_SEARCH_CONTENT, PROMPT_SEARCH_NAME,
    SEARCH_CAPPED, SEARCH_FOUND, SEARCH_LIST_HEIGHT, SEARCH_LIST_LIMIT, SEARCH_NAME_DEFAULT,
    SEARCH_NOTHING, SEARCH_SEARCHING, SEARCH_START, SEARCH_STOP, TITLE_SEARCH, XALIGN_LEFT,
};

use super::{button_row, label, shell};

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
