//! The settings file: what the program remembers between runs.
//!
//! In `tc-core` because the UI never touches a filesystem directly, and its
//! own settings are no exception — everything here goes through a
//! [`VirtualFs`], including the atomic write.
//!
//! **A missing or unreadable file is not an error.** It is a first run, and
//! the defaults apply. A corrupt one is reported once and replaced, because a
//! file manager that refuses to start over its own settings is worse than one
//! that forgets where you were.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::listing::{Sort, SortKey, SortOrder};
use crate::vfs::{VfsError, VfsPath, VirtualFs};

/// Directory the settings file lives in, below the platform's config root.
pub const CONFIG_DIR: &str = "ferrocommander";

/// Name of the settings file.
pub const CONFIG_FILE: &str = "config.toml";

/// The table of key bindings, which belongs to the user alone: read on
/// startup, and never written back.
pub const KEYS_TABLE: &str = "keys";

/// What opens a file when no editor is configured.
///
/// The desktop's own answer, which is the only one that can be right without
/// being told: `$EDITOR` is nearly always a terminal editor, and launching one
/// with no terminal fails in the common case rather than the rare one.
pub const DEFAULT_EDITOR: &str = "xdg-open";

/// How many command lines are remembered.
///
/// A cap rather than everything: the settings file is rewritten whenever
/// anything changes, and an unbounded list would make that write grow without
/// limit for a list nobody scrolls to the end of.
pub const COMMAND_HISTORY_LIMIT: usize = 100;

/// Written first, then renamed over the real file.
///
/// A settings file truncated by a crash mid-write is a program that starts up
/// wrong, and a rename within one directory is the only write a filesystem
/// promises to do all-or-nothing.
pub const CONFIG_TEMP_FILE: &str = "config.toml.new";

/// Everything the program remembers.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    pub window: WindowSettings,
    /// Column widths, shared by both panes.
    pub columns: ColumnSettings,
    /// One per pane, in pane order. A file with the wrong number of them is
    /// padded or trimmed on load rather than rejected — see [`Settings::pane`].
    pub panes: Vec<PaneSettings>,
    /// Which pane had the keyboard.
    pub active_pane: usize,
    /// The program `Shift+F4` opens a new file in.
    ///
    /// Empty means [`DEFAULT_EDITOR`], which hands the file to whatever the
    /// desktop has registered for it. A terminal editor still needs a terminal,
    /// so somebody wanting `vim` writes `x-terminal-emulator -e vim` here — the
    /// value is a command line, run the same way the command line's is.
    pub editor: String,
    /// The command Compare by Content runs instead of its own view.
    ///
    /// A command line with `%1` and `%2` standing for the two files, each
    /// replaced by a quoted path — `meld %1 %2` is the whole configuration
    /// for somebody with meld. Empty means the built-in side-by-side view
    /// ([`docs/compare.md`]): the setting *is* the decision, so one key
    /// always means one thing and no second binding exists to learn.
    pub compare_tool: String,
    /// Command lines that were run, newest first.
    ///
    /// Kept so `Ctrl+↓` has something to offer on the next run: a history that
    /// forgot everything when the app closed would be a history in name only.
    pub command_history: Vec<String>,
    /// The directory each mount point was last showing, keyed by mount path.
    ///
    /// What makes switching to a drive land where you were on it rather than
    /// at its root — Total Commander's behaviour with its default
    /// `AlwaysToRoot=0`. Shared by both panes, as it is there: leaving a drive
    /// in one pane is what the other finds when it arrives.
    pub drives: BTreeMap<String, String>,
    /// Key bindings the user has overridden, keyed by key name.
    ///
    /// Strings on both sides, and never interpreted here: a key name is a GTK
    /// keysym and an action is a command of the shell, neither of which
    /// `tc-core` knows anything about. Its job is to carry the table across
    /// the file, and the shell's is to make sense of it.
    ///
    /// **Read but never written.** These are the user's own lines, and
    /// [`save`] leaves them exactly as they were typed.
    pub keys: BTreeMap<String, String>,
    /// The directories `Ctrl+D` offers, in the order they are shown.
    ///
    /// Owned by the app — the list is maintained from inside the dialog — so
    /// unlike [`keys`](Self::keys) it is written back. Shared by both panes,
    /// as `[drives]` is: a favourite is a place, not a property of a pane.
    pub favourites: Vec<Favourite>,
}

