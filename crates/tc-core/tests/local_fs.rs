//! Behavioral tests for [`LocalFs`] against real directories.
//!
//! Assertions are written as invariants over the fixture — name sets, counts,
//! byte sums — rather than hand-copied expected vectors, so a test fails
//! because the behavior changed, not because someone reordered a literal.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use tc_core::vfs::{Entry, EntryKind, LocalFs, VfsError, VfsPath, VirtualFs};
use tempfile::TempDir;

/// Maps a tempdir onto the VFS path space. On Unix the native path is already
/// the VFS path; on Windows the drive letter becomes the first component.
fn vfs_path(path: &Path) -> VfsPath {
    let text = path.to_string_lossy().replace('\\', "/");
    VfsPath::new(&text)
}

fn names(entries: &[Entry]) -> BTreeSet<String> {
    entries.iter().map(|entry| entry.name.clone()).collect()
}

fn find<'a>(entries: &'a [Entry], name: &str) -> &'a Entry {
    entries
        .iter()
        .find(|entry| entry.name == name)
        .unwrap_or_else(|| panic!("{name} missing from listing"))
}

#[test]
fn listing_returns_exactly_the_entries_that_were_created() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("a.txt"), "aaa").unwrap();
    fs::write(dir.path().join("b.md"), "bb").unwrap();
    fs::create_dir(dir.path().join("sub")).unwrap();

    let entries = LocalFs.read_dir(&vfs_path(dir.path())).unwrap();

    assert_eq!(entries.len(), 3);
    assert_eq!(
        names(&entries),
        ["a.txt", "b.md", "sub"].map(String::from).into()
    );
}

#[test]
fn file_sizes_sum_to_the_bytes_written() {
    let dir = TempDir::new().unwrap();
    let payloads = ["", "x", "hello world", &"z".repeat(5000)];
    for (index, payload) in payloads.iter().enumerate() {
        fs::write(dir.path().join(format!("f{index}")), payload).unwrap();
    }

    let entries = LocalFs.read_dir(&vfs_path(dir.path())).unwrap();

    let written: u64 = payloads.iter().map(|p| p.len() as u64).sum();
    let listed: u64 = entries.iter().map(|entry| entry.size).sum();
    assert_eq!(listed, written);
}

#[test]
fn directories_report_zero_size_and_are_enterable() {
    let dir = TempDir::new().unwrap();
    fs::create_dir(dir.path().join("sub")).unwrap();
    fs::write(dir.path().join("sub/inner.txt"), "content").unwrap();

    let entries = LocalFs.read_dir(&vfs_path(dir.path())).unwrap();
    let sub = find(&entries, "sub");

    assert_eq!(sub.kind, EntryKind::Dir);
    assert_eq!(sub.size, 0);
    assert!(sub.is_dir());
}

#[test]
fn an_empty_directory_lists_nothing() {
    let dir = TempDir::new().unwrap();
    assert!(LocalFs.read_dir(&vfs_path(dir.path())).unwrap().is_empty());
}

#[test]
fn nested_directories_are_listed_at_their_own_level() {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("a/b")).unwrap();
    fs::write(dir.path().join("a/b/deep.txt"), "deep").unwrap();

    let top = LocalFs.read_dir(&vfs_path(dir.path())).unwrap();
    assert_eq!(names(&top), ["a"].map(String::from).into());

    let deep = LocalFs
        .read_dir(&vfs_path(&dir.path().join("a/b")))
        .unwrap();
    assert_eq!(names(&deep), ["deep.txt"].map(String::from).into());
}

#[test]
fn hidden_entries_are_listed_and_flagged_not_filtered_out() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("visible.txt"), "v").unwrap();
    fs::write(dir.path().join(".hidden"), "h").unwrap();

    let entries = LocalFs.read_dir(&vfs_path(dir.path())).unwrap();

    // The VFS never filters; hiding is the listing layer's decision.
    assert_eq!(entries.len(), 2);
    assert!(!find(&entries, "visible.txt").hidden);
    // Only Unix treats the leading dot as hidden; on Windows it is an
    // attribute, and a dot-prefixed name is an ordinary visible file.
    assert_eq!(find(&entries, ".hidden").hidden, cfg!(unix));
}

