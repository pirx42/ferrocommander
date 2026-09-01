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

/// How a path is made into one word for the interpreter that will read it.
///
/// **A parameter rather than a `cfg`**, for the reason
/// `vfs::platform::windows_native` takes its separator as one: the Windows CI
/// job builds and packages but the only gate that runs *everything* is the
/// Linux one, so a `#[cfg(windows)] fn quoted` would be a piece of quoting
/// nobody ever checked. As an argument, both halves are checked by the gate
/// that runs — which is also why this and [`quoted`] are public: the tests
/// name both variants from a platform that will only ever use one, the same
/// arrangement the opener constants have.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Quoting {
    /// `'like this'`. Single quotes take everything literally, and the one
    /// character that cannot appear inside them is spliced in the usual way.
    Posix,
    /// `"like this"`. `cmd` has no way to escape a quote inside a quoted
    /// word — and needs none, because `"` is not a legal character in a
    /// Windows filename.
    Cmd,
}

/// The one this build's interpreter reads.
#[cfg(not(windows))]
pub const QUOTING: Quoting = Quoting::Posix;
#[cfg(windows)]
pub const QUOTING: Quoting = Quoting::Cmd;

/// The `editor` line that means "whatever the desktop opens this with" — the
/// same thing `Enter` does.
///
/// Empty, because that is what an unset setting already is: a blank
/// `editor` in the config and this constant are one value rather than two
/// that have to be kept in step. [`open_with`] recognises it and never
/// builds a command line at all.
pub const DESKTOP_HANDLER: &str = "";

/// The program that hands a file to whatever the desktop opens it with.
///
/// One name per platform and neither is portable: `xdg-open` is
/// freedesktop's, `open` is macOS's. **Windows has no such program** — its
/// answer is an API call, not a name, which is why [`open_in_desktop`]
/// rather than this constant is what the rest of the program uses. Both are
/// named here rather than only the one this build runs, so the gate that
/// does run can still check the one it will never call.
pub const MACOS_OPENER: &str = "open";
pub const FREEDESKTOP_OPENER: &str = "xdg-open";

#[cfg(target_os = "macos")]
const DESKTOP_OPENER: &str = MACOS_OPENER;
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
const DESKTOP_OPENER: &str = FREEDESKTOP_OPENER;

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
    let mut command = Command::new(&shell);
    command.arg(SHELL_COMMAND_FLAG);
    push_line(&mut command, line);
    let started = command
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

/// Hands the whole line to the interpreter as one argument.
///
/// **`raw_arg` on Windows, and the third of the three bugs behind the
/// `Enter` dialog.** `Command::arg` quotes for the C runtime's rules — it
/// wraps an argument holding spaces in `"` and escapes the quotes inside it
/// as `\"` — and `cmd` reads neither. A line as ordinary as
/// `notepad "C:\dir\a file.txt"` arrived with backslashes in front of its
/// quotes and was taken apart into something that is not a path. `raw_arg`
/// appends it exactly as written, which is the only way to hand `cmd` a
/// command line.
#[cfg(not(windows))]
fn push_line(command: &mut Command, line: &str) {
    command.arg(line);
}

#[cfg(windows)]
fn push_line(command: &mut Command, line: &str) {
    use std::os::windows::process::CommandExt;
    command.raw_arg(line);
}

/// Opens `path` with `program`, and does not wait for it.
///
/// `program` is a command *line* rather than a program name — the `editor`
/// setting, which may carry arguments of its own
/// (`x-terminal-emulator -e vim`). [`DESKTOP_HANDLER`] means the desktop's
/// own, and is what an unset setting amounts to.
///
/// Through the shell, so the arguments in that line are read as arguments and
/// not as part of a program name. The path is appended quoted, because a
/// filename with a space in it is ordinary and splitting it would open two
/// files that do not exist.
///
/// Detached rather than awaited: an editor runs for as long as somebody is
/// editing, and a file manager that waited would be frozen for all of it.
/// Nothing comes back — an editor that fails to start is between the user and
/// their desktop, and a window from us saying so would arrive long after they
/// noticed.
pub fn open_with(program: &str, path: &VfsPath) {
    let Some(line) = editor_line(program, path) else {
        open_in_desktop(path);
        return;
    };
    let directory = path.parent().unwrap_or_else(VfsPath::root);
    std::thread::spawn(move || {
        let _ = run(&directory, &line);
    });
}

