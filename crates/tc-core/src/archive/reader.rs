//! Reading bytes out of a container that is itself only a [`VirtualFs`] path.
//!
//! An archive is never opened as an operating-system file here. It is read
//! through the backend that holds it, which is what makes an archive inside an
//! archive an ordinary case rather than a special one, and what lets the whole
//! layer be tested against a backend that only exists in a test.

use std::io::{self, Read, Seek, SeekFrom};
use std::sync::Arc;

use crate::vfs::{VfsError, VfsPath, VirtualFs};

use super::constants::CONTAINER_BUFFER;

/// A seekable reader over one file in a [`VirtualFs`].
///
/// Built on `read_at` rather than on `open_read`, because a zip is parsed from
/// its end backwards and no streaming reader can do that. Cheap to clone in
/// spirit — it holds no handle, only a path and an offset — which is why a
/// second one can be made for every entry without the cost adding up.
pub struct Container {
    fs: Arc<dyn VirtualFs>,
    path: VfsPath,
    size: u64,
    offset: u64,
}

impl Container {
    pub fn new(fs: Arc<dyn VirtualFs>, path: VfsPath, size: u64) -> Container {
        Container {
            fs,
            path,
            size,
            offset: 0,
        }
    }

    /// Another reader over the same file, positioned at the start.
    pub fn reopen(&self) -> Container {
        Container::new(self.fs.clone(), self.path.clone(), self.size)
    }

    /// A window of the container itself, for an entry stored uncompressed —
    /// the one case where a window of the entry is a window of the file, with
    /// nothing to decode in between.
    pub fn read_at(&self, offset: u64, len: usize) -> Result<Vec<u8>, VfsError> {
        self.fs.read_at(&self.path, offset, len)
    }
}

impl Read for Container {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let want = buffer.len().min(CONTAINER_BUFFER);
        let bytes = self
            .fs
            .read_at(&self.path, self.offset, want)
            .map_err(io::Error::other)?;
        buffer[..bytes.len()].copy_from_slice(&bytes);
        self.offset += bytes.len() as u64;
        Ok(bytes.len())
    }
}

impl Seek for Container {
    fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
        // Saturating rather than erroring on a seek before the start: the zip
        // parser walks backwards looking for the end-of-directory signature,
        // and a container shorter than the window it wants is a malformed
        // archive, which is reported by the parse failing rather than by an
        // io::Error from underneath it.
        self.offset = match to {
            SeekFrom::Start(offset) => offset,
            SeekFrom::End(delta) => self.size.saturating_add_signed(delta),
            SeekFrom::Current(delta) => self.offset.saturating_add_signed(delta),
        };
        Ok(self.offset)
    }
}

/// The bytes of one entry: a bounded window on the container.
///
/// Bounded rather than trusted to stop on its own, because the length comes
/// out of the archive's own directory and an archive is somebody else's file.
/// A compressed stream that claims to continue past its recorded end is cut
/// off here rather than allowed to read the next entry's bytes.
pub struct Region {
    container: Container,
    remaining: u64,
}

impl Region {
    pub fn new(container: &Container, start: u64, length: u64) -> Region {
        let mut container = container.reopen();
        container.offset = start;
        Region {
            container,
            remaining: length,
        }
    }
}

impl Read for Region {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if self.remaining == 0 {
            return Ok(0);
        }
        let want = buffer.len().min(self.remaining as usize);
        let read = self.container.read(&mut buffer[..want])?;
        self.remaining -= read as u64;
        Ok(read)
    }
}
