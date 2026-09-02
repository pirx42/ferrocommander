//! Explorer's own context menu, for the Windows build.
//!
//! What a right click in Explorer shows is not a list Explorer owns: it is
//! assembled by the shell from the file's type, the verbs registered for it,
//! and every shell extension the machine has — 7-Zip, TortoiseGit, *Send to*,
//! *Properties*. A file manager that wants that menu asks the shell for it
//! (`IShellFolder::GetUIObjectOf` → `IContextMenu`), draws it as a Win32
//! popup, and hands the choice back (`InvokeCommand`). This crate is that
//! conversation and nothing else.
//!
//! **Its own crate, for two reasons that are both about testing.** The
//! application is a GTK program, and GTK for Windows cannot be built on the
//! Linux box that runs the gate — but this crate has no GTK in it, so
//! `cargo clippy -p fc-shellmenu --target x86_64-pc-windows-gnu` type-checks
//! every line of it from Linux. And the Windows CI job can run its tests
//! without building the application in test mode: everything up to the popup
//! — the PIDLs, the folder, the `IContextMenu`, the filled `HMENU`, the verbs
//! in it — runs headless on the runner. Only `TrackPopupMenuEx` and
//! `InvokeCommand` need a desktop, and they are the two calls the owner tries.
//!
//! On any other platform the crate is empty, and the application never
//! mentions it.

#[cfg(windows)]
mod shell;

#[cfg(windows)]
pub use shell::{Error, Menu, Where};