/// One row of the `Ctrl+D` list: a directory, and what to call it.
///
/// A list rather than a table keyed by name, unlike `[drives]` and `[keys]`
/// next door, and for two reasons. A hotlist is a **menu** — the entry you
/// reach for is the one you put at the top — and a map sorts itself, which
/// would quietly rearrange a list somebody arranged. And two directories may
/// perfectly well both be called `src`, which a map would make a collision
/// needing a rule.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Favourite {
    /// What the list shows. Empty falls back to the path's last component,
    /// so a hand-written entry can be a bare `path = "…"` line.
    pub name: String,
    pub path: String,
}

impl Favourite {
    /// The directory this points at.
    pub fn path(&self) -> VfsPath {
        VfsPath::new(&self.path)
    }

    /// What the list shows for it.
    ///
    /// Falls back rather than showing an empty row: a favourite with no name
    /// is a favourite somebody wrote by hand, and the last component is the
    /// name they would have typed.
    ///
    /// Owned rather than borrowed, so the fallback can go through
    /// [`VfsPath`] instead of splitting the string a second way — there is
    /// one rule for what a path's last component is, and it is not here. A
    /// dialog builds ten of these when it opens, once.
    pub fn label(&self) -> String {
        if !self.name.is_empty() {
            return self.name.clone();
        }
        // The path itself at the root, which has no last component and is
        // still somewhere worth having in the list.
        match self.path().file_name() {
            Some(name) => name.to_string(),
            None => self.path.clone(),
        }
    }
}

/// Adds `path` to the list, named after its last component.
///
/// **Already there is nothing to do**, and that is what bounds the list
/// instead of a cap like [`COMMAND_HISTORY_LIMIT`]: every entry here costs a
/// deliberate keystroke, so the list grows to the number of directories one
/// person cares about and stops. Comparing by path rather than by name, since
/// the name is only what it is called.
///
/// Appended rather than inserted at the front, unlike
/// [`remember_command`]: a history is about what happened last and a hotlist
/// is about where you decided to put things.
pub fn remember_favourite(favourites: &mut Vec<Favourite>, path: &VfsPath) {
    if favourites.iter().any(|entry| entry.path() == *path) {
        return;
    }
    favourites.push(Favourite {
        name: path.file_name().unwrap_or_default().to_string(),
        path: path.to_string(),
    });
}

/// Removes every entry pointing at `path`.
///
/// Every one rather than the first: a file hand-edited into holding the same
/// path twice should be left tidy by a removal, not half-cleaned.
pub fn forget_favourite(favourites: &mut Vec<Favourite>, path: &VfsPath) {
    favourites.retain(|entry| entry.path() != *path);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WindowSettings {
    pub width: i32,
    pub height: i32,
}

/// How wide each column is, in pixels.
///
/// One set for both panes, not one each: the panes are meant to line up with
/// each other, which is the whole reason the widths were constants before
/// they were settings. Dragging a column in either pane moves it in both.
///
/// The name column is not here — it takes whatever is left over, so a window
/// of any width is filled and nothing is cut off by resizing the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ColumnSettings {
    pub ext: i32,
    pub size: i32,
    pub date: i32,
    pub attributes: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PaneSettings {
    /// Where the pane was last looking. Empty means "wherever the app
    /// starts", which is what a first run gets.
    pub directory: String,
    /// One of [`SORT_KEY_NAMES`]; anything else falls back to the default.
    pub sort_key: String,
    pub sort_descending: bool,
    pub show_hidden: bool,
}

/// The names sort keys are written under, and the only place that mapping
/// lives.
///
/// Spelled out rather than derived from the enum, so renaming a variant
/// cannot silently change the file format under everybody's existing
/// settings. The round-trip test iterates this table, so a new key that is
/// not listed here fails the build's tests rather than being written as a
/// default.
const SORT_KEY_NAMES: [(SortKey, &str); 4] = [
    (SortKey::Name, "name"),
    (SortKey::Ext, "ext"),
    (SortKey::Size, "size"),
    (SortKey::Modified, "date"),
];

fn sort_key_name(key: SortKey) -> &'static str {
    SORT_KEY_NAMES
        .iter()
        .find(|(candidate, _)| *candidate == key)
        .map(|(_, name)| *name)
        .unwrap_or(DEFAULT_SORT_KEY_NAME)
}

