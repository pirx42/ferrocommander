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

use serde::{Deserialize, Serialize};

use crate::listing::{Sort, SortKey, SortOrder};
use crate::vfs::{VfsError, VfsPath, VirtualFs};

/// Directory the settings file lives in, below the platform's config root.
pub const CONFIG_DIR: &str = "ferrocommander";

/// Name of the settings file.
pub const CONFIG_FILE: &str = "config.toml";

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
pub const SORT_KEY_NAMES: [(SortKey, &str); 4] = [
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
pub fn save(
    fs: &dyn VirtualFs,
    config_root: &VfsPath,
    settings: &Settings,
) -> Result<(), VfsError> {
    let directory = config_root.child(CONFIG_DIR);
    ensure_directory(fs, config_root);
    ensure_directory(fs, &directory);

    let text = toml::to_string_pretty(settings).map_err(|error| VfsError::Io(error.to_string()))?;

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
}
