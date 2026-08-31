//! The compare window: two files' rows side by side, differences marked.
//!
//! Everything about *what* the rows are is [`tc_core::compare`] — this
//! holds the two text views, the tags that tint them, and three keys.

use gtk::gdk::{Key, RGBA};
use gtk::glib;
use gtk::prelude::*;

use tc_core::compare::{Row, RowKind, Side};

use crate::constants::{
    CLASS_OUTPUT, COMPARE_HEIGHT, COMPARE_TINT_CHANGED, COMPARE_TINT_LEFT_ONLY,
    COMPARE_TINT_RIGHT_ONLY, COMPARE_TINT_SPAN, COMPARE_WIDTH, TAG_CHANGED, TAG_LEFT_ONLY,
    TAG_RIGHT_ONLY, TAG_SPAN,
};

/// Opens the side-by-side view over rows the engine already built.
///
/// Both text views sit in one box inside **one** scrolled window, so the
/// two sides share a viewport by construction — there is no second
/// adjustment to keep in sync, and no handler whose absence could let them
/// drift. The cost is that the views lay out their whole buffers at once;
/// the engine's ceiling bounds how much that can be.
pub fn open_compare(parent: &impl IsA<gtk::Window>, title: &str, rows: &[Row]) {
    let left = text_view();
    let right = text_view();
    fill(&left, rows, |row| row.left.as_ref());
    fill(&right, rows, |row| row.right.as_ref());

    let pair = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .homogeneous(true)
        .build();
    pair.append(&left);
    pair.append(&gtk::Separator::new(gtk::Orientation::Vertical));
    pair.append(&right);

    let scroller = gtk::ScrolledWindow::builder()
        .child(&pair)
        .vexpand(true)
        .hexpand(true)
        .build();

    let window = gtk::Window::builder()
        .transient_for(parent)
        .title(title)
        .default_width(COMPARE_WIDTH)
        .default_height(COMPARE_HEIGHT)
        .child(&scroller)
        .build();

    // Where each block of differences starts, for `n`/`p`. A block is a
    // maximal run of non-Same rows: what reads as one change, jumped to as
    // one change.
    let blocks = block_starts(rows);
    let position = std::cell::Cell::new(usize::MAX);

    let keys = gtk::EventControllerKey::new();
    keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    let closing = window.clone();
    keys.connect_key_pressed(move |_, key, _, _| match key {
        Key::Escape => {
            closing.close();
            glib::Propagation::Stop
        }
        Key::n | Key::p => {
            if blocks.is_empty() {
                return glib::Propagation::Stop;
            }
            // The first press of either key lands on the first block: before
            // any jump there is no "previous" to go back to.
            let next = match (key == Key::n, position.get()) {
                (_, usize::MAX) => 0,
                (true, at) => (at + 1).min(blocks.len() - 1),
                (false, at) => at.saturating_sub(1),
            };
            position.set(next);
            scroll_to_line(&left, &scroller, blocks[next]);
            glib::Propagation::Stop
        }
        _ => glib::Propagation::Proceed,
    });
    window.add_controller(keys);

    window.present();
}

fn text_view() -> gtk::TextView {
    let view = gtk::TextView::builder()
        .editable(false)
        .cursor_visible(false)
        .monospace(true)
        .hexpand(true)
        .build();
    view.add_css_class(CLASS_OUTPUT);
    view
}

/// Writes one side's column and tints it.
///
/// A row where this side is absent becomes an empty line, which is what
/// keeps the two columns aligned: same row count, same font, same height.
/// The tint follows the **row's** kind in both views — the filler opposite
/// a left-only line wears the left-only color too, so the hole reads as
/// part of the same change rather than as one of its own.
fn fill<'a>(view: &gtk::TextView, rows: &'a [Row], side: impl Fn(&'a Row) -> Option<&'a Side>) {
    let buffer = view.buffer();
    let table = buffer.tag_table();
    for (name, color) in [
        (TAG_LEFT_ONLY, COMPARE_TINT_LEFT_ONLY),
        (TAG_RIGHT_ONLY, COMPARE_TINT_RIGHT_ONLY),
        (TAG_CHANGED, COMPARE_TINT_CHANGED),
    ] {
        let tag = gtk::TextTag::new(Some(name));
        tag.set_paragraph_background_rgba(Some(&RGBA::parse(color).unwrap()));
        table.add(&tag);
    }
    let span = gtk::TextTag::new(Some(TAG_SPAN));
    span.set_background_rgba(Some(&RGBA::parse(COMPARE_TINT_SPAN).unwrap()));
    table.add(&span);

    let text: String = rows
        .iter()
        .map(|row| match side(row) {
            Some(side) => format!("{}\n", side.text),
            None => "\n".to_string(),
        })
        .collect();
    buffer.set_text(&text);

    for (line, row) in rows.iter().enumerate() {
        let line = line as i32;
        let tag = match row.kind {
            RowKind::Same => continue,
            RowKind::Changed => TAG_CHANGED,
            RowKind::LeftOnly => TAG_LEFT_ONLY,
            RowKind::RightOnly => TAG_RIGHT_ONLY,
        };
        let (Some(start), Some(end)) = (buffer.iter_at_line(line), buffer.iter_at_line(line + 1))
        else {
            continue;
        };
        buffer.apply_tag_by_name(tag, &start, &end);

        let Some(side) = side(row) else { continue };
        for range in &side.changed {
            // The engine's spans count chars, which is exactly what a text
            // iter's line offset counts — the audit removed the byte round
            // trip that used to sit here.
            let (Some(start), Some(end)) = (
                buffer.iter_at_line_offset(line, range.start as i32),
                buffer.iter_at_line_offset(line, range.end as i32),
            ) else {
                continue;
            };
            buffer.apply_tag_by_name(TAG_SPAN, &start, &end);
        }
    }
}

/// The first line of every run of non-Same rows.
fn block_starts(rows: &[Row]) -> Vec<i32> {
    let mut starts = Vec::new();
    let mut in_block = false;
    for (line, row) in rows.iter().enumerate() {
        match (row.kind == RowKind::Same, in_block) {
            (false, false) => {
                starts.push(line as i32);
                in_block = true;
            }
            (true, _) => in_block = false,
            _ => {}
        }
    }
    starts
}

/// Puts `line` at the top of the shared viewport.
///
/// Through the adjustment rather than `scroll_to_iter`, because the views
/// do not scroll themselves — the box around them does, and the box's
/// scroller only listens to its adjustment.
fn scroll_to_line(view: &gtk::TextView, scroller: &gtk::ScrolledWindow, line: i32) {
    let Some(iter) = view.buffer().iter_at_line(line) else {
        return;
    };
    let y = view.iter_location(&iter).y();
    scroller.vadjustment().set_value(f64::from(y));
}
