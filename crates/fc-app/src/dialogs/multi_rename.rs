//! The Ctrl+M tool: rules above, a live preview below.

use std::cell::RefCell;
use std::rc::Rc;

use gtk::prelude::*;

use fc_core::rename::{preview, Refusal, Renamed, Rules};

use crate::constants::{
    BUTTON_CANCEL, BUTTON_RENAME, CLASS_DIM, CLASS_SUGGESTED, ENTRY_WIDTH_CHARS,
    PROMPT_RENAME_COUNTER, PROMPT_RENAME_FIND, PROMPT_RENAME_TEMPLATE, PROMPT_RENAME_WITH,
    RENAME_COLLIDES, RENAME_EMPTY, RENAME_LIST_HEIGHT, RENAME_ROW, RENAME_SEPARATOR,
    RENAME_UNCHANGED, TITLE_RENAME, XALIGN_LEFT,
};

use super::{button_row, label, shell};

/// The multi-rename tool: rules above, a live preview below.
///
/// The preview is [`fc_core::rename::preview`] and so is the rename — the same
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
