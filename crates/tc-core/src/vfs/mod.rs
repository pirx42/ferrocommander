//! Virtual filesystems: one read interface over local directories and, from
//! phase 6 on, archives.
//!
//! Phase 1 exposes the read side only. The mutating half of the interface
//! (`create`, `rename`, `remove`, …) arrives in phase 2 together with the
//! operation engine that implements and tests it — trait methods added ahead
//! of a caller are dead code that no test can pin down.

pub mod constants;
mod local;
mod path;
mod platform;
mod types;

pub use local::LocalFs;
pub use path::VfsPath;
pub use types::{Entry, EntryKind, SymlinkTarget, VfsError};

/// A browsable filesystem.
///
/// Object-safe on purpose: a pane holds a `Box<dyn VirtualFs>` and swaps it
/// when the user steps into an archive, without the UI knowing which backend
/// it is talking to.
pub trait VirtualFs {
    /// Lists a directory. The order is unspecified — sorting belongs to the
    /// listing layer.
    fn read_dir(&self, path: &VfsPath) -> Result<Vec<Entry>, VfsError>;

    /// Describes a single path.
    fn stat(&self, path: &VfsPath) -> Result<Entry, VfsError>;
}
