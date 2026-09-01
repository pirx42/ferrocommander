//! Looking inside a file without holding it.

use fc_core::vfs::{LocalFs, VfsPath};
use fc_core::viewer::{decode, hex_dump, Mode, View, HEX_COLUMNS, WINDOW_BYTES};
use tempfile::TempDir;

/// A file of exactly these bytes, and a view on it.
fn viewing(contents: &[u8]) -> (TempDir, View) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("subject");
    std::fs::write(&path, contents).unwrap();
    let view = View::open(&LocalFs, VfsPath::new(path.to_str().unwrap())).unwrap();
    (dir, view)
}

#[test]
fn opening_reads_nothing_but_the_size() {
    // The whole design in one assertion: a viewer that read the file to open
    // it could not open a disk image.
    let (_dir, view) = viewing(&vec![b'x'; 10_000]);

    assert_eq!(view.size(), 10_000);
    assert_eq!(view.offset(), 0);
}

#[test]
fn a_file_larger_than_the_window_still_opens_at_the_top() {
    let (_dir, view) = viewing(&vec![b'x'; WINDOW_BYTES * 4]);

    assert_eq!(view.offset(), 0);
    assert!(view.render(&LocalFs).unwrap().len() <= WINDOW_BYTES);
}

#[test]
fn scrolling_down_a_line_lands_after_the_newline() {
    let (_dir, mut view) = viewing(b"first\nsecond\nthird\n");

    view.scroll_lines(&LocalFs, 1);
    assert!(view.render(&LocalFs).unwrap().starts_with("second"));

    view.scroll_lines(&LocalFs, 1);
    assert!(view.render(&LocalFs).unwrap().starts_with("third"));
}

#[test]
fn scrolling_up_undoes_scrolling_down() {
    // The invariant worth having: down then up is where you were, whatever
    // the lines look like.
    let (_dir, mut view) = viewing(b"alpha\nbeta\ngamma\ndelta\nepsilon\n");

    for steps in 1..=4 {
        view.to_start();
        view.scroll_lines(&LocalFs, steps);
        let there = view.offset();
        view.scroll_lines(&LocalFs, -steps);
        assert_eq!(view.offset(), 0, "down {steps} and back up");

        view.scroll_lines(&LocalFs, steps);
        assert_eq!(view.offset(), there, "and down again");
    }
}

#[test]
fn the_top_of_the_file_is_the_end_of_scrolling_up() {
    let (_dir, mut view) = viewing(b"one\ntwo\n");

    view.scroll_lines(&LocalFs, -10);

    assert_eq!(view.offset(), 0);
}

#[test]
fn a_line_longer_than_a_window_still_moves() {
    // A file with no newline in it at all: "a line down" can only mean a
    // window, and a viewer that refused to move would be stuck at the top of
    // a minified file forever.
    let (_dir, mut view) = viewing(&vec![b'x'; WINDOW_BYTES * 3]);

    view.scroll_lines(&LocalFs, 1);

    assert_eq!(view.offset(), WINDOW_BYTES as u64);
}

#[test]
fn paging_stops_at_the_end_rather_than_past_it() {
    let (_dir, mut view) = viewing(b"short file\n");

    view.scroll_window(1000);

    assert!(
        view.offset() <= view.size(),
        "{} > {}",
        view.offset(),
        view.size()
    );
    // And there is still something to look at.
    assert!(!view.render(&LocalFs).unwrap().is_empty() || view.size() == 0);
}

#[test]
fn an_empty_file_opens_and_shows_nothing() {
    let (_dir, mut view) = viewing(b"");

    assert_eq!(view.render(&LocalFs).unwrap(), "");
    view.scroll_lines(&LocalFs, 1);
    view.scroll_window(1);
    view.to_end();
    assert_eq!(view.offset(), 0, "nowhere to go in an empty file");
}

