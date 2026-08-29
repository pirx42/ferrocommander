//! Virtual filesystems: one interface over local directories and, from
//! phase 6 on, archives.
//!
//! Everything in this project that touches a filesystem goes through here.
//! The read half arrived in phase 1; the mutating half arrived in phase 2
//! together with the operation engine that uses and tests it.

pub mod constants;
mod local;
mod path;
mod platform;
mod types;

use std::io::{Read, Write};
use std::time::SystemTime;

pub use local::LocalFs;
pub use path::VfsPath;
pub use types::{Attributes, Entry, EntryKind, Mount, Store, SymlinkTarget, VfsError};

/// A [`VfsPath`] as the operating system spells it.
///
/// The one place a path leaves this layer, and it exists for the command
/// line: spawning a process needs a working directory the OS understands, and
/// `platform` stays private so that conversion cannot be reinvented per call
/// site with its own idea of what a separator is.
pub fn to_std_path(path: &VfsPath) -> std::path::PathBuf {
    platform::to_std_path(path)
}

/// The places the drive bar offers: mounted filesystems on Unix, drives on
/// Windows.
pub fn mount_points() -> Vec<Mount> {
    platform::mount_points()
}

/// Which mount `path` sits on: the longest one it is at or inside.
///
/// Longest, because mounts nest — `/mnt/backup` is inside `/`, and a file
/// under it belongs to the backup drive rather than to the root. Taking the
/// first match instead would put everything on `/` on Unix, where `/` is a
/// prefix of every path there is.
///
/// The mounts are passed in rather than read here, so the rule can be tested
/// against a made-up machine instead of whatever the test host happens to
/// have mounted.
pub fn mount_for(path: &VfsPath, mounts: &[Mount]) -> Option<VfsPath> {
    mounts
        .iter()
        .filter(|mount| *path == mount.path || path.is_inside(&mount.path))
        .max_by_key(|mount| mount.path.as_str().len())
        .map(|mount| mount.path.clone())
}

/// The attributes an archive's recorded Unix mode amounts to on this platform.
///
/// Unix keeps the bits; Windows has nowhere to put them and reports none. Here
/// rather than in the archive layer because what an `Attributes` *means* is
/// this module's business, and an archive that reached into the raw value
/// would be the second place that decides — which is how the two drift apart.
pub fn attributes_from_unix_mode(mode: u32) -> Attributes {
    platform::attributes_from_unix_mode(mode)
}

/// The Unix mode an archive should record for `attributes`, if any.
///
/// The inverse of [`attributes_from_unix_mode`], and the same reasoning: what
/// the bits *mean* is this module's business, so the archive layer asks rather
/// than reading the raw value itself.
pub fn unix_mode_of(attributes: Attributes) -> Option<u32> {
    platform::unix_mode_of(attributes)
}

/// The attributes of an entry, written the way the platform writes them:
/// `rwxr-xr-x` on Unix, `RHSA` on Windows.
pub fn render_attributes(attributes: Attributes) -> String {
    platform::render_attributes(attributes)
}

/// A browsable, writable filesystem.
///
/// Object-safe on purpose: a pane holds a `dyn VirtualFs` and swaps it when
/// the user steps into an archive, without the UI knowing which backend it is
/// talking to.
///
/// `Send + Sync` because a file operation runs on a worker thread and holds
/// its source and target backends across it. That is a bound rather than a
/// method, so it costs nothing today — `LocalFs` is a unit struct — while
/// phase 6's `ArchiveFs`, which owns an open archive handle, now knows up
/// front that it needs interior mutability instead of discovering it half
/// written.
pub trait VirtualFs: Send + Sync {
    /// Which storage this backend addresses — see [`Store`].
    ///
    /// Asked before anything hands one backend's path to another: the answer
    /// decides whether the two are talking about the same files at all.
    fn store(&self) -> Store;

    /// Whether nothing here can be written to at all.
    ///
    /// A property of the backend rather than of a path, which is why it can be
    /// answered once instead of discovered per file. The operation engine asks
    /// before it scans, so a copy into an archive is one refusal with a reason
    /// rather than a list of a thousand identical ones.
    ///
    /// A default, because a backend that *can* be written to has nothing to
    /// say here and the one that cannot is the exception.
    fn read_only(&self) -> bool {
        false
    }

    /// Lists a directory. The order is unspecified — sorting belongs to the
    /// listing layer.
    fn read_dir(&self, path: &VfsPath) -> Result<Vec<Entry>, VfsError>;

    /// Describes a single path.
    fn stat(&self, path: &VfsPath) -> Result<Entry, VfsError>;

    /// Creates one directory. The parent must exist — creating a chain is the
    /// engine's walk, which is the only place that can report progress for it.
    fn create_dir(&self, path: &VfsPath) -> Result<(), VfsError>;

    /// Removes an **empty** directory, reporting [`VfsError::NotEmpty`]
    /// otherwise.
    ///
    /// Non-recursive on purpose. Only the operation engine can report
    /// per-file progress, log a per-file error and carry on, and honour a
    /// cancel between two entries; a recursive backend delete would be a
    /// second, silent implementation of the same walk with none of that.
    fn remove_dir(&self, path: &VfsPath) -> Result<(), VfsError>;

