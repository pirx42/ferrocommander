//! End-to-end tests: real key presses, the real binary, real files.
//!
//! What they cover is the wiring no unit test can reach — a binding that
//! reaches the wrong action, a dialog that never opens, a button nothing
//! focuses, a job whose result never reaches the panes. Every assertion is on
//! the filesystem or on whether a window is there, never on a screenshot.
//!
//! See [`harness`] for what they need installed and why the setup looks the
//! way it does.

mod harness;

use std::path::Path;

use harness::App;

/// Contents of the file most tests move around, so an assertion can tell the
/// copy apart from whatever was at the destination.
const SOURCE_TEXT: &str = "hello from the source";

/// What a colliding target holds before the job touches it.
const EXISTING_TEXT: &str = "already here";

/// Titles the dialogs are found by. Kept here rather than shared with the
/// crate: a test that reads its expectations out of the code under test can
/// only ever agree with it.
const DIALOG_COPY: &str = "Copy";
const DIALOG_MOVE: &str = "Move / Rename";
const DIALOG_NEW_DIR: &str = "New directory";
const DIALOG_DELETE: &str = "Confirm delete";
const DIALOG_CONFLICT: &str = "Target already exists";
const DIALOG_FAILURES: &str = "Some items were not processed";
const DIALOG_PATTERN: &str = "Select by pattern";

/// A home with `src/` to work in and `dst/` to land in.
fn arrange(home: &Path) {
    use std::fs;
    fs::create_dir_all(home.join("src/nested")).unwrap();
    fs::create_dir(home.join("dst")).unwrap();
    fs::write(home.join("src/notes.txt"), SOURCE_TEXT).unwrap();
    fs::write(home.join("src/data.bin"), vec![9u8; 4096]).unwrap();
    fs::write(home.join("src/nested/inner.txt"), "deep").unwrap();
}

/// Same, with something already at the destination.
fn arrange_with_collision(home: &Path) {
    arrange(home);
    std::fs::write(home.join("dst/notes.txt"), EXISTING_TEXT).unwrap();
}

/// Starts the app and puts the left pane in `src` and the right pane in `dst`.
///
/// Both panes open at the home directory, whose rows are `..`, `dst`, `src` —
/// directories first, then alphabetical.
fn in_src_and_dst(arrange: impl FnOnce(&Path)) -> App {
    let app = App::launch(arrange);
    // Right pane into dst.
    app.keys(&["Tab", "Down", "Return"]);
    // Back to the left pane, which is still on `..`, and into src.
    app.keys(&["Tab", "Down", "Down", "Return"]);
    app
}

/// Puts the cursor on `notes.txt`.
///
/// `src` lists `..`, `nested`, `data.bin`, `notes.txt`: the parent row, then
/// the directory, then the files by name.
fn cursor_on_notes(app: &App) {
    app.keys(&["Home", "Down", "Down", "Down"]);
}

#[test]
fn f7_creates_a_directory_in_the_active_panes_directory() {
    let app = in_src_and_dst(arrange);

    app.key("F7");
    app.focus_dialog(DIALOG_NEW_DIR);
    app.type_text("made-by-f7");
    app.key("Return");

    app.await_exists("src/made-by-f7");
}

#[test]
fn f5_copies_the_cursor_entry_into_the_other_pane() {
    // The prefilled target is the other pane's directory, so accepting the
    // dialog unchanged is the ordinary copy.
    let app = in_src_and_dst(arrange);
    cursor_on_notes(&app);

    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    app.await_contents("dst/notes.txt", SOURCE_TEXT);
    app.await_contents("src/notes.txt", SOURCE_TEXT);
}

#[test]
fn f5_copies_a_whole_directory_tree() {
    let app = in_src_and_dst(arrange);
    // Row 1 is `nested`.
    app.keys(&["Home", "Down"]);

    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    app.await_contents("dst/nested/inner.txt", "deep");
}

#[test]
fn f5_to_a_bare_name_duplicates_beside_the_source() {
    // The target field's rule: no separator means "beside the source, under
    // this name". This is how a file is duplicated.
    let app = in_src_and_dst(arrange);
    cursor_on_notes(&app);

    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.type_text("copy.txt");
    app.key("Return");

    app.await_contents("src/copy.txt", SOURCE_TEXT);
    app.await_contents("src/notes.txt", SOURCE_TEXT);
}

