//! Running a typed command line, and bringing back what it said.
//!
//! In `fc-core` for the reason the filesystem is: the UI does not reach past
//! its own layer, and a process is no different from a file. It also makes
//! the whole thing testable with `cargo test` — and "did it run in the
//! directory the pane was showing" is exactly the sort of claim that is easy
//! to get wrong and impossible to see from the outside.
//!
//! Nothing here is asynchronous. The caller runs it on a worker thread,
//! because a file manager whose prime directive is speed does not get to
//! freeze while `find /` finishes (`docs/performance.md`).

use std::process::Command;

use crate::vfs::VfsPath;

/// Environment variable naming the interpreter to run a line with.
///
/// `$SHELL` on Unix; `%ComSpec%` on Windows, which is how that platform
/// names the same thing and is what a program should use rather than
/// assuming where `cmd.exe` lives.
#[cfg(not(windows))]
const SHELL_VARIABLE: &str = "SHELL";
#[cfg(windows)]
const SHELL_VARIABLE: &str = "ComSpec";

/// Used when the variable says nothing. Present on every installation of its
/// platform by definition.
#[cfg(not(windows))]
const FALLBACK_SHELL: &str = "/bin/sh";
#[cfg(windows)]
const FALLBACK_SHELL: &str = "cmd.exe";

/// Tells the interpreter the rest is a command rather than a file to run.
#[cfg(not(windows))]
const SHELL_COMMAND_FLAG: &str = "-c";
#[cfg(windows)]
const SHELL_COMMAND_FLAG: &str = "/C";

/// What hands a file to whatever the desktop opens it with — the same thing
/// a double-click does.
///
/// One name per platform, and none of them is portable: `xdg-open` is
/// freedesktop's, `open` is macOS's, and on Windows it is `start`, which is
/// not a program at all but a `cmd` builtin. That is only usable because
/// [`run`] spawns `cmd` in the first place, which ties these two constants
/// together more tightly than they look.
///
/// `start`'s first quoted argument is a window **title**, not the file — so
/// the empty pair of quotes is load-bearing, and leaving it out opens
/// something else entirely.
/// All three are named here rather than only the one this build uses, so the
/// two it does not can still be checked by the gate that runs — which is the
/// Linux one, always.
pub const WINDOWS_OPENER: &str = "start \"\"";
pub const MACOS_OPENER: &str = "open";
pub const FREEDESKTOP_OPENER: &str = "xdg-open";

#[cfg(target_os = "windows")]
pub const DESKTOP_OPENER: &str = WINDOWS_OPENER;
#[cfg(target_os = "macos")]
pub const DESKTOP_OPENER: &str = MACOS_OPENER;
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub const DESKTOP_OPENER: &str = FREEDESKTOP_OPENER;

/// How much output is kept.
///
/// A command may print gigabytes, and holding all of it to show in a window
/// nobody will read to the end is a way to be killed by the allocator. The cut
/// is visible rather than silent — see [`TRUNCATION_NOTICE`].
pub const OUTPUT_LIMIT: usize = 256 * 1024;

/// Appended when output was cut, so a truncated tail is never mistaken for
/// the end of what a command said.
pub const TRUNCATION_NOTICE: &str = "\n… output truncated";

/// What a finished command left behind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// Whether it exited successfully. A command killed by a signal did not.
    pub succeeded: bool,
    /// Everything it wrote, both streams together in the order they were
    /// read, trimmed of the trailing newline every command ends with.
    ///
    /// Both streams together because that is how a terminal shows them and
    /// how the user thinks about them: an error printed between two lines of
    /// output belongs between them.
    pub output: String,
}

impl Outcome {
    /// Whether this is worth putting a window in front of the user.
    ///
    /// A command that succeeded silently is one the user watched work; saying
    /// so would be a dialog to dismiss for every `touch`.
    pub fn worth_showing(&self) -> bool {
        !self.succeeded || !self.output.is_empty()
    }
}

/// Runs `line` through the user's shell, in `directory`.
///
/// Through a shell rather than split into words here, so pipes, redirection,
/// globs and `~` all work — which is what someone typing into a file manager
/// expects, and the reason the feature exists at all. It also means the user's
/// shell startup applies, which is the point rather than a side effect.
///
/// A shell that cannot be started at all comes back as a failed [`Outcome`]
/// carrying the reason, not as an error the caller has to render a second way:
/// from the user's side "it did not run" and "it ran and failed" want the same
/// window.
pub fn run(directory: &VfsPath, line: &str) -> Outcome {
    let shell = std::env::var(SHELL_VARIABLE).unwrap_or_else(|_| FALLBACK_SHELL.to_string());
    let started = Command::new(&shell)
        .arg(SHELL_COMMAND_FLAG)
        .arg(line)
        .current_dir(crate::vfs::to_std_path(directory))
        .output();

    match started {
        Ok(output) => {
            let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
            text.push_str(&String::from_utf8_lossy(&output.stderr));
            Outcome {
                succeeded: output.status.success(),
                output: limited(text),
            }
        }
        Err(reason) => Outcome {
            succeeded: false,
            output: format!("{shell}: {reason}"),
        },
    }
}

