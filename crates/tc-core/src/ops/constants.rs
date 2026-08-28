//! Tuning values of the operation engine.

/// Bytes moved per read/write turn of the copy loop.
///
/// The loop checks for cancellation once per turn, so this is also how
/// coarse a cancel is: 64 KiB is small enough that a cancel feels immediate
/// even on a slow disk, and large enough that the per-call overhead does not
/// show up against sequential throughput.
pub const COPY_BUFFER_BYTES: usize = 64 * 1024;

/// First counter tried when [`super::Resolution::KeepBoth`] has to invent a
/// free name. `notes.txt` becomes `notes (2).txt`, matching what a user reads
/// as "the second one".
pub const KEEP_BOTH_FIRST_COUNTER: u32 = 2;

/// How a counter is written into a name that is already taken.
pub const KEEP_BOTH_SUFFIX: &str = " ({counter})";

/// Separator between a stem and its extension when a name is put back
/// together.
pub const EXTENSION_SEPARATOR: char = '.';

/// Why a symlink to a directory is not copied.
///
/// Following it would loop forever on a cycle — `/usr/bin/X11 -> .` is a real
/// example — and recreating it needs a `symlink` call the interface does not
/// have. Deleting one is unaffected: a link is removed as a file, never
/// followed.
pub const SYMLINK_NOT_COPIED: &str = "symbolic links to directories are not copied";
