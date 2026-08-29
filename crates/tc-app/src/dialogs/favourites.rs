//! The `Ctrl+D` window: the directories worth keeping, and getting to one.
//!
//! A modal window rather than the dropdown Total Commander uses, for the
//! reason the drive list is one: a GTK popover is not a window the end-to-end
//! suite can find or send keys to, and a chooser that cannot be tested through
//! a real key press is exactly the kind of thing that ships broken
//! ([`docs/ui-shell.md`]).

use gtk::prelude::*;

use tc_core::config::Favourite;
use tc_core::vfs::VfsPath;

use crate::constants::{CLASS_DIM, FAVOURITES_EMPTY, TITLE_FAVOURITES};

use super::{label, labelled_row, list_scroller, shell};

/// Offers `favourites` and calls `go` with the one that was chosen.
///
/// Keyboard-first, like every chooser here: the list opens focused with the
/// first row selected, the arrows walk it, Enter takes it and Escape leaves
/// without choosing.
pub fn open(
    parent: &impl IsA<gtk::Window>,
    favourites: &[Favourite],
    go: impl Fn(VfsPath) + 'static,
) {
    let (window, content) = shell(parent, TITLE_FAVOURITES);

    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::Browse);
    for favourite in favourites {
        list.append(&labelled_row(&favourite.label(), &favourite.path));
    }
    // A modal window holding nothing at all reads as broken rather than as
    // empty, and "there is nothing here yet" is the one thing a first-time
    // list has to say.
    if favourites.is_empty() {
        let empty = label(FAVOURITES_EMPTY);
        empty.add_css_class(CLASS_DIM);
        list.append(&empty);
    }
    content.append(&list_scroller(&list));

    // By index rather than by widget: two favourites may carry the same name,
    // and the index is what actually identifies the choice.
    let targets: Vec<VfsPath> = favourites.iter().map(Favourite::path).collect();
    let closing = window.clone();
    list.connect_row_activated(move |_, row| {
        let Some(target) = targets.get(row.index() as usize).cloned() else {
            return;
        };
        closing.close();
        go(target);
    });

    window.present();
}