/// Opens `path` with `editor`, and does not wait for it.
///
/// Hands `path` to `program`, which is a command *line* rather than a
/// program name.
///
/// Two callers with the same shape: `F4` passes the editor from the settings,
/// and `Enter` passes the desktop's own handler. One function because the
/// difference between them is which command line, not what happens to it.
///
/// Through the shell, like a typed command: so `x-terminal-emulator -e vim`
/// works and the quoting is the shell's to read. The path is appended quoted, because a filename with a space in it is
/// ordinary and splitting it would open two files that do not exist.
///
/// Detached rather than awaited: an editor runs for as long as somebody is
/// editing, and a file manager that waited would be frozen for all of it.
/// Nothing comes back — an editor that fails to start is between the user and
/// their desktop, and a window from us saying so would arrive long after they
/// noticed.
pub fn open_with(program: &str, path: &VfsPath) {
    let line = format!("{program} {}", shell_quoted(path.as_str()));
    let directory = path.parent().unwrap_or_else(VfsPath::root);
    std::thread::spawn(move || {
        let _ = run(&directory, &line);
    });
}

/// Runs a command template with `%1` and `%2` replaced by two quoted paths.
///
/// What Compare by Content does with a configured tool
/// (`docs/compare.md`): the template is the user's own line, the paths land
/// where the placeholders are, and the whole thing runs like a typed
/// command — detached, for [`open_with`]'s reason: a diff tool runs for as
/// long as somebody is reading.
///
/// The working directory is the left file's, arbitrarily but not
/// meaninglessly: it is the active pane's side, the same directory a typed
/// command would run in.
pub fn run_with_paths(template: &str, left: &VfsPath, right: &VfsPath) {
    let line = substituted(template, left.as_str(), right.as_str());
    let directory = left.parent().unwrap_or_else(VfsPath::root);
    std::thread::spawn(move || {
        let _ = run(&directory, &line);
    });
}

/// The template with each `%1`/`%2` replaced by its path, quoted.
///
/// **Single-pass on purpose.** Two `str::replace` calls in sequence read
/// their own output: a *path* containing the text `%2` would come out of
/// the first replacement and be substituted again by the second, splicing
/// the other file's path into the middle of this one's. One walk over the
/// template never re-reads what it wrote. Anything else after `%` — `%3`,
/// a lone `%`, `%%` — passes through as written; the template is the
/// user's line and inventing meanings for it here would be guessing.
fn substituted(template: &str, left_path: &str, right_path: &str) -> String {
    let mut line = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(at) = rest.find('%') {
        line.push_str(&rest[..at]);
        match rest.as_bytes().get(at + 1) {
            Some(b'1') => {
                line.push_str(&shell_quoted(left_path));
                rest = &rest[at + 2..];
            }
            Some(b'2') => {
                line.push_str(&shell_quoted(right_path));
                rest = &rest[at + 2..];
            }
            _ => {
                line.push('%');
                rest = &rest[at + 1..];
            }
        }
    }
    line.push_str(rest);
    line
}

/// A string the shell will read back as exactly one word.
///
/// Single quotes take everything literally; the only thing that cannot appear
/// inside them is a single quote, which is spliced in the usual way.
fn shell_quoted(text: &str) -> String {
    format!("'{}'", text.replace('\'', r"'\''"))
}

/// Starts a command on a thread of its own and hands back where its answer
/// will arrive.
///
/// The threading lives here rather than in the shell, for the same reason the
/// job queue's does: `fc-core` owns the channel and the worker, and the UI
/// only awaits. It is a plain thread rather than the job queue because a
/// command is not a file operation — it has no progress to report and no
/// conflicts to answer, and must not wait behind a copy that is still
/// running.
pub fn spawn(directory: VfsPath, line: String) -> async_channel::Receiver<Outcome> {
    // One slot, and the only send is the outcome: the worker never blocks and
    // the channel closes when it ends.
    let (sender, receiver) = async_channel::bounded(1);
    std::thread::spawn(move || {
        let _ = sender.send_blocking(run(&directory, &line));
    });
    receiver
}

/// Trims the trailing newline and caps the length, marking a cut.
fn limited(mut text: String) -> String {
    while text.ends_with('\n') || text.ends_with('\r') {
        text.pop();
    }
    if text.len() <= OUTPUT_LIMIT {
        return text;
    }
    // On a character boundary, or the truncation panics on the first command
    // that prints enough non-ASCII.
    let mut cut = OUTPUT_LIMIT;
    while !text.is_char_boundary(cut) {
        cut -= 1;
    }
    text.truncate(cut);
    text.push_str(TRUNCATION_NOTICE);
    text
}

#[cfg(test)]
mod tests {
    use super::substituted;

    #[test]
    fn both_placeholders_land_quoted_where_they_stand() {
        assert_eq!(
            substituted("meld %1 %2", "/a/left file", "/b/right"),
            "meld '/a/left file' '/b/right'"
        );
    }

    #[test]
    fn a_quote_in_a_path_survives_the_shell() {
        assert_eq!(
            substituted("diff %1 %2", "/it's here", "/plain"),
            r#"diff '/it'\''s here' '/plain'"#
        );
    }

    #[test]
    fn a_placeholder_named_twice_is_filled_twice_and_one_missing_is_missing() {
        // The template is the user's line: `%1` twice means they wanted the
        // path twice, and no `%2` means they did not want the second path.
        assert_eq!(substituted("x %1 %1", "/a", "/b"), "x '/a' '/a'");
        assert_eq!(substituted("x %1", "/a", "/b"), "x '/a'");
    }

    #[test]
    fn stray_percents_pass_through_as_written() {
        assert_eq!(substituted("x %3 %% % %", "/a", "/b"), "x %3 %% % %");
    }

    // The reason the walk is single-pass: sequential `replace` calls read
    // their own output, and a path *containing* `%2` would have the other
    // file's path spliced into its middle.
    #[test]
    fn a_path_containing_a_placeholder_is_not_substituted_again() {
        assert_eq!(
            substituted("x %1 %2", "/dir/100%2off.txt", "/b"),
            "x '/dir/100%2off.txt' '/b'"
        );
    }
}
