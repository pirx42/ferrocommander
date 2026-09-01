//! Path vocabulary of the virtual filesystem layer.
//!
//! VFS paths use `/` regardless of the host platform: an archive's internal
//! paths are `/`-separated by specification, and using one separator for every
//! backend keeps `VfsPath` free of per-backend special cases.

/// Separator between path components.
pub const SEPARATOR: char = '/';

/// The root of every `VirtualFs`.
pub const ROOT: &str = "/";

/// Component that refers to the parent directory.
pub const PARENT: &str = "..";

/// Component that refers to the directory itself.
pub const CURRENT: &str = ".";
