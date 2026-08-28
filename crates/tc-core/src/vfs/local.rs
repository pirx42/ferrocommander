//! The local filesystem as a [`VirtualFs`].

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::constants::ROOT;
use super::path::VfsPath;
use super::platform;
use super::types::{Attributes, Entry, EntryKind, SymlinkTarget, VfsError};
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
            // `DirEntry::metadata` stats relative to the directory that is
            // already open, so it neither walks the whole path again nor
            // builds one: 50 000 entries cost 56 ms rather than 72 ms. Like
            // `symlink_metadata`, it does not follow a link. The path is
            // built only for the entries that turn out to be one.
            let link_metadata = match item.metadata() {
                Ok(metadata) => metadata,
                // The entry was there when the directory was enumerated and
                // is not there now. Leaving it out is the truth; failing the
                // whole listing over one file that a job elsewhere deleted
                // half a millisecond ago is not.
                Err(error) if vanished(&error) => continue,
                Err(error) => return Err(error.into()),
            };
            entries.push(entry_from(name, link_metadata, || item.path())?);
        }
        // Deliberately unsorted — ordering is the listing layer's decision.
        Ok(entries)
    }

    fn create_dir(&self, path: &VfsPath) -> Result<(), VfsError> {
        Ok(fs::create_dir(platform::to_std_path(path))?)
    }

    fn remove_dir(&self, path: &VfsPath) -> Result<(), VfsError> {
        Ok(fs::remove_dir(platform::to_std_path(path))?)
    }

    fn remove_file(&self, path: &VfsPath) -> Result<(), VfsError> {
        Ok(fs::remove_file(platform::to_std_path(path))?)
    }

    fn rename(&self, from: &VfsPath, to: &VfsPath) -> Result<(), VfsError> {
        Ok(fs::rename(
            platform::to_std_path(from),
            platform::to_std_path(to),
        )?)
    }

    fn open_read(&self, path: &VfsPath) -> Result<Box<dyn Read + Send>, VfsError> {
        Ok(Box::new(fs::File::open(platform::to_std_path(path))?))
    }

    fn create_file(&self, path: &VfsPath) -> Result<Box<dyn Write + Send>, VfsError> {
        Ok(Box::new(fs::File::create(platform::to_std_path(path))?))
    }

    fn set_attributes(&self, path: &VfsPath, attributes: Attributes) -> Result<(), VfsError> {
        platform::set_attributes(&platform::to_std_path(path), attributes)
    }

    fn set_modified(&self, path: &VfsPath, time: SystemTime) -> Result<(), VfsError> {
        // Write access is what the platform demands to stamp a file, which is
        // also why this cannot serve directories.
        let file = fs::File::options()
            .write(true)
            .open(platform::to_std_path(path))?;
        Ok(file.set_modified(time)?)
    }

    fn trash(&self, path: &VfsPath) -> Result<(), VfsError> {
        trash::delete(platform::to_std_path(path)).map_err(platform::trash_error)
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
                attributes: Attributes::default(),
                hidden: false,
            });
        };
        entry_at(&platform::to_std_path(path), name.to_string())
    }
}

/// Whether a per-entry failure means the entry disappeared mid-listing.
///
/// The distinction that matters: a vanished entry is dropped, while anything
/// else — a permission problem, an I/O error — still fails the listing,
/// because silently returning a short directory would be a lie about what is
/// there. Separated out so the rule has a test; provoking the race itself
/// would need a seam the standard library does not offer.
fn vanished(error: &std::io::Error) -> bool {
    error.kind() == std::io::ErrorKind::NotFound
}

/// Builds an [`Entry`] for one native path.
///
/// Symlinks are described by their target's kind and size — that is what the
/// user wants to see — while the link's own metadata decides the hidden flag,
/// since it is the link that appears in this directory.
fn entry_at(path: &Path, name: String) -> Result<Entry, VfsError> {
    let link_metadata = fs::symlink_metadata(path)?;
    entry_from(name, link_metadata, || path.to_path_buf())
}

/// Builds an [`Entry`] from metadata that has already been read.
///
/// `target_path` is called only for a symlink, whose target has to be stat'ed
/// separately — every other entry never pays for the path at all.
fn entry_from(
    name: String,
    link_metadata: fs::Metadata,
    target_path: impl FnOnce() -> PathBuf,
) -> Result<Entry, VfsError> {
    let hidden = platform::is_hidden(&name, &link_metadata);
    let attributes = platform::attributes(&link_metadata);

    if link_metadata.file_type().is_symlink() {
        let (kind, size, modified) = match fs::metadata(target_path()) {
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
            attributes,
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
        attributes,
        hidden,
    })
}

#[cfg(test)]
mod tests {
    use std::io;

    use super::*;

    #[test]
    fn only_a_missing_entry_is_dropped_from_a_listing() {
        assert!(vanished(&io::Error::from(io::ErrorKind::NotFound)));
        // Everything else must still fail the listing rather than quietly
        // shorten it.
        for kind in [
            io::ErrorKind::PermissionDenied,
            io::ErrorKind::NotADirectory,
            io::ErrorKind::OutOfMemory,
        ] {
            assert!(!vanished(&io::Error::from(kind)), "{kind:?}");
        }
    }
}
