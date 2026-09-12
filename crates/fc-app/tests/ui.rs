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

use fc_core::vfs::{LocalFs, VfsPath};
use harness::{App, DATA_HOME, PRIVATE_BIN};

/// Contents of the file most tests move around, so an assertion can tell the
/// copy apart from whatever was at the destination.
const SOURCE_TEXT: &str = "hello from the source";

/// What a colliding target holds before the job touches it.
const EXISTING_TEXT: &str = "already here";

/// How many rows [`with_a_tall_directory`] adds. Comfortably more than a
/// 700-pixel window shows, so one Page Down cannot reach the end.
const TALL_DIRECTORY_ROWS: usize = 200;

/// Contents of the file inside `src/bundle.zip`, so an assertion can tell an
/// unpacked copy from anything else with that name.
const ARCHIVED_TEXT: &str = "this one came out of the zip";

/// Titles the dialogs are found by. Kept here rather than shared with the
/// crate: a test that reads its expectations out of the code under test can
/// only ever agree with it.
const DIALOG_COPY: &str = "Copy";
/// The progress window's title, spelled out like every other expectation.
const TITLE_PROGRESS: &str = "Working";
/// Files copied after the collision is answered.
///
/// Two different things have to be true for this test to have a window to
/// press a button on, and they need two different levers. The collision
/// makes the app's clock pass `PROGRESS_DELAY`, so the window is *allowed*
/// to open. These make the job go on running afterwards, so the window is
/// still there when the test reaches for it — twenty of them opened it and
/// closed it again inside a microsecond.
///
/// Counted in files rather than bytes on purpose: a copy of many small files
/// is bound by syscalls per file, which varies far less between machines
/// than throughput does. Half a gigabyte of bytes was the first attempt and
/// it was hardware-sensitive enough to be fast on the runner and slow here.
const TRAILING_FILES: usize = 20_000;
const DIALOG_MOVE: &str = "Move / Rename";
const DIALOG_NEW_DIR: &str = "New directory";
const DIALOG_DELETE: &str = "Confirm delete";
const DIALOG_CONFLICT: &str = "Target already exists";
const DIALOG_FAILURES: &str = "Some items were not processed";
const DIALOG_PATTERN: &str = "Select by pattern";
/// `Num −` opens a window of its own, titled for what it does — which
/// nothing here knew until a test pressed the key.
const DIALOG_UNMARK_PATTERN: &str = "Deselect by pattern";
const DIALOG_DRIVES: &str = "Drives";
const DIALOG_FAVOURITES: &str = "Favourite directories";
const DIALOG_OUTPUT: &str = "Command output";
const DIALOG_HISTORY: &str = "Command history";
const DIALOG_NEW_FILE: &str = "New file";
const DIALOG_SEARCH: &str = "Find files";
const DIALOG_RENAME: &str = "Multi-rename";
const DIALOG_PACK: &str = "Pack";

/// Where the settings file lands inside a test's private home. Spelled out
/// rather than read from `fc-core`, for the same reason the dialog titles
/// are: a test that asks the code where it saves can only agree with it.
const SETTINGS_FILE: &str = ".config/ferrocommander/config.toml";

/// The settings file as it stands, or empty if it has not been written yet.
///
/// Three readers pull different shapes out of it — a repeated table, an
/// inline array, a table of integers — and all three start here. Missing is
/// not an error: a run that has changed nothing has nothing to write, and
/// "not there yet" is what the polling readers are waiting out.
fn settings_text(app: &App) -> String {
    std::fs::read_to_string(app.path(SETTINGS_FILE)).unwrap_or_default()
}

/// Where the header band sits, and what the Ext column starts out wide.
/// Spelled out rather than imported, like every other expectation here.
///
/// It moved from 70 to 59 on 2026-09-01, when the rows and the header they
/// sit above were made compact: the band the drag has to land on is where it
/// is, and this constant is the one thing in the suite that knows a pixel of
/// the pane's layout. A row-height change is therefore expected to move it —
/// and the test failing loudly is exactly right, since a drag that misses the
/// header would otherwise silently be a drag over the rows.
const EXT_DIVIDER_Y: i32 = 59;
/// The divider at the right edge of Ext: the name column is 260 wide and Ext
/// 70. Dragged a hundred pixels right, which is far enough that no rounding
/// could account for the difference.
const EXT_DIVIDER_X: i32 = 330;
const EXT_DIVIDER_DRAGGED_X: i32 = 430;
const DEFAULT_EXT_WIDTH: i32 = 70;

/// What the app prints when GTK will not parse a rule. Spelled out rather
/// than imported, like every other string these tests look for.
const STYLESHEET_REJECTED: &str = "stylesheet rule rejected";

/// A point well down the *right* pane, past its first rows. The screen is
/// 1400 wide and the two panes split it, so this is in the right half
/// whatever the divider does with the odd pixel.
const RIGHT_PANE_ROW: (i32, i32) = (1000, 500);

/// A home with `src/` to work in and `dst/` to land in.
fn arrange(home: &Path) {
    use std::fs;
    fs::create_dir_all(home.join("src/nested")).unwrap();
    fs::create_dir(home.join("dst")).unwrap();
    fs::write(home.join("src/notes.txt"), SOURCE_TEXT).unwrap();
    fs::write(home.join("src/data.bin"), vec![9u8; 4096]).unwrap();
    fs::write(home.join("src/nested/inner.txt"), "deep").unwrap();
    register_an_application_for_text(home);
}

/// Registers exactly one application for `text/plain` in the private home,
/// and it is in **every** fixture on purpose.
///
/// The row menu carries an *Open with* submenu only when the desktop knows
/// an application for the row's type — so without this, the menu has one
/// shape on a machine with no applications installed and another on a
/// machine with some, and a test that reaches an entry by counting `Down`
/// presses reaches a different entry on each. That is not hypothetical: it
/// is how three tests passed here and failed on the CI runner, where
/// `Down`×3 landed on *Edit* instead of *Copy* and opened `notes.txt` in
/// the runner's browser.
///
/// One application, always present, in the home the test owns: the menu's
/// shape is then the fixture's to state rather than the machine's to
/// decide. What the *machine* has registered can still add rows **inside**
/// the submenu, which is why the one test that opens it puts its own
/// association in `mimeapps.list`, where it outranks them.
///
/// `mimeapps.list` rather than a compiled `mimeinfo.cache`: GIO reads the
/// former directly, and `update-desktop-database` is not on every machine
/// that runs this suite — checked on 2026-09-02 with `gio mime` before this
/// was written.
fn register_an_application_for_text(home: &Path) {
    write_private_program(home, OPENER_PROGRAM, OPENED_WITH);
    let applications = home.join(DATA_HOME).join("applications");
    std::fs::create_dir_all(&applications).unwrap();
    std::fs::write(
        applications.join("fake-opener.desktop"),
        format!(
            "[Desktop Entry]\nType=Application\nName=Fake Opener\nExec={} %f\nMimeType=text/plain;\n",
            home.join(PRIVATE_BIN).join(OPENER_PROGRAM).display()
        ),
    )
    .unwrap();
    std::fs::write(
        applications.join("mimeapps.list"),
        "[Added Associations]\ntext/plain=fake-opener.desktop;\n",
    )
    .unwrap();
}

/// Same, with `src/bundle.zip` holding two files and a nested directory.
fn with_an_archive(home: &Path) {
    arrange(home);
    let mut writer =
        zip::ZipWriter::new(std::fs::File::create(home.join("src/bundle.zip")).unwrap());
    let options: zip::write::FileOptions<'_, ()> = zip::write::FileOptions::default();
    for (name, contents) in [("packed.txt", ARCHIVED_TEXT), ("deeper/also.txt", "also")] {
        writer.start_file(name, options).unwrap();
        std::io::Write::write_all(&mut writer, contents.as_bytes()).unwrap();
    }
    writer.finish().unwrap();
}

/// A `src` with far more rows than fit on screen, so Page Down has somewhere
/// to go. Names sort in creation order and say which row they are.
fn with_a_tall_directory(home: &Path) {
    arrange(home);
    for index in 0..TALL_DIRECTORY_ROWS {
        std::fs::write(home.join(format!("src/row{index:03}.txt")), "x").unwrap();
    }
}

/// Same, with the rows in `dst` instead — so the *right* pane is the one with
/// something to click.
fn with_a_tall_destination(home: &Path) {
    arrange(home);
    for index in 0..TALL_DIRECTORY_ROWS {
        std::fs::write(home.join(format!("dst/row{index:03}.txt")), "x").unwrap();
    }
}

/// Where the stand-in application writes the path it was launched with, and
/// what it is called in the private bin.
const OPENED_WITH: &str = "opened-with";
const OPENER_PROGRAM: &str = "fake-opener";

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
    // And wait until both panes are really there.
    //
    // A directory is read on a worker thread now, so a navigation is no longer
    // finished by the time the next key is sent — and a test that marked files
    // straight afterwards was marking the directory it was leaving. Every test
    // starting here shares that precondition, so it is established once, here,
    // rather than left to each of them to get right.
    await_panes_at(&app, "/src", "/dst");
    app
}

/// The favourite paths the settings file holds, in order.
///
/// The `[[favourites]]` blocks, extracted — not a search for a substring.
/// Every one of these paths is also somewhere a *pane* has been, so "the file
/// mentions src/nested" says nothing at all about whether it is a favourite.
fn recorded_favourites(app: &App) -> Vec<String> {
    let written = settings_text(app);
    let mut found = Vec::new();
    let mut inside = false;
    for line in written.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            inside = line == "[[favourites]]";
        } else if inside {
            if let Some(path) = line.strip_prefix("path = ") {
                found.push(path.trim_matches('"').to_string());
            }
        }
    }
    found
}

/// Waits until the settings file holds exactly `count` favourites.
///
/// The write is debounced, so reading straight after the keystroke is a test
/// of the delay rather than of the change.
fn await_favourites(app: &App, count: usize) -> Vec<String> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        let found = recorded_favourites(app);
        if found.len() == count {
            return found;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "the file never held {count} favourites: {found:?}"
        );
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
}

/// Waits until the app has recorded both panes at those directories.
///
/// The settings file is the only place a pane's directory is observable from
/// outside, and it is written when one changes — so this is a poll on the
/// thing itself rather than a sleep long enough to probably do.
fn await_panes_at(app: &App, left: &str, right: &str) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        let recorded = recorded_directories(app.home());
        if recorded[0].ends_with(left) && recorded[1].ends_with(right) {
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "the panes never reached {left} and {right}: {recorded:?}"
        );
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
}

/// `with_an_archive`, plus two favourite directories in the settings file.
///
/// Written rather than added through the dialog, because until the list can
/// be maintained from inside itself there is no other way to have one — and
/// because a hand-written `[[favourites]]` table is a thing people are meant
/// to be able to write.
///
/// On the archive fixture so that "a favourite takes a pane out of an
/// archive" has an archive to be in. It costs the other tests nothing: the
/// zip is inside `src`, and every index they walk is in the home directory.
fn with_favourites(home: &Path) {
    with_an_archive(home);

    let settings = home.join(SETTINGS_FILE);
    std::fs::create_dir_all(settings.parent().unwrap()).unwrap();
    std::fs::write(
        settings,
        format!(
            "[[favourites]]\nname = \"deep\"\npath = \"{nested}\"\n\n\
             [[favourites]]\npath = \"{dst}\"\n",
            nested = home.join("src/nested").display(),
            dst = home.join("dst").display(),
        ),
    )
    .unwrap();
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

/// Chooses the `index`th entry of a context menu that is up.
///
/// The menu opens with its first entry highlighted, so `Down` once per
/// index reaches the entry and `Return` chooses it. Written out rather than
/// read from the code, like every expectation here: which entry is where is
/// exactly the fact the tests below are about. Up, not opening: the caller
/// opens the menu through `App::menu_after`, which waits until it is really
/// there — the keys below go to whatever has the keyboard, and a pane that
/// still has it would read them as its own.
fn choose_menu_entry(app: &App, index: usize) {
    for _ in 0..index {
        app.key("Down");
    }
    app.key("Return");
}

/// Where `Copy…` sits in the row menu: after Open, Open with, View and Edit.
///
/// *Open with* is there because every fixture registers an application for
/// `text/plain` and every row these tests open the menu on is a `.txt` —
/// see [`register_an_application_for_text`], which is where the whole
/// reason this constant can be a constant is written down.
const MENU_COPY: usize = 4;

/// What the application id is, and where the desktop entry lives. Spelled
/// out rather than imported, like every other expectation here.
const APPLICATION_ID: &str = "st.rose.Ferrocommander";
/// From the crate's own directory, because that is where a test runs — not
/// from the workspace root, which is where the path reads as if it were.
const DESKTOP_ENTRY: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../packaging/st.rose.Ferrocommander.desktop"
);

#[test]
fn the_window_calls_itself_what_the_desktop_entry_calls_it() {
    // How a desktop finds an application's icon: it matches what the window
    // says it is against the entry in `/usr/share/applications`, and takes
    // `Icon=` from there. GTK takes that string from the program name, which
    // is the basename of argv[0] — so until 2026-09-12 the window said
    // `ferrocommander` while the entry was named for the application id, and
    // Ubuntu showed a placeholder with the `.deb` installed and every file
    // in it correct.
    //
    // Two halves, because either alone can drift: the window reports the
    // application id, and the entry that ships says the same thing.
    let app = App::launch(arrange);

    assert!(
        app.has_window_of_class(APPLICATION_ID),
        "no window of class {APPLICATION_ID}: a shell would find no icon for it"
    );

    let entry = std::fs::read_to_string(DESKTOP_ENTRY).expect("the desktop entry ships");
    let claimed = entry
        .lines()
        .find_map(|line| line.strip_prefix("StartupWMClass="))
        .expect("the entry names a window class");
    assert_eq!(
        claimed, APPLICATION_ID,
        "the desktop entry looks for a class the window does not report"
    );
}

#[test]
fn shift_f10_opens_a_menu_whose_entry_runs_like_its_key() {
    // The menu is the action table drawn as a popover: choosing Copy must
    // land in the same dialog F5 does, on the same row, and accepting it
    // must copy the same file. Nothing in the menu is allowed to be a second
    // way of doing anything.
    let app = in_src_and_dst(arrange);
    cursor_on_notes(&app);

    app.menu_after(|app| app.key("shift+F10"));
    choose_menu_entry(&app, MENU_COPY);
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    app.await_contents("dst/notes.txt", SOURCE_TEXT);
}

#[test]
fn the_menu_acts_on_what_is_marked_not_on_the_row_it_points_at() {
    // Insert marks data.bin and steps the cursor down onto notes.txt, so the
    // menu opens pointing at notes.txt while data.bin is the one marked. An
    // entry chosen from it has to act on the marks, exactly as F5 would — a
    // menu that quietly acted on the row it was drawn beside would copy a
    // file nobody chose.
    let app = in_src_and_dst(arrange);
    // `..`, nested, data.bin, notes.txt.
    app.keys(&["Home", "Down", "Down", "Insert"]);

    app.menu_after(|app| app.key("shift+F10"));
    choose_menu_entry(&app, MENU_COPY);
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    app.await_exists("dst/data.bin");
    app.settle();
    assert!(
        !app.path("dst/notes.txt").exists(),
        "the menu copied the row it pointed at rather than the marked file"
    );
}

/// Where `Open with` sits in the row menu: right after `Open`.
const MENU_OPEN_WITH: usize = 1;

#[test]
fn open_with_lists_the_desktops_applications_and_launches_the_chosen_one() {
    // The one native piece a Linux desktop has to offer a context menu: the
    // application registry a double-click consults. One application is
    // registered for text/plain in the private home; the submenu must show
    // it, and choosing it must launch it with the file's path — which the
    // stand-in writes down, so the assertion is on what was actually opened
    // rather than on what was shown.
    let app = in_src_and_dst(arrange);
    cursor_on_notes(&app);

    app.menu_after(|app| app.key("shift+F10"));
    for _ in 0..MENU_OPEN_WITH {
        app.key("Down");
    }
    // Into the submenu. GTK opens a submenu as a page with a "back" row at
    // the top, and that row is what has the focus on arrival — so one Down
    // reaches the first application, and Return chooses it.
    app.key("Return");
    app.settle();
    app.keys(&["Down", "Return"]);

    app.await_mentions(OPENED_WITH, "src/notes.txt");
}

/// Two points in the left pane far enough apart to be different rows, and
/// far enough down to be `row*.txt` in a tall directory. Which rows they
/// are is not asserted anywhere — only that they differ.
const LEFT_PANE_ROW: (i32, i32) = (200, 500);
const LEFT_PANE_OTHER_ROW: (i32, i32) = (200, 300);

