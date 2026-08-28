//! Defaults of the directory model.

use std::time::SystemTime;

use super::sort::{SortKey, SortOrder};

/// Sort a freshly loaded directory by name, ascending — what an orthodox file
/// manager opens with.
pub const DEFAULT_SORT_KEY: SortKey = SortKey::Name;
pub const DEFAULT_SORT_ORDER: SortOrder = SortOrder::Ascending;

/// Hidden entries stay hidden until the user asks for them.
pub const DEFAULT_SHOW_HIDDEN: bool = false;

/// Timestamp reported for the synthetic `..` entry.
///
/// The parent's real mtime would cost a `stat` per directory change and means
/// nothing to the user — `..` is a navigation control, not a file. It always
/// sorts first anyway, so the value never influences ordering.
pub const PARENT_MODIFIED: SystemTime = SystemTime::UNIX_EPOCH;
