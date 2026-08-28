//! Behavioral tests for [`LocalFs`] against real directories.
//!
//! Assertions are written as invariants over the fixture — name sets, counts,
//! byte sums — rather than hand-copied expected vectors, so a test fails
//! because the behavior changed, not because someone reordered a literal.

use std::collections::BTreeSet;
use std::fs;

use tc_core::vfs::{Entry, EntryKind, LocalFs, VfsError, VfsPath, VirtualFs};
use tempfile::TempDir;

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

    let entries = LocalFs.read_dir(&LocalFs::vfs_path(dir.path())).unwrap();

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

    let entries = LocalFs.read_dir(&LocalFs::vfs_path(dir.path())).unwrap();

    let written: u64 = payloads.iter().map(|p| p.len() as u64).sum();
    let listed: u64 = entries.iter().map(|entry| entry.size).sum();
    assert_eq!(listed, written);
}

#[test]
fn directories_report_zero_size_and_are_enterable() {
    let dir = TempDir::new().unwrap();
    fs::create_dir(dir.path().join("sub")).unwrap();
    fs::write(dir.path().join("sub/inner.txt"), "content").unwrap();

    let entries = LocalFs.read_dir(&LocalFs::vfs_path(dir.path())).unwrap();
    let sub = find(&entries, "sub");

    assert_eq!(sub.kind, EntryKind::Dir);
    assert_eq!(sub.size, 0);
    assert!(sub.is_dir());
}

#[test]
fn an_empty_directory_lists_nothing() {
    let dir = TempDir::new().unwrap();
    assert!(LocalFs
        .read_dir(&LocalFs::vfs_path(dir.path()))
        .unwrap()
        .is_empty());
}

#[test]
fn nested_directories_are_listed_at_their_own_level() {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("a/b")).unwrap();
    fs::write(dir.path().join("a/b/deep.txt"), "deep").unwrap();

    let top = LocalFs.read_dir(&LocalFs::vfs_path(dir.path())).unwrap();
    assert_eq!(names(&top), ["a"].map(String::from).into());

    let deep = LocalFs
        .read_dir(&LocalFs::vfs_path(&dir.path().join("a/b")))
        .unwrap();
    assert_eq!(names(&deep), ["deep.txt"].map(String::from).into());
}

#[test]
fn hidden_entries_are_listed_and_flagged_not_filtered_out() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("visible.txt"), "v").unwrap();
    fs::write(dir.path().join(".hidden"), "h").unwrap();

    let entries = LocalFs.read_dir(&LocalFs::vfs_path(dir.path())).unwrap();

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

    let entries = LocalFs.read_dir(&LocalFs::vfs_path(dir.path())).unwrap();

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
        .stat(&LocalFs::vfs_path(&dir.path().join("one.txt")))
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
    let missing = LocalFs::vfs_path(&dir.path().join("nope"));

    assert_eq!(LocalFs.read_dir(&missing), Err(VfsError::NotFound));
    assert_eq!(LocalFs.stat(&missing), Err(VfsError::NotFound));
}

#[test]
fn listing_a_file_is_an_error_rather_than_an_empty_listing() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("file.txt"), "x").unwrap();

    let result = LocalFs.read_dir(&LocalFs::vfs_path(&dir.path().join("file.txt")));

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

/// The mutating half of the interface (phase 2).
///
/// Every assertion is a conservation statement about the *directory* — its
/// name set, its entry count, its byte sum — rather than about the one path
/// the call names. A backend that writes the right file and quietly damages a
/// sibling passes the second kind of test and fails these.
mod writing {
    use std::io::{Read, Write};
    use std::time::{Duration, SystemTime};

    use super::*;

    /// Sizes of everything in a directory, which no call in this module may
    /// change except by exactly the bytes it was asked to add or remove.
    fn byte_sum(fs: &dyn VirtualFs, dir: &VfsPath) -> u64 {
        fs.read_dir(dir).unwrap().iter().map(|e| e.size).sum()
    }

