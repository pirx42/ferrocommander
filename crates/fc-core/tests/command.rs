//! Running a typed command line.
//!
//! The claim that matters and cannot be seen from outside is "it ran in the
//! directory the pane was showing" — everything else the user can check by
//! looking at the output.
//!
//! **This file runs on Windows too**, from the `windows` CI job, and it is
//! the only test target that does. Three rounds of Windows fixes had shipped
//! by 2026-09-01 with nothing but a smoke-start behind them, and the third
//! one was broken in three separate ways on arrival. What each test *claims*
//! is the same on both platforms; only the wording of the line differs, and
//! that lives in [`line`] so a test reads as one claim rather than two.

use tempfile::TempDir;

use fc_core::command::{
    editor_line, quoted, run, Outcome, Quoting, OUTPUT_LIMIT, QUOTING, TRUNCATION_NOTICE,
};
use fc_core::vfs::VfsPath;

/// The same intent in each interpreter's own words.
///
/// `cmd` is not a poorer shell, it is a different one: `;` does not separate
/// commands (`&` does), there is no `true` or `false`, and a wildcard is
/// expanded by the *program* rather than by the interpreter. Writing the
/// difference down here is what lets the claims below be shared.
#[cfg(not(windows))]
mod line {
    pub const LIST: &str = "ls";
    pub const WRITE_A_FILE: &str = "echo written > proof.txt";
    pub const BOTH_STREAMS: &str = "echo out; echo err >&2";
    pub const FAIL_LOUDLY: &str = "echo why >&2; exit 3";
    pub const SUCCEED_SILENTLY: &str = "true";
    pub const FAIL_SILENTLY: &str = "false";
    pub const PIPE_THE_TXT_FILES: &str = "ls *.txt | sort";
    pub const PRINT_A_FILE: &str = "cat";
}

#[cfg(windows)]
mod line {
    pub const LIST: &str = "dir /b";
    /// No space before the `>`: `cmd` would echo it as part of the text.
    pub const WRITE_A_FILE: &str = "echo written>proof.txt";
    pub const BOTH_STREAMS: &str = "echo out& echo err>&2";
    pub const FAIL_LOUDLY: &str = "echo why>&2& exit 3";
    pub const SUCCEED_SILENTLY: &str = "exit 0";
    pub const FAIL_SILENTLY: &str = "exit 1";
    pub const PIPE_THE_TXT_FILES: &str = "dir /b *.txt | sort";
    pub const PRINT_A_FILE: &str = "type";
}

/// A temporary directory holding the named files.
///
/// Local rather than the shared `common::fixture`, which builds one particular
/// tree for the transfer tests; what these need is a directory whose contents
/// they chose.
fn fixture(files: &[(&str, &str)]) -> TempDir {
    let dir = TempDir::new().unwrap();
    for (name, contents) in files {
        std::fs::write(dir.path().join(name), contents).unwrap();
    }
    dir
}

/// A `VfsPath` for a real temporary directory.
///
/// Through `from_std_path` rather than `VfsPath::new`: a temporary directory
/// is a *native* path, and on Windows that is `C:\Users\…` — which
/// `VfsPath::new` would keep verbatim, backslashes and all, and every test
/// here would then run in a directory that does not exist.
fn path_of(dir: &TempDir) -> VfsPath {
    fc_core::vfs::from_std_path(dir.path())
}

#[test]
fn a_command_runs_in_the_directory_it_was_given() {
    // The claim no amount of looking at the screen can check, and the one a
    // command line is useless without: `rm *.txt` has to mean the pane's
    // files and not whatever the app's own working directory happens to be.
    let dir = fixture(&[("marker.txt", "hello")]);

    let outcome = run(&path_of(&dir), line::LIST);

    assert!(outcome.succeeded, "{}", outcome.output);
    assert_eq!(outcome.output, "marker.txt");
}

#[test]
fn a_command_can_change_the_directory_it_was_given() {
    // Not a test of `ls` but of the working directory being real: a command
    // that writes lands the file where the pane is looking.
    let dir = fixture(&[]);

    let outcome = run(&path_of(&dir), line::WRITE_A_FILE);

    assert!(outcome.succeeded, "{}", outcome.output);
    // Trimmed, because the two interpreters end a line differently and what
    // is being tested is where the file landed, not how it ends.
    assert_eq!(
        std::fs::read_to_string(dir.path().join("proof.txt"))
            .unwrap()
            .trim(),
        "written"
    );
}

