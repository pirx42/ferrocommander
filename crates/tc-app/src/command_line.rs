//! The command line across the bottom of the window.
//!
//! Total Commander has one and a hard-core user's fingers expect it: a place
//! to type `chmod +x *.sh` without leaving the file manager, in the directory
//! the active pane is already showing.
//!
//! Nothing here decides what a command *means* — that is
//! [`tc_core::command`], which is also where it runs. This is the entry, its
//! prompt, and the rule about `cd`.

use gtk::prelude::*;

use tc_core::vfs::VfsPath;

use crate::constants::{
    CLASS_COMMAND_LINE, CLASS_COMMAND_PROMPT, COMMAND_PROMPT_SUFFIX, PANE_SPACING,
};

/// What a typed line turns out to be.
pub enum Typed {
    /// A directory to go to, from a `cd`. Handled here rather than spawned: a
    /// `cd` in a child process changes nothing anybody can see, so a command
    /// line that ran it would look broken.
    ChangeDirectory(String),
    /// Anything else, to be handed to the shell.
    Shell(String),
}

/// The `cd` command, and the only one this program reads rather than runs.
const CHANGE_DIRECTORY: &str = "cd";

/// Reads a typed line.
///
/// `cd` alone, with no argument, means the home directory — as it does in
/// every shell, and as a user's fingers assume.
pub fn read(line: &str) -> Typed {
    let line = line.trim();
    let (head, rest) = match line.split_once(char::is_whitespace) {
        Some((head, rest)) => (head, rest.trim()),
        None => (line, ""),
    };
    if head != CHANGE_DIRECTORY {
        return Typed::Shell(line.to_string());
    }
    Typed::ChangeDirectory(rest.to_string())
}

/// Where a `cd` argument leads, from `current`.
///
/// Relative arguments resolve against the pane's directory, `~` against the
/// home directory, and an empty argument *is* the home directory. `VfsPath`
/// normalizes lexically, so `..` and `.` need no case of their own.
pub fn destination(current: &VfsPath, argument: &str, home: Option<VfsPath>) -> Option<VfsPath> {
    if argument.is_empty() || argument == HOME_MARK {
        return home;
    }
    if let Some(rest) = argument.strip_prefix(HOME_PREFIX) {
        return home.map(|home| home.child(rest));
    }
    if argument.starts_with(SEPARATOR) {
        return Some(VfsPath::new(argument));
    }
    Some(current.child(argument))
}

/// What a shell writes for the home directory.
const HOME_MARK: &str = "~";
const HOME_PREFIX: &str = "~/";

/// Path separator as a user types it. The VFS spells paths this way on both
/// platforms — see `docs/vfs.md`.
const SEPARATOR: char = '/';

/// The entry and the prompt beside it.
pub struct CommandLine {
    root: gtk::Box,
    prompt: gtk::Label,
    entry: gtk::Entry,
}

impl CommandLine {
    pub fn new() -> Self {
        let prompt = gtk::Label::builder().xalign(0.0).build();
        prompt.add_css_class(CLASS_COMMAND_PROMPT);

        // No frame, so the line reads as part of the window rather than as a
        // dialog that wandered in.
        let entry = gtk::Entry::builder().hexpand(true).has_frame(false).build();

        let root = gtk::Box::new(gtk::Orientation::Horizontal, PANE_SPACING);
        root.add_css_class(CLASS_COMMAND_LINE);
        root.append(&prompt);
        root.append(&entry);

        CommandLine {
            root,
            prompt,
            entry,
        }
    }

    pub fn widget(&self) -> &gtk::Box {
        &self.root
    }

    pub fn entry(&self) -> &gtk::Entry {
        &self.entry
    }

    /// Says which directory a command would run in.
    ///
    /// Shown rather than left to be remembered: the command line follows the
    /// *active* pane, so which directory it means changes under the user with
    /// every Tab.
    pub fn follow(&self, directory: &VfsPath) {
        self.prompt
            .set_text(&format!("{directory}{COMMAND_PROMPT_SUFFIX}"));
    }

    pub fn text(&self) -> String {
        self.entry.text().to_string()
    }

    /// Puts a whole line in, with the cursor at its end — a history pick,
    /// ready to be edited rather than already running.
    pub fn set_text(&self, text: &str) {
        self.entry.set_text(text);
        self.entry.set_position(-1);
    }

