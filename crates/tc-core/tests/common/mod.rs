//! Fixtures and invariant helpers shared by the integration tests.
//!
//! Phase 1's audit deliberately left the test-helper duplication alone,
//! noting it should be revisited "when phase 2's operation tests need the
//! same shapes". They do: `ops.rs` and `queue.rs` were each carrying their
//! own tree builder and their own tree comparison, and two implementations of
//! "are these the same tree" is exactly one too many for the question every
//! transfer test asks.
//!
//! A `tests/common/` module rather than a `test-support` feature on the
//! crate, because that keeps the helpers out of the shipped library and needs
//! no feature flag to stay honest.

// Each test binary compiles this module separately and uses a subset of it,
// so unused helpers and an unused macro here are expected rather than dead.
#![allow(dead_code, unused_imports, unused_macros)]

use std::collections::BTreeMap;
use std::io::Read;

use tc_core::vfs::{LocalFs, VfsPath, VirtualFs};
use tempfile::TempDir;

/// A whole tree as relative path to contents, `None` for a directory.
///
/// Directories appear so an empty one is not silently lost, and they are
/// `None` rather than empty bytes because an empty file is not an empty
/// directory.
pub type Snapshot = BTreeMap<String, Option<Vec<u8>>>;

/// Builds a tree with nesting, an empty file, an empty directory and a
/// unicode name, so every walk has something awkward in it.
pub fn build_tree(root: &std::path::Path) {
    use std::fs;
    fs::create_dir_all(root.join("sub/deep")).unwrap();
    fs::create_dir(root.join("emptydir")).unwrap();
    fs::write(root.join("a.txt"), "hello world").unwrap();
    fs::write(root.join("empty.txt"), "").unwrap();
    fs::write(root.join("Ünïcødé — ✓.md"), "unicode").unwrap();
    // Big enough to span several turns of the copy loop.
    fs::write(root.join("sub/b.bin"), vec![7u8; 200_000]).unwrap();
    fs::write(root.join("sub/deep/c.txt"), "deep").unwrap();
}

/// A tempdir holding `tree/` to operate on and an empty `into/` to land in.
pub fn fixture() -> (TempDir, VfsPath) {
    let dir = TempDir::new().unwrap();
    build_tree(&dir.path().join("tree"));
    std::fs::create_dir(dir.path().join("into")).unwrap();
    let root = LocalFs::vfs_path(dir.path());
    (dir, root)
}

/// Every path below `root`, with its contents.
///
/// The comparison unit for every transfer test: two trees are the same tree
/// exactly when their snapshots are equal.
pub fn snapshot(fs: &dyn VirtualFs, root: &VfsPath) -> Snapshot {
    let mut found = Snapshot::new();
    collect(fs, root, root, &mut found);
    found
}

fn collect(fs: &dyn VirtualFs, root: &VfsPath, at: &VfsPath, found: &mut Snapshot) {
    for entry in fs.read_dir(at).unwrap_or_default() {
        let path = at.child(&entry.name);
        let relative = path
            .as_str()
            .strip_prefix(root.as_str())
            .unwrap_or_default()
            .to_string();
        if entry.is_dir() {
            found.insert(relative, None);
            collect(fs, root, &path, found);
        } else {
            let mut bytes = Vec::new();
            fs.open_read(&path)
                .unwrap()
                .read_to_end(&mut bytes)
                .unwrap();
            found.insert(relative, Some(bytes));
        }
    }
}

pub fn file_count(snapshot: &Snapshot) -> usize {
    snapshot.values().flatten().count()
}

pub fn byte_sum(snapshot: &Snapshot) -> usize {
    snapshot.values().flatten().map(Vec::len).sum()
}

pub fn exists(fs: &dyn VirtualFs, path: &VfsPath) -> bool {
    fs.stat(path).is_ok()
}

/// Implements [`VirtualFs`] for a decorator by forwarding every call to its
/// `inner` field, except the four a test might want to watch: `read_dir`,
/// `rename`, `open_read` and `trash`, which the type supplies as
/// `*_impl` methods.
///
/// Here rather than in one test file because two of them now wrap a backend
/// to watch it — the operation tests to count and to inject failures, the
/// settings tests to see which file is opened for writing — and a second copy
/// of thirty lines of forwarding is a second place to forget a method when
/// the trait grows.
///
/// A blanket `impl<T: Decorate> VirtualFs for T` would be tidier and is not
/// allowed: `VirtualFs` belongs to another crate, so the orphan rule refuses
/// it for a generic type.
macro_rules! delegate_vfs {
    ($target:ty) => {
        impl tc_core::vfs::VirtualFs for $target {
            fn read_dir(
                &self,
                path: &tc_core::vfs::VfsPath,
            ) -> Result<Vec<tc_core::vfs::Entry>, tc_core::vfs::VfsError> {
                self.read_dir_impl(path)
            }
            fn stat(
                &self,
                path: &tc_core::vfs::VfsPath,
            ) -> Result<tc_core::vfs::Entry, tc_core::vfs::VfsError> {
                self.inner.stat(path)
            }
            fn create_dir(
                &self,
                path: &tc_core::vfs::VfsPath,
            ) -> Result<(), tc_core::vfs::VfsError> {
                self.inner.create_dir(path)
            }
            fn remove_dir(
                &self,
                path: &tc_core::vfs::VfsPath,
            ) -> Result<(), tc_core::vfs::VfsError> {
                self.inner.remove_dir(path)
            }
            fn remove_file(
                &self,
                path: &tc_core::vfs::VfsPath,
            ) -> Result<(), tc_core::vfs::VfsError> {
                self.inner.remove_file(path)
            }
            fn rename(
                &self,
                from: &tc_core::vfs::VfsPath,
                to: &tc_core::vfs::VfsPath,
            ) -> Result<(), tc_core::vfs::VfsError> {
                self.rename_impl(from, to)
            }
            fn open_read(
                &self,
                path: &tc_core::vfs::VfsPath,
            ) -> Result<Box<dyn std::io::Read + Send>, tc_core::vfs::VfsError> {
                self.open_read_impl(path)
            }
            fn read_at(
                &self,
                path: &tc_core::vfs::VfsPath,
                offset: u64,
                len: usize,
            ) -> Result<Vec<u8>, tc_core::vfs::VfsError> {
                self.inner.read_at(path, offset, len)
            }
            fn create_file(
                &self,
                path: &tc_core::vfs::VfsPath,
            ) -> Result<Box<dyn std::io::Write + Send>, tc_core::vfs::VfsError> {
                self.create_file_impl(path)
            }
            fn set_modified(
                &self,
                path: &tc_core::vfs::VfsPath,
                time: std::time::SystemTime,
            ) -> Result<(), tc_core::vfs::VfsError> {
                self.inner.set_modified(path, time)
            }
            fn set_attributes(
                &self,
                path: &tc_core::vfs::VfsPath,
                attributes: tc_core::vfs::Attributes,
            ) -> Result<(), tc_core::vfs::VfsError> {
                self.inner.set_attributes(path, attributes)
            }
            fn trash(&self, path: &tc_core::vfs::VfsPath) -> Result<(), tc_core::vfs::VfsError> {
                self.trash_impl(path)
            }
        }
    };
}

pub(crate) use delegate_vfs;