#[test]
fn both_streams_come_back_together() {
    // As a terminal shows them, and as the user thinks about them: an error
    // between two lines of output belongs between them.
    let dir = fixture(&[]);

    let outcome = run(&path_of(&dir), line::BOTH_STREAMS);

    assert!(outcome.output.contains("out"), "{}", outcome.output);
    assert!(outcome.output.contains("err"), "{}", outcome.output);
}

#[test]
fn a_failing_command_is_reported_as_one_and_keeps_what_it_said() {
    let dir = fixture(&[]);

    let outcome = run(&path_of(&dir), line::FAIL_LOUDLY);

    assert!(!outcome.succeeded);
    assert_eq!(outcome.output, "why");
    assert!(outcome.worth_showing());
}

#[test]
fn a_silent_success_is_not_worth_a_window() {
    // Otherwise every `touch` costs a dialog to dismiss.
    let dir = fixture(&[]);

    let outcome = run(&path_of(&dir), line::SUCCEED_SILENTLY);

    assert_eq!(
        outcome,
        Outcome {
            succeeded: true,
            output: String::new()
        }
    );
    assert!(!outcome.worth_showing());
}

#[test]
fn a_silent_failure_is_still_worth_a_window() {
    // "It did nothing and said nothing" must not be indistinguishable from
    // "it worked".
    let dir = fixture(&[]);

    let outcome = run(&path_of(&dir), line::FAIL_SILENTLY);

    assert!(!outcome.succeeded);
    assert!(outcome.output.is_empty());
    assert!(outcome.worth_showing());
}

#[test]
fn the_interpreter_is_one_so_a_pipe_and_a_wildcard_work() {
    // The reason the line goes through an interpreter rather than being split
    // into words here: a pipe and a pattern are most of what anybody types
    // into a file manager's command line, and neither means anything to
    // `Command::new`.
    //
    // The assertion changed on 2026-09-01, from `ls *.txt | wc -l` and a
    // count of "2", so that it can be made on both platforms (skill 24). The
    // pipe is genuinely shared; the wildcard is not, because `cmd` does no
    // globbing at all — `dir` expands its own pattern. Naming the files that
    // must and must not come back tests the same thing the count did, and
    // says which file was wrong when it fails.
    let dir = fixture(&[("a.txt", "1"), ("b.txt", "2"), ("c.bin", "3")]);

    let outcome = run(&path_of(&dir), line::PIPE_THE_TXT_FILES);

    assert!(outcome.succeeded, "{}", outcome.output);
    assert!(outcome.output.contains("a.txt"), "{}", outcome.output);
    assert!(outcome.output.contains("b.txt"), "{}", outcome.output);
    assert!(!outcome.output.contains("c.bin"), "{}", outcome.output);
}

#[test]
fn enormous_output_is_cut_and_says_so() {
    // A command may print gigabytes, and keeping all of it to fill a window
    // nobody reads to the end is a way to be killed by the allocator. The cut
    // has to be visible, or a truncated tail reads as the end of the output.
    //
    // Printed from a file rather than generated by the interpreter
    // (`yes … | head -c`, until 2026-09-01): `cmd` has no `yes`, and a loop
    // that echoes two million bytes takes minutes there. The claim is about
    // what `run` does with a lot of output, and where the output came from
    // was never part of it (skill 24).
    let much = "ferrocommander\n".repeat(2_000_000 / "ferrocommander\n".len());
    assert!(much.len() > OUTPUT_LIMIT, "the fixture is too small to cut");
    let dir = fixture(&[("much.txt", &much)]);

    let outcome = run(&path_of(&dir), &format!("{} much.txt", line::PRINT_A_FILE));

    assert!(outcome.output.ends_with(TRUNCATION_NOTICE), "not marked");
    assert_eq!(
        outcome.output.len(),
        OUTPUT_LIMIT + TRUNCATION_NOTICE.len(),
        "cut in the wrong place"
    );
}

#[test]
fn output_that_is_not_utf8_does_not_lose_the_rest() {
    // A command printing raw bytes must not cost the lines around them.
    //
    // From a file for the reason above, and one of its own: `printf` and its
    // `\xff` escapes are the shell's, so the old line tested the fixture's
    // interpreter as much as the code. Bytes written by the test are the
    // same bytes on both platforms.
    let dir = fixture(&[]);
    std::fs::write(dir.path().join("raw.bin"), b"before\n\xff\xfe\nafter").unwrap();

    let outcome = run(&path_of(&dir), &format!("{} raw.bin", line::PRINT_A_FILE));

    assert!(outcome.output.contains("before"), "{}", outcome.output);
    assert!(outcome.output.contains("after"), "{}", outcome.output);
}