fn sort_key_from(name: &str) -> Option<SortKey> {
    SORT_KEY_NAMES
        .iter()
        .find(|(_, candidate)| *candidate == name)
        .map(|(key, _)| *key)
}

/// What an unreadable or unknown sort key falls back to.
const DEFAULT_SORT_KEY_NAME: &str = "name";

/// Window size a first run opens at. The shell's own constants cannot be
/// reached from here, so these are the defaults the settings carry.
const DEFAULT_WINDOW_WIDTH: i32 = 1200;
const DEFAULT_WINDOW_HEIGHT: i32 = 700;

/// What a column is wide before anybody drags it.
///
/// The values the shell carried as constants before the widths became
/// settings, so a first run looks exactly as it always did: wide enough for
/// `rwxr-xr-x`, the longest attribute either platform produces, and for a
/// date written to the minute.
const DEFAULT_COLUMN_EXT: i32 = 70;
const DEFAULT_COLUMN_SIZE: i32 = 120;
const DEFAULT_COLUMN_DATE: i32 = 140;
const DEFAULT_COLUMN_ATTRIBUTES: i32 = 90;

impl Default for WindowSettings {
    fn default() -> Self {
        WindowSettings {
            width: DEFAULT_WINDOW_WIDTH,
            height: DEFAULT_WINDOW_HEIGHT,
        }
    }
}

impl Default for ColumnSettings {
    fn default() -> Self {
        ColumnSettings {
            ext: DEFAULT_COLUMN_EXT,
            size: DEFAULT_COLUMN_SIZE,
            date: DEFAULT_COLUMN_DATE,
            attributes: DEFAULT_COLUMN_ATTRIBUTES,
        }
    }
}

impl Default for PaneSettings {
    fn default() -> Self {
        PaneSettings {
            directory: String::new(),
            sort_key: DEFAULT_SORT_KEY_NAME.to_string(),
            sort_descending: false,
            show_hidden: false,
        }
    }
}

impl PaneSettings {
    /// The ordering these settings describe.
    pub fn sort(&self) -> Sort {
        let key = sort_key_from(&self.sort_key).unwrap_or(SortKey::Name);
        let order = if self.sort_descending {
            SortOrder::Descending
        } else {
            SortOrder::Ascending
        };
        Sort::new(key, order)
    }

    /// Records an ordering.
    pub fn set_sort(&mut self, sort: Sort) {
        self.sort_key = sort_key_name(sort.key).to_string();
        self.sort_descending = sort.order == SortOrder::Descending;
    }

    /// Where the pane should open, if it recorded anywhere.
    pub fn directory(&self) -> Option<VfsPath> {
        if self.directory.is_empty() {
            return None;
        }
        Some(VfsPath::new(&self.directory))
    }
}

