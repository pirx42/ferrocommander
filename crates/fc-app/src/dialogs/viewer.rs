//! The F3 viewer: a window over a file it never reads whole.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use gtk::gdk::Key;
use gtk::glib;
use gtk::prelude::*;

use fc_core::vfs::VirtualFs;
use fc_core::viewer::{Mode, View};

use crate::constants::{
    CLASS_OUTPUT, SCROLL_RESTORE_PRIORITY, SCROLL_ROOM_EPSILON, TITLE_VIEWER, VIEWER_EMPTY,
    VIEWER_HEIGHT, VIEWER_WIDTH, XALIGN_LEFT,
};

/// A window over one file: text or hex, paged by keys.
///
/// Its own window rather than a modal dialog: reading a file is not answering
/// a question, and the shell behind it stays usable.
///
/// Everything about *what* is shown is [`fc_core::viewer`] — this holds the
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
            Key::Page_Down | Key::space => return self.page(1),
            Key::Page_Up => return self.page(-1),
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

    /// A page key: the *screen* first, the file second.
    ///
    /// The viewer holds a position in the file and reads one 64 KiB window
    /// around it — and knows nothing about how much of that window fits on
    /// screen. A short file is one window, so there is no page to turn, while
    /// the label holding it is still several screens tall: paging the file
    /// alone left a screenful nobody could reach, and this is the half of the
    /// third testing round's report that the engine could not fix.
    ///
    /// So the label is scrolled while it has room, and only when it runs out
    /// does the offset move. A file that has neither room nor another window
    /// stays exactly where it is, which is what a key at the end of a
    /// document should do.
    fn page(&self, pages: i64) -> glib::Propagation {
        let adjustment = self.scroller.vadjustment();
        if super::room(&adjustment, pages > 0) > SCROLL_ROOM_EPSILON {
            super::page(&adjustment, pages as f64);
            return glib::Propagation::Stop;
        }

        let before = self.view.borrow().offset();
        self.view.borrow_mut().scroll_window(pages);
        if self.view.borrow().offset() == before {
            return glib::Propagation::Stop;
        }
        self.redraw();
        // Forward lands at the top of the new window, which `redraw` has
        // already done. Backward has to land at its *bottom*, or a page back
        // would skip everything between the top of the window just left and
        // the top of the one before it.
        if pages < 0 {
            let landing = adjustment.clone();
            glib::idle_add_local_full(glib::Priority::from(SCROLL_RESTORE_PRIORITY), move || {
                landing.set_value(super::bottom(&landing));
                glib::ControlFlow::Break
            });
        }
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