    /// A directory holding two files and a subdirectory, so every test has
    /// siblings that must survive untouched.
    fn fixture() -> (TempDir, VfsPath) {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("keeper.txt"), "keep me").unwrap();
        fs::write(dir.path().join("other.bin"), vec![0u8; 40]).unwrap();
        fs::create_dir(dir.path().join("sub")).unwrap();
        let path = LocalFs::vfs_path(dir.path());
        (dir, path)
    }

    #[test]
    fn creating_a_directory_adds_exactly_one_entry() {
        let (_dir, path) = fixture();
        let before = names(&LocalFs.read_dir(&path).unwrap());

        LocalFs.create_dir(&path.child("fresh")).unwrap();

        let after = names(&LocalFs.read_dir(&path).unwrap());
        assert_eq!(after.len(), before.len() + 1);
        assert!(before.is_subset(&after), "siblings must survive untouched");
        assert!(find(&LocalFs.read_dir(&path).unwrap(), "fresh").is_dir());
    }

    #[test]
    fn creating_a_directory_that_exists_reports_already_exists() {
        let (_dir, path) = fixture();
        assert_eq!(
            LocalFs.create_dir(&path.child("sub")),
            Err(VfsError::AlreadyExists)
        );
    }

    #[test]
    fn creating_a_directory_without_a_parent_reports_not_found() {
        // Creating a chain is the engine's walk, so the backend refuses.
        let (_dir, path) = fixture();
        assert_eq!(
            LocalFs.create_dir(&path.child("missing").child("child")),
            Err(VfsError::NotFound)
        );
    }

    #[test]
    fn bytes_written_come_back_byte_identical() {
        let (_dir, path) = fixture();
        let payloads: [&[u8]; 4] = [b"", b"x", b"hello world", &[0xffu8; 5000]];

        for (index, payload) in payloads.iter().enumerate() {
            let target = path.child(&format!("written{index}"));
            let mut sink = LocalFs.create_file(&target).unwrap();
            sink.write_all(payload).unwrap();
            drop(sink);

            let mut read_back = Vec::new();
            LocalFs
                .open_read(&target)
                .unwrap()
                .read_to_end(&mut read_back)
                .unwrap();
            assert_eq!(read_back, *payload, "payload {index}");
            assert_eq!(LocalFs.stat(&target).unwrap().size, payload.len() as u64);
        }
    }

    #[test]
    fn creating_a_file_over_an_existing_one_truncates_it() {
        // Documented contract: deciding whether that is allowed happens in
        // the engine, which has to ask the user anyway.
        let (_dir, path) = fixture();
        let target = path.child("keeper.txt");

        LocalFs
            .create_file(&target)
            .unwrap()
            .write_all(b"hi")
            .unwrap();

        assert_eq!(LocalFs.stat(&target).unwrap().size, 2);
    }

    #[test]
    fn renaming_moves_the_name_and_conserves_the_bytes() {
        let (_dir, path) = fixture();
        let before = byte_sum(&LocalFs, &path);
        let count = LocalFs.read_dir(&path).unwrap().len();

        LocalFs
            .rename(&path.child("keeper.txt"), &path.child("renamed.txt"))
            .unwrap();

        let entries = LocalFs.read_dir(&path).unwrap();
        assert_eq!(
            entries.len(),
            count,
            "a rename creates and destroys nothing"
        );
        assert_eq!(byte_sum(&LocalFs, &path), before);
        assert!(!names(&entries).contains("keeper.txt"));
        assert_eq!(find(&entries, "renamed.txt").size, "keep me".len() as u64);
    }

    #[test]
    fn removing_a_file_leaves_every_sibling_alone() {
        let (_dir, path) = fixture();
        let before = names(&LocalFs.read_dir(&path).unwrap());
        let removed_size = LocalFs.stat(&path.child("other.bin")).unwrap().size;
        let before_bytes = byte_sum(&LocalFs, &path);

        LocalFs.remove_file(&path.child("other.bin")).unwrap();

        let after = names(&LocalFs.read_dir(&path).unwrap());
        assert_eq!(after.len(), before.len() - 1);
        assert!(after.is_subset(&before), "nothing new may appear");
        assert!(!after.contains("other.bin"));
        assert_eq!(byte_sum(&LocalFs, &path), before_bytes - removed_size);
    }

    #[test]
    fn removing_an_empty_directory_succeeds() {
        let (_dir, path) = fixture();
        LocalFs.remove_dir(&path.child("sub")).unwrap();
        assert!(!names(&LocalFs.read_dir(&path).unwrap()).contains("sub"));
    }

    #[test]
    fn removing_a_non_empty_directory_reports_not_empty_and_destroys_nothing() {
        // The invariant that makes recursion safe to leave to the engine: a
        // refused remove_dir must not have deleted part of the contents.
        let (_dir, path) = fixture();
        let sub = path.child("sub");
        LocalFs
            .create_file(&sub.child("inner.txt"))
            .unwrap()
            .write_all(b"still here")
            .unwrap();

        assert_eq!(LocalFs.remove_dir(&sub), Err(VfsError::NotEmpty));

        let inner = LocalFs.read_dir(&sub).unwrap();
        assert_eq!(names(&inner), BTreeSet::from(["inner.txt".to_string()]));
        assert_eq!(find(&inner, "inner.txt").size, "still here".len() as u64);
    }

    #[test]
    fn removing_a_directory_as_a_file_reports_is_a_directory() {
        let (_dir, path) = fixture();
        assert_eq!(
            LocalFs.remove_file(&path.child("sub")),
            Err(VfsError::IsADirectory)
        );
        assert!(names(&LocalFs.read_dir(&path).unwrap()).contains("sub"));
    }

    #[test]
    fn removing_something_that_is_gone_reports_not_found() {
        let (_dir, path) = fixture();
        assert_eq!(
            LocalFs.remove_file(&path.child("never")),
            Err(VfsError::NotFound)
        );
        assert_eq!(
            LocalFs.remove_dir(&path.child("never")),
            Err(VfsError::NotFound)
        );
    }

    #[test]
    fn trashing_something_that_is_gone_reports_not_found() {
        // Needs no trash redirection: the call fails while resolving the path,
        // before any trash directory is consulted, so nothing leaves the
        // tempdir. Pins the error mapping, which is a per-platform unwrap of
        // the crate's own error shape and was wrong on the first attempt.
        let (_dir, path) = fixture();
        assert_eq!(LocalFs.trash(&path.child("never")), Err(VfsError::NotFound));
    }

    #[test]
    fn a_stamped_time_survives_a_stat_round_trip() {
        // What lets a copy keep the original's date instead of stamping the
        // moment it was copied.
        let (_dir, path) = fixture();
        let target = path.child("keeper.txt");
        let stamp = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000_000);

        LocalFs.set_modified(&target, stamp).unwrap();

        assert_eq!(LocalFs.stat(&target).unwrap().modified, stamp);
    }

    #[test]
    fn a_directory_cannot_be_stamped() {
        // Pins the documented limit rather than leaving it to a comment:
        // stamping a directory needs a writable handle to it, which no
        // platform hands out. Directory timestamps are a known gap.
        let (_dir, path) = fixture();
        assert!(LocalFs
            .set_modified(&path.child("sub"), SystemTime::UNIX_EPOCH)
            .is_err());
    }
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

        let entries = LocalFs.read_dir(&LocalFs::vfs_path(dir.path())).unwrap();

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

        let entries = LocalFs.read_dir(&LocalFs::vfs_path(dir.path())).unwrap();

        assert_eq!(entries.len(), 2, "the dangling link must not hide the file");
        let dangling = find(&entries, "dangling");
        assert_eq!(dangling.kind, EntryKind::Symlink(SymlinkTarget::Broken));
        assert_eq!(dangling.size, 0);
        assert!(!dangling.is_dir());
    }
}