#[test]
fn a_command_that_does_not_exist_reports_rather_than_vanishing() {
    let dir = fixture(&[]);

    let outcome = run(&path_of(&dir), "no_such_program_anywhere");

    assert!(!outcome.succeeded);
    assert!(outcome.worth_showing());
    assert!(!outcome.output.is_empty(), "said nothing about why");
}

#[test]
fn a_path_is_quoted_the_way_the_interpreter_that_reads_it_expects() {
    // Both halves from whichever platform is running, which is the point of
    // `Quoting` being an argument: the Windows branch is checked by the
    // Linux gate, and was not for the release in which it was wrong.
    assert_eq!(quoted("/a/left file", Quoting::Posix), "'/a/left file'");
    assert_eq!(
        quoted(r"C:\dir\a file", Quoting::Cmd),
        "\"C:\\dir\\a file\""
    );
}

#[test]
fn a_quote_in_a_name_survives_the_interpreter_that_allows_one() {
    // Only POSIX has to answer this: `"` is not a legal character in a
    // Windows filename, so `cmd` needs no escape and has none.
    assert_eq!(quoted("it's here", Quoting::Posix), r#"'it'\''s here'"#);
}

#[test]
fn this_platform_quotes_for_its_own_interpreter() {
    // The one assertion in this file that is *about* the platform rather
    // than shared by both, and the reason the Windows job runs these tests:
    // a build that picks POSIX quoting on Windows is exactly the failure
    // that shipped on 2026-09-01.
    #[cfg(windows)]
    assert_eq!(QUOTING, Quoting::Cmd);
    #[cfg(not(windows))]
    assert_eq!(QUOTING, Quoting::Posix);
}

#[test]
fn the_line_that_opens_a_file_holds_a_native_path_in_the_right_quotes() {
    // The bug, as a string. `Enter` on `C:\Users\pirx\a.exe` composed
    //
    //     start "" '/C:/Users/pirx/a.exe'
    //
    // — the VFS path rather than the native one, in quotes `cmd` does not
    // read — and Windows put up a dialog saying it could not find that.
    //
    // The path goes in the way the VFS holds it on **both** platforms,
    // because that is what a pane hands over; what comes out is each
    // platform's own answer, and only the Windows job can check the Windows
    // one. That is the whole reason this file runs there.
    let line = editor_line("notepad", &VfsPath::new("/C:/Users/pirx/a file.txt"))
        .expect("a named program is a command line");

    #[cfg(windows)]
    assert_eq!(line, "notepad \"C:\\Users\\pirx\\a file.txt\"");
    #[cfg(not(windows))]
    assert_eq!(line, "notepad '/C:/Users/pirx/a file.txt'");
}

#[test]
fn the_desktop_handler_is_no_command_line_at_all() {
    // What `Enter` uses, and what an unconfigured `F4` amounts to. Empty
    // rather than a program name, because the three names it used to hold
    // were only ever there to be pasted into a shell line — and composing
    // that line is what broke.
    assert!(fc_core::command::DESKTOP_HANDLER.is_empty());
    assert_eq!(
        editor_line(
            fc_core::command::DESKTOP_HANDLER,
            &VfsPath::new("/dir/holiday.png")
        ),
        None,
        "the desktop is handed a path, never a line to take apart"
    );
    assert_eq!(
        fc_core::config::DEFAULT_EDITOR,
        fc_core::command::DESKTOP_HANDLER,
        "`Enter` and an unconfigured `F4` are one gesture and must stay one"
    );
}

#[test]
fn the_two_openers_are_program_names_and_not_command_lines() {
    // Named here rather than only the one this build runs, so the gate that
    // does run checks the one it never calls. A space in either would mean
    // arguments, which `open_in_desktop` does not build a line for — it
    // spawns the name and hands over the path as a single argument.
    for opener in [
        fc_core::command::MACOS_OPENER,
        fc_core::command::FREEDESKTOP_OPENER,
    ] {
        assert!(!opener.is_empty());
        assert!(!opener.contains(' '), "{opener} reads as a command line");
    }
}
