//! Writing an archive: one sequential stream, one entry at a time.
//!
//! Deliberately *not* a [`VirtualFs`](crate::vfs::VirtualFs). A zip is a
//! stream with its index at the end and a tar is a stream with no index at
//! all, so a backend promising `create_file` on any path in any order would
//! have to rewrite the container per call — correct, useless, and slow in the
//! way the prime directive forbids (`docs/archives.md`).
//!
//! What it is instead is the one step the copy engine cannot do itself. The
//! scan, the progress, the cancel and the failure list are the engine's, and
//! this replaces only "write these bytes at the destination". Nothing about a
//! format reaches `ops`.

use std::io::{Read, Write};
use std::time::SystemTime;

use crate::vfs::{unix_mode_of, Attributes, VfsError};

use super::constants::{
    DAYS_CIVIL_TO_EPOCH, DAYS_PER_ERA, MARCH_SHIFT_DENOMINATOR, MARCH_SHIFT_NUMERATOR,
    SECONDS_PER_DAY, SECONDS_PER_HOUR, SECONDS_PER_MINUTE, YEARS_PER_ERA, ZIP_EPOCH_YEAR,
};
use super::reader::Wrapper;
use super::Format;

/// Where the bytes of an archive being written go.
pub type Sink = Box<dyn Write + Send>;

/// Appends entries to an archive, in the order they are given.
///
/// One entry at a time and no going back, which is what both formats are.
pub trait Packer {
    fn add_dir(
        &mut self,
        name: &str,
        modified: SystemTime,
        attributes: Attributes,
    ) -> Result<(), VfsError>;

    /// Appends one file, reading its bytes from `source`.
    ///
    /// The reader is the engine's, so it is the engine that counts the bytes
    /// for the progress bar and stops when a cancel arrives — a `Packer` that
    /// took a path instead would have to do both, in each format, twice.
    fn add_file(
        &mut self,
        name: &str,
        size: u64,
        modified: SystemTime,
        attributes: Attributes,
        source: &mut dyn Read,
    ) -> Result<(), VfsError>;

    /// Writes whatever the format keeps for last and flushes the sink.
    ///
    /// Takes `Box<Self>` because finishing consumes the writer, and a boxed
    /// trait object is the only shape the engine can hold one in.
    fn finish(self: Box<Self>) -> Result<(), VfsError>;
}

/// A packer for `format`, writing into `sink`.
pub fn packer(format: Format, sink: Sink) -> Box<dyn Packer> {
    match format {
        Format::Zip => Box::new(ZipPacker::new(sink)),
        Format::Tar => Box::new(TarPacker::new(sink, Wrapper::None)),
        Format::TarGz => Box::new(TarPacker::new(sink, Wrapper::Gzip)),
    }
}

struct ZipPacker {
    writer: zip::ZipWriter<zip::write::StreamWriter<Sink>>,
}

impl ZipPacker {
    fn new(sink: Sink) -> ZipPacker {
        // The streaming writer, because a `Sink` is a `Write` and nothing
        // else: a backend's `create_file` promises no seek, and demanding one
        // would rule out writing an archive anywhere but a local disk.
        ZipPacker {
            writer: zip::ZipWriter::new_stream(sink),
        }
    }

    fn options(
        modified: SystemTime,
        attributes: Attributes,
    ) -> zip::write::FileOptions<'static, ()> {
        let options = zip::write::FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            // Zip64 for everything rather than only where it is needed: the
            // size is known here, but the cost of guessing wrong is an archive
            // that silently truncates a large file at 4 GB.
            .large_file(true);
        let options = match unix_mode_of(attributes) {
            Some(mode) => options.unix_permissions(mode),
            None => options,
        };
        match zip_date(modified) {
            Some(stamp) => options.last_modified_time(stamp),
            None => options,
        }
    }
}

impl Packer for ZipPacker {
    fn add_dir(
        &mut self,
        name: &str,
        modified: SystemTime,
        attributes: Attributes,
    ) -> Result<(), VfsError> {
        self.writer
            .add_directory(name, ZipPacker::options(modified, attributes))
            .map_err(failed)
    }

    fn add_file(
        &mut self,
        name: &str,
        _size: u64,
        modified: SystemTime,
        attributes: Attributes,
        source: &mut dyn Read,
    ) -> Result<(), VfsError> {
        self.writer
            .start_file(name, ZipPacker::options(modified, attributes))
            .map_err(failed)?;
        std::io::copy(source, &mut self.writer).map_err(VfsError::from)?;
        Ok(())
    }

    fn finish(self: Box<Self>) -> Result<(), VfsError> {
        let mut sink = self.writer.finish().map_err(failed)?.into_inner();
        // Explicitly, rather than on drop: a drop that fails to flush has
        // nowhere to report it, and an archive missing its last block is
        // exactly the kind of quiet corruption `docs/reliability.md` is about.
        sink.flush().map_err(VfsError::from)
    }
}

struct TarPacker {
    builder: tar::Builder<Box<dyn Write + Send>>,
}