#[test]
fn f6_renames_the_cursor_entry_in_place() {
    // A rename is a move whose target is a bare name — there is no separate
    // rename command, so this is the only thing that proves it works.
    let app = in_src_and_dst(arrange);
    cursor_on_notes(&app);

    app.key("F6");
    app.focus_dialog(DIALOG_MOVE);
    app.type_text("renamed.txt");
    app.key("Return");

    app.await_contents("src/renamed.txt", SOURCE_TEXT);
    app.await_gone("src/notes.txt");
}

#[test]
fn f6_moves_the_cursor_entry_into_the_other_pane() {
    let app = in_src_and_dst(arrange);
    cursor_on_notes(&app);

    app.key("F6");
    app.focus_dialog(DIALOG_MOVE);
    app.key("Return");

    app.await_contents("dst/notes.txt", SOURCE_TEXT);
    app.await_gone("src/notes.txt");
}

#[test]
fn f8_moves_the_entry_to_the_trash_where_it_stays_recoverable() {
    // Recoverability is the entire difference from a permanent delete, so the
    // test checks the file arrived in the trash rather than only that it left.
    let app = in_src_and_dst(arrange);
    cursor_on_notes(&app);

    app.key("F8");
    app.focus_dialog(DIALOG_DELETE);
    // Delete is the focused button: this question is not the final one.
    app.key("space");

    app.await_gone("src/notes.txt");
    app.await_exists(".local/share/Trash/files/notes.txt");
}

#[test]
fn shift_delete_deletes_for_good_and_opens_on_the_safe_button() {
    // The dangerous question opens with Cancel focused, so activating the
    // focused button must *not* delete anything. Reaching Delete takes a
    // deliberate move — which is the whole point of the design.
    let app = in_src_and_dst(arrange);
    cursor_on_notes(&app);

    app.key("shift+Delete");
    app.focus_dialog(DIALOG_DELETE);
    app.key("Tab");
    app.key("space");

    app.await_gone("src/notes.txt");
    assert!(
        !app.path(".local/share/Trash/files/notes.txt").exists(),
        "a permanent delete must not go through the trash"
    );
}

#[test]
fn a_collision_asks_before_touching_anything() {
    let app = in_src_and_dst(arrange_with_collision);
    cursor_on_notes(&app);

    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    app.focus_dialog(DIALOG_CONFLICT);
    assert_eq!(
        std::fs::read_to_string(app.path("dst/notes.txt")).unwrap(),
        EXISTING_TEXT,
        "nothing may be written while the question is still open"
    );
}

#[test]
fn a_collision_answered_with_skip_leaves_both_sides_alone() {
    let app = in_src_and_dst(arrange_with_collision);
    cursor_on_notes(&app);

    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");
    app.focus_dialog(DIALOG_CONFLICT);
    // Skip is the focused button: the answer that loses nothing.
    app.key("space");

    // That the dialog closed is what proves the answer was delivered — the
    // contents below would be unchanged by an answer that never arrived, too.
    app.await_dialog_closed(DIALOG_CONFLICT);
    app.await_contents("dst/notes.txt", EXISTING_TEXT);
    app.await_contents("src/notes.txt", SOURCE_TEXT);
}

#[test]
fn a_collision_answered_with_overwrite_replaces_the_target() {
    let app = in_src_and_dst(arrange_with_collision);
    cursor_on_notes(&app);

    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");
    app.focus_dialog(DIALOG_CONFLICT);
    // Overwrite sits before the focused Skip in the row.
    app.key("shift+Tab");
    app.key("space");

    app.await_contents("dst/notes.txt", SOURCE_TEXT);
}

#[test]
fn a_collision_answered_with_keep_both_conserves_the_old_one() {
    let app = in_src_and_dst(arrange_with_collision);
    cursor_on_notes(&app);

    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");
    app.focus_dialog(DIALOG_CONFLICT);
    // Keep both sits after the focused Skip.
    app.key("Tab");
    app.key("space");

    app.await_contents("dst/notes (2).txt", SOURCE_TEXT);
    app.await_contents("dst/notes.txt", EXISTING_TEXT);
}

#[test]
fn a_collision_answered_with_abort_changes_nothing() {
    let app = in_src_and_dst(arrange_with_collision);
    cursor_on_notes(&app);

    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");
    app.focus_dialog(DIALOG_CONFLICT);
    // Abort is last in the row: two steps past the focused Skip.
    app.keys(&["Tab", "Tab", "space"]);

    app.await_dialog_closed(DIALOG_CONFLICT);
    app.await_contents("dst/notes.txt", EXISTING_TEXT);
}

