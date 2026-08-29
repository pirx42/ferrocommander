//! Reading a zip's central directory into an [`Index`].
//!
//! The `zip` crate is used here as a **parser and nothing else** — it is
//! depended on with no compression features at all. What it is for is the
//! part nobody should hand-roll: the end-of-directory record, zip64, the two
//! date encodings, and where each entry's bytes actually begin. The bytes
//! themselves are decoded in [`super::entry`], over the raw range this pass
//! records, because a decoder that owns its range can be handed to a worker
//! thread and one that borrows an open archive cannot.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use zip::{CompressionMethod, ZipArchive};

use crate::vfs::{attributes_from_unix_mode, VfsError};

use super::constants::{
    DAYS_CIVIL_TO_EPOCH, DAYS_PER_ERA, MARCH_SHIFT_DENOMINATOR, MARCH_SHIFT_NUMERATOR,
    NOT_AN_ARCHIVE, SECONDS_PER_DAY, SECONDS_PER_HOUR, SECONDS_PER_MINUTE, YEARS_PER_ERA,
};
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
        // which is what makes `data_start` known — and it is the only thing
        // this loop wants from the entry.
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

/// A zip date as a `SystemTime`.
///
/// Zip stores a local wall-clock time with no zone in it, so this reads it as
/// UTC. Being an hour or two out in a date column is a smaller lie than
/// refusing to show a date at all, and there is nothing in the file to do
/// better with.
///
/// Converted by hand rather than by pulling in a date library for six fields:
/// `zip`'s own conversion lives behind a feature that would add one.
fn as_system_time(stamp: zip::DateTime) -> Option<SystemTime> {
    let days = days_from_civil(
        i64::from(stamp.year()),
        u32::from(stamp.month()),
        u32::from(stamp.day()),
    );
    let seconds = days * SECONDS_PER_DAY
        + i64::from(stamp.hour()) * SECONDS_PER_HOUR
        + i64::from(stamp.minute()) * SECONDS_PER_MINUTE
        + i64::from(stamp.second());
    u64::try_from(seconds)
        .ok()
        .map(|seconds| UNIX_EPOCH + Duration::from_secs(seconds))
}

/// Days from 1970-01-01 to a proleptic-Gregorian date, by Howard Hinnant's
/// `days_from_civil`. Shifting the year to start in March makes the leap day
/// the last day of the year, which is what removes every special case.
fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let year = year - i64::from(month <= 2);
    let era = year.div_euclid(YEARS_PER_ERA);
    let year_of_era = year.rem_euclid(YEARS_PER_ERA);
    let month = i64::from(month);
    let day_of_year = (MARCH_SHIFT_NUMERATOR * (month + if month > 2 { -3 } else { 9 }) + 2)
        / MARCH_SHIFT_DENOMINATOR
        + i64::from(day)
        - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * DAYS_PER_ERA + day_of_era - DAYS_CIVIL_TO_EPOCH
}
