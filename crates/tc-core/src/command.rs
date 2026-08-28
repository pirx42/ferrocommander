//! Running a typed command line, and bringing back what it said.
//!
//! In `tc-core` for the reason the filesystem is: the UI does not reach past
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

/// Environment variable naming the user's shell.
const SHELL_VARIABLE: &str = "SHELL";

/// Used when `$SHELL` says nothing. Present on every Unix by definition.
const FALLBACK_SHELL: &str = "/bin/sh";

/// Tells the shell the rest is a command rather than a file to run.
const SHELL_COMMAND_FLAG: &str = "-c";

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
/// Through the shell, like a typed command: the editor is a command *line*,
/// so `x-terminal-emulator -e vim` works and the quoting is the shell's to
/// read. The path is appended quoted, because a filename with a space in it is
/// ordinary and splitting it would open two files that do not exist.
///
/// Detached rather than awaited: an editor runs for as long as somebody is
/// editing, and a file manager that waited would be frozen for all of it.
/// Nothing comes back — an editor that fails to start is between the user and
/// their desktop, and a window from us saying so would arrive long after they
/// noticed.
pub fn open_in_editor(editor: &str, path: &VfsPath) {
    let line = format!("{editor} {}", shell_quoted(path.as_str()));
    let directory = path.parent().unwrap_or_else(VfsPath::root);
    std::thread::spawn(move || {
        let _ = run(&directory, &line);
    });
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
/// job queue's does: `tc-core` owns the channel and the worker, and the UI
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