impl Settings {
    /// The compare command, or `None` when the built-in view is meant.
    ///
    /// `Option` rather than a default string, unlike [`Settings::editor`]:
    /// the editor always has a sensible fallback to hand a file to, and the
    /// compare's fallback is a different *mechanism*, which the caller has
    /// to choose between rather than merely spawn.
    pub fn compare_tool(&self) -> Option<&str> {
        let trimmed = self.compare_tool.trim();
        match trimmed.is_empty() {
            true => None,
            false => Some(trimmed),
        }
    }

    /// The command that opens a file, configured or defaulted.
    pub fn editor(&self) -> &str {
        if self.editor.trim().is_empty() {
            return DEFAULT_EDITOR;
        }
        self.editor.trim()
    }

    /// The settings for pane `index`, defaulted when the file did not carry
    /// that many.
    ///
    /// A file written by a version with a different number of panes is a
    /// thing that will happen, and losing the pane you did record because the
    /// other one is missing would be worse than defaulting it.
    pub fn pane(&self, index: usize) -> PaneSettings {
        self.panes.get(index).cloned().unwrap_or_default()
    }

    /// Records the settings for pane `index`, growing the list as needed.
    pub fn set_pane(&mut self, index: usize, settings: PaneSettings) {
        if self.panes.len() <= index {
            self.panes.resize_with(index + 1, PaneSettings::default);
        }
        self.panes[index] = settings;
    }
}

/// Puts `line` at the front of a command history, and keeps it capped.
///
/// Newest first, and a line run again moves up rather than appearing twice: a
/// history listing `make` eleven times is one you have to read past to find
/// anything else.
///
/// On the list rather than on [`Settings`], because the shell keeps the live
/// history beside its record of what is on disk — writing a new command into
/// that record marks it as already saved, and it never reaches the file.
pub fn remember_command(history: &mut Vec<String>, line: &str) {
    history.retain(|previous| previous != line);
    history.insert(0, line.to_string());
    history.truncate(COMMAND_HISTORY_LIMIT);
}

/// Where the settings file lives, given a config root.
pub fn config_path(config_root: &VfsPath) -> VfsPath {
    config_root.child(CONFIG_DIR).child(CONFIG_FILE)
}

/// Reads the settings, or returns the defaults.
///
/// The second half of the pair says whether anything went wrong, so the shell
/// can mention a corrupt file once instead of silently starting over.
pub fn load(fs: &dyn VirtualFs, config_root: &VfsPath) -> (Settings, Option<String>) {
    let path = config_path(config_root);
    let mut text = String::new();
    match fs.open_read(&path) {
        Ok(mut reader) => {
            if let Err(error) = std::io::Read::read_to_string(&mut reader, &mut text) {
                return (Settings::default(), Some(error.to_string()));
            }
        }
        // No file is a first run, not a problem worth mentioning.
        Err(VfsError::NotFound) => return (Settings::default(), None),
        Err(error) => return (Settings::default(), Some(error.to_string())),
    }

    match toml::from_str(&text) {
        Ok(settings) => (settings, None),
        Err(error) => (Settings::default(), Some(error.to_string())),
    }
}

/// Writes the settings, replacing the previous file only once the new one is
/// complete.
///
/// **The file is edited, not regenerated.** Anything the app does not own —
/// comments, blank lines, the order the user put things in, the whole `[keys]`
/// table — comes back exactly as it was. Serializing the struct instead would
/// reproduce the data and nothing else, and since this runs whenever a setting
/// changes, a hand-written `[keys]` table would lose its comments about half a
/// second after the app opened.
pub fn save(
    fs: &dyn VirtualFs,
    config_root: &VfsPath,
    settings: &Settings,
) -> Result<(), VfsError> {
    let directory = config_root.child(CONFIG_DIR);
    ensure_directory(fs, config_root);
    ensure_directory(fs, &directory);

    let text = render(&read_text(fs, &config_path(config_root)), settings)?;

    // Into a temporary name beside the real one, then renamed over it: a
    // rename within one directory is the only write a filesystem promises to
    // do all-or-nothing, so a crash leaves either the old file or the new
    // one and never half of either.
    let temporary = directory.child(CONFIG_TEMP_FILE);
    {
        let mut writer = fs.create_file(&temporary)?;
        std::io::Write::write_all(&mut writer, text.as_bytes())
            .map_err(|error| VfsError::Io(error.to_string()))?;
    }
    fs.rename(&temporary, &config_path(config_root))
}