/// Every file that has arrived in `dst`, sorted.
fn arrived_in_dst(app: &App) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(app.path("dst"))
        .expect("dst exists")
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn a_right_click_marks_the_row_and_a_second_row_joins_it() {
    // Total Commander's mouse: the right button marks. Two clicks on two
    // rows leave two marks, and F5 then copies exactly those two — a cursor
    // that merely moved would copy one. `Home` parks the model on `..`
    // first, so a click that never reached the model copies nothing at all.
    let app = in_src_and_dst(with_a_tall_directory);
    app.key("Home");

    app.right_click(LEFT_PANE_ROW);
    app.right_click(LEFT_PANE_OTHER_ROW);
    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    await_any_copy(&app);
    app.settle();
    let arrived = arrived_in_dst(&app);
    assert_eq!(arrived.len(), 2, "two right clicks, two marks: {arrived:?}");
    assert!(
        arrived.iter().all(|name| name.starts_with("row")),
        "{arrived:?}"
    );
}

#[test]
fn a_second_right_click_on_the_same_row_takes_the_mark_back() {
    // A toggle, like Space. Twice on one row leaves nothing marked, so F5
    // falls back to the cursor row — which is that row, because the click
    // moved the cursor there — and copies it alone.
    let app = in_src_and_dst(with_a_tall_directory);
    app.key("Home");

    app.right_click(LEFT_PANE_ROW);
    app.right_click(LEFT_PANE_ROW);
    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    let copied = await_any_copy(&app);
    app.settle();
    assert_eq!(arrived_in_dst(&app).len(), 1, "the mark was not taken back");
    assert!(
        copied.starts_with("row"),
        "F5 copied {copied:?}, not the clicked row"
    );
}

#[test]
fn holding_the_right_button_opens_the_menu_on_that_row_and_marks_nothing() {
    // The other half of TC's button: held, it opens the menu on the row
    // under the pointer, and it does *not* mark — a hold that marked would
    // make the menu act on something other than what was on screen when the
    // button went down. Two holds on two rows, both dismissed, then F5:
    // one file, the cursor's, is the proof that neither hold marked. Then a
    // hold that is not dismissed chooses Copy from the menu it opened.
    let app = in_src_and_dst(with_a_tall_directory);
    app.key("Home");

    app.menu_after(|app| app.right_hold(LEFT_PANE_ROW));
    app.key("Escape");
    app.menu_after(|app| app.right_hold(LEFT_PANE_OTHER_ROW));
    app.key("Escape");
    app.settle();
    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");
    let first = await_any_copy(&app);
    app.settle();
    assert_eq!(
        arrived_in_dst(&app).len(),
        1,
        "a held right button marked the row"
    );
    assert!(first.starts_with("row"), "{first:?}");

    app.menu_after(|app| app.right_hold(LEFT_PANE_ROW));
    choose_menu_entry(&app, MENU_COPY);
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    await_until_dst_holds(&app, 2);
}