/// The line [`open_with`] would run, or `None` when there is no line to run
/// because [`DESKTOP_HANDLER`] was asked for.
///
/// Split out so the composition is a value a test can look at. It was wrong
/// in three ways for a whole release precisely because nothing could:
/// `open_with` builds a line and spawns a thread, and neither of those is
/// something to assert on.
pub fn editor_line(program: &str, path: &VfsPath) -> Option<String> {
    match program.trim() == DESKTOP_HANDLER {
        true => None,
        false => Some(format!("{program} {}", quoted(&native(path), QUOTING))),
    }
}

/// Opens `path` the way a double-click in the desktop's own file manager
/// does, and does not wait for it.
///
/// **No shell, and no command line.** This is what `Enter` does, and building
/// a line for an interpreter to take apart again is where every one of the
/// three `Enter` bugs came from: a path is handed over as one argument —
/// `xdg-open`'s or `open`'s — and on Windows as one argument to
/// `ShellExecuteW`, which is the call a double-click in Explorer makes.
/// There is no quoting, so there is none to get wrong.
///
/// On its own thread for [`open_with`]'s reason and one more: the handler may
/// be slow to start, and `ShellExecuteW` wants a thread whose apartment it
/// may initialise.
///
/// Nothing comes back. A file the desktop has no handler for is between the
/// user and their desktop — which says so itself, in its own words, on all
/// three platforms.
pub fn open_in_desktop(path: &VfsPath) {
    let file = native(path);
    let directory = crate::vfs::to_std_path(&path.parent().unwrap_or_else(VfsPath::root));
    std::thread::spawn(move || desktop::open(&file, &directory));
}

/// The one call per platform behind [`open_in_desktop`].
#[cfg(not(target_os = "windows"))]
mod desktop {
    /// Spawned with the path as a **single argument**, never a line: a
    /// filename holding a space, a quote or a `$` is ordinary, and no shell
    /// ever sees this one.
    pub fn open(file: &str, directory: &std::path::Path) {
        let _ = std::process::Command::new(super::DESKTOP_OPENER)
            .arg(file)
            .current_dir(directory)
            .output();
    }
}

