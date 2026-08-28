//! Headless engine for the file manager: virtual filesystems, the directory
//! listing model, file operations, search and multi-rename.
//!
//! This crate must never depend on GTK. Everything here is testable with a
//! plain `cargo test` against tempdirs, which is what keeps the UI layer thin
//! enough to be verified by hand.

pub mod listing;
pub mod vfs;

/// Version of the engine, taken from the crate manifest.
///
/// The UI reports this rather than carrying its own version string, so the two
/// crates cannot disagree about which build is running.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_reported_from_the_manifest() {
        assert_eq!(version(), env!("CARGO_PKG_VERSION"));
    }
}