impl TarPacker {
    fn new(sink: Sink, wrapper: Wrapper) -> TarPacker {
        let sink: Sink = match wrapper {
            Wrapper::None => sink,
            Wrapper::Gzip => Box::new(flate2::write::GzEncoder::new(
                sink,
                flate2::Compression::default(),
            )),
        };
        TarPacker {
            builder: tar::Builder::new(sink),
        }
    }

    fn header(size: u64, modified: SystemTime, attributes: Attributes) -> tar::Header {
        let mut header = tar::Header::new_gnu();
        header.set_size(size);
        header.set_mode(unix_mode_of(attributes).unwrap_or(TAR_DEFAULT_MODE));
        header.set_mtime(
            modified
                .duration_since(SystemTime::UNIX_EPOCH)
                .map_or(0, |since| since.as_secs()),
        );
        header
    }
}

/// What a tar entry's mode is where the platform has no such thing to copy.
const TAR_DEFAULT_MODE: u32 = 0o644;

impl Packer for TarPacker {
    fn add_dir(
        &mut self,
        name: &str,
        modified: SystemTime,
        attributes: Attributes,
    ) -> Result<(), VfsError> {
        let mut header = TarPacker::header(0, modified, attributes);
        header.set_entry_type(tar::EntryType::Directory);
        // The trailing separator is how a tar says "directory", and the
        // readers that care look at the name rather than the type byte.
        let name = format!("{name}/");
        self.builder
            .append_data(&mut header, name, std::io::empty())
            .map_err(VfsError::from)
    }

    fn add_file(
        &mut self,
        name: &str,
        size: u64,
        modified: SystemTime,
        attributes: Attributes,
        source: &mut dyn Read,
    ) -> Result<(), VfsError> {
        let mut header = TarPacker::header(size, modified, attributes);
        self.builder
            .append_data(&mut header, name, source)
            .map_err(VfsError::from)
    }

    fn finish(self: Box<Self>) -> Result<(), VfsError> {
        let mut sink = self.builder.into_inner().map_err(VfsError::from)?;
        sink.flush().map_err(VfsError::from)
    }
}

fn failed(err: zip::result::ZipError) -> VfsError {
    VfsError::Io(err.to_string())
}

/// A `SystemTime` as a zip date, or `None` for one zip cannot express.
///
/// Zip counts from 1980 and has no zone, so this writes the UTC calendar date
/// — the same reading the reader gives it back. A file dated before 1980, or
/// after the format's own end, gets no date rather than a wrong one.
fn zip_date(modified: SystemTime) -> Option<zip::DateTime> {
    let seconds = modified
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()?
        .as_secs() as i64;
    let days = seconds.div_euclid(SECONDS_PER_DAY);
    let rest = seconds.rem_euclid(SECONDS_PER_DAY);
    let (year, month, day) = civil_from_days(days);
    if year < ZIP_EPOCH_YEAR {
        return None;
    }
    zip::DateTime::from_date_and_time(
        u16::try_from(year).ok()?,
        u8::try_from(month).ok()?,
        u8::try_from(day).ok()?,
        (rest / SECONDS_PER_HOUR) as u8,
        (rest % SECONDS_PER_HOUR / SECONDS_PER_MINUTE) as u8,
        (rest % SECONDS_PER_MINUTE) as u8,
    )
    .ok()
}

/// The proleptic-Gregorian date `days` after 1970-01-01, by Howard Hinnant's
/// `civil_from_days` — the exact inverse of `days_from_civil` in
/// [`super::zip`], and pinned as one by a round-trip test.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let shifted = days + DAYS_CIVIL_TO_EPOCH;
    let era = shifted.div_euclid(DAYS_PER_ERA);
    let day_of_era = shifted.rem_euclid(DAYS_PER_ERA);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * YEARS_PER_ERA;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (MARCH_SHIFT_DENOMINATOR * day_of_year + 2) / MARCH_SHIFT_NUMERATOR;
    let day =
        day_of_year - (MARCH_SHIFT_NUMERATOR * shifted_month + 2) / MARCH_SHIFT_DENOMINATOR + 1;
    let month = shifted_month + if shifted_month < 10 { 3 } else { -9 };
    (year + i64::from(month <= 2), month as u32, day as u32)
}

#[cfg(test)]
mod tests {
    use super::super::zip::days_from_civil;
    use super::*;

    #[test]
    fn the_two_date_conversions_are_each_other() {
        // The reader turns a calendar date into a day count and the writer
        // turns it back. A pair that disagreed anywhere would move every date
        // in an archive that was unpacked and repacked.
        //
        // Every day from 1901 to 2099, which covers every leap rule the
        // calendar has: the four-year one, the hundred-year exception, and the
        // four-hundred-year exception to that.
        for days in -25_000i64..47_000 {
            let (year, month, day) = civil_from_days(days);
            assert_eq!(
                days_from_civil(year, month, day),
                days,
                "{year}-{month}-{day}"
            );
        }
    }
}
