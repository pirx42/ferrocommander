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

/// Where the settings file lands inside a test's private home. Spelled out
/// rather than read from `tc-core`, for the same reason the dialog titles
/// are: a test that asks the code where it saves can only agree with it.
const SETTINGS_FILE: &str = ".config/ferrocommander/config.toml";

/// A home with `src/` to work in and `dst/` to land in.
fn arrange(home: &Path) {
    use std::fs;
    fs::create_dir_all(home.join("src/nested")).unwrap();
    fs::create_dir(home.join("dst")).unwrap();
    fs::write(home.join("src/notes.txt"), SOURCE_TEXT).unwrap();
    fs::write(home.join("src/data.bin"), vec![9u8; 4096]).unwrap();
    fs::write(home.join("src/nested/inner.txt"), "deep").unwrap();
}

/// Same, with a second `.txt` so the extension keys have something to pick
/// from and something to leave behind.
fn with_two_text_files(home: &Path) {
    arrange(home);
    std::fs::write(home.join("src/other.txt"), SOURCE_TEXT).unwrap();
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
fn space_marks_the_cursor_row_and_leaves_the_cursor_on_it() {
    // Space is the other half of Insert: it marks without moving, which is
    // how one file is picked out of a list. Marking then stepping *back* onto
    // the row would prove nothing about the cursor, so this presses F5 with
    // the cursor still where Space left it and checks that only the marked
    // row travelled.
    let app = in_src_and_dst(arrange);
    // `..`, nested, data.bin, notes.txt — Space on data.bin.
    app.keys(&["Home", "Down", "Down"]);
    app.key("space");

    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    app.await_exists("dst/data.bin");
    app.settle();
    assert!(
        !app.path("dst/notes.txt").exists(),
        "the cursor moved off the row Space marked"
    );
}

#[test]
fn pressing_the_mark_key_twice_takes_the_mark_back() {
    // A toggle, not a set. Space twice on one row leaves nothing marked, so
    // F5 falls back to the row under the cursor — which is still that row,
    // and nothing else comes with it.
    let app = in_src_and_dst(arrange);
    // `..`, nested, data.bin, notes.txt. Space twice on data.bin.
    app.keys(&["Home", "Down", "Down"]);
    app.keys(&["space", "space"]);
    // Insert marks `nested` and steps down, so taking that mark back means
    // stepping back up first — a second Insert where it landed would mark the
    // next row instead. It lands back on data.bin, the row the fallback wants.
    app.keys(&["Home", "Down", "Insert", "Up", "Insert"]);

    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    // Nothing was marked, so the copy is the cursor row alone.
    app.await_exists("dst/data.bin");
    app.settle();
    for stayed in ["dst/notes.txt", "dst/nested"] {
        assert!(
            !app.path(stayed).exists(),
            "a mark that was taken back still travelled: {stayed}"
        );
    }
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
fn shift_down_marks_a_run_the_way_insert_does() {
    // Total Commander's other way of marking a run, and the same behaviour:
    // mark the row being left, then move. Two presses from data.bin take
    // data.bin and notes.txt.
    let app = in_src_and_dst(arrange);
    app.keys(&["Home", "Down", "Down"]);
    app.keys(&["shift+Down", "shift+Down"]);

    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    app.await_exists("dst/data.bin");
    app.await_exists("dst/notes.txt");
}

#[test]
fn running_back_over_a_row_with_shift_takes_its_mark_off() {
    // The consequence of marking the row being *left*: Shift+Down then
    // Shift+Up passes over the same row twice and toggles it twice. This is
    // TC's own quirk, and reproducing it is the point.
    let app = in_src_and_dst(arrange);
    // From data.bin: down marks it and lands on notes.txt; up marks notes.txt
    // and lands back on data.bin; down marks data.bin *off* again and lands on
    // notes.txt, which is left marked and is the only mark.
    app.keys(&["Home", "Down", "Down"]);
    app.keys(&["shift+Down", "shift+Up", "shift+Down"]);

    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    app.await_exists("dst/notes.txt");
    app.settle();
    assert!(
        !app.path("dst/data.bin").exists(),
        "a row passed over twice kept its mark"
    );
}

#[test]
fn shift_end_marks_everything_from_the_cursor_down() {
    // A jump marks a range rather than toggling: "to the end" does not mean
    // "flip everything I passed".
    let app = in_src_and_dst(arrange);
    // `..`, nested, data.bin, notes.txt — from data.bin to the last row.
    app.keys(&["Home", "Down", "Down"]);
    app.key("shift+End");

    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    app.await_exists("dst/data.bin");
    app.await_exists("dst/notes.txt");
    app.settle();
    assert!(
        !app.path("dst/nested").exists(),
        "the range reached above the cursor"
    );
}

#[test]
fn shift_home_marks_up_to_the_top_but_never_the_parent_row() {
    // `..` is a navigation control, and a range that runs over it must not
    // pick it up — deleting "the parent directory" is never what was meant.
    let app = in_src_and_dst(arrange);
    app.keys(&["Home", "Down", "Down"]);
    app.key("shift+Home");

    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    app.await_exists("dst/data.bin");
    app.await_exists("dst/nested");
    app.settle();
    // Had `..` been marked, the copy would have tried to put src's own parent
    // into dst. The home directory holds `src` and `dst` themselves.
    assert!(
        !app.path("dst/src").exists() && !app.path("dst/dst").exists(),
        "the parent row was marked and copied"
    );
}

#[test]
fn shift_page_down_marks_across_a_screenful() {
    // The page keys measure a page off the widget, since the model has no
    // idea how tall the viewport is. A screen this size holds every row, so
    // one press marks everything below the cursor.
    let app = in_src_and_dst(arrange);
    app.keys(&["Home", "Down"]);
    app.key("shift+Next");

    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    app.await_exists("dst/nested");
    app.await_exists("dst/data.bin");
    app.await_exists("dst/notes.txt");
}

#[test]
fn ctrl_numminus_takes_every_mark_back() {
    // The gap Ctrl+A left: before this there was no way to unmark everything.
    let app = in_src_and_dst(arrange);
    app.key("ctrl+a");
    app.key("ctrl+KP_Subtract");

    // Nothing marked, so F5 falls back to the cursor row alone — `..`, which
    // is not something to operate on, so the copy does nothing at all.
    app.key("F5");
    app.settle();
    assert!(!app.has_dialog(DIALOG_COPY), "F5 still had marks to act on");
    for untouched in ["dst/nested", "dst/data.bin", "dst/notes.txt"] {
        assert!(
            !app.path(untouched).exists(),
            "{untouched} was still marked"
        );
    }
}

#[test]
fn alt_numplus_marks_the_files_sharing_an_extension() {
    let app = in_src_and_dst(with_two_text_files);
    // `..`, nested, data.bin, notes.txt, other.txt — cursor on notes.txt.
    app.keys(&["Home", "Down", "Down", "Down"]);
    app.key("alt+KP_Add");

    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    app.await_exists("dst/notes.txt");
    app.await_exists("dst/other.txt");
    app.settle();
    assert!(
        !app.path("dst/data.bin").exists(),
        "a file with another extension came along"
    );
}

#[test]
fn num_slash_brings_back_the_selection_the_last_job_spent() {
    // TC's Num /. The job ends with a fresh listing, so the marks it acted on
    // are gone by then — they have to have been put away when it started.
    let app = in_src_and_dst(with_two_text_files);
    app.keys(&["Home", "Down", "Down", "Down"]);
    app.key("alt+KP_Add");

    // Copy the two .txt files, which clears the marks.
    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");
    app.await_exists("dst/other.txt");

    // Bring them back and delete them, which proves the marks are real again
    // and that they are the same two files, not everything in the pane.
    app.focus_main();
    app.key("KP_Divide");
    app.key("F8");
    app.focus_dialog(DIALOG_DELETE);
    app.key("space");

    app.await_gone("src/notes.txt");
    app.await_gone("src/other.txt");
    assert!(
        app.path("src/data.bin").exists(),
        "the restored selection was wider than the one that was put away"
    );
}

#[test]
fn num_star_inverts_the_files_and_shift_num_star_the_directories_too() {
    // TC's split. Marking the one directory and inverting files-only must
    // leave it marked; inverting with Shift must take it off again.
    let app = in_src_and_dst(arrange);
    // Mark `nested`, the only directory.
    app.keys(&["Home", "Down"]);
    app.key("space");
    app.key("KP_Multiply");

    // nested (still marked) plus both files (newly marked) all travel.
    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");
    app.await_exists("dst/nested");
    app.await_exists("dst/data.bin");
    app.await_exists("dst/notes.txt");
}

#[test]
fn shift_num_star_takes_the_directories_with_it() {
    let app = in_src_and_dst(arrange);
    app.keys(&["Home", "Down"]);
    app.key("space");
    app.key("shift+KP_Multiply");

    // `nested` was marked and is now not; the two files are.
    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");
    app.await_exists("dst/data.bin");
    app.await_exists("dst/notes.txt");
    app.settle();
    assert!(
        !app.path("dst/nested").exists(),
        "the directory kept its mark through an inversion that includes them"
    );
}

#[test]
fn ctrl_right_shows_the_left_panes_directory_on_the_right() {
    // The arrow points at the pane being written. Standing in the left pane,
    // Ctrl+Right sends its directory across.
    let app = in_src_and_dst(arrange);
    app.key("ctrl+Right");

    // The right pane is now in src too, so F7 there creates inside src.
    app.key("Tab");
    app.key("F7");
    app.focus_dialog(DIALOG_NEW_DIR);
    app.type_text("cloned");
    app.key("Return");

    app.await_exists("src/cloned");
    assert!(
        !app.path("dst/cloned").exists(),
        "the right pane stayed in dst"
    );
}

#[test]
fn an_arrow_pointing_at_the_pane_you_are_in_does_nothing() {
    // TC-relative: Ctrl+Left names the left pane as the target, so pressing it
    // while standing there has no direction left to mean. "Nothing" has to be
    // literal — sending the pane to where it already is looks identical until
    // you notice it re-read the directory, which drops the marks and puts the
    // cursor back at the top. So this marks a file first and spends the mark
    // afterwards.
    let app = in_src_and_dst(arrange);
    // `..`, nested, data.bin, notes.txt — mark data.bin in the left pane.
    app.keys(&["Home", "Down", "Down"]);
    app.key("space");

    app.key("ctrl+Left");

    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    app.await_exists("dst/data.bin");
    app.settle();
    assert!(
        !app.path("dst/notes.txt").exists(),
        "the pane was reloaded by an arrow pointing at itself"
    );
}

#[test]
fn ctrl_u_exchanges_the_panes_and_the_marks_come_along() {
    // Swapping the whole listing rather than rebuilding it field by field is
    // what makes the marks travel with the directory. The keyboard stays in
    // the same physical pane, which is now showing the other side.
    let app = in_src_and_dst(arrange);
    // Mark data.bin in the left pane, which is in src.
    app.keys(&["Home", "Down", "Down"]);
    app.key("space");

    app.key("ctrl+u");

    // The left pane now shows dst; the right one shows src with its mark.
    // F5 from the right pane copies the marked file back to the left, which
    // is dst — proving both the swap and that the mark survived it.
    app.key("Tab");
    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    app.await_exists("dst/data.bin");
    app.settle();
    assert!(
        !app.path("dst/notes.txt").exists(),
        "the mark was lost in the exchange and F5 fell back to the cursor"
    );
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

#[test]
fn the_panes_open_where_they_were_left() {
    // Settings are only real if a second process finds them, so this closes
    // the app and starts another one on the same home directory.
    let first = in_src_and_dst(arrange);
    first.key("ctrl+h");
    let home = first.close();

    let app = App::relaunch(home);

    // The left pane is back in src, so the first row after `..` is a real
    // entry there rather than one of the home directory's.
    app.keys(&["Home", "Down"]);
    app.key("F7");
    app.focus_dialog(DIALOG_NEW_DIR);
    app.type_text("proof");
    app.key("Return");

    app.await_exists("src/proof");
}

#[test]
fn settings_survive_the_app_being_killed() {
    // The reason settings are written as they change rather than on the way
    // out: a kill, a crash and a lost session all skip the close handler, and
    // a file manager that forgets where you were every time it dies is a file
    // manager nobody trusts to remember anything.
    let first = in_src_and_dst(arrange);

    // Wait for the change to reach the disk before pulling the plug —
    // the write is debounced, and killing inside that window is a test of
    // the delay, not of the saving.
    first.await_mentions(SETTINGS_FILE, "src");
    let home = first.kill();

    let app = App::relaunch(home);

    // The left pane is back in src: the first row after `..` is a real entry
    // there rather than one of the home directory's.
    app.keys(&["Home", "Down"]);
    app.key("F7");
    app.focus_dialog(DIALOG_NEW_DIR);
    app.type_text("survived");
    app.key("Return");

    app.await_exists("src/survived");
}

/// A size no default and no other test uses, so finding it in the file can
/// only mean the resize was noticed.
const RESIZED_WIDTH: u32 = 1003;
const RESIZED_HEIGHT: u32 = 787;

#[test]
fn a_resized_window_is_remembered_without_being_closed() {
    // The window's size changes outside the keymap, so nothing in `dispatch`
    // would ever notice it. Killing rather than closing is what makes this a
    // test of the size *notification* and not of the close handler.
    let app = App::launch(arrange);
    app.resize(RESIZED_WIDTH, RESIZED_HEIGHT);

    app.await_mentions(SETTINGS_FILE, &format!("width = {RESIZED_WIDTH}"));
    app.await_mentions(SETTINGS_FILE, &format!("height = {RESIZED_HEIGHT}"));
    app.kill();
}

/// A settings file with the user's own bindings and their own comments.
const HAND_WRITTEN_KEYS: &str = "\
# my bindings
[keys]
# F9 makes a directory; F7 is somebody else's idea
\"f9\" = \"create_dir\"
\"f7\" = \"\"
\"ctrl+e\" = \"exchange_panes\"
\"nonsense\" = \"quit\"
";

/// Writes `HAND_WRITTEN_KEYS` into a home directory before the app opens.
fn with_hand_written_keys(home: &Path) {
    arrange(home);
    let settings = home.join(SETTINGS_FILE);
    std::fs::create_dir_all(settings.parent().unwrap()).unwrap();
    std::fs::write(settings, HAND_WRITTEN_KEYS).unwrap();
}

#[test]
fn a_key_the_settings_file_rebinds_does_what_it_says() {
    let app = in_src_and_dst(with_hand_written_keys);

    app.key("F9");

    app.focus_dialog(DIALOG_NEW_DIR);
    app.type_text("rebound");
    app.key("Return");
    app.await_exists("src/rebound");
}

#[test]
fn a_key_the_settings_file_unbinds_stops_doing_anything() {
    // An empty action takes a key away, which is not the same as leaving the
    // line out — that would keep the default.
    let app = in_src_and_dst(with_hand_written_keys);

    app.key("F7");

    app.settle();
    assert!(
        !app.has_dialog(DIALOG_NEW_DIR),
        "F7 still opened the dialog it was unbound from"
    );
}

#[test]
fn one_binding_nobody_can_read_does_not_cost_the_others() {
    // The file above also contains a key name that means nothing. Nothing
    // about the settings may stop the program starting, and a misspelling in
    // one line must not cost the other three.
    let app = in_src_and_dst(with_hand_written_keys);

    app.key("ctrl+e");

    // The panes swapped: the left one is in dst now, so F7 there lands in dst.
    app.key("F9");
    app.focus_dialog(DIALOG_NEW_DIR);
    app.type_text("swapped");
    app.key("Return");
    app.await_exists("dst/swapped");
}

#[test]
fn the_app_saving_does_not_touch_the_bindings_the_user_wrote() {
    // The consequence of saving on every change: the file the user hand-edits
    // is rewritten about half a second after the app opens. Serializing the
    // settings struct would reproduce the data and drop everything else.
    let first = in_src_and_dst(with_hand_written_keys);
    // Wait for a save to have happened at all, or this proves nothing.
    first.await_mentions(SETTINGS_FILE, "src");
    let home = first.kill();

    let written = std::fs::read_to_string(home.path().join(SETTINGS_FILE)).unwrap();
    for kept in [
        "# my bindings",
        "# F9 makes a directory; F7 is somebody else's idea",
        "\"ctrl+e\" = \"exchange_panes\"",
        "\"f7\" = \"\"",
    ] {
        assert!(written.contains(kept), "{kept:?} was lost:\n{written}");
    }

    // And the bindings still work in the next run, which is the point of
    // keeping them.
    let app = App::relaunch(home);
    app.key("F9");
    app.focus_dialog(DIALOG_NEW_DIR);
}

#[test]
fn a_settings_file_that_is_nonsense_does_not_stop_the_program() {
    // A file manager that refuses to start over its own settings is worse
    // than one that forgets where you were.
    let app = App::launch(|home| {
        arrange(home);
        let settings = home.join(SETTINGS_FILE);
        std::fs::create_dir_all(settings.parent().unwrap()).unwrap();
        std::fs::write(settings, "{{{ not toml at all").unwrap();
    });

    // It came up at all, and still works.
    app.key("F7");
    app.focus_dialog(DIALOG_NEW_DIR);
    app.type_text("started-anyway");
    app.key("Return");

    app.await_exists("started-anyway");
}

#[test]
fn an_ordinary_plus_reaches_the_keypad_binding() {
    // `+` needs Shift on most layouts, so it arrives as `plus` carrying one
    // and matched nothing at all — the twin binding shipped in phase 3 was
    // dead, and a real key press is the only thing that could have said so.
    let app = in_src_and_dst(arrange);

    app.key("plus");

    app.focus_dialog(DIALOG_PATTERN);
    app.type_text("*.bin");
    app.key("Return");
    app.focus_main();
    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    app.await_exists("dst/data.bin");
    app.settle();
    assert!(
        !app.path("dst/notes.txt").exists(),
        "the pattern dialog acted on more than it matched"
    );
}
