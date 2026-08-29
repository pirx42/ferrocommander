//! Reading a zip's central directory into an [`Index`].
//!
//! The `zip` crate is used here as a **parser and nothing else** — it is
//! depended on with no compression features at all. What it is for is the
//! part nobody should hand-roll: the end-of-directory record, zip64, the two
//! date encodings, and where each entry's bytes actually begin. The bytes
//! themselves are decoded in [`super::entry`], over the raw range this pass
//! records, because a decoder that owns its range can be handed to a worker
//! thread and one that borrows an open archive cannot.

use std::time::SystemTime;

use zip::{CompressionMethod, ZipArchive};

use crate::vfs::{attributes_from_unix_mode, VfsError};

use super::constants::NOT_AN_ARCHIVE;
use super::date::as_system_time;
use super::index::{Bytes, Index, Method};
use super::reader::Container;

/// Reads the whole directory of a zip. `fallback` dates anything the archive
/// does not date itself.
pub fn index(container: &Container, fallback: SystemTime) -> Result<Index, VfsError> {
    // Buffered, and at the default 8 KiB rather than the 64 KiB the rest of
    // this layer reads in. Parsing seeks constantly — end of directory, then
    // each entry's local header — and a buffer refilled after every seek is
    // work thrown away: 10 000 entries take 49 ms buffered at 8 KiB, 63 ms
    // unbuffered, and 152 ms buffered at 64 KiB (docs/performance.md).
    let mut archive = ZipArchive::new(std::io::BufReader::new(container.reopen()))
        .map_err(|err| VfsError::Io(NOT_AN_ARCHIVE.replace("{reason}", &err.to_string())))?;

    let mut index = Index::new(fallback);
    for position in 0..archive.len() {
        // `by_index_raw` reads the local header without starting a decoder,
        // which is what makes `data_start` known — the offset past the 30-byte
        // header and its variable-length name and extra fields, and the only
        // thing this loop wants from the entry.
        let entry = archive
            .by_index_raw(position)
            .map_err(|err| VfsError::Io(NOT_AN_ARCHIVE.replace("{reason}", &err.to_string())))?;

        let is_dir = entry.is_dir();
        let bytes = (!is_dir)
            .then(|| {
                entry.data_start().map(|start| Bytes {
                    start,
                    stored: entry.compressed_size(),
                    method: method_of(&entry),
                    crc32: Some(entry.crc32()),
                })
            })
            .flatten();

        index.insert(
            entry.name(),
            is_dir,
            entry.size(),
            entry
                .last_modified()
                .and_then(as_system_time)
                .unwrap_or(fallback),
            entry
                .unix_mode()
                .map_or_else(Default::default, attributes_from_unix_mode),
            bytes,
        );
    }
    Ok(index)
}

/// How this build will read an entry's bytes.
///
/// An entry it cannot decode is named rather than dropped from the listing:
/// the file is in the archive, and a listing that hid it would be a listing
/// of something else. The failure arrives when somebody tries to read it.
fn method_of(entry: &zip::read::ZipFile<'_, std::io::BufReader<Container>>) -> Method {
    if entry.encrypted() {
        return Method::Encrypted;
    }
    match entry.compression() {
        CompressionMethod::Stored => Method::Stored,
        CompressionMethod::Deflated => Method::Deflated,
        other => Method::Unsupported(format!("{other:?}")),
    }
}