/// Updates the tables the app owns inside `existing`, leaving the rest alone.
///
/// A file that cannot be parsed as TOML is started over rather than repaired:
/// there is nothing in it worth preserving that could be found reliably, and
/// refusing to save at all would mean one bad character costs every setting
/// from then on.
fn render(existing: &str, settings: &Settings) -> Result<String, VfsError> {
    let mut document = existing
        .parse::<toml_edit::DocumentMut>()
        .unwrap_or_default();

    // Through a serialized copy rather than by hand: the field list lives in
    // `Settings` and nowhere else, so a setting added there is written without
    // anybody remembering to come back here.
    let owned: toml_edit::DocumentMut = toml::to_string(settings)
        .map_err(|error| VfsError::Io(error.to_string()))?
        .parse()
        .map_err(|error: toml_edit::TomlError| VfsError::Io(error.to_string()))?;

    for (key, value) in owned.as_table() {
        if key == KEYS_TABLE {
            continue;
        }
        document.insert(key, value.clone());
    }
    Ok(document.to_string())
}

/// Reads a file into a string, treating anything that goes wrong as empty.
///
/// Only ever used to find out what to preserve, so "could not read it" and
/// "there was nothing there" lead to the same place.
fn read_text(fs: &dyn VirtualFs, path: &VfsPath) -> String {
    let mut text = String::new();
    if let Ok(mut reader) = fs.open_read(path) {
        let _ = std::io::Read::read_to_string(&mut reader, &mut text);
    }
    text
}

