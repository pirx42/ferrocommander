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
// so unused helpers here are expected rather than dead.
#![allow(dead_code)]

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