#[test]
fn text_that_is_utf8_is_read_as_utf8() {
    // The detection is the validity test: the sequences that make a file valid
    // UTF-8 do not happen by accident.
    assert_eq!(
        decode("Grüße, Ferrocommander".as_bytes()),
        "Grüße, Ferrocommander"
    );
}

#[test]
fn text_that_is_not_utf8_is_read_as_latin1_rather_than_refused() {
    // 0xFF is not valid UTF-8 anywhere. Latin-1 maps every byte to some
    // character, so this cannot fail — the honest answer for a file whose
    // encoding nobody recorded.
    let decoded = decode(b"caf\xe9 \xff");

    assert_eq!(decoded, "café ÿ");
}

#[test]
fn a_hex_dump_lines_up_and_says_where_it_is() {
    let dump = hex_dump(0x1000, b"AB\x00\xff");
    let line = dump.lines().next().unwrap();

    assert!(line.starts_with("00001000  "), "{line}");
    assert!(line.contains("41 42 00 ff"), "{line}");
    // Two printable, two not.
    assert!(line.ends_with("AB.."), "{line}");
}

#[test]
fn a_short_last_row_still_lines_its_text_up_with_the_rows_above() {
    // The padding earns its place here: without it the text column of the
    // last row slides left and stops being a column.
    let dump = hex_dump(0, &[b'A'; HEX_COLUMNS + 3]);
    let lines: Vec<&str> = dump.lines().collect();
    // Eight offset digits, two spaces, three characters per byte column, one
    // space. Spelled out rather than searched for, so the test fails when the
    // layout changes instead of quietly following it.
    let text_starts = 8 + 2 + HEX_COLUMNS * 3 + 1;

    assert_eq!(lines.len(), 2);
    assert_eq!(&lines[0][text_starts..], &"A".repeat(HEX_COLUMNS));
    assert_eq!(&lines[1][text_starts..], "AAA", "the short row slid left");
}

#[test]
fn hex_mode_shows_the_same_bytes_the_offset_is_at() {
    let (_dir, mut view) = viewing(b"0123456789abcdef0123456789abcdef");
    view.set_mode(Mode::Hex);
    view.scroll_window(0);

    let dump = view.render(&LocalFs).unwrap();

    assert!(dump.starts_with("00000000  30 31 32"), "{dump}");
    assert_eq!(view.mode(), Mode::Hex);
}

/// A file shorter than half a read window — the case the offset arithmetic
/// never had, and the one the third testing round reported. Three hundred
/// short lines: too small to page, far too tall for one screen.
fn short_file() -> (TempDir, View) {
    let body: String = (0..300).map(|i| format!("line {i:04}\n")).collect();
    viewing(body.as_bytes())
}

#[test]
fn paging_a_short_file_never_walks_backwards() {
    // It did: `last_page` is zero when the file is shorter than half a
    // window, and the clamp to it dragged the offset to the top — so both
    // page keys jumped to the start of the file however far down you were.
    let (_dir, mut view) = short_file();
    view.scroll_lines(&LocalFs, 5);
    let five_lines_down = view.offset();
    assert!(five_lines_down > 0, "the arrows did not move");

    view.scroll_window(1);

    assert_eq!(
        view.offset(),
        five_lines_down,
        "a page forward moved the offset backwards"
    );
}

#[test]
fn a_page_back_from_the_top_of_a_short_file_stays_at_the_top() {
    let (_dir, mut view) = short_file();

    view.scroll_window(-1);

    assert_eq!(view.offset(), 0);
}

#[test]
fn paging_a_long_file_still_turns_pages() {
    // The guard must not cost the case that always worked.
    let (_dir, mut view) = viewing(&vec![b'x'; WINDOW_BYTES * 4]);

    view.scroll_window(1);
    let one_page = view.offset();
    view.scroll_window(1);

    assert!(one_page > 0, "a page forward moved nothing");
    assert!(view.offset() > one_page, "the second page went nowhere");
    view.scroll_window(-1);
    assert_eq!(view.offset(), one_page, "a page back did not undo it");
}