#[test]
fn escape_closes_a_dialog_without_doing_anything() {
    // A dialog with no way out but the mouse is a trap in a keyboard-first
    // program, and GTK does not give a plain modal window this behavior.
    let app = in_src_and_dst(arrange);

    app.key("F7");
    app.focus_dialog(DIALOG_NEW_DIR);
    app.type_text("never-created");
    app.key("Escape");

    app.await_dialog_closed(DIALOG_NEW_DIR);
    app.settle();
    assert!(!app.path("src/never-created").exists());
}

#[test]
fn cancelling_the_target_dialog_copies_nothing() {
    let app = in_src_and_dst(arrange);
    cursor_on_notes(&app);

    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Escape");

    app.await_dialog_closed(DIALOG_COPY);
    app.settle();
    assert!(!app.path("dst/notes.txt").exists());
}

#[test]
fn the_parent_row_is_not_something_an_operation_acts_on() {
    // `..` is a navigation control. Deleting "the parent directory" from
    // inside it is never what the user means, so nothing should even ask.
    let app = in_src_and_dst(arrange);
    app.key("Home");

    app.key("F8");

    app.settle();
    assert!(
        !app.has_dialog(DIALOG_DELETE),
        "the parent row must not offer to be deleted"
    );
    assert!(app.path("src").exists());
}

#[test]
fn f5_with_both_panes_in_one_directory_will_not_copy_a_file_onto_itself() {
    // Both panes open at the home directory, so the prefilled target is the
    // directory the file is already in — no editing, no unusual input, just
    // F5 and Enter. This destroyed the file before it was caught: the
    // destination was opened for writing while the source handle was still
    // open, so the copy read back nothing and reported success.
    let app = App::launch(|home| {
        std::fs::write(home.join("precious.txt"), SOURCE_TEXT).unwrap();
    });
    app.keys(&["Home", "Down"]);

    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    // The refusal has to reach the user, not just the log.
    app.focus_dialog(DIALOG_FAILURES);
    app.await_contents("precious.txt", SOURCE_TEXT);
}

#[test]
fn insert_marks_a_file_and_f5_copies_what_is_marked() {
    // The whole point of marks: press F5 once, move several files. The cursor
    // is deliberately left somewhere else afterwards, so a copy of the cursor
    // row rather than the marks would produce the wrong file.
    let app = in_src_and_dst(arrange);
    // `..`, nested, data.bin, notes.txt — mark data.bin, then step past it.
    app.keys(&["Home", "Down", "Down"]);
    app.key("Insert");

    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    app.await_exists("dst/data.bin");
    app.settle();
    assert!(
        !app.path("dst/notes.txt").exists(),
        "the cursor row was copied instead of the mark"
    );
}

#[test]
fn insert_steps_down_so_it_can_be_held() {
    // Two presses mark two consecutive rows, which is how a run of files gets
    // selected in Total Commander.
    let app = in_src_and_dst(arrange);
    app.keys(&["Home", "Down", "Down"]);
    app.keys(&["Insert", "Insert"]);

    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    app.await_exists("dst/data.bin");
    app.await_exists("dst/notes.txt");
}

#[test]
fn ctrl_a_then_f8_deletes_everything_in_the_pane() {
    let app = in_src_and_dst(arrange);

    app.key("ctrl+a");
    app.key("F8");
    app.focus_dialog(DIALOG_DELETE);
    app.key("space");

    app.await_gone("src/notes.txt");
    app.await_gone("src/data.bin");
    app.await_gone("src/nested");
    // The directory the pane is standing in is not one of its own entries.
    assert!(app.path("src").exists());
}

#[test]
fn a_wildcard_marks_only_what_it_matches() {
    let app = in_src_and_dst(arrange);

    app.key("KP_Add");
    app.focus_dialog(DIALOG_PATTERN);
    app.type_text("*.txt");
    app.key("Return");

    app.focus_main();
    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    app.await_exists("dst/notes.txt");
    app.settle();
    assert!(
        !app.path("dst/data.bin").exists(),
        "the pattern marked a file it should not have"
    );
}

#[test]
fn ctrl_s_narrows_the_pane_as_you_type() {
    // The filter has to reach the pane, and the letters have to reach the
    // filter: the shell's key controller sits ahead of the entry, so every
    // one of them would otherwise be swallowed as a command.
    let app = in_src_and_dst(arrange);

    app.key("ctrl+s");
    app.type_text("notes");
    // Only notes.txt matches, so it is the first row after `..`.
    app.key("Return");
    app.keys(&["Home", "Down"]);

    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    app.await_exists("dst/notes.txt");
    app.settle();
    assert!(
        !app.path("dst/data.bin").exists(),
        "the filtered-out file was still reachable by the cursor"
    );
}

