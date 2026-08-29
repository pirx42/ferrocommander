//! Archives as browsable directories.
//!
//! This is the module the whole two-crate split was for. A pane holds an
//! `Arc<dyn VirtualFs>`; entering an archive swaps in one of these, and the
//! listing, the viewer and the copy engine carry on without knowing. What that
//! claim cost in lines elsewhere is written down in `docs/archives.md`.
//!
//! **Read-only, on purpose.** `VirtualFs` promises a `create_file` that can be
//! called on any path in any order, and a zip is a sequential stream with its
//! index at the end. Honouring that promise would mean rewriting the container
//! per call — correct, useless, and slow in exactly the way the prime
//! directive forbids. Packing is therefore its own job with its own writer,
//! and every mutating call here reports [`VfsError::ReadOnly`].

pub mod constants;
mod entry;
mod index;
mod reader;
mod zip;

use std::io::{Read, Write};
use std::sync::Arc;
use std::time::SystemTime;

use crate::vfs::{Attributes, Entry, VfsError, VfsPath, VirtualFs};

use constants::ZIP_EXTENSION;
use index::Index;
use reader::Container;

/// The archive formats this build can read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Zip,
}

/// Which format a file name announces, if any.
///
/// By extension, lower-cased. Sniffing the first bytes would be more honest
/// about a mislabelled file and would cost a read of every row in every
/// listing — the pane asks this question about every entry it draws, so it has
/// to be free (`docs/performance.md`).
pub fn format_for(name: &str) -> Option<Format> {
    let extension = name.rsplit_once('.')?.1.to_ascii_lowercase();
    (extension == ZIP_EXTENSION).then_some(Format::Zip)
}

/// A read-only [`VirtualFs`] over one archive.
pub struct ArchiveFs {
    container: Container,
    index: Index,
}

impl ArchiveFs {
    /// Reads `path`'s directory out of `fs` and opens it as a filesystem.
    ///
    /// The container is read **through** `fs` rather than as an operating
    /// system file, so an archive on any backend works — including, later, one
    /// inside another archive.
    pub fn open(fs: Arc<dyn VirtualFs>, path: &VfsPath) -> Result<ArchiveFs, VfsError> {
        let stat = fs.stat(path)?;
        let format = format_for(&stat.name).ok_or(VfsError::NotAnArchive)?;
        let container = Container::new(fs, path.clone(), stat.size);
        let index = match format {
            Format::Zip => zip::index(&container, stat.modified)?,
        };
        Ok(ArchiveFs { container, index })
    }

    fn node(&self, path: &VfsPath) -> Result<&index::Node, VfsError> {
        self.index.node(path).ok_or(VfsError::NotFound)
    }
}

impl VirtualFs for ArchiveFs {
    fn read_dir(&self, path: &VfsPath) -> Result<Vec<Entry>, VfsError> {
        // Missing and not-a-directory are told apart, because the listing
        // layer shows them differently and "not found" for a file that is
        // plainly there reads as a bug in the archive reader.
        self.index.read_dir(path).ok_or_else(|| {
            if self.index.node(path).is_some() {
                VfsError::NotADirectory
            } else {
                VfsError::NotFound
            }
        })
    }

    fn stat(&self, path: &VfsPath) -> Result<Entry, VfsError> {
        Ok(self.node(path)?.entry.clone())
    }

    fn open_read(&self, path: &VfsPath) -> Result<Box<dyn Read + Send>, VfsError> {
        let node = self.node(path)?;
        let bytes = node.bytes.as_ref().ok_or(VfsError::IsADirectory)?;
        entry::open(&self.container, &node.entry.name, bytes)
    }

    /// Reads a window out of an entry.
    ///
    /// A stored entry is read straight out of the container, which is what
    /// makes paging the viewer through a large uncompressed member as fast as
    /// paging a file. A compressed one has to be decoded from its start, so
    /// this is O(offset) — the honest cost of a format with no seek, written
    /// down in `docs/performance.md` rather than hidden behind a cache that
    /// would only move it.
    fn read_at(&self, path: &VfsPath, offset: u64, len: usize) -> Result<Vec<u8>, VfsError> {
        let node = self.node(path)?;
        let bytes = node.bytes.as_ref().ok_or(VfsError::IsADirectory)?;
        if bytes.method == index::Method::Stored {
            let start = bytes.start.saturating_add(offset.min(bytes.stored));
            let len = len.min(bytes.stored.saturating_sub(offset) as usize);
            return self.container.read_at(start, len);
        }

        let mut reader = entry::open(&self.container, &node.entry.name, bytes)?;
        skip(&mut reader, offset)?;
        let mut window = vec![0u8; len];
        let filled = fill(&mut reader, &mut window)?;
        window.truncate(filled);
        Ok(window)
    }

    fn create_dir(&self, _path: &VfsPath) -> Result<(), VfsError> {
        Err(VfsError::ReadOnly)
    }

    fn remove_dir(&self, _path: &VfsPath) -> Result<(), VfsError> {
        Err(VfsError::ReadOnly)
    }

    fn remove_file(&self, _path: &VfsPath) -> Result<(), VfsError> {
        Err(VfsError::ReadOnly)
    }

    fn rename(&self, _from: &VfsPath, _to: &VfsPath) -> Result<(), VfsError> {
        Err(VfsError::ReadOnly)
    }

    fn create_file(&self, _path: &VfsPath) -> Result<Box<dyn Write + Send>, VfsError> {
        Err(VfsError::ReadOnly)
    }

    fn set_modified(&self, _path: &VfsPath, _time: SystemTime) -> Result<(), VfsError> {
        Err(VfsError::ReadOnly)
    }

    fn set_attributes(&self, _path: &VfsPath, _attributes: Attributes) -> Result<(), VfsError> {
        Err(VfsError::ReadOnly)
    }

    /// An archive has no trash, as the trait's own documentation predicted.
    fn trash(&self, _path: &VfsPath) -> Result<(), VfsError> {
        Err(VfsError::ReadOnly)
    }
}

/// Throws away `count` bytes. The only way past a decoder that cannot seek.
fn skip(reader: &mut Box<dyn Read + Send>, count: u64) -> Result<(), VfsError> {
    let mut sink = vec![0u8; constants::CONTAINER_BUFFER];
    let mut left = count;
    while left > 0 {
        let want = sink.len().min(left as usize);
        let read = reader.read(&mut sink[..want]).map_err(VfsError::from)?;
        if read == 0 {
            return Ok(());
        }
        left -= read as u64;
    }
    Ok(())
}

/// Reads until the buffer is full or the entry ends, and reports how much
/// arrived. A single `read` is allowed to come back short and a window that
/// stopped at the first short read would look like the end of the file.
fn fill(reader: &mut Box<dyn Read + Send>, buffer: &mut [u8]) -> Result<usize, VfsError> {
    let mut filled = 0;
    while filled < buffer.len() {
        let read = reader.read(&mut buffer[filled..]).map_err(VfsError::from)?;
        if read == 0 {
            break;
        }
        filled += read;
    }
    Ok(filled)
}
