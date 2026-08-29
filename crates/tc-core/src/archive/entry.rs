//! Turning an indexed entry into a reader over its decoded bytes.
//!
//! Every reader built here **owns** its range of the container, so it can be
//! handed to a worker thread and read at its own pace. That is the whole
//! reason the index records offsets instead of keeping an open archive handle
//! behind a mutex: `VirtualFs::open_read` promises a `Box<dyn Read + Send>`,
//! and a reader borrowing a shared archive cannot be one.

use std::io::{self, Read};

use crate::vfs::VfsError;

use super::constants::{CHECKSUM_MISMATCH, ENCRYPTED_ENTRY, UNSUPPORTED_METHOD};
use super::index::{Bytes, Method};
use super::reader::{Container, Region};

/// A reader over one entry's decoded bytes.
pub fn open(
    container: &Container,
    name: &str,
    bytes: &Bytes,
) -> Result<Box<dyn Read + Send>, VfsError> {
    let region = Region::new(container, bytes.start, bytes.stored);
    let decoded: Box<dyn Read + Send> = match &bytes.method {
        Method::Stored => Box::new(region),
        Method::Deflated => Box::new(flate2::read::DeflateDecoder::new(region)),
        Method::Encrypted => return Err(VfsError::Io(ENCRYPTED_ENTRY.replace("{name}", name))),
        Method::Unsupported(method) => {
            return Err(VfsError::Io(UNSUPPORTED_METHOD.replace("{method}", method)))
        }
    };
    Ok(match bytes.crc32 {
        Some(expected) => Box::new(Checked::new(decoded, expected, name.to_string())),
        None => decoded,
    })
}

/// A reader that fails at the end if the bytes were not the bytes the archive
/// recorded.
///
/// Checking rather than trusting is the reliability requirement applied to the
/// one operation where a silent corruption is plausible: a truncated download,
/// a bad sector, a decoder fed the wrong range. A copy that reports success
/// over half a file is exactly the failure `docs/reliability.md` is about, and
/// the check costs one pass over bytes already in cache.
///
/// It fires at end of stream, which means a partly written target has to be
/// cleaned up by whoever was writing it — the operation engine already does
/// that for any error mid-copy.
struct Checked<R> {
    inner: R,
    crc: flate2::Crc,
    expected: u32,
    name: String,
}

impl<R: Read> Checked<R> {
    fn new(inner: R, expected: u32, name: String) -> Checked<R> {
        Checked {
            inner,
            crc: flate2::Crc::new(),
            expected,
            name,
        }
    }
}

impl<R: Read> Read for Checked<R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let read = self.inner.read(buffer)?;
        if read == 0 {
            if self.crc.sum() != self.expected {
                return Err(io::Error::other(
                    CHECKSUM_MISMATCH.replace("{name}", &self.name),
                ));
            }
            return Ok(0);
        }
        self.crc.update(&buffer[..read]);
        Ok(read)
    }
}
