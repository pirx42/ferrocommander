//! Running a typed command line.
//!
//! The claim that matters and cannot be seen from outside is "it ran in the
//! directory the pane was showing" — everything else the user can check by
//! looking at the output.

use tempfile::TempDir;

use fc_core::command::{run, Outcome, OUTPUT_LIMIT, TRUNCATION_NOTICE};
use fc_core::vfs::VfsPath;

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
fn path_of(dir: &TempDir) -> VfsPath {
    VfsPath::new(dir.path().to_str().unwrap())
}

#[test]
fn a_command_runs_in_the_directory_it_was_given() {
    // The claim no amount of looking at the screen can check, and the one a
    // command line is useless without: `rm *.txt` has to mean the pane's
    // files and not whatever the app's own working directory happens to be.
    let dir = fixture(&[("marker.txt", "hello")]);

    let outcome = run(&path_of(&dir), "ls");

    assert!(outcome.succeeded, "{}", outcome.output);
    assert_eq!(outcome.output, "marker.txt");
}

#[test]
fn a_command_can_change_the_directory_it_was_given() {
    // Not a test of `ls` but of the working directory being real: a command
    // that writes lands the file where the pane is looking.
    let dir = fixture(&[]);

    let outcome = run(&path_of(&dir), "echo written > proof.txt");

    assert!(outcome.succeeded, "{}", outcome.output);
    assert_eq!(
        std::fs::read_to_string(dir.path().join("proof.txt")).unwrap(),
        "written\n"
    );
}

#[test]
fn both_streams_come_back_together() {
    // As a terminal shows them, and as the user thinks about them: an error
    // between two lines of output belongs between them.
    let dir = fixture(&[]);

    let outcome = run(&path_of(&dir), "echo out; echo err >&2");

    assert!(outcome.output.contains("out"), "{}", outcome.output);
    assert!(outcome.output.contains("err"), "{}", outcome.output);
}

#[test]
fn a_failing_command_is_reported_as_one_and_keeps_what_it_said() {
    let dir = fixture(&[]);

    let outcome = run(&path_of(&dir), "echo why >&2; exit 3");

    assert!(!outcome.succeeded);
    assert_eq!(outcome.output, "why");
    assert!(outcome.worth_showing());
}

#[test]
fn a_silent_success_is_not_worth_a_window() {
    // Otherwise every `touch` costs a dialog to dismiss.
    let dir = fixture(&[]);

    let outcome = run(&path_of(&dir), "true");

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

    let outcome = run(&path_of(&dir), "false");

    assert!(!outcome.succeeded);
    assert!(outcome.output.is_empty());
    assert!(outcome.worth_showing());
}

#[test]
fn the_shell_is_a_shell_so_pipes_and_globs_work() {
    // The reason the line goes through `$SHELL -c` rather than being split
    // into words here. Without it `ls *.txt | wc -l` simply fails, and that
    // is most of what anybody types into a file manager's command line.
    let dir = fixture(&[("a.txt", "1"), ("b.txt", "2"), ("c.bin", "3")]);

    let outcome = run(&path_of(&dir), "ls *.txt | wc -l");

    assert!(outcome.succeeded, "{}", outcome.output);
    assert_eq!(outcome.output.trim(), "2");
}

#[test]
fn enormous_output_is_cut_and_says_so() {
    // A command may print gigabytes, and keeping all of it to fill a window
    // nobody reads to the end is a way to be killed by the allocator. The cut
    // has to be visible, or a truncated tail reads as the end of the output.
    let dir = fixture(&[]);

    let outcome = run(&path_of(&dir), "yes ferrocommander | head -c 2000000");

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
    let dir = fixture(&[]);

    let outcome = run(&path_of(&dir), "printf 'before\\n\\xff\\xfe\\nafter'");

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
fn windows_opener_keeps_the_empty_title_that_makes_it_work() {
    // `start` is a `cmd` builtin whose first *quoted* argument is a window
    // title, not a file. Without the empty pair of quotes it takes the path
    // as the title and opens nothing — a failure with no error, which is why
    // this is asserted from a platform that will never run it.
    assert!(fc_core::command::WINDOWS_OPENER.starts_with("start "));
    assert!(
        fc_core::command::WINDOWS_OPENER.contains("\"\""),
        "the empty title is gone, and `start` would take the path for one"
    );
}

#[test]
fn every_platform_has_an_opener_and_this_one_is_freedesktop() {
    // Each is a real name on its own platform; none of them is portable,
    // which is the whole reason this is three constants and not one.
    for opener in [
        fc_core::command::WINDOWS_OPENER,
        fc_core::command::MACOS_OPENER,
        fc_core::command::FREEDESKTOP_OPENER,
    ] {
        assert!(!opener.is_empty());
    }
    assert_eq!(
        fc_core::command::DESKTOP_OPENER,
        fc_core::command::FREEDESKTOP_OPENER,
        "the gate runs on Linux, so this is the one it can check"
    );
}

#[test]
fn opening_a_file_and_editing_one_use_the_same_default() {
    // `Enter` and an unconfigured `F4` are the same gesture with different
    // names, and they drifted apart once already — the editor default was
    // still `xdg-open` on every platform when the opener stopped being.
    assert_eq!(
        fc_core::config::DEFAULT_EDITOR,
        fc_core::command::DESKTOP_OPENER
    );
}
