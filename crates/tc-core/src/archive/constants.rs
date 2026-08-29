//! What the archive layer names and how big its buffers are.

/// Extensions that make a file enterable as a directory.
///
/// Lower-cased before comparison, because `.ZIP` off a Windows machine is the
/// same archive.
pub const ZIP_EXTENSION: &str = "zip";

/// How much of the container one `read_at` of an entry pulls in.
///
/// Reading a compressed entry means decompressing from its start, so a small
/// buffer costs a syscall per few kilobytes over the whole prefix. 64 KiB is
/// the same window the viewer asks for, which makes one viewer page one pass.
pub const CONTAINER_BUFFER: usize = 64 * 1024;

/// What an entry compressed by a method this build cannot decode reports.
pub const UNSUPPORTED_METHOD: &str = "compression method {method} is not supported";

/// What an entry whose bytes do not match the checksum the archive recorded
/// reports. Not a warning: a wrong byte in an unpacked file is exactly the
/// failure this project's reliability requirement is about.
pub const CHECKSUM_MISMATCH: &str = "{name}: the archive's checksum does not match the data";

/// What an encrypted entry reports. Reading one needs a password, and there
/// is nowhere to ask for it that would not also have to hold it.
pub const ENCRYPTED_ENTRY: &str = "{name}: encrypted entries are not supported";

/// What a container that does not parse reports.
pub const NOT_AN_ARCHIVE: &str = "not a readable archive: {reason}";

/// Turning a calendar date into a `SystemTime`, which archives need and no
/// dependency here provides. Named rather than spelled inline: `146_097` in
/// the middle of an expression is exactly the kind of number skill 16 is
/// about.
pub const SECONDS_PER_MINUTE: i64 = 60;
pub const SECONDS_PER_HOUR: i64 = 60 * SECONDS_PER_MINUTE;
pub const SECONDS_PER_DAY: i64 = 24 * SECONDS_PER_HOUR;
/// A Gregorian era is 400 years, after which the calendar repeats exactly.
pub const YEARS_PER_ERA: i64 = 400;
/// Days in one era: 400 × 365 plus its 97 leap days.
pub const DAYS_PER_ERA: i64 = 146_097;
/// Days from 0000-03-01, where the shifted year starts, to 1970-01-01.
pub const DAYS_CIVIL_TO_EPOCH: i64 = 719_468;
/// The month-length pattern: 153 days per 5 months, exactly.
pub const MARCH_SHIFT_NUMERATOR: i64 = 153;
pub const MARCH_SHIFT_DENOMINATOR: i64 = 5;

/// What an archive written on Windows separates its entry names with. Turned
/// into the one separator `VfsPath` knows before anything else looks at a name.
pub const WINDOWS_SEPARATOR: char = '\\';

/// The tar extensions, and the double one `.tar.gz` ends in.
pub const TAR_EXTENSION: &str = "tar";
pub const TGZ_EXTENSION: &str = "tgz";
pub const TAR_GZ_SUFFIX: &str = ".tar.gz";

/// The first year a zip date can express. Written as no date at all rather
/// than clamped: a file dated 1970 in an archive that says 1980 is a wrong
/// answer, and none is an honest one.
pub const ZIP_EPOCH_YEAR: i64 = 1980;

/// What a pack writes to before it is a finished archive.
///
/// Renamed into place at the end, so an interrupted pack leaves nothing that
/// looks like an archive — and the rename is on the same directory, so it is
/// atomic.
pub const PACKING_SUFFIX: &str = ".packing";