    /// Removes one file.
    fn remove_file(&self, path: &VfsPath) -> Result<(), VfsError>;

    /// Moves a path within this backend.
    ///
    /// Reports [`VfsError::CrossDevice`] rather than falling back to a copy:
    /// that fallback is a decision with a progress bar and a rollback
    /// attached, which makes it engine policy, not backend behavior.
    fn rename(&self, from: &VfsPath, to: &VfsPath) -> Result<(), VfsError>;

    /// Opens a file for reading.
    fn open_read(&self, path: &VfsPath) -> Result<Box<dyn Read + Send>, VfsError>;

    /// Reads at most `len` bytes starting `offset` into a file.
    ///
    /// Random access rather than a seekable reader, because a seekable reader
    /// is a promise not every backend can keep: an entry inside a compressed
    /// archive has no cheap seek, and a trait method some backends must fake
    /// is worse than one they implement honestly and slowly.
    ///
    /// What the viewer is built on. It never holds a file — it holds an
    /// offset, reads a window around it, and moves the offset — which is the
    /// only way a four-gigabyte file opens instantly (`docs/performance.md`).
    ///
    /// **A read past the end comes back short**, and one entirely past the end
    /// comes back empty. Not an error: that is what every caller wants at the
    /// end of a file, and the alternative is each of them clamping against a
    /// size that may have changed since they asked.
    fn read_at(&self, path: &VfsPath, offset: u64, len: usize) -> Result<Vec<u8>, VfsError>;

    /// Creates a file for writing, **truncating** an existing one.
    ///
    /// Deciding whether overwriting is allowed happens before this call — the
    /// engine has to ask the user anyway, and a second existence check here
    /// would be a second answer to the same question.
    fn create_file(&self, path: &VfsPath) -> Result<Box<dyn Write + Send>, VfsError>;

    /// Stamps a **file** with a modification time, so a copy keeps the
    /// original's date. Directories are not supported: stamping one needs a
    /// writable handle to it, which no platform hands out.
    fn set_modified(&self, path: &VfsPath, time: SystemTime) -> Result<(), VfsError>;

    /// Puts `attributes` back onto a path, so a copy keeps the original's
    /// permissions.
    ///
    /// Called after [`VirtualFs::set_modified`], because removing write
    /// permission first would stop the timestamp being set at all.
    fn set_attributes(&self, path: &VfsPath, attributes: Attributes) -> Result<(), VfsError>;

    /// Moves a path to the platform's trash, from where the user can undo it.
    ///
    /// A backend capability rather than an engine step: only the backend
    /// knows whether its storage has such a thing. Phase 6's archives will
    /// not.
    fn trash(&self, path: &VfsPath) -> Result<(), VfsError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A machine with nested mounts, which is the only case where the rule
    /// has anything to decide.
    fn mounts() -> Vec<Mount> {
        ["/", "/mnt/backup", "/mnt/backup/old"]
            .iter()
            .map(|path| Mount {
                path: VfsPath::new(path),
                label: path.to_string(),
            })
            .collect()
    }

    #[test]
    fn a_path_belongs_to_the_longest_mount_it_is_under() {
        // Taking the first match instead would put everything on `/`, which is
        // a prefix of every path on Unix there is.
        for (path, expected) in [
            ("/home/pirx", "/"),
            ("/mnt/backup", "/mnt/backup"),
            ("/mnt/backup/2026", "/mnt/backup"),
            ("/mnt/backup/old", "/mnt/backup/old"),
            ("/mnt/backup/old/2019", "/mnt/backup/old"),
        ] {
            assert_eq!(
                mount_for(&VfsPath::new(path), &mounts()),
                Some(VfsPath::new(expected)),
                "{path}"
            );
        }
    }

    #[test]
    fn a_mount_owns_itself() {
        // The pane standing *on* a drive's root is the ordinary case, and a
        // rule that only matched things strictly below would miss it.
        assert_eq!(
            mount_for(&VfsPath::new("/mnt/backup"), &mounts()),
            Some(VfsPath::new("/mnt/backup"))
        );
    }

    #[test]
    fn a_neighbour_sharing_a_name_prefix_is_not_inside() {
        // Whole components, not text: `/mnt/backup2` is not on the backup
        // drive, and reading it as one would file its directory under the
        // wrong mount and send the user somewhere else entirely.
        assert_eq!(
            mount_for(&VfsPath::new("/mnt/backup2/2026"), &mounts()),
            Some(VfsPath::new("/"))
        );
    }

    #[test]
    fn a_path_on_no_mount_at_all_belongs_nowhere() {
        // Windows, where a path on an unlisted drive has no mount to be filed
        // under, and inventing one would record it against the wrong disk.
        let mounts = [Mount {
            path: VfsPath::new("/mnt/backup"),
            label: "backup".to_string(),
        }];
        assert_eq!(mount_for(&VfsPath::new("/home/pirx"), &mounts), None);
    }
}
