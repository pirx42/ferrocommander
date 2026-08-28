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
    /// One per pane, in pane order. A file with the wrong number of them is
    /// padded or trimmed on load rather than rejected — see [`Settings::pane`].
    pub panes: Vec<PaneSettings>,
    /// Which pane had the keyboard.
    pub active_pane: usize,
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WindowSettings {
    pub width: i32,
    pub height: i32,
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

impl Default for WindowSettings {
    fn default() -> Self {
        WindowSettings {
            width: DEFAULT_WINDOW_WIDTH,
            height: DEFAULT_WINDOW_HEIGHT,
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