#[cfg(target_os = "windows")]
mod desktop {
    /// Windows has no `xdg-open` program. It has this call — the one
    /// Explorer itself makes for a double-click, which is exactly what was
    /// asked for: a `.png` opens in the system viewer, an `.exe` starts.
    ///
    /// **Hand-declared**, like `GetDiskFreeSpaceExW` next door in
    /// `vfs::platform`, and refused a Windows API crate for the same reason:
    /// two entry points do not earn a dependency. The signatures are
    /// `ShellExecuteW` from `shellapi.h` and `CoInitializeEx` from
    /// `objbase.h`.
    ///
    /// COM is initialised first because the documentation asks for it: the
    /// call may hand over to a Shell extension, and some of those require a
    /// single-threaded apartment. This thread is ours and has none yet, so
    /// there is nothing to conflict with.
    ///
    /// A null verb rather than `"open"`: it means *the default verb*, which
    /// for a document is `open` and for an installer may be something else
    /// the file itself declares — the same choice Explorer makes.
    pub fn open(file: &str, directory: &std::path::Path) {
        /// Single-threaded apartment, and no OLE1 DDE — the pair the Shell
        /// documentation names for this call.
        const COINIT: u32 = 0x2 | 0x4;
        /// `SW_SHOWNORMAL`: a window, at whatever size the program wants.
        const SHOW_NORMAL: i32 = 1;

        #[link(name = "ole32")]
        extern "system" {
            fn CoInitializeEx(reserved: *mut core::ffi::c_void, flags: u32) -> i32;
        }
        #[link(name = "shell32")]
        extern "system" {
            fn ShellExecuteW(
                window: *mut core::ffi::c_void,
                operation: *const u16,
                file: *const u16,
                parameters: *const u16,
                directory: *const u16,
                show: i32,
            ) -> *mut core::ffi::c_void;
        }

        let file = crate::vfs::wide_nul(file);
        let directory = crate::vfs::wide_nul(&directory.to_string_lossy());
        // SAFETY: both strings are NUL-terminated and outlive the call; the
        // window, verb and parameter pointers are null, which this entry
        // point documents as "no parent window", "the default verb" and "no
        // arguments".
        unsafe {
            CoInitializeEx(std::ptr::null_mut(), COINIT);
            ShellExecuteW(
                std::ptr::null_mut(),
                std::ptr::null(),
                file.as_ptr(),
                std::ptr::null(),
                directory.as_ptr(),
                SHOW_NORMAL,
            );
        }
    }
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
    let line = substituted(template, &native(left), &native(right), QUOTING);
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
fn substituted(template: &str, left_path: &str, right_path: &str, style: Quoting) -> String {
    let mut line = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(at) = rest.find('%') {
        line.push_str(&rest[..at]);
        match rest.as_bytes().get(at + 1) {
            Some(b'1') => {
                line.push_str(&quoted(left_path, style));
                rest = &rest[at + 2..];
            }
            Some(b'2') => {
                line.push_str(&quoted(right_path, style));
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

/// A string the named interpreter will read back as exactly one word.
pub fn quoted(text: &str, style: Quoting) -> String {
    match style {
        Quoting::Posix => format!("'{}'", text.replace('\'', r"'\''")),
        Quoting::Cmd => format!("\"{text}\""),
    }
}

/// The path as the interpreter must see it: `C:\\dir\\file` on Windows, and
/// the VFS path itself everywhere else.
///
/// **The whole first half of the `Enter` bug.** The line was built from
/// `VfsPath::as_str`, which on Windows is `/C:/dir/file` — a leading slash
/// `cmd` reads as the start of a switch, and separators the wrong way round.
/// `to_std_path` is the function that already knew better.
fn native(path: &VfsPath) -> String {
    crate::vfs::to_std_path(path).to_string_lossy().into_owned()
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
    use super::{substituted, Quoting};

    /// What the gate that runs is running: every case below is stated in
    /// this style, and the one Windows case says so in its name.
    const POSIX: Quoting = Quoting::Posix;

    #[test]
    fn both_placeholders_land_quoted_where_they_stand() {
        assert_eq!(
            substituted("meld %1 %2", "/a/left file", "/b/right", POSIX),
            "meld '/a/left file' '/b/right'"
        );
    }

    #[test]
    fn the_same_template_is_quoted_for_cmd_on_windows() {
        // The compare tool's line goes through the same composition `Enter`
        // did, so it broke the same way and is checked the same way.
        assert_eq!(
            substituted("meld %1 %2", r"C:\a\left file", r"C:\b\right", Quoting::Cmd),
            "meld \"C:\\a\\left file\" \"C:\\b\\right\""
        );
    }

    #[test]
    fn a_quote_in_a_path_survives_the_shell() {
        assert_eq!(
            substituted("diff %1 %2", "/it's here", "/plain", POSIX),
            r#"diff '/it'\''s here' '/plain'"#
        );
    }

    #[test]
    fn a_placeholder_named_twice_is_filled_twice_and_one_missing_is_missing() {
        // The template is the user's line: `%1` twice means they wanted the
        // path twice, and no `%2` means they did not want the second path.
        assert_eq!(substituted("x %1 %1", "/a", "/b", POSIX), "x '/a' '/a'");
        assert_eq!(substituted("x %1", "/a", "/b", POSIX), "x '/a'");
    }

    #[test]
    fn stray_percents_pass_through_as_written() {
        assert_eq!(substituted("x %3 %% % %", "/a", "/b", POSIX), "x %3 %% % %");
    }

    // The reason the walk is single-pass: sequential `replace` calls read
    // their own output, and a path *containing* `%2` would have the other
    // file's path spliced into its middle.
    #[test]
    fn a_path_containing_a_placeholder_is_not_substituted_again() {
        assert_eq!(
            substituted("x %1 %2", "/dir/100%2off.txt", "/b", POSIX),
            "x '/dir/100%2off.txt' '/b'"
        );
    }
}
