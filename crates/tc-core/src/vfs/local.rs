//! The local filesystem as a [`VirtualFs`].

use std::fs;
use std::path::Path;
use std::time::SystemTime;

use super::constants::ROOT;
use super::path::VfsPath;
use super::platform;
use super::types::{Entry, EntryKind, SymlinkTarget, VfsError};
use super::VirtualFs;

/// Size reported for directories.
///
/// A directory's on-disk inode size says nothing a user cares about and would
/// turn size-sorting into noise, so directories report zero and the UI renders
/// them as `<DIR>`.
const DIR_SIZE: u64 = 0;

/// Reads the real filesystem through the VFS interface.
pub struct LocalFs;

impl LocalFs {
    /// Maps a native path into the VFS path space.
    ///
    /// The inverse of what the backend does internally, exposed because
    /// callers outside the VFS — the UI's start directory, tests working with
    /// tempdirs — hold native paths and must not hand-roll the mapping.
    pub fn vfs_path(native: &Path) -> VfsPath {
        platform::from_std_path(native)
    }

    /// The user's home directory, where panes start.
    pub fn home_dir() -> Option<VfsPath> {
        platform::home_dir().map(|native| Self::vfs_path(&native))
    }
}

impl VirtualFs for LocalFs {
    fn read_dir(&self, path: &VfsPath) -> Result<Vec<Entry>, VfsError> {
        if path.is_root() {
            if let Some(entries) = platform::root_entries() {
                return Ok(entries);
            }
        }

        let dir = platform::to_std_path(path);
        let mut entries = Vec::new();
        for item in fs::read_dir(dir)? {
            let item = item?;
            // Lossy is right here: a name the OS cannot render as UTF-8 must
            // still appear in the pane rather than fail the whole listing.
            let name = item.file_name().to_string_lossy().into_owned();
            entries.push(entry_at(&item.path(), name)?);
        }
        // Deliberately unsorted — ordering is the listing layer's decision.
        Ok(entries)
    }

    fn stat(&self, path: &VfsPath) -> Result<Entry, VfsError> {
        // The root is synthesized rather than stat'ed: on Windows it is the
        // drive list, which has no metadata of its own.
        let Some(name) = path.file_name() else {
            return Ok(Entry {
                name: ROOT.to_string(),
                kind: EntryKind::Dir,
                size: DIR_SIZE,
                modified: SystemTime::UNIX_EPOCH,
                hidden: false,
            });
        };
        entry_at(&platform::to_std_path(path), name.to_string())
    }
}

/// Builds an [`Entry`] for one native path.
///
/// Symlinks are described by their target's kind and size — that is what the
/// user wants to see — while the link's own metadata decides the hidden flag,
/// since it is the link that appears in this directory.
fn entry_at(path: &Path, name: String) -> Result<Entry, VfsError> {
    let link_metadata = fs::symlink_metadata(path)?;
    let hidden = platform::is_hidden(&name, &link_metadata);

    if link_metadata.file_type().is_symlink() {
        let (kind, size, modified) = match fs::metadata(path) {
            Ok(target) if target.is_dir() => (SymlinkTarget::Dir, DIR_SIZE, target.modified()?),
            Ok(target) => (SymlinkTarget::File, target.len(), target.modified()?),
            // A dangling link is a normal directory inhabitant, not a failure.
            Err(_) => (SymlinkTarget::Broken, 0, link_metadata.modified()?),
        };
        return Ok(Entry {
            name,
            kind: EntryKind::Symlink(kind),
            size,
            modified,
            hidden,
        });
    }

    let (kind, size) = if link_metadata.is_dir() {
        (EntryKind::Dir, DIR_SIZE)
    } else {
        (EntryKind::File, link_metadata.len())
    };
    Ok(Entry {
        name,
        kind,
        size,
        modified: link_metadata.modified()?,
        hidden,
    })
}
