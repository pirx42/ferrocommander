//! The `Ctrl+D` window: the directories worth keeping, and getting to one.
//!
//! A modal window rather than the dropdown Total Commander uses, for the
//! reason [`super::choose_one`] gives.
//!
//! **The list is maintained from inside itself**, which is why this is a
//! module and not another `choose_one`: that one hands back a value and
//! closes, and this window outlives its own choice — the rows change under
//! it. What the two share, they share as code: [`super::labelled_row`] and
//! [`super::list_scroller`].

use std::cell::RefCell;
use std::rc::Rc;

use gtk::gdk::Key;
use gtk::glib;
use gtk::prelude::*;

use fc_core::config::Favourite;
use fc_core::vfs::VfsPath;

use crate::constants::{CLASS_DIM, FAVOURITES_ADD, FAVOURITES_ADD_HINT, TITLE_FAVOURITES};

use super::{label, labelled_row, list_scroller, shell};

/// What the window needs done for it, since it knows nothing about panes or
/// settings files.
///
/// `add` and `remove` hand back the list as it now is rather than returning
/// nothing: the shell owns it, the window only draws it, and a window that
/// guessed at the result would be a second copy of the rules.
pub struct Hooks {
    /// Go to this directory. Closes the window.
    pub go: Box<dyn Fn(VfsPath)>,
    /// Keep where the active pane is, or say why not.
    pub add: Box<dyn Fn() -> Result<Vec<Favourite>, &'static str>>,
    /// Stop keeping this one.
    pub remove: Box<dyn Fn(VfsPath) -> Vec<Favourite>>,
}

/// Offers `favourites`, and lets them be added to and taken from.
///
/// Keyboard-first, like every chooser here: the list opens focused with the
/// first row selected, the arrows walk it, Enter takes it and Escape leaves
/// without choosing. `Delete` removes the row under the cursor, and the last
/// row keeps where the active pane is.
pub fn open(parent: &impl IsA<gtk::Window>, favourites: Vec<Favourite>, hooks: Hooks) {
    let (window, content) = shell(parent, TITLE_FAVOURITES);

    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::Browse);
    content.append(&list_scroller(&list));

    // Why it was refused, in the window where it was asked rather than in a
    // second modal on top of the first. Hidden until there is something to
    // say, so the window does not carry an empty line around.
    let refusal = label("");
    refusal.add_css_class(CLASS_DIM);
    refusal.set_visible(false);
    content.append(&refusal);

    let shown = Rc::new(RefCell::new(favourites));
    let hooks = Rc::new(hooks);
    fill(&list, &shown.borrow(), None);

    let closing = window.clone();
    let activated = Rc::clone(&shown);
    let acting = Rc::clone(&hooks);
    let saying = refusal.clone();
    let filling = list.clone();
    list.connect_row_activated(move |_, row| {
        let index = row.index() as usize;
        // By index rather than by widget: two favourites may carry the same
        // name, and the index is what actually identifies the choice.
        //
        // Read out and the borrow dropped before the hook runs, which is the
        // rule this crate keeps everywhere a callback can reach back in.
        let chosen = activated.borrow().get(index).map(Favourite::path);
        if let Some(target) = chosen {
            closing.close();
            (acting.go)(target);
            return;
        }
        // Past the end is the one command row.
        match (acting.add)() {
            Ok(list) => {
                // The last row, which is the one just added — or, when the
                // directory was already there and adding did nothing, the
                // last existing one. Landing on it either way is right: it
                // says "this is the row you meant".
                let added = list.len().saturating_sub(1);
                *activated.borrow_mut() = list;
                saying.set_visible(false);
                fill(&filling, &activated.borrow(), Some(added));
            }
            Err(reason) => {
                saying.set_text(reason);
                saying.set_visible(true);
            }
        }
    });

    let removing = Rc::clone(&shown);
    let acting = Rc::clone(&hooks);
    let selection = list.clone();
    let saying = refusal.clone();
    let controller = gtk::EventControllerKey::new();
    controller.connect_key_pressed(move |_, key, _, _| {
        if key != Key::Delete {
            return glib::Propagation::Proceed;
        }
        let Some(index) = selection.selected_row().map(|row| row.index() as usize) else {
            return glib::Propagation::Proceed;
        };
        // Delete on the command row is not a row to remove, and pressing it
        // there should do nothing rather than take the row above.
        let chosen = removing.borrow().get(index).map(Favourite::path);
        let Some(target) = chosen else {
            return glib::Propagation::Proceed;
        };
        // Into a local first: assigning through `borrow_mut()` would hold the
        // borrow while the hook runs, which is the shape this crate does not
        // write even where it happens to be safe today.
        let left = (acting.remove)(target);
        *removing.borrow_mut() = left;
        saying.set_visible(false);
        // The row that moved up into the gap, so holding Delete clears the
        // list from wherever it started rather than jumping to the top.
        fill(&selection, &removing.borrow(), Some(index));
        glib::Propagation::Stop
    });
    list.add_controller(controller);

    window.present();
}

/// Draws the list as it now is, and puts the cursor on `focus`.
///
/// Rebuilt rather than spliced: this runs when a person pressed a key on a
/// list of a dozen rows, so the cost is invisible and the alternative is
/// index arithmetic that has to agree with `connect_row_activated`'s.
fn fill(list: &gtk::ListBox, favourites: &[Favourite], focus: Option<usize>) {
    while let Some(row) = list.first_child() {
        list.remove(&row);
    }
    for favourite in favourites {
        list.append(&labelled_row(&favourite.label(), &favourite.path));
    }
    // The command row is also the empty state: a window whose only row says
    // "Add the current directory" needs no note explaining that the list is
    // empty. A note *as a row* would be worse than none, since every row past
    // the favourites is the command row as far as the activation handler is
    // concerned.
    list.append(&labelled_row(FAVOURITES_ADD, FAVOURITES_ADD_HINT));

    // Past the end after removing the last one, which is the command row and
    // a fine place to be left.
    let wanted = focus.unwrap_or(0).min(favourites.len());
    if let Some(row) = list.row_at_index(wanted as i32) {
        list.select_row(Some(&row));
        row.grab_focus();
    }
}