/// Permissions, and the copy that has to keep them.
///
/// Unix only: on Windows `std` can set the read-only flag and nothing else,
/// so there is no round trip to assert. `LocalFs` reads the attributes on
/// both platforms.
#[cfg(unix)]
mod attributes {
    use std::os::unix::fs::PermissionsExt;

    use super::*;

    fn mode_of(path: &std::path::Path) -> u32 {
        fs::metadata(path).unwrap().permissions().mode() & 0o7777
    }

    #[test]
    fn permissions_survive_a_stat_round_trip() {
        let dir = TempDir::new().unwrap();
        let native = dir.path().join("script.sh");
        fs::write(&native, "#!/bin/sh\n").unwrap();
        fs::set_permissions(&native, fs::Permissions::from_mode(0o755)).unwrap();

        let entry = LocalFs.stat(&LocalFs::vfs_path(&native)).unwrap();

        let elsewhere = dir.path().join("copy.sh");
        fs::write(&elsewhere, "").unwrap();
        LocalFs
            .set_attributes(&LocalFs::vfs_path(&elsewhere), entry.attributes)
            .unwrap();

        assert_eq!(mode_of(&elsewhere), 0o755);
    }

    #[test]
    fn a_listing_reports_each_entry_with_its_own_permissions() {
        let dir = TempDir::new().unwrap();
        for (name, mode) in [("open.txt", 0o644), ("script.sh", 0o755)] {
            let path = dir.path().join(name);
            fs::write(&path, "x").unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap();
        }

        let entries = LocalFs.read_dir(&LocalFs::vfs_path(dir.path())).unwrap();

        assert_ne!(
            find(&entries, "open.txt").attributes,
            find(&entries, "script.sh").attributes,
            "two files with different modes must not read alike"
        );
    }
}