#[test]
fn a_right_click_in_the_other_pane_makes_it_the_active_one() {
    // The button acts on the pane it landed in — TC switches panes on a
    // right click, and a menu opened over one pane must never act on the
    // other. The left pane is active and in `src`; a right click on a row
    // of the right pane, in `dst`, marks it there and makes that pane
    // active, so F5 copies the row *from* dst *into* src.
    let app = in_src_and_dst(with_a_tall_destination);

    app.right_click(RIGHT_PANE_ROW);
    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        let landed = std::fs::read_dir(app.path("src"))
            .unwrap()
            .filter_map(|entry| entry.ok())
            .any(|entry| entry.file_name().to_string_lossy().starts_with("row"));
        if landed {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "nothing arrived in src: the right click did not make the right pane active"
        );
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

/// A point in the left pane well below its last row when the pane shows
/// `arrange`'s four — empty space, where the background menu lives.
const LEFT_PANE_BACKGROUND: (i32, i32) = (200, 450);

/// Where `New folder…` sits in the background menu: second, after Paste.
const BACKGROUND_MENU_NEW_FOLDER: usize = 1;

#[test]
fn a_right_click_on_empty_space_opens_the_background_menu() {
    // Below the last row there is nothing to mark, so the button has one
    // meaning there: the menu for *where*, not for a row. Its second entry
    // is New folder, and the row menu's second is View — so the dialog that
    // opens says which menu it was, and the directory that appears says the
    // entry ran like F7 would.
    let app = in_src_and_dst(arrange);

    app.menu_after(|app| app.right_click(LEFT_PANE_BACKGROUND));
    choose_menu_entry(&app, BACKGROUND_MENU_NEW_FOLDER);
    app.focus_dialog(DIALOG_NEW_DIR);
    app.type_text("from-the-background-menu");
    app.key("Return");

    app.await_exists("src/from-the-background-menu");
}

#[test]
fn holding_the_right_button_on_empty_space_opens_the_same_menu() {
    // A hold is what opens the menu on a row, so it opens this one too —
    // a hand that learned one gesture must not find it dead on the space
    // below the rows.
    let app = in_src_and_dst(arrange);

    app.menu_after(|app| app.right_hold(LEFT_PANE_BACKGROUND));
    choose_menu_entry(&app, BACKGROUND_MENU_NEW_FOLDER);
    app.focus_dialog(DIALOG_NEW_DIR);
    app.type_text("from-a-held-button");
    app.key("Return");

    app.await_exists("src/from-a-held-button");
}

/// Waits until `dst` holds `count` files.
fn await_until_dst_holds(app: &App, count: usize) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while arrived_in_dst(app).len() < count {
        assert!(
            std::time::Instant::now() < deadline,
            "dst holds {:?}, not {count} files",
            arrived_in_dst(app)
        );
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

#[test]
fn the_menu_key_opens_the_same_menu_and_escape_hands_the_rows_back() {
    // Two halves. `Menu` is the other spelling of Shift+F10, so the same
    // entries are there; and Escape has to leave the keyboard with the rows,
    // or the next key after a dismissed menu goes to a popover that is no
    // longer on screen. F7 is the witness: it opens a dialog only if the
    // pane heard it.
    let app = in_src_and_dst(arrange);
    cursor_on_notes(&app);

    app.menu_after(|app| app.key("Menu"));
    app.key("Escape");
    app.settle();
    app.key("F7");

    app.focus_dialog(DIALOG_NEW_DIR);
    app.key("Escape");
    app.await_dialog_closed(DIALOG_NEW_DIR);
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

/// A collision on the *first* file, and more files behind it.
///
/// This is how a job is held open long enough to have a progress window,
/// without depending on how fast a disk is. The first version of this
/// fixture wrote half a gigabyte and reasoned about copy speed; the runner
/// copied it in under the 300 ms threshold, no window appeared, and `main`
/// went red. Bytes were never the right lever: the fixture writes the file
/// immediately before the copy reads it, so it is in the page cache and the
/// copy runs at memory speed.
///
/// A conflict is a lever that owes nothing to hardware. The engine stops and
/// waits for an answer, so the test decides how long the job takes; the files
/// behind the collision are there to keep progress events coming afterwards,
/// which is what opens the window.
fn with_a_collision_then_more(home: &Path) {
    arrange(home);
    std::fs::write(home.join("src/aaa-collides.txt"), "source").unwrap();
    std::fs::write(home.join("dst/aaa-collides.txt"), "target").unwrap();
    for index in 0..TRAILING_FILES {
        std::fs::write(home.join(format!("src/zzz-{index:03}.txt")), "trailing").unwrap();
    }
}

/// A home whose `PATH` holds an `xdg-open` that records what it was given.
///
/// Enter hands a file to the desktop's handler, which on a test machine is
/// whatever `xdg-open` resolves to. A script of that name in the private bin
/// proves the same two things the recording editor does — that the handler
/// ran, and that it ran on the right file — without opening anything.
fn with_recording_opener(home: &Path) {
    arrange(home);
    write_private_program(home, "xdg-open", "opened-by-handler.log");
}

/// A script in the private bin that writes its first argument to `log`.
fn write_private_program(home: &Path, name: &str, log: &str) {
    let bin = home.join(PRIVATE_BIN);
    std::fs::create_dir_all(&bin).unwrap();
    let script = bin.join(name);
    std::fs::write(
        &script,
        format!(
            "#!/bin/sh\nprintf '%s' \"$1\" > \"{}/{log}\"\n",
            home.display()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&script, std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();
}

/// A home whose settings name an "editor" that records what it was given.
///
/// A real editor would open a window this suite cannot drive; a script that
/// writes down the path it was handed proves the same two things — that the
/// editor ran, and that it ran on the right file. A script rather than an
/// inline shell command because the settings file, the shell and this source
/// would otherwise each want their own layer of quoting.
fn with_recording_editor(home: &Path) {
    arrange(home);

    let script = home.join("record-editor.sh");
    std::fs::write(
        &script,
        "#!/bin/sh\nprintf '%s' \"$1\" > \"$(dirname \"$0\")/opened.log\"\n",
    )
    .unwrap();
    std::fs::set_permissions(&script, std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();

    let settings = home.join(SETTINGS_FILE);
    std::fs::create_dir_all(settings.parent().unwrap()).unwrap();
    // The compare_tool line rides along as a second witness for the
    // write-back test below: the editor proves a field the shell reads,
    // this proves one it only carries.
    std::fs::write(
        settings,
        format!(
            "editor = \"{}\"\ncompare_tool = \"diff -u %1 %2\"\n",
            script.display()
        ),
    )
    .unwrap();
}

#[test]
fn a_pane_notices_a_file_another_process_created() {
    // No key pressed at all: the pane is watching its directory and re-reads
    // itself when it settles.
    let app = in_src_and_dst(arrange);
    std::fs::write(app.path("src/appeared.txt"), "from outside").unwrap();

    // Sorted: `..`, nested, appeared.txt, data.bin, notes.txt. Until the pane
    // has re-read, the third row is still data.bin — so waiting for the
    // *right* file to arrive in dst is the whole assertion.
    app.settle();
    app.keys(&["Home", "Down", "Down"]);
    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    app.await_contents("dst/appeared.txt", "from outside");
}

#[test]
fn a_watched_pane_follows_the_directory_it_moves_to() {
    // The watch is about one directory, so navigating leaves it behind. A
    // pane that kept the old one would go quiet the first time you stepped
    // into a folder — which is most of the time.
    let app = in_src_and_dst(arrange);
    // Into src/nested, which nothing has watched yet.
    app.keys(&["Home", "Down", "Return"]);

    std::fs::write(app.path("src/nested/late.txt"), "arrived late").unwrap();
    app.settle();

    // `..`, inner.txt, late.txt — the third row exists only after a re-read.
    app.keys(&["Home", "Down", "Down"]);
    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    app.await_contents("dst/late.txt", "arrived late");
}

#[test]
fn ctrl_r_picks_up_a_file_another_process_created() {
    // Nothing tells the pane that a file appeared, so until this key existed
    // the only way to see it was to navigate away and back.
    let app = in_src_and_dst(arrange);
    std::fs::write(app.path("src/appeared.txt"), "from outside").unwrap();

    app.key("ctrl+r");

    // Sorted: `..`, nested, then appeared.txt, data.bin, notes.txt. Without
    // the re-read the third row is data.bin, so which file lands in dst says
    // whether the pane saw it.
    app.keys(&["Home", "Down", "Down"]);
    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    app.await_contents("dst/appeared.txt", "from outside");
    app.settle();
    assert!(
        !app.path("dst/data.bin").exists(),
        "the pane did not re-read and copied the row that used to be third"
    );
}

#[test]
fn a_re_read_keeps_the_marks() {
    // The reason this goes through Listing::reload rather than a fresh load:
    // dropping a selection somebody spent a minute building would be worse
    // than not re-reading at all.
    let app = in_src_and_dst(arrange);
    // `..`, nested, data.bin, notes.txt — mark data.bin and then step off it.
    // The cursor must not be on the marked row: the cursor survives a re-read
    // either way, so with it there, losing the mark is invisible — F5 falls
    // back to the same file and the test passes over the bug.
    app.keys(&["Home", "Down", "Down"]);
    app.key("space");
    app.key("Down");

    std::fs::write(app.path("src/appeared.txt"), "from outside").unwrap();
    app.key("ctrl+r");

    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    app.await_exists("dst/data.bin");
    app.settle();
    assert!(
        !app.path("dst/notes.txt").exists(),
        "the mark was lost and F5 fell back to the cursor row"
    );
}

#[test]
fn a_re_read_keeps_the_cursor_on_the_same_entry() {
    // Not on the same row *number*: a file appearing above it shifts every
    // index below, and a cursor restored by index would quietly move.
    let app = in_src_and_dst(arrange);
    // Cursor on notes.txt, the last row.
    app.keys(&["Home", "Down", "Down", "Down"]);

    std::fs::write(app.path("src/aaa-first.txt"), "sorts above everything").unwrap();
    app.key("ctrl+r");

    // notes.txt is one row further down now. F5 with nothing marked copies
    // the cursor row, which must still be notes.txt.
    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    app.await_contents("dst/notes.txt", SOURCE_TEXT);
}

#[test]
fn a_re_read_of_a_directory_that_has_gone_lands_somewhere_real() {
    // Showing an error where a listing belongs strands the user somewhere
    // they cannot navigate out of.
    let app = in_src_and_dst(arrange);
    std::fs::remove_dir_all(app.path("src")).unwrap();

    app.key("ctrl+r");

    // The pane fell back to the home directory, where dst still is: F7 lands
    // there rather than nowhere.
    app.key("F7");
    app.focus_dialog(DIALOG_NEW_DIR);
    app.type_text("still-usable");
    app.key("Return");

    app.await_exists("still-usable");
}

#[test]
fn alt_f7_finds_a_file_and_going_to_it_lands_on_it() {
    // What a search is *for*: getting to the file. Finding it and leaving the
    // user to hunt for the row is half the job.
    let app = in_src_and_dst(arrange);

    app.key("alt+F7");
    app.focus_dialog(DIALOG_SEARCH);
    app.type_text("inner*");
    app.key("Return");

    // One result, selected, so Enter takes it: the pane goes to src/nested
    // with the cursor on inner.txt.
    app.settle();
    app.key("Return");
    app.await_dialog_closed(DIALOG_SEARCH);

    // F5 with nothing marked copies the cursor row, which must be the result.
    app.focus_main();
    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    app.await_contents("dst/inner.txt", "deep");
}

#[test]
fn a_search_by_content_finds_only_what_says_it() {
    let app = in_src_and_dst(arrange);

    app.key("alt+F7");
    app.focus_dialog(DIALOG_SEARCH);
    // The name pattern stays `*`; the content is what narrows it.
    app.key("Tab");
    app.type_text("deep");
    app.key("Return");

    app.settle();
    app.key("Return");
    app.await_dialog_closed(DIALOG_SEARCH);

    app.focus_main();
    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    app.await_contents("dst/inner.txt", "deep");
}

#[test]
fn a_search_that_finds_nothing_says_so_and_stays_open() {
    // Rather than closing, which would look like it had done something.
    let app = in_src_and_dst(arrange);

    app.key("alt+F7");
    app.focus_dialog(DIALOG_SEARCH);
    app.type_text("no-such-file-anywhere");
    app.key("Return");

    app.settle();
    assert!(app.has_dialog(DIALOG_SEARCH), "the search window closed");
}

#[test]
fn alt_f5_packs_what_is_marked_and_the_result_opens_again() {
    // Packing and then walking into what was packed, which is the only
    // assertion that covers both halves at once: a writer nothing can read is
    // not a feature.
    let app = in_src_and_dst(arrange);
    // `..`, nested, data.bin, notes.txt — mark both files.
    app.keys(&["Home", "Down", "Down"]);
    app.keys(&["Insert", "Insert"]);

    app.key("alt+F5");
    app.focus_dialog(DIALOG_PACK);
    // A bare name lands beside the *other* pane, which is where the prefilled
    // path points and where the archive is written.
    app.type_text("packed.zip");
    app.key("Return");
    app.await_exists("dst/packed.zip");

    // Send the left pane somewhere the unpacked file will not collide.
    app.focus_main();
    app.keys(&["Home", "Down"]);
    app.key("Return");
    await_panes_at(&app, "/src/nested", "/dst");

    // The right pane walks into the archive: `..`, packed.zip.
    app.key("Tab");
    app.keys(&["Home", "Down"]);
    app.key("Return");
    await_panes_at(&app, "/src/nested", "/dst/packed.zip");

    // Inside: `..`, data.bin, notes.txt. Copy the text file back out.
    app.keys(&["Home", "Down", "Down"]);
    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    app.await_contents("src/nested/notes.txt", SOURCE_TEXT);
}

#[test]
fn packing_into_a_name_that_names_no_format_is_refused_before_anything_starts() {
    // The extension is the whole choice of format, so a name with none is a
    // question the program cannot answer — and answering it by guessing would
    // write a zip called `.rar`. The refusal belongs to the name, not to a
    // job that fails afterwards: nothing is written, nothing is left
    // half-written under a temporary name, and the next answer still works.
    let app = in_src_and_dst(arrange);
    cursor_on_notes(&app);

    app.key("alt+F5");
    app.focus_dialog(DIALOG_PACK);
    app.type_text("nope.rar");
    app.key("Return");

    app.focus_dialog(DIALOG_FAILURES);
    app.key("Return");
    app.settle();
    let left: Vec<String> = std::fs::read_dir(app.path("dst"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with("nope"))
        .collect();
    assert!(left.is_empty(), "the refused name left {left:?}");

    app.focus_main();
    app.key("alt+F5");
    app.focus_dialog(DIALOG_PACK);
    app.type_text("yes.zip");
    app.key("Return");
    app.await_exists("dst/yes.zip");
}

#[test]
fn enter_on_an_archive_walks_into_it_and_f5_unpacks_from_it() {
    // The claim the whole two-crate split was made for: a pane holds a
    // different backend and every key goes on meaning what it meant. F5 is
    // the copy engine reading one filesystem and writing another, with no
    // idea that it is unpacking.
    let app = in_src_and_dst(with_an_archive);
    // `..`, nested, bundle.zip, data.bin, notes.txt.
    app.keys(&["Home", "Down", "Down"]);
    app.key("Return");
    await_panes_at(&app, "/src/bundle.zip", "/dst");

    // Inside: `..`, deeper, packed.txt. The `..` row is there even though the
    // archive's root has no parent inside it.
    app.keys(&["Home", "Down", "Down"]);
    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    app.await_contents("dst/packed.txt", ARCHIVED_TEXT);
}

#[test]
fn leaving_an_archive_lands_back_on_the_archive_file() {
    // `..` at an archive's root means what `..` always means here: where you
    // came from. Landing in the containing directory but at the top of it
    // would be half the job, so this checks the cursor as well — with F5,
    // which acts on the row under it.
    let app = in_src_and_dst(with_an_archive);
    app.keys(&["Home", "Down", "Down"]);
    app.key("Return");
    await_panes_at(&app, "/src/bundle.zip", "/dst");

    app.key("BackSpace");
    await_panes_at(&app, "/src", "/dst");

    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    app.await_exists("dst/bundle.zip");
    app.settle();
    assert!(
        !app.path("dst/data.bin").exists(),
        "the cursor landed at the top of the directory rather than on the archive"
    );
}

#[test]
fn a_directory_inside_an_archive_is_a_directory() {
    // Nested entries and their synthesised directories, through the real
    // shell rather than through the index's own tests.
    let app = in_src_and_dst(with_an_archive);
    app.keys(&["Home", "Down", "Down"]);
    app.key("Return");
    await_panes_at(&app, "/src/bundle.zip", "/dst");

    // `..`, deeper, packed.txt — `deeper` exists only because an entry is
    // named `deeper/also.txt`.
    app.keys(&["Home", "Down"]);
    app.key("Return");
    await_panes_at(&app, "/src/bundle.zip/deeper", "/dst");

    app.keys(&["Home", "Down"]);
    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    app.await_contents("dst/also.txt", "also");
}

#[test]
fn enter_on_the_parent_row_at_an_archive_root_leaves_the_archive() {
    // Backspace is not the only way out. The root of an archive has no parent
    // inside it, so the row exists only because there is somewhere to go.
    let app = in_src_and_dst(with_an_archive);
    app.keys(&["Home", "Down", "Down"]);
    app.key("Return");
    await_panes_at(&app, "/src/bundle.zip", "/dst");

    app.keys(&["Home", "Return"]);

    await_panes_at(&app, "/src", "/dst");
}

#[test]
fn f7_inside_an_archive_makes_no_directory_anywhere() {
    // The bug this exists for: a job's two backends used to be the active
    // pane's and the *other* pane's, always. F7 builds its path from the
    // active pane, so inside an archive it would have been created on the
    // disk, at the path the archive calls it — `/made-in-here` at the root of
    // the filesystem. Not a failure: a real directory in the wrong place.
    let app = in_src_and_dst(with_an_archive);
    app.keys(&["Home", "Down", "Down"]);
    app.key("Return");
    await_panes_at(&app, "/src/bundle.zip", "/dst");

    app.key("F7");
    app.focus_dialog(DIALOG_NEW_DIR);
    app.type_text("made-in-here");
    app.key("Return");

    // One refusal, and nothing created — not in the archive, not beside it,
    // and not in the other pane either.
    app.focus_dialog(DIALOG_FAILURES);
    app.key("Return");
    app.settle();
    for nowhere in ["src/made-in-here", "dst/made-in-here", "made-in-here"] {
        assert!(!app.path(nowhere).exists(), "it was created at {nowhere}");
    }
}

#[test]
fn ctrl_u_carries_the_archive_across_with_the_listing() {
    // Exchanging the panes swapped the listings and left the backends where
    // they were, so each pane showed the other's entries through its own
    // filesystem. Invisible with one backend; with two it is both panes
    // reading the wrong one.
    let app = in_src_and_dst(with_an_archive);
    app.keys(&["Home", "Down", "Down"]);
    app.key("Return");
    await_panes_at(&app, "/src/bundle.zip", "/dst");

    app.key("ctrl+u");
    await_panes_at(&app, "/dst", "/src/bundle.zip");

    // The archive is on the right now, and still readable: Tab into it and
    // copy a file out to the left, which is `dst`.
    app.key("Tab");
    app.keys(&["Home", "Down", "Down"]);
    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    app.await_contents("dst/packed.txt", ARCHIVED_TEXT);
}

#[test]
fn ctrl_right_carries_the_archive_across_too() {
    // The same failure as Ctrl+U, from the other direction: copying only the
    // *path* would send the right pane's own backend to a path that belongs
    // to the left's — inside an archive, a path on the disk that has nothing
    // to do with it.
    let app = in_src_and_dst(with_an_archive);
    app.keys(&["Home", "Down", "Down"]);
    app.key("Return");
    await_panes_at(&app, "/src/bundle.zip", "/dst");

    app.key("ctrl+Right");
    await_panes_at(&app, "/src/bundle.zip", "/src/bundle.zip");

    // Both panes are in the archive; the right one can still be read out of.
    app.key("Tab");
    app.keys(&["Home", "Down", "Down"]);
    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");
    app.settle();

    // Copying between two panes in the same archive is a write into one:
    // refused, and nothing left behind on the disk.
    assert!(!app.path("src/packed.txt").exists());
    assert!(!app.path("packed.txt").exists());
}

#[test]
fn a_drive_button_takes_a_pane_out_of_an_archive() {
    // Navigating on the current backend would send the *archive* to
    // `/mnt/whatever`, which it has never heard of — so the pane would show an
    // error and still be inside, with Backspace the only way out.
    let app = in_src_and_dst(with_an_archive);
    app.keys(&["Home", "Down", "Down"]);
    app.key("Return");
    await_panes_at(&app, "/src/bundle.zip", "/dst");

    app.key("alt+F1");
    app.focus_dialog(DIALOG_DRIVES);
    app.key("Return");

    // Wherever the first drive is, the pane is no longer inside the archive.
    await_left_pane_leaves(&app, "bundle.zip");
}

/// Waits until the left pane's recorded directory no longer mentions `name`.
///
/// The settings file is the only place a pane's directory is observable from
/// outside, and it is rewritten whenever one changes.
fn await_left_pane_leaves(app: &App, name: &str) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        let recorded = recorded_directories(app.home());
        if !recorded[0].contains(name) {
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "the left pane never left {name}: {recorded:?}"
        );
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
}

#[test]
fn f4_inside_an_archive_does_not_open_an_editor_on_the_disk() {
    // An editor takes an operating-system path and a file inside an archive
    // has none. Handing over what the archive calls it would open the editor
    // on a path of the same name on the disk — and create it there when
    // saved.
    let app = in_src_and_dst(with_an_archive);
    app.keys(&["Home", "Down", "Down"]);
    app.key("Return");
    await_panes_at(&app, "/src/bundle.zip", "/dst");

    // `..`, deeper, packed.txt.
    app.keys(&["Home", "Down", "Down"]);
    app.key("F4");

    app.focus_dialog(DIALOG_OUTPUT);
    app.key("Return");
    app.settle();
    assert!(
        !app.path("packed.txt").exists(),
        "a file was made on the disk"
    );
}

#[test]
fn a_command_typed_inside_an_archive_is_refused_rather_than_run() {
    // A path inside an archive is not somewhere a process can run, and running
    // it against whatever that path means on the real filesystem is how a
    // command meant for an archive acts on a home directory instead.
    let app = in_src_and_dst(with_an_archive);
    app.keys(&["Home", "Down", "Down"]);
    app.key("Return");
    await_panes_at(&app, "/src/bundle.zip", "/dst");

    // The command line is reached with the arrow now that letters search.
    app.key("Right");
    app.type_text("touch escaped.txt");
    app.key("Return");

    app.focus_dialog(DIALOG_OUTPUT);
    app.key("Return");
    app.settle();
    for nowhere in ["src/escaped.txt", "escaped.txt"] {
        assert!(
            !app.path(nowhere).exists(),
            "the command ran anyway: {nowhere}"
        );
    }
}

#[test]
fn a_pane_left_inside_an_archive_reopens_beside_it() {
    // The settings file records the path the pane was actually showing, which
    // only the archive understands. Restoring it is the ordinary walk up to
    // the nearest readable ancestor, so the next start lands in the directory
    // holding the archive rather than failing to open one.
    let app = in_src_and_dst(with_an_archive);
    app.keys(&["Home", "Down", "Down"]);
    app.key("Return");
    await_panes_at(&app, "/src/bundle.zip", "/dst");

    let home = app.close();
    let app = App::relaunch(home);

    // The walk up happens on the reading thread, so the pane is briefly still
    // "about" the archive path it was told to open — this waits for where it
    // actually landed, exactly as every test that starts in two directories
    // does.
    await_panes_at(&app, "/src", "/dst");

    // And it is a directory that can be worked in, not just a string in a
    // settings file: F7 makes its directory in the active pane.
    app.key("F7");
    app.focus_dialog(DIALOG_NEW_DIR);
    app.type_text("landed-here");
    app.key("Return");

    app.await_exists("src/landed-here");
}

#[test]
fn ctrl_m_renames_the_marked_files_by_the_template() {
    // The counter numbers the files in the order the pane shows them, which
    // is the only order anybody can predict from looking at the screen.
    let app = in_src_and_dst(arrange);
    // `..`, nested, data.bin, notes.txt — Insert twice marks both files.
    app.keys(&["Home", "Down", "Down"]);
    app.keys(&["Insert", "Insert"]);

    app.key("ctrl+m");
    app.focus_dialog(DIALOG_RENAME);
    // The template arrives selected, so this replaces it rather than
    // appending to it.
    app.type_text("page[C].[E]");
    app.key("Return");

    app.await_exists("src/page1.bin");
    app.await_contents("src/page2.txt", SOURCE_TEXT);
    app.await_gone("src/notes.txt");
}

#[test]
fn ctrl_z_puts_the_renamed_names_back() {
    // Undo is a rename back, so it has to know where the files went — not
    // just what they used to be called.
    let app = in_src_and_dst(arrange);
    app.keys(&["Home", "Down", "Down"]);
    app.keys(&["Insert", "Insert"]);

    app.key("ctrl+m");
    app.focus_dialog(DIALOG_RENAME);
    app.type_text("page[C].[E]");
    app.key("Return");
    app.await_contents("src/page2.txt", SOURCE_TEXT);
    // Let the progress window close before the keyboard is asked for back.
    app.settle();

    app.focus_main();
    app.key("ctrl+z");

    app.await_contents("src/notes.txt", SOURCE_TEXT);
    app.await_exists("src/data.bin");
    app.await_gone("src/page1.bin");
    app.await_gone("src/page2.txt");
}

#[test]
fn two_files_that_would_get_one_name_do_not_overwrite_each_other() {
    // The batch's own collisions are refused in the preview, before anything
    // runs. The failure this guards against is not a bad name but a lost
    // file: the second rename landing on the first.
    let app = in_src_and_dst(arrange);
    app.keys(&["Home", "Down", "Down"]);
    app.keys(&["Insert", "Insert"]);

    app.key("ctrl+m");
    app.focus_dialog(DIALOG_RENAME);
    // No [C] and no [E]: both marked files want to be called `same`.
    app.type_text("same");
    app.key("Return");

    // data.bin sorts first, so it takes the name; notes.txt keeps its own
    // rather than being renamed over the top of it.
    app.await_exists("src/same");
    app.await_contents("src/notes.txt", SOURCE_TEXT);
    assert_eq!(
        std::fs::read(app.path("src/same")).unwrap(),
        vec![9u8; 4096],
        "the second file was renamed over the first"
    );
    // And refused *before* it ran, not caught by the conflict question
    // afterwards. Without this the test passes on a build that submits the
    // colliding move and leaves the user staring at a dialog nobody asked
    // for — the file survives either way, so only the absent dialog tells
    // the two apart.
    app.settle();
    assert!(
        !app.has_dialog(DIALOG_CONFLICT),
        "the colliding rename was submitted instead of being refused"
    );
}

#[test]
fn a_preview_that_is_cancelled_renames_nothing() {
    // Typing rules redraws the preview on every keystroke. If that ever ran
    // the rename instead of previewing it, this is the test that says so.
    let app = in_src_and_dst(arrange);
    app.keys(&["Home", "Down", "Down"]);
    app.keys(&["Insert", "Insert"]);

    app.key("ctrl+m");
    app.focus_dialog(DIALOG_RENAME);
    app.type_text("page[C].[E]");
    app.key("Escape");
    app.await_dialog_closed(DIALOG_RENAME);

    app.settle();
    assert!(app.path("src/notes.txt").exists(), "the file was renamed");
    assert!(
        !app.path("src/page1.bin").exists(),
        "the preview renamed something"
    );
}

#[test]
fn f3_opens_a_viewer_on_the_file_under_the_cursor() {
    // The window is titled with the file's name, which is how the test finds
    // it — and also what tells a person which of several open viewers is
    // which.
    let app = in_src_and_dst(arrange);
    // `..`, nested, data.bin, notes.txt.
    app.keys(&["Home", "Down", "Down", "Down"]);

    app.key("F3");

    app.focus_dialog("notes.txt");
    // Escape closes it and hands the keyboard back to the rows.
    app.key("Escape");
    app.await_dialog_closed("notes.txt");
    app.key("F7");
    app.focus_dialog(DIALOG_NEW_DIR);
}

#[test]
fn the_viewer_pages_through_a_file_it_never_read() {
    // The percentage in the title is the only thing about the viewer that is
    // observable from outside — and it happens to be exactly the thing worth
    // asserting: it says where in the file the offset is, so watching it move
    // is watching the paging work.
    //
    // The file is larger than one window on purpose. A viewer that read the
    // whole thing would open just as well; one that could only *show* what it
    // had read would not move at all.
    let app = App::launch(|home| {
        arrange(home);
        let big: String = (0..200_000).map(|i| format!("line {i}\n")).collect();
        std::fs::write(home.join("src/big.txt"), big).unwrap();
    });
    app.keys(&["Home", "Down", "Down", "Return"]);
    await_panes_at(&app, "/src", "");
    // `..`, nested, big.txt.
    app.keys(&["Home", "Down", "Down"]);

    app.key("F3");
    app.focus_dialog("big.txt");
    assert!(app.has_dialog("0%"), "it did not open at the top");

    app.key("End");
    app.settle();
    // That it moved, not how far: End stops half a window short of the size so
    // the last page still has something in it, and pinning the exact
    // percentage here would be pinning that arithmetic twice — it has its own
    // test in `fc-core`.
    assert!(!app.has_dialog("0%"), "End did not move the offset");

    app.key("Home");
    app.settle();
    assert!(app.has_dialog("0%"), "Home did not come back");
}

#[test]
fn f3_does_nothing_on_a_directory_or_on_the_parent_row() {
    // There is nothing to read, and Total Commander does not offer either.
    let app = in_src_and_dst(arrange);

    // A viewer is titled `<name> — <percent>%`, so `0%` is the marker that a
    // viewer is open at all — and unlike the file's own name it cannot match
    // anything else on the display. (`..` as a search would match every
    // window there is: xdotool takes a regex, and `.` is any character.)
    app.key("Home");
    app.key("F3");
    app.settle();
    assert!(!app.has_dialog("0%"), "a viewer opened on the parent row");

    app.key("Down");
    app.key("F3");
    app.settle();
    assert!(!app.has_dialog("0%"), "a viewer opened on a directory");
}

// Quick view (`Ctrl+Q`, docs/viewer.md). What these can honestly assert is
// bounded: the suite reads the window title, the log, the settings file and
// the filesystem — never a pane's text. So the *content* of a preview is the
// engine's tests to carry, and what is left here is that the key stopped
// quitting, that the previewed pane comes back with its place intact, and
// that the preview follows the keyboard instead of sticking to one side.

#[test]
fn ctrl_q_previews_instead_of_quitting() {
    // It quit until 2026-09-01. That it does not any more is checked by the
    // app still being there to answer the next key — and by every other test
    // in this file, which close with `Alt+F4` through the harness.
    let app = in_src_and_dst(arrange);

    app.key("ctrl+q");
    app.settle();

    app.key("F7");
    app.focus_dialog(DIALOG_NEW_DIR);
    app.key("Escape");
    app.await_dialog_closed(DIALOG_NEW_DIR);
}

#[test]
fn a_previewed_pane_comes_back_where_it_was() {
    // The listing is the other page of a stack rather than a widget torn down
    // and rebuilt, and the cursor is what proves it: `nested` is row 1, so a
    // pane that came back rebuilt would have its cursor on `..` and this
    // `Return` would go to the home directory instead of into `nested`.
    let app = in_src_and_dst(arrange);
    // `..`, nested, data.bin, notes.txt.
    app.keys(&["Home", "Down"]);

    // The keyboard has to be elsewhere for this pane to be the previewed one.
    app.key("Tab");
    app.key("ctrl+q");
    app.settle();
    app.key("ctrl+q");
    app.settle();
    app.key("Tab");

    app.key("Return");
    await_panes_at(&app, "/src/nested", "/dst");
}

#[test]
fn a_previewed_pane_has_no_rows_to_click() {
    // The one thing the suite *can* see about a preview being on the screen:
    // rows that are not there cannot be clicked. Both halves are in one test,
    // because either alone would pass against a `Ctrl+Q` that did nothing at
    // all — which is the shape of test this repository has twice caught
    // itself writing.
    //
    // A click moves the clicked pane's cursor without giving it the keyboard,
    // and `F5` on `..` with nothing marked opens no dialog. So the dialog is
    // the answer to "did the click find a row".
    let app = in_src_and_dst(with_a_tall_destination);

    app.click(RIGHT_PANE_ROW);
    app.key("Tab");
    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Escape");
    app.await_dialog_closed(DIALOG_COPY);

    // The right pane back on `..`, and the keyboard back on the left — which
    // makes the right one the preview.
    app.key("Home");
    app.key("Tab");
    app.key("ctrl+q");
    app.settle();

    app.click(RIGHT_PANE_ROW);
    // Tab hands the right pane the keyboard, and with it its listing back —
    // with the cursor exactly where the click could not move it.
    app.key("Tab");
    app.key("F5");
    app.settle();
    assert!(
        !app.has_dialog(DIALOG_COPY),
        "the click reached a row of a pane that was supposed to be a preview"
    );

    // And the rows come back. Without this the test would pass just as well
    // against a pane that never stopped being a preview, which is the same
    // hole from the other side.
    app.key("ctrl+q");
    app.settle();
    app.key("Tab");
    app.click(RIGHT_PANE_ROW);
    app.key("Tab");
    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Escape");
    app.await_dialog_closed(DIALOG_COPY);
}

#[test]
fn the_preview_follows_the_keyboard_rather_than_a_pane() {
    // `Tab` keeps its plain meaning: the preview is simply whichever pane the
    // keyboard is not in. So both panes stay drivable while the mode is on,
    // one after the other — which is also what says the pane that stopped
    // being a preview stopped being one.
    let app = in_src_and_dst(|home| {
        arrange(home);
        std::fs::create_dir(home.join("dst/inner")).unwrap();
    });

    app.key("ctrl+q");
    app.settle();

    // The right pane takes the keyboard, and with it its own rows back.
    app.key("Tab");
    app.keys(&["Home", "Down", "Return"]);
    await_panes_at(&app, "/src", "/dst/inner");

    // And the left one, which has spent that time as the preview.
    app.key("Tab");
    app.keys(&["Home", "Down", "Return"]);
    await_panes_at(&app, "/src/nested", "/dst/inner");
}

#[test]
fn f4_hands_the_file_under_the_cursor_to_the_editor() {
    let app = in_src_and_dst(with_recording_editor);
    app.keys(&["Home", "Down", "Down", "Down"]);

    app.key("F4");

    app.await_contents(
        "opened.log",
        &format!("{}/src/notes.txt", app.home().display()),
    );
}

#[test]
fn shift_f4_creates_a_file_and_opens_it() {
    let app = in_src_and_dst(with_recording_editor);

    app.key("shift+F4");
    app.focus_dialog(DIALOG_NEW_FILE);
    app.type_text("notes-from-f4.md");
    app.key("Return");

    app.await_exists("src/notes-from-f4.md");
    // Empty, as Total Commander leaves it: what an editor makes of a
    // zero-byte file is the editor's business.
    app.await_contents("src/notes-from-f4.md", "");
    // And the editor was handed that exact path.
    app.await_contents(
        "opened.log",
        &format!("{}/src/notes-from-f4.md", app.home().display()),
    );
}

#[test]
fn shift_f4_will_not_empty_a_file_that_is_already_there() {
    // The dangerous case: `create_file` truncates, so without a check this
    // would empty the very file the user meant to open — and then hand it to
    // an editor, which would save the emptiness back.
    let app = in_src_and_dst(with_recording_editor);

    app.key("shift+F4");
    app.focus_dialog(DIALOG_NEW_FILE);
    app.key("ctrl+a");
    app.type_text("notes.txt");
    app.key("Return");

    app.focus_dialog(DIALOG_FAILURES);
    app.await_contents("src/notes.txt", SOURCE_TEXT);
    assert!(
        !app.path("opened.log").exists(),
        "the editor was launched on a file the job refused to create"
    );
}

#[test]
fn a_setting_the_shell_does_not_own_survives_being_written_back() {
    // This has gone wrong twice: `[keys]` when it arrived, and `editor` a
    // phase later. Both times the settings the app writes were built from the
    // *defaults*, so a field nobody thought to copy was zeroed and the zero
    // written back over the user's own line — half a second after the app
    // opened, because saving happens on change.
    //
    // The editor is the witness because losing it is silent: Shift+F4 would
    // just stop opening anything.
    let first = in_src_and_dst(with_recording_editor);
    // Move around, which is a change worth saving and so triggers a write.
    first.key("ctrl+h");
    let home = first.close();

    let written = std::fs::read_to_string(home.path().join(SETTINGS_FILE)).unwrap();
    assert!(
        written.contains("record-editor.sh"),
        "the editor line was written away:\n{written}"
    );
    assert!(
        written.contains("diff -u %1 %2"),
        "the compare_tool line was written away:\n{written}"
    );

    // And it still works on the next run, which is what the line is for.
    let app = App::relaunch(home);
    app.key("shift+F4");
    app.focus_dialog(DIALOG_NEW_FILE);
    app.type_text("second-run.md");
    app.key("Return");
    app.await_exists("src/second-run.md");
    app.await_contents(
        "opened.log",
        &format!("{}/src/second-run.md", app.home().display()),
    );
}

#[test]
fn shift_f6_renames_in_the_list_itself() {
    // Total Commander's Shift+F6: the name turns into a field in the list
    // rather than a dialog covering the thing being renamed.
    let app = in_src_and_dst(arrange);
    // `..`, nested, data.bin, notes.txt — the cursor on notes.txt.
    app.keys(&["Home", "Down", "Down", "Down"]);

    app.key("shift+F6");
    // The editor opens with the stem selected, so typing replaces `notes` and
    // leaves `.txt` — the ordinary case, and the one that saves the typing.
    app.type_text("renamed");
    app.key("Return");

    app.await_exists("src/renamed.txt");
    app.await_gone("src/notes.txt");
    app.await_contents("src/renamed.txt", SOURCE_TEXT);
}

#[test]
fn an_inline_rename_can_change_the_extension_too() {
    // The editor carries the whole filename, not just the name column: the
    // split into name and ext is presentation, and a rename that silently
    // kept an extension the user had selected over would be a wrong answer.
    let app = in_src_and_dst(arrange);
    app.keys(&["Home", "Down", "Down", "Down"]);

    app.key("shift+F6");
    app.key("ctrl+a");
    app.type_text("all-new.md");
    app.key("Return");

    app.await_exists("src/all-new.md");
    app.await_gone("src/notes.txt");
}

#[test]
fn escape_abandons_an_inline_rename() {
    // A field with no way out but the mouse is a trap, and here the trap
    // would be holding a half-typed filename.
    let app = in_src_and_dst(arrange);
    app.keys(&["Home", "Down", "Down", "Down"]);

    app.key("shift+F6");
    app.type_text("never");
    app.key("Escape");

    app.settle();
    assert!(app.path("src/notes.txt").exists(), "the file was renamed");
    assert!(!app.path("src/never.txt").exists(), "and to that");

    // The keyboard is back on the rows rather than stuck in the field.
    app.key("F7");
    app.focus_dialog(DIALOG_NEW_DIR);
    app.type_text("after-escape");
    app.key("Return");
    app.await_exists("src/after-escape");
}

#[test]
fn an_unchanged_inline_rename_does_nothing_at_all() {
    // Pressing Enter straight away is somebody deciding not to rename, not a
    // job to run and certainly not a conflict to ask about.
    let app = in_src_and_dst(arrange);
    app.keys(&["Home", "Down", "Down", "Down"]);

    app.key("shift+F6");
    app.key("Return");

    app.settle();
    assert!(!app.has_dialog(DIALOG_CONFLICT), "it asked about itself");
    // And no failure report either. Submitting the job anyway would not lose
    // the file — the engine refuses a move onto itself — but it would put a
    // window in front of somebody who only pressed Enter, which is why
    // checking the file survived was not enough on its own.
    assert!(
        !app.has_dialog(DIALOG_FAILURES),
        "a job was submitted and refused"
    );
    assert!(app.path("src/notes.txt").exists(), "the file survived");
}

#[test]
fn the_parent_row_cannot_be_renamed() {
    // `..` is a navigation control, not a file. Opening an editor on it would
    // offer to rename the directory you are standing in from inside it.
    let app = in_src_and_dst(arrange);
    app.key("Home");

    app.key("shift+F6");
    // Proof by where the typing went. With no editor open the letters reach
    // the *rows* and search them, so `..` keeps its name and the cursor moves
    // instead. (This used to prove the same thing through the command line,
    // back when an unclaimed letter typed there.)
    app.type_text("notes");
    app.key("Return");

    // Still pins the behaviour rather than the guard that implements it:
    // removing the guard leaves this green, because nothing else today opens
    // an editor on `..` either. Said plainly rather than left to look like
    // coverage it is not; the guard's own note in `pane.rs` says the same.
    assert!(app.path("src/nested").exists());
    assert!(app.path("src/notes.txt").exists());
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
fn shift_page_up_marks_across_a_screenful_the_other_way() {
    // The upward twin, and the one that has to clamp: a page above the second
    // row runs off the top, where `..` sits and must not be picked up.
    let app = in_src_and_dst(arrange);
    app.key("End");
    app.key("shift+Prior");

    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    app.await_exists("dst/nested");
    app.await_exists("dst/data.bin");
    app.await_exists("dst/notes.txt");
    app.settle();
    assert!(
        !app.path("dst/src").exists() && !app.path("dst/dst").exists(),
        "the page ran off the top and marked the parent row"
    );
}

#[test]
fn alt_numminus_takes_the_extension_marks_back() {
    let app = in_src_and_dst(with_two_text_files);
    app.key("ctrl+a");
    // `..`, nested, data.bin, notes.txt, other.txt — cursor on notes.txt.
    app.keys(&["Home", "Down", "Down", "Down"]);
    app.key("alt+KP_Subtract");

    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    // The two .txt files lost their marks; the rest kept theirs.
    app.await_exists("dst/data.bin");
    app.await_exists("dst/nested");
    app.settle();
    for gone in ["dst/notes.txt", "dst/other.txt"] {
        assert!(!app.path(gone).exists(), "{gone} kept its mark");
    }
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

/// Where the app recorded each pane, read back the way the app wrote it.
///
/// The pane directory is the one part of a drive change that is observable
/// from outside: the destination is a real mount point, and a test must not
/// go writing files into `/` to prove it got there.
fn recorded_directories(home: &Path) -> Vec<String> {
    let config = VfsPath::new(home.join(".config").to_str().unwrap());
    let (settings, complaint) = fc_core::config::load(&LocalFs, &config);
    // A file that is not there yet is a first run that has not saved, not a
    // problem: this is polled while the app is starting up.
    assert_eq!(complaint, None, "the settings could not be read back");
    (0..2).map(|index| settings.pane(index).directory).collect()
}

/// The place the drive selector offers at `index`, in the order it lists
/// them — the same call the app itself makes.
fn drive_at(index: usize) -> String {
    fc_core::vfs::mount_points()
        .get(index)
        .unwrap_or_else(|| panic!("the machine has no mount point {index}"))
        .path
        .as_str()
        .to_string()
}

/// Which row of the drive list is the mount this app's files are on.
///
/// **Asked, not assumed.** These tests used to index the list positionally —
/// slot 0 for "the drive the pane is already on", slot 1 for "somewhere
/// else" — with comments saying the pane starts on the root mount. That is
/// true of the machine they were written on and of nothing else: the list is
/// `/proc/self/mounts` order, and a test report from a stock Ubuntu desktop
/// had `/run` at slot 0 and `/` at slot 1. Five tests then went to the mount
/// the pane was already on, which a drive that remembers where it was left
/// makes invisible — the exact trap one of their comments says slot 1 was
/// chosen to avoid.
fn home_drive_index(app: &App) -> usize {
    let mounts = fc_core::vfs::mount_points();
    let home = fc_core::vfs::LocalFs::vfs_path(app.home());
    let on = fc_core::vfs::mount_for(&home, &mounts)
        .unwrap_or_else(|| panic!("nothing in {mounts:?} holds {home}"));
    mounts
        .iter()
        .position(|mount| mount.path == on)
        .expect("the mount it named is in the list it came from")
}

/// A row that is some *other* drive, so going there is a move.
///
/// Panics rather than skips when the machine offers only one: a drive test
/// with nowhere to switch to proves nothing, and a test that quietly does not
/// run is worse than no test — the same rule the harness applies to a missing
/// `xdotool`.
fn other_drive_index(app: &App) -> usize {
    other_drive_index_from(app, 0)
}

/// The same, but never the first row — so reaching it takes an arrow press.
///
/// What the "the list opens focused" test needs, and it needs *both* halves:
/// a row an arrow has to reach, and one the pane is not already on, since a
/// drive that remembers where it was left makes returning to it look exactly
/// like never having moved.
fn arrowed_drive_index(app: &App) -> usize {
    other_drive_index_from(app, 1)
}

fn other_drive_index_from(app: &App, first: usize) -> usize {
    let home = home_drive_index(app);
    (first..fc_core::vfs::mount_points().len())
        .find(|&index| index != home)
        .unwrap_or_else(|| {
            panic!(
                "no drive at row {first} or later that the pane is not already on, \
                 so there is nowhere for this test to switch to: {:?}",
                fc_core::vfs::mount_points()
            )
        })
}

#[test]
fn alt_f1_sends_the_left_pane_to_a_drive() {
    let app = in_src_and_dst(arrange);

    // Some *other* mount than the one the pane is already on: a drive
    // remembers where it was left, so picking the one it is on would land it
    // back where it started and prove nothing.
    let elsewhere = other_drive_index(&app);
    pick_drive(&app, elsewhere);

    // Closed rather than killed: the settings write is debounced, and
    // `await_mentions` cannot wait for it here — the file already says
    // "directory" from an earlier save, so there is no new text to watch for.
    // The close handler flushes, which makes this exact rather than lucky.
    let home = app.close();
    assert_eq!(recorded_directories(home.path())[0], drive_at(elsewhere));
}

#[test]
fn the_f_key_number_is_the_pane_number_whatever_has_the_keyboard() {
    // Absolute, as in Total Commander: Alt+F2 names the right pane even when
    // the keyboard is in the left one. This is the opposite rule to
    // Ctrl+arrow, and pressing it from the "wrong" side is the only way to
    // tell the two apart.
    let app = in_src_and_dst(arrange);
    // The left pane has the keyboard after `in_src_and_dst`.
    let elsewhere = other_drive_index(&app);
    pick_drive_with(&app, "alt+F2", elsewhere);

    // Closed rather than killed: the settings write is debounced, and
    // `await_mentions` cannot wait for it here — the file already says
    // "directory" from an earlier save, so there is no new text to watch for.
    // The close handler flushes, which makes this exact rather than lucky.
    let home = app.close();
    let recorded = recorded_directories(home.path());
    assert_eq!(
        recorded[1],
        drive_at(elsewhere),
        "the right pane did not move"
    );
    assert!(
        recorded[0].ends_with("/src"),
        "the left pane moved too: {}",
        recorded[0]
    );
}

/// `src` with two folders of very different weight, and an empty `dst`.
///
/// Uncounted they are both zero and tie, so the name breaks it; counted they
/// do not. That is what makes the sort able to say whether counting happened.
fn with_two_folders(home: &Path) {
    std::fs::create_dir_all(home.join("src/big")).unwrap();
    std::fs::create_dir_all(home.join("src/small")).unwrap();
    std::fs::create_dir(home.join("dst")).unwrap();
    std::fs::write(home.join("src/big/data.bin"), vec![9u8; 4096]).unwrap();
    std::fs::write(home.join("src/small/tiny.txt"), "x").unwrap();
}

#[test]
fn alt_shift_enter_counts_the_marked_folders_and_the_sort_can_see_it() {
    // The size column is not something this suite can read, so the counted
    // number is observed through the one thing that reacts to it: sorting by
    // size. Uncounted, both folders are zero and tie, so the name breaks it
    // and `big` comes first. Counted, `small` is genuinely smaller and leads.
    // Copying whichever the cursor lands on says which happened.
    let app = App::launch(with_two_folders);
    app.keys(&["Tab", "Down", "Return", "Tab", "Down", "Down", "Return"]);
    await_panes_at(&app, "/src", "/dst");

    // src is `..`, big, small. Mark both folders.
    app.keys(&["Home", "Down"]);
    app.keys(&["Insert", "Insert"]);

    app.key("alt+shift+Return");
    app.settle();

    // Unmark, or F5 would act on the marks rather than on the cursor row.
    app.key("ctrl+KP_Subtract");
    app.key("ctrl+F6");
    app.keys(&["Home", "Down"]);
    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    app.settle();
    let landed: Vec<String> = std::fs::read_dir(app.path("dst"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(landed, ["small"], "the sort could not tell them apart");
}

#[test]
fn space_counts_the_folder_it_marks_and_the_second_does_not_cost_the_first() {
    // Total Commander counts a folder as `Space` marks it, which is what
    // makes the status line's marked-bytes total true — a directory's size is
    // zero until something counts it, so marking folders reports `0 B`.
    //
    // The size column is not something this suite can read, so the count is
    // observed the way the `Alt+Shift+Enter` test above observes it: through
    // sorting by size. Uncounted, both folders are zero and tie, so the name
    // breaks it and `big` leads. Counted, `small` is genuinely smaller.
    //
    // **Two folders, not one**, and that is the point: marking the second
    // must not cancel the first one's scan, which is what the shared
    // `abandon_background` would have done.
    let app = App::launch(with_two_folders);
    app.keys(&["Tab", "Down", "Return", "Tab", "Down", "Down", "Return"]);
    await_panes_at(&app, "/src", "/dst");

    // src is `..`, big, small. `Space` does not advance, so step between them.
    app.keys(&["Home", "Down"]);
    app.key("space");
    app.key("Down");
    app.key("space");
    app.settle();

    // Unmark, or F5 would act on the marks rather than on the cursor row.
    app.key("ctrl+KP_Subtract");
    app.key("ctrl+F6");
    app.keys(&["Home", "Down"]);
    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    app.settle();
    let landed: Vec<String> = std::fs::read_dir(app.path("dst"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        landed,
        ["small"],
        "the sort could not tell the folders apart, so at least one went uncounted"
    );
}

#[test]
fn insert_marks_a_folder_without_counting_it() {
    // The owner's split, and Total Commander's: `Space` counts, `Insert` and
    // `Shift+Down` only mark. `Insert` is the key you hold to sweep a run of
    // rows, and holding it down a list of folders would start a walk per row.
    //
    // Read against the test above, which is the same sequence with the same
    // assertion inverted: uncounted, the two folders tie at zero and the name
    // breaks it, so `big` leads.
    let app = App::launch(with_two_folders);
    app.keys(&["Tab", "Down", "Return", "Tab", "Down", "Down", "Return"]);
    await_panes_at(&app, "/src", "/dst");

    app.keys(&["Home", "Down"]);
    app.keys(&["Insert", "Insert"]);
    app.settle();

    app.key("ctrl+KP_Subtract");
    app.key("ctrl+F6");
    app.keys(&["Home", "Down"]);
    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    app.settle();
    let landed: Vec<String> = std::fs::read_dir(app.path("dst"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        landed,
        ["big"],
        "Insert counted a folder; only Space is meant to"
    );
}

#[test]
fn a_re_read_forgets_the_counted_sizes() {
    // A re-read is a fresh answer from the filesystem, and a count carried
    // over from before it could be stale in a way nothing on screen admits.
    // Observed the same way the counting is: uncounted, the two folders tie
    // at zero and the name breaks it, so `big` leads again.
    //
    // This pins the **sizes** going, which is the user-visible half. It does
    // not pin the `measured` flags going with them: `read_dir` zeroes the
    // sizes either way, so a listing that kept stale flags would still tie
    // here — it would merely draw `0` where `<DIR>` belongs, which is a
    // column this suite cannot read. That half is pinned headlessly, by
    // `a_re_read_forgets_the_sizes_but_keeps_the_marks`.
    let app = App::launch(with_two_folders);
    app.keys(&["Tab", "Down", "Return", "Tab", "Down", "Down", "Return"]);
    await_panes_at(&app, "/src", "/dst");

    app.keys(&["Home", "Down"]);
    app.keys(&["Insert", "Insert"]);
    app.key("alt+shift+Return");
    app.settle();
    app.key("ctrl+KP_Subtract");

    app.key("ctrl+r");
    app.settle();

    app.key("ctrl+F6");
    app.keys(&["Home", "Down"]);
    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");
    app.settle();

    let landed: Vec<String> = std::fs::read_dir(app.path("dst"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(landed, ["big"], "a stale size survived the re-read");
}

#[test]
fn ctrl_b_lists_the_whole_tree_and_f5_copies_from_two_levels_down() {
    // The whole feature in one sequence: flatten, then act on a file the pane
    // could not otherwise reach without navigating to it. F5 with nothing
    // marked copies the cursor row, so what lands in dst says which row that
    // was — and `nested/inner.txt` is a row that exists only because the tree
    // was flattened.
    //
    // The assertion distinguishes the two outcomes rather than merely
    // checking that something arrived: had the walk not landed, row two would
    // still be `nested`, and copying *that* would put the file at
    // `dst/nested/inner.txt` instead.
    let app = in_src_and_dst(arrange);

    app.key("ctrl+b");
    // The walk is on a worker, and there is nothing outside the window that
    // changes when it lands — the pane's recorded directory is the same root
    // it was walked from. So this is one of the few places the suite waits by
    // the clock rather than for an effect.
    app.settle();
    // src flattened is `..`, data.bin, nested/inner.txt, notes.txt — sorted
    // by name, which for a branch row is its path.
    app.keys(&["Home", "Down", "Down"]);

    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    app.await_contents("dst/inner.txt", "deep");
    assert!(
        !app.path("dst/nested").exists(),
        "the copy came from the plain listing, not the branch view"
    );
}

#[test]
fn a_job_from_a_branch_view_leaves_a_branch_view() {
    // The pane reloads after a job, and the reload must not quietly turn the
    // flattened tree back into one directory. Copying and then copying again
    // from a row that only a branch view has is what says it did not.
    let app = in_src_and_dst(arrange);
    app.key("ctrl+b");
    app.settle();

    app.keys(&["Home", "Down", "Down"]);
    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");
    app.await_contents("dst/inner.txt", "deep");

    // Still flat: the same row is still two down, and copying it again is
    // still a copy of the deep file rather than of the `nested` directory.
    std::fs::remove_file(app.path("dst/inner.txt")).unwrap();
    app.keys(&["Home", "Down", "Down"]);
    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");
    app.await_contents("dst/inner.txt", "deep");
    assert!(
        !app.path("dst/nested").exists(),
        "the pane fell back to one directory"
    );
}

#[test]
fn escape_in_a_settled_branch_view_still_clears_the_filter() {
    // Escape gained a first job — stopping a walk — and must not keep eating
    // the keystroke once there is no walk to stop. A landed arrival clears
    // the token, and this is what says so: without that, the filter would
    // survive the Escape and row two would still be `notes.txt`.
    //
    // The mid-walk cancel itself is *not* tested here. On a fixture small
    // enough for this suite the walk lands before the second keystroke
    // arrives, so a test that pressed Escape into it would be a race dressed
    // as an assertion. The walk's half of the cancel is pinned headlessly in
    // `fc-core/tests/branch.rs`.
    let app = in_src_and_dst(arrange);
    app.key("ctrl+b");
    app.settle();

    // Return first, which hands the keyboard back to the rows while the
    // filter stays applied — the field's own handler takes Escape, so this is
    // the only way the keymap's ClearFilter is reached at all.
    app.key("ctrl+s");
    app.type_text("notes");
    app.key("Return");
    app.key("Escape");
    app.settle();

    // The filter is gone, so the flattened rows are all back: `..`,
    // data.bin, nested/inner.txt, notes.txt.
    app.keys(&["Home", "Down", "Down"]);
    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    app.await_contents("dst/inner.txt", "deep");
}

#[test]
fn escape_still_clears_a_filter_when_no_walk_is_running() {
    // Escape gained a first job and must not have lost its old one. The
    // filter narrows src to `notes.txt` alone; clearing it puts the other
    // rows back, which the copy of row two then proves.
    let app = in_src_and_dst(arrange);

    // Return hands the keyboard back to the rows with the filter still on;
    // the field's own handler takes Escape, so this is the only way the
    // keymap's ClearFilter is reached.
    app.key("ctrl+s");
    app.type_text("notes");
    app.key("Return");
    app.key("Escape");
    app.settle();

    app.keys(&["Home", "Down", "Down"]);
    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    app.await_exists("dst/data.bin");
}

#[test]
fn a_branch_row_cannot_be_renamed_in_the_list() {
    // A branch row is named by its path, and an inline rename edits a name.
    // Opening on `nested/inner.txt` would offer the whole path for editing
    // and then either fail or move the file somewhere nobody asked for.
    //
    // Unlike the `..` guard this joins, the case is reachable — removing the
    // check turns this test red.
    let app = in_src_and_dst(arrange);
    app.key("ctrl+b");
    app.settle();
    app.keys(&["Home", "Down", "Down"]);

    app.key("shift+F6");
    // Proof by where the typing went. With no editor open the letters reach
    // the *rows* and search them, so the cursor moves — which a copy then
    // shows. (This used to prove the same thing through the command line,
    // back when an unclaimed letter typed there.)
    app.type_text("inner");
    app.key("Return");

    assert!(
        app.path("src/nested/inner.txt").exists(),
        "the deep file was renamed out from under the tree"
    );
}

#[test]
fn alt_f5_in_a_branch_view_offers_a_name_and_not_a_path() {
    // The prefill is `inner.zip`, not `nested/inner.zip` — which would name a
    // directory the pane being written to need not have. Accepting the
    // prefill unchanged is the whole test: if it carried the directory, the
    // pack would fail instead of producing an archive.
    let app = in_src_and_dst(arrange);
    app.key("ctrl+b");
    app.settle();
    app.keys(&["Home", "Down", "Down"]);

    app.key("alt+F5");
    app.focus_dialog(DIALOG_PACK);
    app.key("Return");

    app.await_exists("dst/inner.zip");
}

#[test]
fn marks_from_several_directories_copy_in_one_job_and_land_flat() {
    // What a branch view is for: collecting files from all over a tree and
    // then acting on them at once. They land flat, because a copy's
    // destination is built from each source's own file name.
    let app = in_src_and_dst(arrange);
    app.key("ctrl+b");
    app.settle();

    // Everything visible: `..`, data.bin, nested/inner.txt, notes.txt.
    app.key("ctrl+a");
    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    app.await_contents("dst/inner.txt", "deep");
    app.await_contents("dst/notes.txt", SOURCE_TEXT);
    app.await_exists("dst/data.bin");
    assert!(
        !app.path("dst/nested").exists(),
        "the copy rebuilt the tree instead of landing flat"
    );
}

#[test]
fn navigating_out_of_a_branch_view_leaves_it() {
    // Leaving is not a key of its own: any step lands an ordinary listing of
    // somewhere, and the flat rows go with it. Backspace to the parent, then
    // F7 — which creates in the pane's directory — proves the pane is a plain
    // listing of home.
    let app = in_src_and_dst(arrange);
    app.key("ctrl+b");
    app.settle();

    // Up to the home directory, whose path is the tempdir's and so has no
    // suffix `await_panes_at` could match on.
    app.key("BackSpace");
    app.settle();

    app.key("F7");
    app.focus_dialog(DIALOG_NEW_DIR);
    app.type_text("made-at-home");
    app.key("Return");
    app.await_exists("made-at-home");
    assert!(
        !app.path("src/made-at-home").exists(),
        "the pane was still showing the branch view of src"
    );
}

#[test]
fn ctrl_d_sends_the_pane_that_has_the_keyboard() {
    // A key with no direction and no number in it acts on the active pane —
    // the ordinary rule, and the opposite of Alt+F1/Alt+F2 where the F-key
    // number *is* the pane number. The other pane not moving is half the
    // claim, and the half a bug would break.
    let app = in_src_and_dst(with_favourites);

    app.key("ctrl+d");
    app.focus_dialog(DIALOG_FAVOURITES);
    app.key("Return");

    await_panes_at(&app, "/src/nested", "/dst");
}

#[test]
fn ctrl_d_in_the_right_pane_sends_the_right_pane() {
    // The same key from the other side. Without this, an implementation that
    // always acts on pane 0 passes the test above.
    let app = in_src_and_dst(with_favourites);
    app.key("Tab");

    app.key("ctrl+d");
    app.focus_dialog(DIALOG_FAVOURITES);
    app.key("Return");

    await_panes_at(&app, "/src", "/src/nested");
}

#[test]
fn a_favourite_takes_a_pane_out_of_an_archive() {
    // A favourite is a path on the real filesystem, which the archive backend
    // has never heard of. Navigating on the current backend would send the
    // *archive* there, so the pane would show an error and still be inside,
    // with Backspace the only way out.
    let app = in_src_and_dst(with_favourites);
    // `..`, nested, bundle.zip, data.bin, notes.txt.
    app.keys(&["Home", "Down", "Down"]);
    app.key("Return");
    await_panes_at(&app, "/src/bundle.zip", "/dst");

    app.key("ctrl+d");
    app.focus_dialog(DIALOG_FAVOURITES);
    // The second favourite, so arriving somewhere is visible rather than a
    // return to a directory this pane was already showing.
    app.key("Down");
    app.key("Return");

    await_panes_at(&app, "/dst", "/dst");
    // And it really left: F7 creates in the real directory, not in a zip.
    app.key("F7");
    app.focus_dialog(DIALOG_NEW_DIR);
    app.type_text("out-of-the-archive");
    app.key("Return");
    app.await_exists("dst/out-of-the-archive");
}

#[test]
fn a_favourite_whose_directory_is_gone_leaves_the_pane_where_it_was() {
    // Unlike a drive, a favourite has no mount to fall back to — so the right
    // answer is the ordinary one for a directory that cannot be read: stay,
    // and say why. Sending the pane somewhere nobody named would be worse.
    let app = in_src_and_dst(with_favourites);
    std::fs::remove_dir_all(app.path("src/nested")).unwrap();

    app.key("ctrl+d");
    app.focus_dialog(DIALOG_FAVOURITES);
    app.key("Return");
    app.settle();

    // Still in src, and still working: F7 lands where the pane says it is.
    app.key("F7");
    app.focus_dialog(DIALOG_NEW_DIR);
    app.type_text("still-here");
    app.key("Return");
    app.await_exists("src/still-here");
}

#[test]
fn the_list_keeps_the_directory_the_pane_is_in_and_still_has_it_next_time() {
    // The whole of Total Commander's rule that the hotlist is maintained from
    // inside itself: no second key, and the moment you notice a directory is
    // missing is the moment you are looking at the list.
    //
    // The relaunch is the half that matters. A list that is added to and
    // never written looks exactly like one that works, until the next start.
    let first = in_src_and_dst(with_favourites);
    // Somewhere that is not already a favourite, or adding it is correctly a
    // no-op and the test would be measuring the de-duplication instead.
    first.key("F7");
    first.focus_dialog(DIALOG_NEW_DIR);
    first.type_text("kept-place");
    first.key("Return");
    first.await_exists("src/kept-place");
    // src lists `..`, kept-place, nested, then the files.
    first.keys(&["Home", "Down"]);
    first.key("Return");
    await_panes_at(&first, "/src/kept-place", "/dst");

    first.key("ctrl+d");
    first.focus_dialog(DIALOG_FAVOURITES);
    // Past the two favourites, onto the row that adds.
    first.keys(&["Down", "Down"]);
    first.key("Return");
    let kept = await_favourites(&first, 3);
    assert!(
        kept[2].ends_with("src/kept-place"),
        "the added favourite is not the directory the pane was in: {kept:?}"
    );
    // Escape rather than closing on the add: the window stays open, which is
    // what makes adding and then going somewhere one visit to the list.
    assert!(
        first.has_dialog(DIALOG_FAVOURITES),
        "adding closed the window"
    );
    first.key("Escape");
    first.await_dialog_closed(DIALOG_FAVOURITES);
    let home = first.kill();

    // It is there on the next run, and going to it works. The pane starts
    // where it was left, so it has to leave first or arriving there again
    // would be indistinguishable from never moving.
    let app = App::relaunch(home);
    app.key("BackSpace");
    await_panes_at(&app, "/src", "/dst");

    app.key("ctrl+d");
    app.focus_dialog(DIALOG_FAVOURITES);
    app.keys(&["Down", "Down"]);
    app.key("Return");
    app.await_dialog_closed(DIALOG_FAVOURITES);

    app.key("F7");
    app.focus_dialog(DIALOG_NEW_DIR);
    app.type_text("added-then-used");
    app.key("Return");
    app.await_exists("src/kept-place/added-then-used");
}

#[test]
fn delete_takes_a_favourite_off_the_list() {
    // The other half of maintaining it from inside: a list that can only grow
    // is one nobody keeps tidy.
    let app = in_src_and_dst(with_favourites);

    app.key("ctrl+d");
    app.focus_dialog(DIALOG_FAVOURITES);
    app.key("Delete");
    app.key("Escape");
    app.await_dialog_closed(DIALOG_FAVOURITES);

    // One of the two is gone, and it is the one the cursor was on.
    let left = await_favourites(&app, 1);
    assert!(
        left[0].ends_with("dst"),
        "Delete took the wrong row: {left:?}"
    );
}

#[test]
fn a_directory_inside_an_archive_is_refused_rather_than_kept() {
    // A path in there belongs to the archive's own store, where the same
    // spelling means a completely different file. The probe that removed the
    // refusal is what says why this matters: what got kept was `/`, the
    // archive's own root — a favourite that on the next run would quietly
    // send the pane to the root of the disk.
    let app = in_src_and_dst(with_favourites);
    // `..`, nested, bundle.zip, data.bin, notes.txt.
    app.keys(&["Home", "Down", "Down"]);
    app.key("Return");
    await_panes_at(&app, "/src/bundle.zip", "/dst");

    app.key("ctrl+d");
    app.focus_dialog(DIALOG_FAVOURITES);
    app.keys(&["Down", "Down"]);
    app.key("Return");
    // The window stays open with the reason in it, so Escape is what closes
    // it — a dialog that vanished would look like the add had worked.
    assert!(
        app.has_dialog(DIALOG_FAVOURITES),
        "the refusal closed the window"
    );
    app.key("Escape");
    app.await_dialog_closed(DIALOG_FAVOURITES);
    app.settle();

    let kept = recorded_favourites(&app);
    assert_eq!(kept.len(), 2, "a path inside an archive was kept: {kept:?}");
}

#[test]
fn the_drive_list_opens_focused_so_the_arrows_work_without_a_click() {
    // What the explicit focus and first-row selection are *for*. Enter alone
    // would land on the first row anyway, so only reaching the second one
    // says whether the list has the keyboard.
    let app = in_src_and_dst(arrange);

    // A row an arrow has to reach, and one the pane is not already on.
    let elsewhere = arrowed_drive_index(&app);
    pick_drive(&app, elsewhere);

    // Closed rather than killed: the settings write is debounced, and
    // `await_mentions` cannot wait for it here — the file already says
    // "directory" from an earlier save, so there is no new text to watch for.
    // The close handler flushes, which makes this exact rather than lucky.
    let home = app.close();
    assert_eq!(recorded_directories(home.path())[0], drive_at(elsewhere));
}

/// Opens the drive list on the left pane and takes the `index`-th place.
fn pick_drive(app: &App, index: usize) {
    pick_drive_with(app, "alt+F1", index);
}

/// The same, for whichever pane the key names.
fn pick_drive_with(app: &App, key: &str, index: usize) {
    app.focus_main();
    app.key(key);
    app.focus_dialog(DIALOG_DRIVES);
    for _ in 0..index {
        app.key("Down");
    }
    app.key("Return");
    app.await_dialog_closed(DIALOG_DRIVES);
}

#[test]
fn a_drive_remembers_the_directory_it_was_left_in() {
    // Total Commander's behaviour with its default AlwaysToRoot=0: switching
    // away from a drive and back is not a trip to the root and a walk down
    // again. The pane starts in `src`, wherever that is mounted.
    let app = in_src_and_dst(arrange);

    // Away to another mount, then back to the one the pane's files are on.
    pick_drive(&app, other_drive_index(&app));
    pick_drive(&app, home_drive_index(&app));

    // Closed rather than killed: the settings write is debounced, and
    // `await_mentions` cannot wait for it here — the file already says
    // "directory" from an earlier save, so there is no new text to watch for.
    // The close handler flushes, which makes this exact rather than lucky.
    let home = app.close();
    assert!(
        recorded_directories(home.path())[0].ends_with("/src"),
        "the drive forgot where it was left: {}",
        recorded_directories(home.path())[0]
    );
}

#[test]
fn the_two_panes_share_what_a_drive_remembers() {
    // As in Total Commander, where leaving a drive in one panel is what the
    // other finds when it arrives there.
    let app = in_src_and_dst(arrange);

    // Both panes start on the mount their files are on — the left in src, the
    // right in dst. Send them both somewhere else. The right pane leaves
    // last, so that mount's memory ends up saying `dst`.
    let (here, elsewhere) = (home_drive_index(&app), other_drive_index(&app));
    pick_drive(&app, elsewhere);
    pick_drive_with(&app, "alt+F2", elsewhere);

    // Now bring the *left* pane back. It lands in dst, which only the other
    // pane has ever been in.
    pick_drive(&app, here);

    // Closed rather than killed: the settings write is debounced, and
    // `await_mentions` cannot wait for it here — the file already says
    // "directory" from an earlier save, so there is no new text to watch for.
    // The close handler flushes, which makes this exact rather than lucky.
    let home = app.close();
    assert!(
        recorded_directories(home.path())[0].ends_with("/dst"),
        "the pane did not inherit what the other one left on that drive: {}",
        recorded_directories(home.path())[0]
    );
}

#[test]
fn a_remembered_directory_that_has_gone_falls_back_to_the_drive_itself() {
    // An unplugged disk or a deleted folder must not leave the pane showing
    // an error about a path the user never asked for by name.
    let app = App::launch(|home| {
        arrange(home);
        std::fs::create_dir(home.join("temporary")).unwrap();
    });
    // Into `temporary`, so the root mount remembers it. `..`, dst, nested is
    // not here — the home directory holds dst, src, temporary.
    app.keys(&["Home", "Down", "Down", "Down", "Return"]);

    let here = home_drive_index(&app);
    pick_drive(&app, other_drive_index(&app));
    std::fs::remove_dir(app.path("temporary")).unwrap();
    pick_drive(&app, here);

    // Closed rather than killed: the settings write is debounced, and
    // `await_mentions` cannot wait for it here — the file already says
    // "directory" from an earlier save, so there is no new text to watch for.
    // The close handler flushes, which makes this exact rather than lucky.
    let home = app.close();
    assert_eq!(
        recorded_directories(home.path())[0],
        drive_at(here),
        "a directory that had gone was not replaced by the drive itself"
    );
}

#[test]
fn escape_leaves_the_drive_selector_without_going_anywhere() {
    // A chooser with no way out but the mouse is a trap in a keyboard-first
    // program, and this one has no Cancel button to fall back on.
    let app = in_src_and_dst(arrange);

    app.key("alt+F1");
    app.focus_dialog(DIALOG_DRIVES);
    app.key("Escape");
    app.await_dialog_closed(DIALOG_DRIVES);

    app.focus_main();
    // Closed rather than killed: the settings write is debounced, and
    // `await_mentions` cannot wait for it here — the file already says
    // "directory" from an earlier save, so there is no new text to watch for.
    // The close handler flushes, which makes this exact rather than lucky.
    let home = app.close();
    assert!(
        recorded_directories(home.path())[0].ends_with("/src"),
        "escaping the drive list still moved the pane"
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
fn page_down_then_f5_acts_on_the_row_the_widget_moved_to() {
    // Page Up/Down are deliberately *not* bound: the widget knows the height
    // of the viewport and the model does not. So the widget moves its own
    // selection without telling the model, and every action starts by
    // adopting it — otherwise the next key acts on a row the user stopped
    // looking at a page ago.
    //
    // Recorded in future-improvements.md as needing "an assertion about which
    // row the cursor is on, which the filesystem cannot answer". It can: F5
    // with nothing marked copies the cursor row, so the filesystem says which
    // row that was.
    let app = in_src_and_dst(with_a_tall_directory);

    // Home puts the model cursor on `..`, which is not a thing F5 will copy.
    app.key("Home");
    app.key("Next");

    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    // Something arrived, and it is not the row Home left the model on: the
    // model took over the selection the widget had moved.
    let copied = await_any_copy(&app);
    assert!(
        copied.starts_with("row"),
        "F5 copied {copied:?} rather than a row the page landed on"
    );
}

#[test]
fn page_down_moves_the_cursor_a_screenful_and_page_up_brings_it_back() {
    // The plain page keys are the model's now, not the widget's. A round trip
    // is what says the two directions agree about how far a page is: down
    // then up has to land where it started, or one of them is off by the
    // overlap row and only a screenshot would ever say so.
    //
    // F5 with nothing marked copies the cursor row, so the filesystem reports
    // where the cursor ended up.
    let app = in_src_and_dst(with_a_tall_directory);

    app.key("Home");
    app.key("Next");
    app.key("Prior");
    // Down off `..`, which F5 will not copy, and onto the first real row.
    app.key("Down");

    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    // Home is `..`; one row down from it is the only directory, `nested`.
    let copied = await_any_copy(&app);
    assert_eq!(
        copied, "nested",
        "a page down and back up did not return the cursor to where it started"
    );
}

#[test]
fn page_down_lands_a_screenful_further_on_not_at_the_end() {
    // The other half: that a page is a *page*. If the step were the whole
    // listing the round trip above would still pass, because both directions
    // would clamp.
    let app = in_src_and_dst(with_a_tall_directory);

    app.key("Home");
    app.key("Next");

    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    let copied = await_any_copy(&app);
    assert!(
        copied.starts_with("row"),
        "a page down landed on {copied:?}, which is not one of the 200 rows"
    );
    // The last row is what a step of the whole listing would reach.
    assert_ne!(
        copied,
        format!("row{:03}.txt", TALL_DIRECTORY_ROWS - 1),
        "a page down ran to the end of the listing rather than one screenful"
    );
}

#[test]
fn a_click_then_f5_copies_the_row_that_was_clicked() {
    // The dispatched half of the click. Its sibling above goes through
    // type-ahead, which is not an action; this one goes through `dispatch`,
    // and between them they cover both routes out of a click.
    //
    // `Home` parks the model on `..`, which F5 refuses to copy with nothing
    // marked. So if the click never reaches the model, F5 does nothing at all
    // and `await_any_copy` times out saying so — the absence is the assertion.
    let app = in_src_and_dst(with_a_tall_directory);

    app.key("Home");
    app.click((200, 500));

    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    let copied = await_any_copy(&app);
    assert!(
        copied.starts_with("row"),
        "F5 copied {copied:?} rather than the row that was clicked"
    );
}

#[test]
fn a_click_then_a_letter_searches_from_the_row_that_was_clicked() {
    // A mouse click is the only thing left that moves the widget's selection
    // without the model hearing: the keyboard routes all adopt, and the page
    // keys stopped being the widget's on 2026-08-30. So this is the last
    // instance of the class, and nothing covered it.
    //
    // `Home` parks the model on `..`. A click well down the pane moves the
    // widget somewhere else entirely. Typing `r` then searches from one of
    // the two, and F5 with nothing marked copies whatever the cursor found —
    // so the filesystem says which cursor answered.
    let app = in_src_and_dst(with_a_tall_directory);

    app.key("Home");
    // Left pane, low enough to be well past the first `row*.txt`. Which row
    // this is does not matter and must not be asserted on.
    app.click((200, 500));
    app.key("r");

    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    let copied = await_any_copy(&app);
    assert!(
        copied.starts_with("row"),
        "F5 copied {copied:?} rather than a row the search landed on"
    );
    assert_ne!(
        copied, "row000.txt",
        "type-ahead searched from the model's stale cursor, not from the clicked row"
    );
}

#[test]
fn page_down_then_a_letter_searches_from_the_row_on_screen() {
    // The same stale-cursor trap as the two tests above, on the one path that
    // is *not* a dispatched action: a key no binding claims goes straight to
    // type-ahead, which never adopted the widget's selection. So the search
    // began from the row the model still believed in — off the top of the
    // screen — while the user looked at another one.
    //
    // `Home` puts the model on `..`, then a page moves the widget well past
    // the first `row*.txt`. Typing `r` from the stale cursor would find
    // `row000.txt`, the first of them; from the row actually on screen it
    // finds one much further down. F5 with nothing marked copies the cursor
    // row, so the filesystem says which cursor was asked.
    let app = in_src_and_dst(with_a_tall_directory);

    app.key("Home");
    app.key("Next");
    app.key("r");

    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    let copied = await_any_copy(&app);
    assert!(
        copied.starts_with("row"),
        "F5 copied {copied:?} rather than a row the search landed on"
    );
    assert_ne!(
        copied, "row000.txt",
        "type-ahead searched from the cursor the page left behind, not from the row on screen"
    );
}

#[test]
fn page_down_then_space_marks_the_row_the_widget_moved_to() {
    // The same stale-cursor trap as the F5 test above, on the path where the
    // adopting used to be written out again: marking. The mark has to land on
    // the row the page left the widget on, not on the row the model still
    // thinks it is on — so `Home` afterwards, which moves the cursor away and
    // leaves the mark as the only thing F5 can be acting on.
    let app = in_src_and_dst(with_a_tall_directory);

    app.key("Home");
    app.key("Next");
    app.key("space");
    app.key("Home");

    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    let copied = await_any_copy(&app);
    assert!(
        copied.starts_with("row"),
        "F5 copied {copied:?} rather than the row the page landed on"
    );
}

/// Waits for anything at all to appear in `dst`, and says what it was.
fn await_any_copy(app: &App) -> String {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        let arrived: Vec<String> = std::fs::read_dir(app.path("dst"))
            .expect("dst exists")
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        if let Some(name) = arrived.first() {
            return name.clone();
        }
        assert!(
            std::time::Instant::now() < deadline,
            "nothing was copied: the model acted on a stale cursor"
        );
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
}

#[test]
fn ctrl_u_carries_the_sort_order_and_the_hidden_files_setting() {
    // The exchange swaps six fields by name. Two tests already cover the
    // listing and the backend; these are the ones nothing was watching, and
    // the settings file is where they are observable from outside.
    let app = in_src_and_dst(arrange);
    // Left pane only: sort by size, and show the dot-files.
    app.key("ctrl+F6");
    app.key("ctrl+h");
    await_pane_sort(&app, 0, "size", true);

    app.key("ctrl+u");

    // What the left pane had is now the right pane's, and the right pane's
    // defaults are now the left's.
    await_pane_sort(&app, 1, "size", true);
    await_pane_sort(&app, 0, "name", false);
}

#[test]
fn ctrl_u_carries_the_selection_a_job_spent() {
    // `Num /` restores the marks the last operation consumed, and those live
    // beside the listing rather than in it — so the exchange has to carry
    // them too, and nothing was checking that it did.
    let app = in_src_and_dst(with_two_text_files);
    // Mark both .txt files and spend them on a copy.
    app.keys(&["Home", "Down", "Down", "Down"]);
    app.key("alt+KP_Add");
    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");
    app.await_exists("dst/other.txt");
    app.settle();

    app.focus_main();
    app.key("ctrl+u");
    await_panes_at(&app, "/dst", "/src");

    // The right pane shows src now. Bring the spent marks back there and
    // delete them: the two .txt files go, data.bin stays.
    app.key("Tab");
    app.key("KP_Divide");
    app.key("F8");
    app.focus_dialog(DIALOG_DELETE);
    app.key("space");

    app.await_gone("src/notes.txt");
    app.await_gone("src/other.txt");
    assert!(
        app.path("src/data.bin").exists(),
        "the exchange lost the spent selection, or widened it"
    );
}

/// Waits until the app has recorded that pane's ordering and hidden-file flag.
fn await_pane_sort(app: &App, index: usize, key: &str, hidden: bool) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        let recorded = recorded_pane(app.home(), index);
        if recorded == (key.to_string(), hidden) {
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "pane {index} never recorded {key}/hidden={hidden}: {recorded:?}"
        );
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
}

/// One pane's sort key and hidden-file flag, read back the way the app wrote
/// them.
fn recorded_pane(home: &Path, index: usize) -> (String, bool) {
    let config = VfsPath::new(home.join(".config").to_str().unwrap());
    let (settings, complaint) = fc_core::config::load(&LocalFs, &config);
    assert_eq!(complaint, None, "the settings could not be read back");
    let pane = settings.pane(index);
    (pane.sort_key.clone(), pane.show_hidden)
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
fn a_wildcard_can_take_a_mark_off_again() {
    // `Num −` is the only marking key with no end-to-end test, which read as
    // covered because its twin `Num +` has one and `Ctrl+Num −` has three.
    // Mark everything, then take one file back out of the selection: what
    // arrives is the difference.
    let app = in_src_and_dst(arrange);

    app.key("ctrl+a");
    app.key("KP_Subtract");
    app.focus_dialog(DIALOG_UNMARK_PATTERN);
    app.type_text("*.bin");
    app.key("Return");

    app.focus_main();
    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    // The rest of the selection still arrives — an unmark that took the whole
    // selection with it would pass a test that only checked the absence.
    app.await_exists("dst/notes.txt");
    app.await_exists("dst/nested/inner.txt");
    app.settle();
    assert!(
        !app.path("dst/data.bin").exists(),
        "the pattern did not take the mark off the file it named"
    );
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
fn the_title_names_the_build_it_is_running() {
    // "Which build were you running" is the first question a screenshot or a
    // bug report raises, and the title bar is the one part of the window that
    // survives into either. The values are stamped in by `build.rs`, so this
    // is also the only check that the build script ran at all.
    let app = App::launch(arrange);

    let title = app.title();
    let stamp = title
        .strip_prefix("FerroCommander #")
        .unwrap_or_else(|| panic!("the title does not name the product and a build: {title:?}"));
    let (number, hash) = stamp
        .split_once(" (")
        .unwrap_or_else(|| panic!("the title carries no commit hash: {title:?}"));
    let hash = hash
        .strip_suffix(')')
        .unwrap_or_else(|| panic!("the hash is not bracketed: {title:?}"));

    assert!(
        !number.is_empty() && number.chars().all(|digit| digit.is_ascii_digit()),
        "the build number is not a number: {number:?}"
    );
    assert!(
        !hash.is_empty() && hash.chars().all(|digit| digit.is_ascii_hexdigit()),
        "the commit hash is not a hash: {hash:?}"
    );
}

#[test]
fn ctrl_c_in_the_command_line_belongs_to_the_command_line() {
    // A regression this suite caught after the fact: while a text field has
    // the focus the shell used to dispatch any modified key the keymap
    // claimed, so binding Ctrl+C took it from the entry.
    //
    // Proved by its absence — if the pane had seen it, notes.txt would be on
    // the clipboard and the paste below would produce it.
    let app = in_src_and_dst(arrange);
    // `..`, nested, data.bin, notes.txt.
    app.keys(&["Home", "Down", "Down", "Down"]);

    app.key("Right");
    app.key("ctrl+c");
    app.key("Escape");

    app.key("Tab");
    app.key("ctrl+v");
    app.settle();

    assert!(
        !app.path("dst/notes.txt").exists(),
        "Ctrl+C in the command line reached the pane"
    );
}

#[test]
fn ctrl_x_in_the_command_line_does_not_move_a_file() {
    // The same key with more at stake: a hijacked Ctrl+X followed by a paste
    // moves somebody's file while they were typing a command.
    let app = in_src_and_dst(arrange);
    app.keys(&["Home", "Down", "Down", "Down"]);

    app.key("Right");
    app.key("ctrl+x");
    app.key("Escape");

    app.key("Tab");
    app.key("ctrl+v");
    app.settle();

    assert!(
        app.path("src/notes.txt").exists(),
        "Ctrl+X in the command line moved the file under the cursor"
    );
    assert!(!app.path("dst/notes.txt").exists());
}

#[test]
fn ctrl_v_in_the_command_line_pastes_text_rather_than_files() {
    // The one people meet first: pasting a path into a command used to start
    // a file copy. The clipboard is deliberately loaded with a *file* here,
    // so a hijacked Ctrl+V has something to copy.
    let app = in_src_and_dst(arrange);
    app.keys(&["Home", "Down", "Down", "Down"]);
    app.key("ctrl+c");

    // The other pane, so a hijacked paste would land somewhere observable.
    app.key("Tab");
    app.key("Right");
    app.key("ctrl+v");
    app.settle();

    assert!(
        !app.path("dst/notes.txt").exists(),
        "Ctrl+V in the command line pasted files into the pane"
    );
}

#[test]
fn ctrl_a_in_the_command_line_does_not_mark_the_pane() {
    // Not new, and not introduced by the clipboard work: Ctrl+A has meant
    // "mark everything" since long before, so select-all has never worked in
    // the command line. The same list fixes it.
    let app = in_src_and_dst(arrange);
    // Onto notes.txt, so a copy of the cursor row is one file.
    app.keys(&["Home", "Down", "Down", "Down"]);

    app.key("Right");
    app.key("ctrl+a");
    app.key("Escape");

    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");
    app.await_exists("dst/notes.txt");
    app.settle();

    assert!(
        !app.path("dst/data.bin").exists(),
        "Ctrl+A in the command line marked the whole pane"
    );
}

#[test]
fn ctrl_c_then_ctrl_v_copies_into_the_other_pane() {
    // The system clipboard, not a buffer of our own — which is why this works
    // between the panes *and* with Nautilus, for the same code.
    let app = in_src_and_dst(arrange);
    // `..`, nested, data.bin, notes.txt.
    app.keys(&["Home", "Down", "Down", "Down"]);
    app.key("ctrl+c");

    app.key("Tab");
    app.key("ctrl+v");

    app.await_exists("dst/notes.txt");
    app.await_contents("dst/notes.txt", SOURCE_TEXT);
    // A copy leaves the source where it was.
    assert!(app.path("src/notes.txt").exists(), "the original went");
}

#[test]
fn ctrl_x_then_ctrl_v_moves_rather_than_copies() {
    // The verb is the one thing `text/uri-list` cannot carry and the GNOME
    // format exists for. Losing it turns a move into a copy silently.
    let app = in_src_and_dst(arrange);
    app.keys(&["Home", "Down", "Down", "Down"]);
    app.key("ctrl+x");

    app.key("Tab");
    app.key("ctrl+v");

    app.await_exists("dst/notes.txt");
    app.await_contents("dst/notes.txt", SOURCE_TEXT);
    // The engine's move: it copies, checks, and only then removes the source.
    app.await_gone("src/notes.txt");
}

#[test]
fn the_clipboard_carries_everything_that_is_marked() {
    // The same rule F5 follows — everything marked, or the cursor row — so
    // the two keys never disagree about what an operation is for.
    let app = in_src_and_dst(arrange);
    app.keys(&["Home", "Down", "Down"]);
    app.key("Insert");
    app.key("Insert");
    app.key("ctrl+c");

    app.key("Tab");
    app.key("ctrl+v");

    app.await_exists("dst/data.bin");
    app.await_exists("dst/notes.txt");
}

#[test]
fn a_name_with_a_space_survives_the_clipboard() {
    // A file called `my notes.txt` is ordinary, and a URI that carried the
    // space raw would be one nothing can read back — including us.
    let app = in_src_and_dst(|home| {
        arrange(home);
        std::fs::write(home.join("src/my notes.txt"), SOURCE_TEXT).unwrap();
    });
    // `..`, nested, data.bin, my notes.txt, notes.txt.
    app.keys(&["Home", "Down", "Down", "Down"]);
    app.key("ctrl+c");

    app.key("Tab");
    app.key("ctrl+v");

    app.await_exists("dst/my notes.txt");
}

#[test]
fn a_cut_is_spent_by_being_pasted() {
    // Leaving it on the clipboard invites a second paste that can only fail,
    // over files the first one already moved. Every desktop file manager
    // clears it for the same reason.
    let app = in_src_and_dst(arrange);
    app.keys(&["Home", "Down", "Down", "Down"]);
    app.key("ctrl+x");

    app.key("Tab");
    app.key("ctrl+v");
    app.await_exists("dst/notes.txt");
    app.await_gone("src/notes.txt");

    // Into `nested`, and paste again: there is nothing left on the clipboard,
    // so nothing arrives and nothing is reported as failing either.
    app.key("Tab");
    app.keys(&["Home", "Down"]);
    app.key("Return");
    await_panes_at(&app, "/src/nested", "/dst");
    app.key("ctrl+v");
    app.settle();

    assert!(
        !app.path("src/nested/notes.txt").exists(),
        "a spent cut was pasted a second time"
    );
    assert!(
        !app.has_dialog(DIALOG_FAILURES),
        "the second paste failed loudly instead of doing nothing"
    );
}

#[test]
fn copying_inside_an_archive_is_refused_rather_than_pointing_at_the_disk() {
    // An entry in an archive has no operating-system path, so a URI naming
    // one would point at a file on the disk that merely shares its name.
    let app = in_src_and_dst(with_an_archive);
    app.keys(&["Home", "Down", "Down"]);
    app.key("Return");
    await_panes_at(&app, "/src/bundle.zip", "/dst");

    // `..`, deeper, packed.txt.
    app.keys(&["Home", "Down", "Down"]);
    app.key("ctrl+c");

    app.focus_dialog(DIALOG_OUTPUT);
    app.key("Return");
    app.settle();
    assert!(
        !app.path("dst/packed.txt").exists(),
        "something was put on the clipboard after all"
    );
}

#[test]
fn the_right_arrow_puts_the_keyboard_in_the_command_line() {
    // A pane has no horizontal movement to spend the key on, and the command
    // line needs a way in that does not cost the letter keys — which
    // type-ahead is about to take.
    //
    // Asserted through Space rather than by typing a command, and that is not
    // squeamishness: *today* a letter reaches the command line whether or not
    // this key works, so a test that typed one would pass with the binding
    // deleted. It was written that way first and the probe caught it.
    //
    // Space is the one key whose meaning differs at the only moment focus
    // differs — before anything has been typed. In the rows it marks; in the
    // entry it is a space. So: mark from the entry (which marks nothing),
    // move on, and copy. What lands in `dst` says where the keyboard was.
    let app = in_src_and_dst(arrange);
    // `..`, nested, data.bin, notes.txt.
    app.keys(&["Home", "Down", "Down"]);

    app.key("Right");
    app.key("space");
    app.key("Escape");

    // Onto notes.txt, which is what F5 takes when nothing is marked.
    app.key("Down");
    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    app.await_exists("dst/notes.txt");
    app.settle();
    assert!(
        !app.path("dst/data.bin").exists(),
        "Space marked a row, so the keyboard never left the pane"
    );
}

#[test]
fn typing_a_name_moves_the_cursor_to_it() {
    // **This test replaced one that said the opposite** (skill 24). Typing a
    // letter used to put it in the command line, because that was the only
    // way in from the keyboard; `→` is that way now, so the letters are free
    // for what a file manager wants them for.
    //
    // What the cursor is on is asserted by copying it: F5 takes the row under
    // the cursor when nothing is marked.
    let app = in_src_and_dst(arrange);
    // `..`, nested, data.bin, notes.txt — the cursor starts on `..`.
    app.type_text("not");

    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    app.await_exists("dst/notes.txt");
    app.settle();
    assert!(
        !app.path("dst/data.bin").exists(),
        "the cursor went somewhere else"
    );
}

#[test]
fn type_ahead_matches_the_extension_too() {
    // The whole name is what is matched, extension included — the same rule
    // the quick filter uses, and the same function.
    let app = in_src_and_dst(arrange);
    app.type_text(".bin");

    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    app.await_exists("dst/data.bin");
}

#[test]
fn the_same_letter_again_walks_to_the_next_match() {
    // `nested`, `data.bin` and `notes.txt` all hold an `n`. A second `n` is
    // "show me the next one" rather than a longer needle, which would match
    // nothing and sit still exactly when somebody is pressing it to move on.
    let app = in_src_and_dst(arrange);

    // First `n` finds nested; second walks past it to data.bin.
    app.type_text("nn");

    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    app.await_exists("dst/data.bin");
}

#[test]
fn a_search_that_matches_nothing_leaves_the_cursor_alone() {
    // One mistyped letter must not throw away what came before it, and must
    // not move the cursor somewhere arbitrary either.
    let app = in_src_and_dst(arrange);
    app.type_text("not");
    // `notq` matches nothing; the cursor stays on notes.txt.
    app.type_text("q");

    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    app.await_exists("dst/notes.txt");
}

#[test]
fn a_command_runs_in_the_pane_that_has_the_keyboard() {
    // Tab moves the command line with it. A line that kept running in the
    // pane you left would be the worst kind of wrong: plausible until it
    // deletes something.
    let app = in_src_and_dst(arrange);
    app.key("Tab");

    // The command line is reached with the arrow now that letters search.
    app.key("Right");
    app.type_text("touch on-the-right");
    app.key("Return");

    app.await_exists("dst/on-the-right");
    app.settle();
    assert!(
        !app.path("src/on-the-right").exists(),
        "the command ran in the pane that no longer had the keyboard"
    );
}

#[test]
fn a_command_that_says_something_opens_a_window_saying_it() {
    let app = in_src_and_dst(arrange);

    // The command line is reached with the arrow now that letters search.
    app.key("Right");
    app.type_text("echo hello from the shell");
    app.key("Return");

    app.focus_dialog(DIALOG_OUTPUT);
    app.key("Return");
    app.await_dialog_closed(DIALOG_OUTPUT);
}

#[test]
fn a_silent_command_opens_no_window_at_all() {
    // Otherwise every `touch` costs a dialog to dismiss, and a command line
    // that interrupts after every command is one nobody uses twice.
    let app = in_src_and_dst(arrange);

    // The command line is reached with the arrow now that letters search.
    app.key("Right");
    app.type_text("touch quietly");
    app.key("Return");
    app.await_exists("src/quietly");

    app.settle();
    assert!(
        !app.has_dialog(DIALOG_OUTPUT),
        "a command that said nothing still opened a window"
    );
}

#[test]
fn a_failing_command_says_so_even_when_it_printed_nothing() {
    // "It did nothing and said nothing" must not be indistinguishable from
    // "it worked".
    let app = in_src_and_dst(arrange);

    // The command line is reached with the arrow now that letters search.
    app.key("Right");
    app.type_text("false");
    app.key("Return");

    app.focus_dialog(DIALOG_OUTPUT);
}

#[test]
fn cd_moves_the_pane_instead_of_being_run() {
    // A `cd` in a child process changes nothing anybody can see, so a command
    // line that spawned one would look broken.
    let app = in_src_and_dst(arrange);

    // The command line is reached with the arrow now that letters search.
    app.key("Right");
    app.type_text("cd nested");
    app.key("Return");
    app.settle();

    // The pane is in src/nested now, so F7 lands there.
    app.key("F7");
    app.focus_dialog(DIALOG_NEW_DIR);
    app.type_text("proof");
    app.key("Return");

    app.await_exists("src/nested/proof");
}

#[test]
fn escape_empties_the_command_line_and_hands_back_the_keyboard() {
    // A field with no way out but the mouse is a trap in a keyboard-first
    // program, and this one has no Cancel button.
    let app = in_src_and_dst(arrange);
    // The command line is reached with the arrow now that letters search.
    app.key("Right");
    app.type_text("touch never-run");
    app.key("Escape");

    // The keyboard is back on the rows: F7 opens its dialog rather than
    // typing into the line.
    app.key("F7");
    app.focus_dialog(DIALOG_NEW_DIR);
    app.type_text("after-escape");
    app.key("Return");
    app.await_exists("src/after-escape");

    app.settle();
    assert!(
        !app.path("src/never-run").exists(),
        "Escape left the command behind to be run later"
    );
}

#[test]
fn ctrl_down_offers_a_command_that_was_run_before() {
    // Total Commander's Ctrl+Down. Picking puts the line in the entry rather
    // than running it, so it can be edited first — which is most of why
    // anybody opens a history — so this edits it before pressing Enter.
    let app = in_src_and_dst(arrange);
    // The command line is reached with the arrow now that letters search.
    app.key("Right");
    app.type_text("touch first");
    app.key("Return");
    app.await_exists("src/first");

    app.focus_main();
    app.key("ctrl+Down");
    app.focus_dialog(DIALOG_HISTORY);
    app.key("Return");
    app.await_dialog_closed(DIALOG_HISTORY);

    // The line came back and the cursor is at its end, so typing extends it.
    app.type_text("-again");
    app.key("Return");

    app.await_exists("src/first-again");
}

#[test]
fn a_job_sent_to_the_background_finishes_without_its_window() {
    // Reported from the field: a copy could not be put in the background, so
    // the whole program waited on it. Background closes the window and
    // nothing else — the token is not pulled, the events go on being drained,
    // and the copy runs to its end.
    let app = in_src_and_dst(with_a_collision_then_more);

    app.key("ctrl+a");
    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    // The job stops here until it is answered, which is what makes the rest
    // of this deterministic: by the time the answer goes in, more than
    // PROGRESS_DELAY has passed on the app's own clock, so the next progress
    // event opens the window whatever the disk did.
    app.focus_dialog(DIALOG_CONFLICT);
    app.settle();
    app.key("Return");

    app.focus_dialog(TITLE_PROGRESS);
    // Background is the focused button, so Enter is it — which is also the
    // point: Enter must never be the one that stops a copy halfway.
    app.key("Return");
    app.await_dialog_closed(TITLE_PROGRESS);

    // The window is gone and the copy is not: this is the whole claim. The
    // last file to be copied is the one asserted on, so a job that stopped
    // when its window did would not have reached it.
    app.await_exists(&format!("dst/zzz-{:03}.txt", TRAILING_FILES - 1));
    app.await_exists("dst/notes.txt");
}

#[test]
fn a_dragged_column_width_reaches_the_settings_file() {
    // What this can see is the file: whether the *other* pane followed is not
    // readable from outside — a column width is not a window title or a file
    // on disk — so the mirroring is checked by eye and by the shared field
    // both panes are set from, not here.
    //
    // The panes are meant to line up with each other, which is why the widths
    // were constants before they became settings.
    let app = in_src_and_dst(arrange);

    // The divider at the right edge of Ext, in the left pane's header: the
    // name column is 260 wide and Ext 70, and the header is the band under
    // the path bar.
    app.drag(
        (EXT_DIVIDER_X, EXT_DIVIDER_Y),
        (EXT_DIVIDER_DRAGGED_X, EXT_DIVIDER_Y),
    );

    let widths = await_column_widths(&app);
    assert!(
        widths.ext > DEFAULT_EXT_WIDTH,
        "the drag did not reach the settings: {widths:?}"
    );
}

/// The column widths the settings file currently records.
#[derive(Debug, Default, PartialEq, Eq)]
struct ColumnWidths {
    ext: i32,
}

/// Polls the settings file until it holds a column table, and reads it.
///
/// Block extraction rather than a grep for `ext =`: the file has other tables
/// with short keys in them, and a test that matched the wrong line would pass
/// for the wrong reason (skill 58).
fn await_column_widths(app: &App) -> ColumnWidths {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        let written = settings_text(app);
        let table = written
            .split("[columns]")
            .nth(1)
            .map(|rest| rest.split("\n[").next().unwrap_or_default().to_string())
            .unwrap_or_default();
        let ext = table
            .lines()
            .find_map(|line| line.trim().strip_prefix("ext = ")?.parse::<i32>().ok())
            .unwrap_or_default();
        if ext > DEFAULT_EXT_WIDTH {
            return ColumnWidths { ext };
        }
        assert!(
            std::time::Instant::now() < deadline,
            "the settings never recorded a wider Ext column: {table}"
        );
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
}

#[test]
fn the_cursor_follows_a_renamed_file_to_its_new_name() {
    // Reported from the field: after Shift+F6 and Enter the cursor jumped to
    // the top row, leaving the file just named off screen under a name the
    // user then had to go and find.
    //
    // Asserted by acting on the cursor rather than by looking at it: F5 copies
    // the row under it, so a copy of the new name is proof of where it is.
    let app = in_src_and_dst(arrange);
    // `..`, nested, data.bin, notes.txt.
    app.keys(&["Home", "Down", "Down", "Down"]);
    app.key("shift+F6");
    app.settle();
    app.key("ctrl+a");
    app.type_text("zzz-last.txt");
    app.key("Return");
    app.await_exists("src/zzz-last.txt");

    // It sorts last, so a cursor that fell back to the top is nowhere near it.
    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");
    app.await_exists("dst/zzz-last.txt");
}

#[test]
fn enter_on_a_file_hands_it_to_the_desktop() {
    // Reported from the field: Enter on a file did nothing at all, which is
    // what `keymap.md` said it did. It now opens the file the way the desktop
    // would — never by executing it, whatever its permission bits say.
    // `..`, nested, data.bin, notes.txt — directories first.
    let app = in_src_and_dst(with_recording_opener);
    app.keys(&["Home", "Down", "Down", "Down"]);
    app.key("Return");

    app.await_contents(
        "opened-by-handler.log",
        &format!("{}/src/notes.txt", app.home().display()),
    );
}

#[test]
fn enter_on_a_directory_still_enters_it() {
    // The half that already worked, kept honest: teaching Enter to open files
    // must not cost it the thing it was for.
    let app = in_src_and_dst(with_recording_opener);
    app.keys(&["Home", "Down"]);
    app.key("Return");
    await_panes_at(&app, "/src/nested", "/dst");
    assert!(
        !app.path("opened-by-handler.log").exists(),
        "entering a directory handed something to the desktop"
    );
}

#[test]
fn enter_inside_an_archive_opens_nothing_on_the_disk() {
    // The same reason F4 is refused there: a handler takes an
    // operating-system path and an entry in an archive has none.
    let app = in_src_and_dst(with_an_archive);
    app.keys(&["Home", "Down", "Down"]);
    app.key("Return");
    await_panes_at(&app, "/src/bundle.zip", "/dst");

    // `..`, deeper, packed.txt.
    app.keys(&["Home", "Down", "Down"]);
    app.key("Return");

    app.focus_dialog(DIALOG_OUTPUT);
    app.key("Return");
    app.settle();
    assert!(
        !app.path("packed.txt").exists(),
        "a file was made on the disk"
    );
}

#[test]
fn the_stylesheet_is_one_gtk_accepts_whole() {
    // A selector GTK cannot parse costs nothing at startup: the rule is
    // dropped, the app runs, and the effect is missing somewhere nobody is
    // looking. That is how the inactive pane's outline cursor would fail —
    // silently — so the parse is asserted rather than assumed.
    let app = in_src_and_dst(arrange);
    let log = app.log();
    assert!(
        !log.contains(STYLESHEET_REJECTED),
        "GTK rejected part of the stylesheet: {log}"
    );
}

#[test]
fn a_cd_is_remembered_the_way_a_command_is() {
    // Reported from the field: `cd ..` moved the pane and then could not be
    // got back with Ctrl+Down. The two arms of `read` sat twelve lines apart
    // and only one of them called `remember_command`.
    //
    // Asserted on the settings file rather than by reopening the dialog: the
    // file is where the history lives between runs, so a `cd` that reaches it
    // is a `cd` the next run can offer too.
    let app = in_src_and_dst(arrange);
    // The command line is reached with the arrow now that letters search.
    app.key("Right");
    app.type_text("cd ..");
    app.key("Return");
    // The pane moved, which is the half that always worked.
    await_panes_at(&app, "", "/dst");

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        let history = recorded_command_history(&app);
        if history.iter().any(|line| line == "cd ..") {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "the history never recorded the cd: {history:?}"
        );
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
}

/// The command lines the settings file has kept, newest first.
///
/// Block extraction rather than a grep for the text: `cd ..` would match a
/// pane directory or a favourite just as happily, and a test that passes on
/// the wrong line is worse than one that fails (skill 58).
fn recorded_command_history(app: &App) -> Vec<String> {
    let written = settings_text(app);
    let Some(start) = written.find("command_history = [") else {
        return Vec::new();
    };
    let rest = &written[start..];
    let end = rest.find(']').unwrap_or(rest.len());
    // The array is written inline — `command_history = ["cd ..", "touch a"]`
    // — so the entries are the quoted runs inside it, not one per line.
    rest[..end]
        .split('"')
        .skip(1)
        .step_by(2)
        .map(|entry| entry.to_string())
        .collect()
}

#[test]
fn alt_f8_opens_the_same_history() {
    // TC's other way to the same list, and both are muscle memory.
    let app = in_src_and_dst(arrange);
    // The command line is reached with the arrow now that letters search.
    app.key("Right");
    app.type_text("touch remembered");
    app.key("Return");
    app.await_exists("src/remembered");

    app.focus_main();
    app.key("alt+F8");
    app.focus_dialog(DIALOG_HISTORY);
}

#[test]
fn an_empty_history_opens_no_window() {
    // Nothing run yet is not worth a window listing nothing.
    let app = in_src_and_dst(arrange);

    app.key("ctrl+Down");

    app.settle();
    assert!(
        !app.has_dialog(DIALOG_HISTORY),
        "an empty history still opened a window"
    );
}

#[test]
fn the_history_survives_a_restart() {
    // A history that forgot everything when the app closed would be one in
    // name only.
    let first = in_src_and_dst(arrange);
    // The command line is reached with the arrow now that letters search.
    first.key("Right");
    first.type_text("touch before-restart");
    first.key("Return");
    first.await_exists("src/before-restart");
    let home = first.close();

    let app = App::relaunch(home);
    app.key("ctrl+Down");
    app.focus_dialog(DIALOG_HISTORY);
    app.key("Return");
    app.await_dialog_closed(DIALOG_HISTORY);
    // No arrow needed here: picking from the history puts the line back and
    // the keyboard with it.
    app.type_text("-two");
    app.key("Return");

    app.await_exists("src/before-restart-two");
}

#[test]
fn ctrl_enter_puts_the_name_under_the_cursor_into_the_command_line() {
    // The one shortcut that makes a command line in a file manager worth
    // having: act on the file you are looking at without typing its name.
    let app = in_src_and_dst(arrange);
    // `..`, nested, data.bin, notes.txt — cursor on notes.txt.
    app.keys(&["Home", "Down", "Down", "Down"]);

    // The command line is reached with the arrow now that letters search.
    app.key("Right");
    app.type_text("cp");
    app.key("ctrl+Return");
    app.type_text(" copied.txt");
    app.key("Return");

    app.await_exists("src/copied.txt");
    app.await_contents("src/copied.txt", SOURCE_TEXT);
}

#[test]
fn the_inserted_name_comes_from_the_pane_that_has_the_keyboard() {
    // Ctrl+Enter follows the active pane, and typing into the command line
    // does not change which one that is: the focus moves into the entry, but
    // the pane the keyboard came from is still the one being looked at. A
    // name taken from the other side would be plausible right up until it
    // named a file that exists on both.
    let app = in_src_and_dst(arrange_with_collision);
    app.key("Tab");
    // dst holds only notes.txt, so `..` then it.
    app.keys(&["Home", "Down"]);

    // The command line is reached with the arrow now that letters search.
    app.key("Right");
    app.type_text("cat");
    app.key("ctrl+Return");
    app.type_text(" > out.txt");
    app.key("Return");

    // dst/notes.txt holds EXISTING_TEXT and src/notes.txt holds SOURCE_TEXT,
    // and the left pane's cursor is on `..` — so what landed in out.txt says
    // which pane the name came from. The redirection creates the file either
    // way, which is why the contents are the assertion and not the file.
    app.await_contents("dst/out.txt", EXISTING_TEXT);
}

#[test]
fn an_inserted_name_is_a_separate_word() {
    // The difference between `lsnotes.txt` and `ls notes.txt`. Having to
    // reach for the space bar first would make the shortcut not worth using,
    // and a missing space turns a command into a typo that runs.
    let app = in_src_and_dst(arrange);
    app.keys(&["Home", "Down", "Down", "Down"]);

    // The command line is reached with the arrow now that letters search.
    app.key("Right");
    app.type_text("cat");
    app.key("ctrl+Return");
    app.type_text(" > out.txt");
    app.key("Return");

    // `cat notes.txt > out.txt` copies the file. Glued together it is not a
    // command at all — and the redirection still creates `out.txt`, empty, so
    // only the *contents* tell the two apart. Asserting that a window opened
    // would not: a failing command opens one too.
    app.await_contents("src/out.txt", SOURCE_TEXT);
}

#[test]
fn the_history_is_reachable_while_a_command_is_being_typed() {
    // The moment it is actually wanted. Typing moves the focus into the
    // entry, and the shell otherwise keeps its hands off a text field — so
    // without an exception for modified keys, Ctrl+Down would be unreachable
    // at the only time anybody reaches for it.
    let app = in_src_and_dst(arrange);
    // The command line is reached with the arrow now that letters search.
    app.key("Right");
    app.type_text("touch from-history");
    app.key("Return");
    app.await_exists("src/from-history");

    // Start typing something else, then go looking for the old line.
    app.type_text("touch abandoned");
    app.key("ctrl+Down");
    app.focus_dialog(DIALOG_HISTORY);
    app.key("Return");
    app.await_dialog_closed(DIALOG_HISTORY);
    app.type_text("-2");
    app.key("Return");

    app.await_exists("src/from-history-2");
    app.settle();
    assert!(
        !app.path("src/abandoned").exists(),
        "the half-typed line was run as well"
    );
}

#[test]
fn a_shortcut_this_program_does_not_have_types_nothing() {
    // Ctrl+J and Alt+J still report the letter J. Somebody reaching for a
    // shortcut meant a shortcut, and silently typing `j` into the command
    // line would turn a missing feature into a wrong answer — then Enter
    // would run it.
    let app = in_src_and_dst(arrange);

    app.keys(&["ctrl+j", "alt+j"]);
    app.key("Return");

    app.settle();
    assert!(
        !app.has_dialog(DIALOG_OUTPUT),
        "an unbound shortcut typed its letter and Enter ran it"
    );
}

#[test]
fn the_quick_filter_still_gets_its_own_letters() {
    // The risk in letting unbound letters type: the filter field is a text
    // widget the shell must keep its hands off, and breaking it would be a
    // silent regression in a feature nothing else covers.
    let app = in_src_and_dst(arrange);

    app.key("ctrl+s");
    app.type_text("notes");
    app.key("Return");

    // Only notes.txt is visible now, so Home+Down lands on it and F5 copies
    // that one file.
    app.keys(&["Home", "Down"]);
    app.key("F5");
    app.focus_dialog(DIALOG_COPY);
    app.key("Return");

    app.await_exists("dst/notes.txt");
    app.settle();
    assert!(
        !app.path("dst/data.bin").exists(),
        "the filter did not narrow the pane"
    );
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

#[test]
fn a_key_pressed_before_a_listing_lands_still_means_the_new_directory() {
    // Reading a directory happens on a worker thread now, and keys arrive
    // faster than listings: Enter and F7 in quick succession are both
    // dispatched before the first read comes back. A pane asked where it *is*
    // would answer with the directory it is leaving, and F7 would make the
    // directory in the wrong place — which is what happened until panes
    // started answering with where they are going.
    //
    // No pause between the two keys on purpose: the harness sends them 50 ms
    // apart, and the app processes both before the read lands.
    let app = App::launch(arrange);
    app.keys(&["Home", "Down", "Down", "Return", "F7"]);

    app.focus_dialog(DIALOG_NEW_DIR);
    app.type_text("in-the-new-place");
    app.key("Return");

    app.await_exists("src/in-the-new-place");
    assert!(
        !app.home().join("in-the-new-place").exists(),
        "it was made in the directory the pane was leaving"
    );
}

/// `arrange`, plus a namesake `notes.txt` in dst that differs in its text —
/// the pair the compare cascade's middle arm finds.
fn with_a_namesake(home: &Path) {
    arrange(home);
    std::fs::write(home.join("dst/notes.txt"), "hello from the other side").unwrap();
}

/// A home whose settings name a compare tool that records what it was given.
///
/// A real diff tool would open a window this suite cannot drive; a script
/// that writes both arguments proves the tool ran, that the cascade picked
/// the right pair, and that the `%1`/`%2` substitution put each path where
/// its placeholder stood.
fn with_recording_compare_tool(home: &Path) {
    with_a_namesake(home);

    let script = home.join("record-compare.sh");
    std::fs::write(
        &script,
        "#!/bin/sh\nprintf '%s|%s' \"$1\" \"$2\" > \"$(dirname \"$0\")/compared.log\"\n",
    )
    .unwrap();
    std::fs::set_permissions(&script, std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();

    let settings = home.join(SETTINGS_FILE);
    std::fs::create_dir_all(settings.parent().unwrap()).unwrap();
    std::fs::write(
        settings,
        format!("compare_tool = \"{} %1 %2\"\n", script.display()),
    )
    .unwrap();
}

#[test]
fn ctrl_shift_c_compares_the_cursor_file_with_its_namesake() {
    // The window is titled with both names, which is how the test finds it
    // and how a person with several compares open tells them apart.
    let app = in_src_and_dst(with_a_namesake);
    // `..`, nested, data.bin, notes.txt.
    app.keys(&["Home", "Down", "Down", "Down"]);

    app.key("ctrl+shift+c");

    app.focus_dialog("notes.txt ↔ notes.txt");
    app.key("Escape");
    app.await_dialog_closed("notes.txt ↔ notes.txt");
    // And the keyboard is back on the rows.
    app.key("F7");
    app.focus_dialog(DIALOG_NEW_DIR);
}

#[test]
fn two_marked_files_outrank_the_namesake_as_the_pair() {
    let app = in_src_and_dst(with_a_namesake);
    // Mark data.bin and notes.txt; the cursor's namesake in dst must lose
    // to the marked pair, so the title names two files from *this* pane.
    app.keys(&["Home", "Down", "Down"]);
    app.key("Insert");
    app.key("Insert");

    app.key("ctrl+shift+c");

    app.focus_dialog("data.bin ↔ notes.txt");
    app.key("Escape");
    app.await_dialog_closed("data.bin ↔ notes.txt");
}

#[test]
fn a_compare_with_nothing_to_compare_says_so() {
    // The cursor on a directory and nothing marked names no pair. Unlike F3
    // on a directory, silence here would read as breakage — the gesture
    // asked about two things — so one line says what was missing.
    let app = in_src_and_dst(arrange);
    // `..`, then nested.
    app.keys(&["Home", "Down"]);

    app.key("ctrl+shift+c");

    app.focus_dialog("Compare");
    app.key("Escape");
    app.await_dialog_closed("Compare");
}

#[test]
fn a_configured_compare_tool_gets_both_paths_where_its_placeholders_stood() {
    // The whole external contract in one press: the cascade picks the
    // cursor file and its namesake, the settings line is read, and %1/%2
    // become the two paths in order.
    let app = in_src_and_dst(with_recording_compare_tool);
    app.keys(&["Home", "Down", "Down", "Down"]);

    app.key("ctrl+shift+c");

    app.await_contents(
        "compared.log",
        &format!(
            "{home}/src/notes.txt|{home}/dst/notes.txt",
            home = app.home().display()
        ),
    );
}