#[test]
fn names_with_spaces_and_unicode_round_trip_through_the_listing() {
    let dir = TempDir::new().unwrap();
    let tricky = ["my file.txt", "Ünïcødé ✓.md", "with-dash_and.dots.tar.gz"];
    for name in tricky {
        fs::write(dir.path().join(name), name).unwrap();
    }

    let entries = LocalFs.read_dir(&vfs_path(dir.path())).unwrap();

    assert_eq!(names(&entries), tricky.map(String::from).into());
    for name in tricky {
        assert_eq!(find(&entries, name).size, name.len() as u64);
    }
}

#[test]
fn stat_describes_a_single_file() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("one.txt"), "12345").unwrap();

    let entry = LocalFs
        .stat(&vfs_path(&dir.path().join("one.txt")))
        .unwrap();

    assert_eq!(entry.name, "one.txt");
    assert_eq!(entry.kind, EntryKind::File);
    assert_eq!(entry.size, 5);
}

#[test]
fn stat_of_the_root_succeeds_without_touching_the_filesystem() {
    let root = LocalFs.stat(&VfsPath::root()).unwrap();
    assert_eq!(root.kind, EntryKind::Dir);
    assert!(root.is_dir());
}

#[test]
fn the_root_can_always_be_listed() {
    // Unix lists `/`; Windows lists the drives. Either way the pane has
    // somewhere to start.
    assert!(LocalFs.read_dir(&VfsPath::root()).is_ok());
}

#[test]
fn a_missing_path_reports_not_found() {
    let dir = TempDir::new().unwrap();
    let missing = vfs_path(&dir.path().join("nope"));

    assert_eq!(LocalFs.read_dir(&missing), Err(VfsError::NotFound));
    assert_eq!(LocalFs.stat(&missing), Err(VfsError::NotFound));
}

#[test]
fn listing_a_file_is_an_error_rather_than_an_empty_listing() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("file.txt"), "x").unwrap();

    let result = LocalFs.read_dir(&vfs_path(&dir.path().join("file.txt")));

    // The exact variant differs by platform; what matters is that it fails
    // instead of quietly pretending the file is an empty directory.
    assert!(result.is_err(), "listing a file must not succeed");
}

#[test]
fn a_backend_can_be_used_through_a_trait_object() {
    // Panes hold `Box<dyn VirtualFs>` so they can swap in an archive backend.
    let fs: Box<dyn VirtualFs> = Box::new(LocalFs);
    assert!(fs.stat(&VfsPath::root()).is_ok());
}

#[cfg(unix)]
mod symlinks {
    use super::*;
    use tc_core::vfs::SymlinkTarget;

    #[test]
    fn a_symlink_reports_its_targets_kind_and_size() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("target.txt"), "abcdef").unwrap();
        fs::create_dir(dir.path().join("target_dir")).unwrap();
        std::os::unix::fs::symlink(dir.path().join("target.txt"), dir.path().join("to_file"))
            .unwrap();
        std::os::unix::fs::symlink(dir.path().join("target_dir"), dir.path().join("to_dir"))
            .unwrap();

        let entries = LocalFs.read_dir(&vfs_path(dir.path())).unwrap();

        let to_file = find(&entries, "to_file");
        assert_eq!(to_file.kind, EntryKind::Symlink(SymlinkTarget::File));
        assert_eq!(to_file.size, 6);
        assert!(!to_file.is_dir());

        let to_dir = find(&entries, "to_dir");
        assert_eq!(to_dir.kind, EntryKind::Symlink(SymlinkTarget::Dir));
        // A symlinked directory is enterable, which is the whole point of
        // resolving the target kind instead of stopping at "it is a link".
        assert!(to_dir.is_dir());
    }

    #[test]
    fn a_broken_symlink_is_listed_instead_of_failing_the_directory() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("keeper.txt"), "kept").unwrap();
        std::os::unix::fs::symlink(dir.path().join("gone"), dir.path().join("dangling")).unwrap();

        let entries = LocalFs.read_dir(&vfs_path(dir.path())).unwrap();

        assert_eq!(entries.len(), 2, "the dangling link must not hide the file");
        let dangling = find(&entries, "dangling");
        assert_eq!(dangling.kind, EntryKind::Symlink(SymlinkTarget::Broken));
        assert_eq!(dangling.size, 0);
        assert!(!dangling.is_dir());
    }
}