/// Random access, which the viewer is built on: it never holds a file, only an
/// offset into one.
mod read_at {
    use super::*;

    /// A file of known bytes, and its path.
    fn file(contents: &[u8]) -> (TempDir, VfsPath) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("bytes.bin");
        std::fs::write(&path, contents).unwrap();
        let vfs = VfsPath::new(path.to_str().unwrap());
        (dir, vfs)
    }

    #[test]
    fn a_window_is_the_same_bytes_the_whole_file_has_there() {
        // The invariant that makes paging trustworthy: reading a window is
        // reading the file and slicing it, however the window is placed.
        let whole: Vec<u8> = (0..=255u8).cycle().take(5000).collect();
        let (_dir, path) = file(&whole);

        for (offset, len) in [(0, 10), (1, 1), (255, 512), (4990, 10), (1234, 1000)] {
            let window = LocalFs.read_at(&path, offset as u64, len).unwrap();
            assert_eq!(
                window,
                &whole[offset..offset + len],
                "window at {offset} of {len}"
            );
        }
    }

    #[test]
    fn a_read_running_off_the_end_comes_back_short() {
        // What every caller wants at the end of a file. An error instead would
        // have each of them clamping against a size that may have changed
        // since they asked.
        let (_dir, path) = file(b"0123456789");

        let window = LocalFs.read_at(&path, 6, 100).unwrap();

        assert_eq!(window, b"6789");
    }

    #[test]
    fn a_read_entirely_past_the_end_comes_back_empty() {
        let (_dir, path) = file(b"0123456789");

        assert!(LocalFs.read_at(&path, 10, 100).unwrap().is_empty());
        assert!(LocalFs.read_at(&path, 1_000_000, 100).unwrap().is_empty());
    }

    #[test]
    fn an_empty_read_is_empty_rather_than_an_error() {
        let (_dir, path) = file(b"0123456789");

        assert!(LocalFs.read_at(&path, 0, 0).unwrap().is_empty());
    }

    #[test]
    fn a_directory_or_a_missing_file_is_an_error_not_a_panic() {
        let dir = TempDir::new().unwrap();
        let root = VfsPath::new(dir.path().to_str().unwrap());

        assert!(LocalFs.read_at(&root, 0, 10).is_err(), "a directory");
        assert!(
            LocalFs.read_at(&root.child("nothing"), 0, 10).is_err(),
            "a missing file"
        );
    }
}