#[test]
fn escape_stops_filtering_and_brings_the_rows_back() {
    let app = in_src_and_dst(arrange);

    app.key("ctrl+s");
    app.type_text("notes");
    app.key("Escape");

    // Everything is back, so the second row down is data.bin again.
    app.keys(&["Home", "Down", "Down"]);
    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    app.await_exists("dst/data.bin");
}

#[test]
fn a_filter_that_matches_nothing_still_leaves_a_way_out() {
    // Filtering must never strand a pane: `..` survives any filter, and
    // pressing Enter on it goes up.
    let app = in_src_and_dst(arrange);

    app.key("ctrl+s");
    app.type_text("no-such-name");
    app.key("Return");
    app.keys(&["Home", "Return"]);

    // Back in the home directory, where `dst` exists as a sibling of `src`.
    app.key("F7");
    app.focus_dialog(DIALOG_NEW_DIR);
    app.type_text("escaped");
    app.key("Return");

    app.await_exists("escaped");
}

#[test]
fn ctrl_f6_sorts_by_size_and_ctrl_f6_again_reverses_it() {
    // Checked by which file the cursor lands on, because that is the only
    // thing about the order the filesystem can be asked about afterwards.
    // data.bin is 4 KiB and notes.txt is a line of text.
    let app = App::launch(|home| {
        std::fs::create_dir_all(home.join("src")).unwrap();
        std::fs::create_dir(home.join("dst")).unwrap();
        std::fs::write(home.join("src/notes.txt"), SOURCE_TEXT).unwrap();
        std::fs::write(home.join("src/data.bin"), vec![9u8; 4096]).unwrap();
    });
    app.keys(&["Tab", "Down", "Return", "Tab", "Down", "Down", "Return"]);

    // Ascending by size puts the small file first.
    app.key("ctrl+F6");
    app.keys(&["Home", "Down"]);
    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");
    app.await_exists("dst/notes.txt");

    // The same key again reverses it, so the big one is first now.
    app.focus_main();
    app.key("ctrl+F6");
    app.keys(&["Home", "Down"]);
    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");
    app.await_exists("dst/data.bin");
}

#[test]
fn ctrl_h_shows_the_dot_files_and_hides_them_again() {
    let app = App::launch(|home| {
        std::fs::create_dir_all(home.join("src")).unwrap();
        std::fs::create_dir(home.join("dst")).unwrap();
        std::fs::write(home.join("src/.hidden"), SOURCE_TEXT).unwrap();
    });
    app.keys(&["Tab", "Down", "Return", "Tab", "Down", "Down", "Return"]);

    // Hidden to begin with: nothing below `..` to copy, so F5 does nothing.
    app.keys(&["Home", "Down"]);
    app.key("F5");
    app.settle();
    assert!(
        !app.has_dialog(DIALOG_COPY),
        "a hidden entry must not be reachable by the cursor"
    );

    app.key("ctrl+h");
    app.keys(&["Home", "Down"]);
    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    app.await_contents("dst/.hidden", SOURCE_TEXT);
}

#[test]
fn the_sort_order_survives_a_finished_job() {
    // Every navigation and every finished job builds a fresh listing, so the
    // pane has to carry its own ordering or sorting by size and then copying
    // one file quietly puts the order back to name.
    //
    // The names are chosen so the two orders disagree: by name it is a.bin,
    // b.txt, c.txt; by descending size it is b.txt, c.txt, a.bin. Copying the
    // third row afterwards therefore says which order was in force.
    let app = App::launch(|home| {
        std::fs::create_dir_all(home.join("src")).unwrap();
        std::fs::create_dir(home.join("dst")).unwrap();
        std::fs::write(home.join("src/a.bin"), vec![1u8; 10]).unwrap();
        std::fs::write(home.join("src/b.txt"), vec![2u8; 8192]).unwrap();
        std::fs::write(home.join("src/c.txt"), vec![3u8; 4096]).unwrap();
    });
    app.keys(&["Tab", "Down", "Return", "Tab", "Down", "Down", "Return"]);

    // Descending by size: b.txt, c.txt, a.bin.
    app.keys(&["ctrl+F6", "ctrl+F6"]);
    app.keys(&["Home", "Down"]);
    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");
    app.await_exists("dst/b.txt");

    // Now the third row, with no re-sorting in between.
    app.focus_main();
    app.keys(&["Home", "Down", "Down", "Down"]);
    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    app.await_exists("dst/a.bin");
    assert!(
        !app.path("dst/c.txt").exists(),
        "the pane fell back to sorting by name after the job"
    );
}
