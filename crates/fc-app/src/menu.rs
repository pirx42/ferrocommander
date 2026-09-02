//! The context menu: the action table as a popover, on the row it is about.
//!
//! Nothing here decides anything. An entry is a name from `constants::ROW_MENU`
//! — the same name a `[keys]` line would use — and choosing one goes through
//! `keymap::action_named` and then `dispatch`, exactly the route a key takes.
//! So the menu cannot do what a key cannot, it acts on what a key would act
//! on (everything marked, or the cursor row — `jobs::sources`), and the
//! shortcut shown beside each entry is read from the keymap the user built.
//!
//! What the menu *is* differs by platform ([`docs/ui-shell.md`]): on Windows
//! Explorer's own menu takes this one's place for anything with a path on
//! the disk. That half is not here yet.

use std::cell::RefCell;
use std::rc::Rc;

use gtk::prelude::*;
use gtk::{gdk, gio, glib};

use crate::actions::dispatch;
use crate::constants::{BACKGROUND_MENU, MENU_ACTION_GROUP, MENU_ACTION_RUN, ROW_MENU};
use crate::keymap::{action_named, Keymap};
use crate::shell::Shell;

/// The menu model's attribute a popover reads the shortcut to show from.
const ACCEL_ATTRIBUTE: &str = "accel";

/// Opens the row menu on the active pane's cursor row — `Shift+F10`, `Menu`,
/// and a held right button.
pub(crate) fn open_for_cursor_row(shell: &Rc<RefCell<Shell>>) {
    let (rows, at, model) = {
        let state = shell.borrow();
        let rows = state.panes[state.active].rows().clone();
        let at = cursor_row_bounds(state.window().as_ref(), &rows);
        (rows, at, build(ROW_MENU, &state.keymap))
    };
    show(shell, &rows, at, model);
}

/// Opens the background menu on the active pane, at a point of its rows
/// widget — where the right button landed on empty space.
pub(crate) fn open_background_at(shell: &Rc<RefCell<Shell>>, x: f64, y: f64) {
    let (rows, model) = {
        let state = shell.borrow();
        let rows = state.panes[state.active].rows().clone();
        (rows, build(BACKGROUND_MENU, &state.keymap))
    };
    let at = gdk::Rectangle::new(x as i32, y as i32, 1, 1);
    show(shell, &rows, at, model);
}

/// Where the cursor row is, in the rows widget's own coordinates.
///
/// The cursor row is the focused one: `sync_cursor` scrolls with `FOCUS`,
/// so the widget GTK reports as focused is that row or a cell in it, and
/// its bounds are the answer. Asking the focus rather than computing a row's
/// place from its index means no arithmetic here has to agree with the
/// row height. With nothing focused inside the rows — an empty pane — the
/// menu opens at the top-left corner, which is where an empty pane's first
/// row would be.
fn cursor_row_bounds(
    window: Option<&gtk::ApplicationWindow>,
    rows: &gtk::ColumnView,
) -> gdk::Rectangle {
    let origin = gdk::Rectangle::new(0, 0, 1, 1);
    let Some(focused) = window.and_then(GtkWindowExt::focus) else {
        return origin;
    };
    if !focused.is_ancestor(rows) {
        return origin;
    }
    match focused.compute_bounds(rows) {
        Some(bounds) => gdk::Rectangle::new(
            bounds.x() as i32,
            bounds.y() as i32,
            bounds.width() as i32,
            bounds.height() as i32,
        ),
        None => origin,
    }
}

/// The menu model: every entry of `sections`, each carrying the name it
/// stands for and the key that reaches it.
fn build(sections: &[&[(&str, &str)]], keymap: &Keymap) -> gio::Menu {
    let menu = gio::Menu::new();
    let target = format!("{MENU_ACTION_GROUP}.{MENU_ACTION_RUN}");
    for section in sections {
        let part = gio::Menu::new();
        for (label, name) in section.iter() {
            let item = gio::MenuItem::new(Some(label), None);
            item.set_action_and_target_value(Some(&target), Some(&name.to_variant()));
            if let Some(accel) =
                action_named(name).and_then(|action| keymap.accelerator_for(action))
            {
                item.set_attribute_value(ACCEL_ATTRIBUTE, Some(&accel.to_variant()));
            }
            part.append_item(&item);
        }
        menu.append_section(None, &part);
    }
    menu
}

/// Pops `model` up over `parent`, pointing at `at`.
///
/// One action for the whole menu, taking the entry's name: the entries are a
/// table of names already, and a `SimpleAction` per row would be that table
/// written out a second time. The popover is unparented once it has closed —
/// on an idle, because GTK is still inside the popover's own signal at that
/// moment — so a menu leaves nothing behind in the widget tree.
fn show(
    shell: &Rc<RefCell<Shell>>,
    parent: &gtk::ColumnView,
    at: gdk::Rectangle,
    model: gio::Menu,
) {
    let run = gio::SimpleAction::new(MENU_ACTION_RUN, Some(glib::VariantTy::STRING));
    let running = shell.clone();
    run.connect_activate(move |_, name| {
        let Some(action) = name.and_then(|name| name.str()).and_then(action_named) else {
            return;
        };
        dispatch(&running, action);
    });
    let group = gio::SimpleActionGroup::new();
    group.add_action(&run);
    parent.insert_action_group(MENU_ACTION_GROUP, Some(&group));

    let popover = gtk::PopoverMenu::from_model(Some(&model));
    popover.set_parent(parent);
    popover.set_pointing_to(Some(&at));
    popover.set_has_arrow(false);
    popover.connect_closed(|popover| {
        let popover = popover.clone();
        glib::idle_add_local_once(move || popover.unparent());
    });
    popover.popup();
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The table's whole reason for being names rather than actions: a
    /// name that reaches nothing is a menu entry that does nothing, and this
    /// is what turns that into a failing test instead of a dead row.
    #[test]
    fn every_menu_entry_names_an_action() {
        for section in ROW_MENU.iter().chain(BACKGROUND_MENU) {
            for (label, name) in section.iter() {
                assert!(
                    action_named(name).is_some(),
                    "menu entry {label:?} names {name:?}, which is no action"
                );
            }
        }
    }

    #[test]
    fn the_model_holds_every_entry_in_its_section_with_its_key() {
        // No `gtk::init()`: a menu *model* is GIO's and needs no display,
        // which is what lets this run headless with the rest of the crate.
        let keymap = Keymap::default();

        let menu = build(ROW_MENU, &keymap);

        assert_eq!(
            menu.n_items() as usize,
            ROW_MENU.len(),
            "one section per group"
        );
        let copy_section = menu
            .item_link(1, gio::MENU_LINK_SECTION)
            .expect("the second section");
        assert_eq!(copy_section.n_items() as usize, ROW_MENU[1].len());
        // The shortcut is the model's business to carry, or the popover has
        // nothing to show — and it is F5 because that is what the default
        // keymap says, not because this test says so.
        let accel = copy_section
            .item_attribute_value(0, ACCEL_ATTRIBUTE, Some(glib::VariantTy::STRING))
            .expect("the copy entry carries its key");
        assert_eq!(
            accel.str(),
            keymap
                .accelerator_for(crate::keymap::Action::Copy)
                .as_deref()
        );
    }
}
