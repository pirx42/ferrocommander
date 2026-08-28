//! Everything about the local filesystem that differs between Linux and
//! Windows, collected in one module.
//!
//! Keeping the `cfg` split here rather than sprinkling it through `local.rs`
//! means the platform contract is a short, readable list — three functions —
//! and adding a platform touches exactly one file.
//!
//! The contract:
//! - [`to_std_path`] maps a [`VfsPath`] onto a native path.
//! - [`is_hidden`] decides whether an entry is hidden, per platform
//!   convention. This cannot be left to the listing layer: on Windows
//!   "hidden" is a file attribute that only the filesystem knows, not
//!   something derivable from the name.
//! - [`root_entries`] answers what the VFS root contains when the platform
//!   has no single filesystem root.
//! - [`from_std_path`] is the inverse of [`to_std_path`].
//! - [`home_dir`] locates the user's home directory.

use std::path::{Path, PathBuf};

use super::path::VfsPath;
use super::types::Entry;

#[cfg(unix)]
mod imp {
    use super::*;

    /// On Unix the VFS path *is* the native path — both are `/`-rooted.
    pub fn to_std_path(path: &VfsPath) -> PathBuf {
        PathBuf::from(path.as_str())
    }

    /// Unix convention: a leading dot hides the entry.
    pub fn is_hidden(name: &str, _metadata: &std::fs::Metadata) -> bool {
        name.starts_with('.')
    }

    /// Unix has a real root directory, so the generic code path handles it.
    pub fn root_entries() -> Option<Vec<Entry>> {
        None
    }

    /// Native and VFS paths coincide on Unix.
    pub fn from_std_path(path: &Path) -> VfsPath {
        VfsPath::new(&path.to_string_lossy())
    }

    pub fn home_dir() -> Option<PathBuf> {
        std::env::var_os("HOME").map(PathBuf::from)
    }
}

#[cfg(windows)]
mod imp {
    use std::os::windows::fs::MetadataExt;
    use std::time::SystemTime;

    use super::super::constants::SEPARATOR;
    use super::super::types::EntryKind;
    use super::*;

    /// `FILE_ATTRIBUTE_HIDDEN` from the Win32 API.
    const FILE_ATTRIBUTE_HIDDEN: u32 = 0x0000_0002;

    /// Drive letters probed when enumerating the synthetic root.
    const DRIVE_LETTERS: std::ops::RangeInclusive<char> = 'A'..='Z';

    /// Maps `/C:/Users/pirx` onto `C:\Users\pirx`.
    ///
    /// The first VFS component is the drive; `root_entries` guarantees the
    /// root itself is never routed here, so there is always a drive to take.
    pub fn to_std_path(path: &VfsPath) -> PathBuf {
        let mut components = path.components();
        let drive = components.next().unwrap_or_default();
        let mut native = format!("{drive}{}", std::path::MAIN_SEPARATOR);
        for component in components {
            native.push_str(component);
            native.push(std::path::MAIN_SEPARATOR);
        }
        PathBuf::from(native.trim_end_matches(std::path::MAIN_SEPARATOR))
    }

    /// Windows convention: hidden is an attribute, not a naming rule. A
    /// dot-prefixed name such as `.gitignore` is a normal visible file here.
    pub fn is_hidden(_name: &str, metadata: &std::fs::Metadata) -> bool {
        metadata.file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0
    }

    /// Windows has no single root, so the VFS root is the list of drives —
    /// which is exactly the drive selector the UI needs anyway.
    pub fn root_entries() -> Option<Vec<Entry>> {
        let drives = DRIVE_LETTERS
            .filter_map(|letter| {
                let name = format!("{letter}:");
                std::fs::metadata(format!("{name}{SEPARATOR}"))
                    .ok()
                    .map(|_| Entry {
                        name,
                        kind: EntryKind::Dir,
                        size: 0,
                        modified: SystemTime::UNIX_EPOCH,
                        hidden: false,
                    })
            })
            .collect();
        Some(drives)
    }

    /// `C:\\Users\\pirx` becomes `/C:/Users/pirx`.
    pub fn from_std_path(path: &Path) -> VfsPath {
        VfsPath::new(
            &path
                .to_string_lossy()
                .replace(std::path::MAIN_SEPARATOR, "/"),
        )
    }

    pub fn home_dir() -> Option<PathBuf> {
        std::env::var_os("USERPROFILE").map(PathBuf::from)
    }
}

pub use imp::{from_std_path, home_dir, is_hidden, root_entries, to_std_path};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_maps_onto_a_native_path_exactly_when_the_platform_has_one() {
        match root_entries() {
            // Unix: the generic path handles the root, so it must map.
            None => assert_eq!(to_std_path(&VfsPath::root()), PathBuf::from("/")),
            // Windows: the root is synthetic and lists drives instead.
            Some(drives) => assert!(drives.iter().all(|drive| drive.name.ends_with(':'))),
        }
    }

    #[test]
    fn a_native_path_survives_the_round_trip_through_the_vfs_path_space() {
        // The listing tests and the UI both map native paths into the VFS
        // path space; if that mapping is not an inverse, a pane opens
        // somewhere other than where it was told to.
        let native = home_dir().expect("a home directory");
        assert_eq!(to_std_path(&from_std_path(&native)), native);
    }

    #[cfg(unix)]
    #[test]
    fn unix_hides_dot_prefixed_names() {
        let metadata = std::fs::metadata(".").expect("cwd is readable");
        assert!(is_hidden(".config", &metadata));
        assert!(!is_hidden("config", &metadata));
    }

    #[cfg(windows)]
    #[test]
    fn windows_maps_the_first_component_onto_a_drive() {
        assert_eq!(
            to_std_path(&VfsPath::new("/C:/Users/pirx")),
            PathBuf::from("C:\\Users\\pirx")
        );
        assert_eq!(to_std_path(&VfsPath::new("/C:")), PathBuf::from("C:"));
    }
}
