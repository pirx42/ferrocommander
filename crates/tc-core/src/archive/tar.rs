//! Reading a tar's headers into an [`Index`].
//!
//! Two things are different from a zip and both matter:
//!
//! - **A tar has no index.** Opening one is a full scan of its headers, and
//!   opening a `.tar.gz` decompresses the whole stream to do it. There is no
//!   version of this that is fast; what there is, is a version that is honest
//!   about it (`docs/performance.md`).
//! - **A tar records no checksum of an entry's contents** — only of its
//!   header — so nothing here can verify what a zip's CRC verifies.
//!
//! The format looks simple enough not to need a crate and is not: long names,
//! the GNU and PAX dialects, and sparse members are exactly what a hand-rolled
//! reader gets wrong on somebody else's backup.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::vfs::{attributes_from_unix_mode, VfsError};

use super::constants::NOT_AN_ARCHIVE;
use super::index::{Bytes, Index, Method};
use super::reader::{whole, Container, Wrapper};

/// Reads every header in a tar. `fallback` dates anything the archive does
/// not date itself.
pub fn index(
    container: &Container,
    wrapper: Wrapper,
    fallback: SystemTime,
) -> Result<Index, VfsError> {
    let mut archive = tar::Archive::new(whole(container, wrapper));
    let mut index = Index::new(fallback);
    for entry in archive.entries().map_err(unreadable)? {
        let entry = entry.map_err(unreadable)?;
        let kind = entry.header().entry_type();
        // Symlinks, hard links, devices and fifos are passed over. There is
        // nowhere in a listing to say what a symlink inside an archive points
        // at, and unpacking one as an empty file — which is what its recorded
        // size would produce — would be a copy that quietly got it wrong.
        // Named in `docs/future-improvements.md` rather than left as a
        // surprise.
        if !kind.is_file() && !kind.is_dir() {
            continue;
        }
        let header = entry.header();
        let size = header.size().map_err(unreadable)?;
        let modified = header.mtime().map_or(fallback, |seconds| {
            UNIX_EPOCH + Duration::from_secs(seconds)
        });
        let attributes = header
            .mode()
            .map_or_else(|_| Default::default(), attributes_from_unix_mode);
        // The bytes rather than the `Path`: a tar name is arbitrary bytes, and
        // going through a platform path would mangle it differently on the two
        // targets before this layer ever saw it.
        let name = String::from_utf8_lossy(&entry.path_bytes()).into_owned();

        let bytes = (!kind.is_dir()).then(|| Bytes {
            // Where the data sits in the stream the entries were read from,
            // which for a `.tar.gz` is the decompressed one.
            start: entry.raw_file_position(),
            stored: size,
            // A tar stores its members as they are; only the container may be
            // compressed, and that is the wrapper's business.
            method: Method::Stored,
            crc32: None,
        });
        index.insert(&name, kind.is_dir(), size, modified, attributes, bytes);
    }
    Ok(index)
}

fn unreadable(err: std::io::Error) -> VfsError {
    VfsError::Io(NOT_AN_ARCHIVE.replace("{reason}", &err.to_string()))
}
