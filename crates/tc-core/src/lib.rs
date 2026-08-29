//! Headless engine for the file manager: virtual filesystems, the directory
//! listing model, file operations, search, multi-rename and archives.
//!
//! This crate must never depend on GTK. Everything here is testable with a
//! plain `cargo test` against tempdirs, which is what keeps the UI layer thin
//! enough to be verified by hand.

pub mod archive;
pub mod branch;
pub mod command;
pub mod config;
pub mod glob;
pub mod listing;
pub mod ops;
pub mod rename;
pub mod search;
pub mod sizes;
pub mod vfs;
pub mod viewer;
pub mod watch;