    /// Appends a word, with a space before it when there is already
    /// something there.
    ///
    /// The space is the difference between `lstouch` and `ls touch`: a name
    /// inserted after a command has to be an argument, and having to reach for
    /// the space bar first would make the shortcut not worth using.
    pub fn append_word(&self, word: &str) {
        let current = self.text();
        let separator = if current.is_empty() || current.ends_with(' ') {
            ""
        } else {
            " "
        };
        self.set_text(&format!("{current}{separator}{word}"));
        self.entry.grab_focus();
        self.entry.set_position(-1);
    }

    pub fn grab_focus(&self) {
        self.entry.grab_focus();
        self.entry.set_position(-1);
    }

    pub fn clear(&self) {
        self.entry.set_text("");
    }

    /// Adds one typed character and takes the keyboard.
    ///
    /// Total Commander's feel, and what makes the line usable at all: a
    /// letter that no binding claims starts a command instead of being
    /// dropped. Without it there is no way in from the keyboard, which for a
    /// keyboard-first program is the same as no command line.
    pub fn accept(&self, character: char) {
        let mut text = self.text();
        text.push(character);
        self.entry.set_text(&text);
        self.entry.grab_focus();
        // After the grab, or the focus-in selects everything and the next
        // character replaces the line.
        self.entry.set_position(-1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shell_line(line: &str) -> String {
        match read(line) {
            Typed::Shell(line) => line,
            Typed::ChangeDirectory(argument) => panic!("read as cd {argument:?}"),
        }
    }

    fn cd_argument(line: &str) -> String {
        match read(line) {
            Typed::ChangeDirectory(argument) => argument,
            Typed::Shell(line) => panic!("read as a command: {line:?}"),
        }
    }

    #[test]
    fn cd_is_read_rather_than_run() {
        // A `cd` in a child process changes nothing anybody can see, so a
        // command line that spawned one would look broken.
        assert_eq!(cd_argument("cd /tmp"), "/tmp");
        assert_eq!(cd_argument("  cd   /tmp  "), "/tmp");
        assert_eq!(cd_argument("cd"), "", "bare cd means home");
    }

    #[test]
    fn a_command_that_merely_starts_with_cd_is_not_a_cd() {
        // `cdparanoia` and `cdrecord` are real programs, and swallowing them
        // as a directory change would be a silent wrong answer.
        for line in ["cdparanoia -B", "cdda2wav", "cd_something"] {
            assert_eq!(shell_line(line), line, "{line}");
        }
    }

    #[test]
    fn everything_else_goes_to_the_shell_untouched() {
        // Including the quoting and the pipes, which are the shell's to read
        // and not this program's to second-guess.
        for line in ["ls -la | wc -l", "echo 'cd /tmp'", "chmod +x *.sh"] {
            assert_eq!(shell_line(line), line, "{line}");
        }
    }

    #[test]
    fn a_relative_cd_resolves_against_the_pane() {
        let here = VfsPath::new("/home/pirx");
        let home = Some(VfsPath::new("/home/pirx"));
        assert_eq!(
            destination(&here, "projects", home.clone()),
            Some(VfsPath::new("/home/pirx/projects"))
        );
        // `VfsPath` normalizes lexically, so `..` and `.` need no case of
        // their own here.
        assert_eq!(
            destination(&here, "..", home.clone()),
            Some(VfsPath::new("/home"))
        );
        assert_eq!(destination(&here, ".", home), Some(here.clone()));
    }

    #[test]
    fn an_absolute_cd_goes_where_it_says() {
        let here = VfsPath::new("/home/pirx");
        assert_eq!(
            destination(&here, "/mnt/backup", None),
            Some(VfsPath::new("/mnt/backup"))
        );
    }

    #[test]
    fn a_tilde_means_the_home_directory() {
        let here = VfsPath::new("/mnt/backup");
        let home = Some(VfsPath::new("/home/pirx"));
        assert_eq!(destination(&here, "~", home.clone()), home.clone());
        assert_eq!(destination(&here, "", home.clone()), home.clone());
        assert_eq!(
            destination(&here, "~/projects", home),
            Some(VfsPath::new("/home/pirx/projects"))
        );
    }

    #[test]
    fn a_tilde_with_no_home_to_expand_leads_nowhere() {
        // Better than resolving it against the pane, which would make `cd ~`
        // create a directory called `~` the next time anybody typed F7 there.
        let here = VfsPath::new("/mnt/backup");
        assert_eq!(destination(&here, "~", None), None);
        assert_eq!(destination(&here, "~/projects", None), None);
    }
}
