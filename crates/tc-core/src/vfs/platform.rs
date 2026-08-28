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
//! - [`home_dir`] locates the user's home directory, [`config_dir`] the
//!   place per-user settings belong.
//! - [`attributes`] and [`render_attributes`] read and show the permission
//!   or attribute bits, which are entirely different things on the two
//!   platforms.
//! - [`set_attributes`] puts them back onto a copy.
//! - [`trash_error`] maps a `trash` failure onto [`VfsError`]. It lives here
//!   because the crate's error *shape* differs by target: the freedesktop
//!   backend wraps the underlying `io::Error`, the Windows one does not.

use std::path::{Path, PathBuf};

use super::path::VfsPath;
use super::types::{Attributes, Entry, VfsError};

#[cfg(unix)]
mod imp {
    use super::*;

    /// On Unix the VFS path *is* the native path — both are `/`-rooted.
    pub fn to_std_path(path: &VfsPath) -> PathBuf {
        PathBuf::from(path.as_str())
    }

    /// Unix permissions, as `st_mode`.
    pub fn attributes(metadata: &std::fs::Metadata) -> Attributes {
        use std::os::unix::fs::PermissionsExt;
        Attributes::from_raw(metadata.permissions().mode())
    }

    /// `rwxr-xr-x`, the form every Unix tool prints.
    pub fn render_attributes(attributes: Attributes) -> String {
        const FLAGS: [(u32, char); 9] = [
            (0o400, 'r'),
            (0o200, 'w'),
            (0o100, 'x'),
            (0o040, 'r'),
            (0o020, 'w'),
            (0o010, 'x'),
            (0o004, 'r'),
            (0o002, 'w'),
            (0o001, 'x'),
        ];
        FLAGS
            .iter()
            .map(|&(bit, letter)| {
                if attributes.raw() & bit != 0 {
                    letter
                } else {
                    '-'
                }
            })
            .collect()
    }

    pub fn set_attributes(path: &Path, attributes: Attributes) -> Result<(), VfsError> {
        use std::os::unix::fs::PermissionsExt;
        // Only the permission bits: the rest of st_mode says what kind of
        // thing this is, which a copy has already decided.
        let permissions = std::fs::Permissions::from_mode(attributes.raw() & 0o7777);
        Ok(std::fs::set_permissions(path, permissions)?)
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

    /// `$XDG_CONFIG_HOME`, or `~/.config` when it is unset — the freedesktop
    /// rule, and the one every other program on the machine follows.
    pub fn config_dir() -> Option<PathBuf> {
        if let Some(configured) = std::env::var_os("XDG_CONFIG_HOME") {
            let path = PathBuf::from(configured);
            if path.is_absolute() {
                return Some(path);
            }
        }
        home_dir().map(|home| home.join(".config"))
    }

    /// The freedesktop backend wraps the real `io::Error`, so unwrapping it
    /// keeps a failed trash in the same closed error set as every other call —
    /// a path that is already gone reports `NotFound`, not an opaque string.
    ///
    /// The arm carries the crate's own `cfg`: `Error::FileSystem` exists on
    /// freedesktop targets only, and `unix` also covers macOS.
    pub fn trash_error(err: trash::Error) -> VfsError {
        match err {
            #[cfg(not(any(target_os = "macos", target_os = "ios", target_os = "android")))]
            trash::Error::FileSystem { source, .. } => VfsError::from(source),
            other => VfsError::Io(other.to_string()),
        }
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

    /// The Win32 file attributes.
    pub fn attributes(metadata: &std::fs::Metadata) -> Attributes {
        Attributes::from_raw(metadata.file_attributes())
    }

    /// `RHSA`, the letters Total Commander shows, with a dash where a flag is
    /// absent.
    pub fn render_attributes(attributes: Attributes) -> String {
        const FLAGS: [(u32, char); 4] = [
            (0x0000_0001, 'R'),
            (FILE_ATTRIBUTE_HIDDEN, 'H'),
            (0x0000_0004, 'S'),
            (0x0000_0020, 'A'),
        ];
        FLAGS
            .iter()
            .map(|&(bit, letter)| {
                if attributes.raw() & bit != 0 {
                    letter
                } else {
                    '-'
                }
            })
            .collect()
    }

    /// Only the read-only flag, which is all `std` can set here.
    ///
    /// Hidden, system and archive would need `SetFileAttributesW`, and this
    /// project has no Win32 binding. Recorded rather than silently dropped:
    /// see `docs/future-improvements.md`.
    pub fn set_attributes(path: &Path, attributes: Attributes) -> Result<(), VfsError> {
        const FILE_ATTRIBUTE_READONLY: u32 = 0x0000_0001;
        let mut permissions = std::fs::metadata(path)?.permissions();
        permissions.set_readonly(attributes.raw() & FILE_ATTRIBUTE_READONLY != 0);
        Ok(std::fs::set_permissions(path, permissions)?)
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
                        attributes: Attributes::default(),
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

    /// `%APPDATA%`, where per-user settings belong on Windows.
    pub fn config_dir() -> Option<PathBuf> {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .or_else(|| home_dir().map(|home| home.join("AppData").join("Roaming")))
    }

    /// The Windows backend reports Win32 status codes, which this layer does
    /// not model. Hand-mapping them would be a table of guesses; the original
    /// description survives in `Io`, which is what that variant is for.
    pub fn trash_error(err: trash::Error) -> VfsError {
        VfsError::Io(err.to_string())
    }
}

pub use imp::{
    attributes, config_dir, from_std_path, home_dir, is_hidden, render_attributes, root_entries,
    set_attributes, to_std_path, trash_error,
};

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