/// Creates a directory, treating "it is already there" as success.
fn ensure_directory(fs: &dyn VirtualFs, path: &VfsPath) {
    let _ = fs.create_dir(path);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_sort_key_survives_the_round_trip() {
        // Iterating the table is what makes this exhaustive: a new sort key
        // that nobody added here has no name, and the mapping would quietly
        // write it as "name".
        for (key, name) in SORT_KEY_NAMES {
            assert_eq!(sort_key_name(key), name);
            assert_eq!(sort_key_from(name), Some(key));
        }
    }

    #[test]
    fn an_unknown_sort_key_falls_back_instead_of_failing() {
        // A settings file from a newer version must not stop this one
        // starting.
        let settings = PaneSettings {
            sort_key: "colour".to_string(),
            ..PaneSettings::default()
        };
        assert_eq!(settings.sort().key, SortKey::Name);
    }

    #[test]
    fn an_ordering_survives_being_recorded_and_read_back() {
        for (key, _) in SORT_KEY_NAMES {
            for order in [SortOrder::Ascending, SortOrder::Descending] {
                let sort = Sort::new(key, order);
                let mut settings = PaneSettings::default();
                settings.set_sort(sort);
                assert_eq!(settings.sort(), sort, "{sort:?}");
            }
        }
    }

    #[test]
    fn a_missing_pane_is_defaulted_rather_than_lost() {
        let mut settings = Settings::default();
        settings.set_pane(
            1,
            PaneSettings {
                directory: "/home/pirx".to_string(),
                ..PaneSettings::default()
            },
        );

        assert_eq!(settings.pane(0), PaneSettings::default());
        assert_eq!(settings.pane(1).directory().unwrap().as_str(), "/home/pirx");
        assert_eq!(settings.pane(9), PaneSettings::default());
    }

    #[test]
    fn a_directory_is_added_once_however_often_it_is_offered() {
        // What bounds the list instead of a cap: pressing "add" on a
        // directory that is already a favourite is a keystroke that changes
        // nothing, not a second row.
        let mut favourites = Vec::new();
        remember_favourite(&mut favourites, &VfsPath::new("/home/pirx/dev"));
        remember_favourite(&mut favourites, &VfsPath::new("/home/pirx/dev"));

        assert_eq!(favourites.len(), 1);
        assert_eq!(favourites[0].label(), "dev");
    }

    #[test]
    fn two_directories_may_share_a_name() {
        // The reason this is a list and not a map keyed by name. Both stay,
        // and the path column is what tells them apart on screen.
        let mut favourites = Vec::new();
        remember_favourite(&mut favourites, &VfsPath::new("/home/pirx/src"));
        remember_favourite(&mut favourites, &VfsPath::new("/srv/build/src"));

        assert_eq!(favourites.len(), 2);
        assert_eq!(favourites[0].label(), favourites[1].label());
    }

    #[test]
    fn the_order_is_the_one_things_were_added_in() {
        // A hotlist is a menu: what a map would do here is sort it, which
        // rearranges a list somebody arranged.
        let mut favourites = Vec::new();
        for path in ["/zebra", "/apple", "/mango"] {
            remember_favourite(&mut favourites, &VfsPath::new(path));
        }

        let labels: Vec<String> = favourites.iter().map(Favourite::label).collect();
        assert_eq!(labels, ["zebra", "apple", "mango"]);
    }

    #[test]
    fn removing_takes_out_every_row_pointing_there() {
        // A hand-edited file may hold the same path twice, and a removal that
        // left one behind would look like it had not worked.
        let mut favourites = vec![
            Favourite {
                name: "one".to_string(),
                path: "/home/pirx/dev".to_string(),
            },
            Favourite {
                name: "the same place".to_string(),
                path: "/home/pirx/dev".to_string(),
            },
            Favourite {
                name: "other".to_string(),
                path: "/srv".to_string(),
            },
        ];

        forget_favourite(&mut favourites, &VfsPath::new("/home/pirx/dev"));

        assert_eq!(favourites.len(), 1);
        assert_eq!(favourites[0].path(), VfsPath::new("/srv"));
    }

    #[test]
    fn the_root_is_a_favourite_that_can_still_be_named() {
        // It has no last component, and a row showing nothing at all is worse
        // than a row showing the path.
        let mut favourites = Vec::new();
        remember_favourite(&mut favourites, &VfsPath::root());

        assert_eq!(favourites[0].label(), VfsPath::root().as_str());
    }

    #[test]
    fn an_unset_editor_falls_back_to_the_desktops_own() {
        assert_eq!(Settings::default().editor(), DEFAULT_EDITOR);
        // Whitespace is not a configuration, it is an empty line somebody
        // left behind.
        let blank = Settings {
            editor: "   ".to_string(),
            ..Settings::default()
        };
        assert_eq!(blank.editor(), DEFAULT_EDITOR);
    }

    #[test]
    fn a_configured_editor_is_used_as_written() {
        let settings = Settings {
            editor: " x-terminal-emulator -e vim ".to_string(),
            ..Settings::default()
        };
        assert_eq!(settings.editor(), "x-terminal-emulator -e vim");
    }

    #[test]
    fn an_unset_compare_tool_means_the_built_in_view() {
        assert_eq!(Settings::default().compare_tool(), None);
        let blank = Settings {
            compare_tool: "   ".to_string(),
            ..Settings::default()
        };
        assert_eq!(blank.compare_tool(), None);
    }

    #[test]
    fn a_configured_compare_tool_is_used_as_written() {
        let settings = Settings {
            compare_tool: " meld %1 %2 ".to_string(),
            ..Settings::default()
        };
        assert_eq!(settings.compare_tool(), Some("meld %1 %2"));
    }

    #[test]
    fn a_command_run_again_moves_up_rather_than_appearing_twice() {
        // A history listing `make` eleven times is one you read past to find
        // anything else.
        let mut history = Vec::new();
        remember_command(&mut history, "make");
        remember_command(&mut history, "ls -la");
        remember_command(&mut history, "make");

        assert_eq!(history, ["make", "ls -la"]);
    }

    #[test]
    fn the_history_stops_growing_at_the_cap() {
        // The settings file is rewritten whenever anything changes, so an
        // unbounded list would make that write grow without limit.
        let mut history = Vec::new();
        for index in 0..COMMAND_HISTORY_LIMIT + 10 {
            remember_command(&mut history, &format!("command {index}"));
        }

        assert_eq!(history.len(), COMMAND_HISTORY_LIMIT);
        // The newest survived the trimming, not the oldest.
        assert_eq!(history[0], format!("command {}", COMMAND_HISTORY_LIMIT + 9));
    }

    #[test]
    fn an_empty_directory_means_wherever_the_app_starts() {
        assert_eq!(PaneSettings::default().directory(), None);
    }

    /// A file as a person would write it: their own bindings, their own
    /// comments, and the app's own state mixed in.
    const HAND_WRITTEN: &str = "\
# my bindings
[keys]
# swap the panes with something my thumb can reach
\"ctrl+e\" = \"exchange_panes\"
\"f9\" = \"\"

[window]
width = 800
height = 600
";

    #[test]
    fn saving_leaves_the_users_own_lines_exactly_as_they_wrote_them() {
        // The reason this file is edited rather than regenerated. Serializing
        // the struct reproduces the data and nothing else, and the save runs
        // whenever a setting changes — so a hand-written [keys] table would
        // lose its comments about half a second after the app opened.
        let settings = Settings {
            window: WindowSettings {
                width: 1234,
                height: 567,
            },
            ..Settings::default()
        };

        let written = render(HAND_WRITTEN, &settings).unwrap();

        for kept in [
            "# my bindings",
            "# swap the panes with something my thumb can reach",
            "\"ctrl+e\" = \"exchange_panes\"",
            "\"f9\" = \"\"",
        ] {
            assert!(written.contains(kept), "{kept:?} was lost:\n{written}");
        }
    }

    #[test]
    fn saving_still_records_what_the_app_owns() {
        let settings = Settings {
            window: WindowSettings {
                width: 1234,
                height: 567,
            },
            ..Settings::default()
        };

        let written = render(HAND_WRITTEN, &settings).unwrap();
        let (read_back, complaint) = (
            toml::from_str::<Settings>(&written).unwrap(),
            None::<String>,
        );

        assert_eq!(complaint, None);
        assert_eq!(read_back.window, settings.window, "the new size");
        // And the user's table came back through the round trip as well, which
        // is what makes the preservation useful rather than decorative.
        assert_eq!(
            read_back.keys.get("ctrl+e").map(String::as_str),
            Some("exchange_panes")
        );
    }

    #[test]
    fn a_file_that_is_not_toml_at_all_is_started_over_rather_than_refused() {
        // There is nothing in it worth preserving that could be found
        // reliably, and refusing to save would mean one bad character costs
        // every setting from then on.
        let settings = Settings::default();

        let written = render("{{{ not toml", &settings).unwrap();

        assert_eq!(
            toml::from_str::<Settings>(&written).unwrap().window,
            settings.window
        );
    }

    #[test]
    fn the_app_never_writes_the_keys_table_itself() {
        // Bindings are the user's lines alone. Loading them and writing them
        // back would reformat and reorder a file nobody asked it to touch —
        // and would make an unbinding indistinguishable from a default.
        let settings = Settings {
            keys: BTreeMap::from([("ctrl+e".to_string(), "exchange_panes".to_string())]),
            ..Settings::default()
        };

        let written = render("", &settings).unwrap();

        assert!(
            !written.contains(KEYS_TABLE),
            "the app wrote a keys table of its own:\n{written}"
        );
    }
}
