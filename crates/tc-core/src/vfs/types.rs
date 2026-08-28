//! Entries and errors shared by every `VirtualFs` backend.

use std::fmt;
use std::io;
use std::time::SystemTime;

/// What a directory entry points at.
///
/// Symlinks keep their own variant instead of being resolved away, so the UI
/// can render them as links while still knowing whether they are enterable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    Dir,
    File,
    Symlink(SymlinkTarget),
}

/// What a symlink resolves to.
///
/// `Broken` is a first-class outcome rather than an error: a dangling link is
/// a normal thing to find in a directory, and a file manager that refuses to
/// list the directory because of one is useless.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymlinkTarget {
    Dir,
    File,
    Broken,
}

/// The platform's permission or attribute bits for one entry.
///
/// An opaque `Copy` value rather than a rendered string: the pane renders it
/// for the Attr column and the copy engine restores it onto the copy, and
/// both need the same thing. What the bits *mean* is the platform's business
/// — Unix mode bits, Windows file attributes — so only `vfs::platform` reads
/// them.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Attributes(pub(super) u32);

impl Attributes {
    pub(super) fn from_raw(raw: u32) -> Self {
        Attributes(raw)
    }

    pub(super) fn raw(self) -> u32 {
        self.0
    }
}

/// One entry in a directory listing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Final path component. Never contains a separator.
    pub name: String,
    pub kind: EntryKind,
    /// Size in bytes. Directories report 0 — see [`Entry::is_dir`].
    pub size: u64,
    pub modified: SystemTime,
    /// Permissions on Unix, file attributes on Windows.
    ///
    /// Carried on the entry because a copy has to restore them: an executable
    /// that arrives without its `+x` is a broken copy, which makes this a
    /// reliability concern rather than a column
    /// (`docs/reliability.md`).
    pub attributes: Attributes,
    /// Whether the platform considers this entry hidden.
    ///
    /// Decided by the backend, not by the listing layer: on Windows this is a
    /// file attribute that cannot be derived from the name, while on Unix it
    /// is the leading dot. Filtering on the flag stays platform-agnostic.
    pub hidden: bool,
}

impl Entry {
    /// Whether the entry can be descended into, symlinks to directories
    /// included. This is the predicate navigation asks, so it lives here
    /// rather than being re-derived at every call site.
    pub fn is_dir(&self) -> bool {
        matches!(
            self.kind,
            EntryKind::Dir | EntryKind::Symlink(SymlinkTarget::Dir)
        )
    }
}

/// Why a VFS operation failed.
///
/// Carries no `io::Error`: the variants must stay comparable so tests can
/// assert on them, and the UI needs a closed set of cases to render, not an
/// open-ended platform error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VfsError {
    NotFound,
    PermissionDenied,
    NotADirectory,
    /// The target of a create already exists. The operation engine turns this
    /// into a conflict prompt rather than an error.
    AlreadyExists,
    /// A directory that had to be empty was not. Recursion is the engine's
    /// job, so the backend reports this instead of deleting the contents.
    NotEmpty,
    /// A file operation was aimed at a directory.
    IsADirectory,
    /// A rename would have crossed filesystems, which no filesystem can do
    /// atomically. The engine answers with copy + delete.
    CrossDevice,
    /// Anything the layer does not model explicitly, with the original message
    /// preserved for the job log.
    Io(String),
}

impl From<io::Error> for VfsError {
    fn from(err: io::Error) -> Self {
        match err.kind() {
            io::ErrorKind::NotFound => VfsError::NotFound,
            io::ErrorKind::PermissionDenied => VfsError::PermissionDenied,
            io::ErrorKind::NotADirectory => VfsError::NotADirectory,
            io::ErrorKind::AlreadyExists => VfsError::AlreadyExists,
            io::ErrorKind::DirectoryNotEmpty => VfsError::NotEmpty,
            io::ErrorKind::IsADirectory => VfsError::IsADirectory,
            io::ErrorKind::CrossesDevices => VfsError::CrossDevice,
            _ => VfsError::Io(err.to_string()),
        }
    }
}

impl fmt::Display for VfsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VfsError::NotFound => write!(f, "no such file or directory"),
            VfsError::PermissionDenied => write!(f, "permission denied"),
            VfsError::NotADirectory => write!(f, "not a directory"),
            VfsError::AlreadyExists => write!(f, "already exists"),
            VfsError::NotEmpty => write!(f, "directory is not empty"),
            VfsError::IsADirectory => write!(f, "is a directory"),
            VfsError::CrossDevice => write!(f, "on a different filesystem"),
            VfsError::Io(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for VfsError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(kind: EntryKind) -> Entry {
        Entry {
            name: "x".to_string(),
            kind,
            size: 0,
            modified: SystemTime::UNIX_EPOCH,
            attributes: Attributes::default(),
            hidden: false,
        }
    }

    #[test]
    fn directories_and_symlinks_to_directories_are_enterable() {
        assert!(entry(EntryKind::Dir).is_dir());
        assert!(entry(EntryKind::Symlink(SymlinkTarget::Dir)).is_dir());
    }

    #[test]
    fn files_and_broken_symlinks_are_not_enterable() {
        assert!(!entry(EntryKind::File).is_dir());
        assert!(!entry(EntryKind::Symlink(SymlinkTarget::File)).is_dir());
        assert!(!entry(EntryKind::Symlink(SymlinkTarget::Broken)).is_dir());
    }

    #[test]
    fn io_error_kinds_map_onto_modelled_variants() {
        let cases = [
            (io::ErrorKind::NotFound, VfsError::NotFound),
            (io::ErrorKind::PermissionDenied, VfsError::PermissionDenied),
            (io::ErrorKind::NotADirectory, VfsError::NotADirectory),
            (io::ErrorKind::AlreadyExists, VfsError::AlreadyExists),
            (io::ErrorKind::DirectoryNotEmpty, VfsError::NotEmpty),
            (io::ErrorKind::IsADirectory, VfsError::IsADirectory),
            (io::ErrorKind::CrossesDevices, VfsError::CrossDevice),
        ];
        for (kind, expected) in cases {
            assert_eq!(VfsError::from(io::Error::from(kind)), expected);
        }
    }

    #[test]
    fn unmodelled_io_errors_keep_their_message() {
        let err = io::Error::new(io::ErrorKind::OutOfMemory, "no room");
        assert_eq!(VfsError::from(err), VfsError::Io("no room".to_string()));
    }
}
