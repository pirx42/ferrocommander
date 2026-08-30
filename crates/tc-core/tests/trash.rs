//! Behavioral test for [`LocalFs::trash`].
//!
//! **Its own test binary, holding exactly one test, on purpose.** Redirecting
//! the trash means setting `XDG_DATA_HOME`, which is process-global; a second
//! test in the same binary would race it. Cargo gives every integration test
//! file its own process, so one file with one test is the isolation.
//!
//! Linux only, and the boundary is sharper than "unix": what this asserts is
//! the **freedesktop** trash layout, redirected through `XDG_DATA_HOME`. The
//! macOS backend ignores XDG and files deletions under `~/.Trash` — so on a
//! Mac this test would not merely fail, it would trash its fixtures **for
//! real**, into the account's actual bin. That is the same failure the
//! end-to-end harness once shipped on Linux before it pinned `XDG_DATA_HOME`
//! ([`docs/ui-shell.md`]), met from the other side. The Windows recycle bin
//! cannot be redirected either. `LocalFs::trash` itself works on all three
//! platforms; recoverability is simply only *assertable* where the spec gives
//! the trash an address.
#![cfg(target_os = "linux")]

use std::collections::BTreeSet;
use std::fs;

use tc_core::vfs::{LocalFs, VirtualFs};

/// Where the freedesktop spec puts trashed files below `XDG_DATA_HOME`.
const TRASH_FILES: &str = "Trash/files";

#[test]
fn trashing_removes_the_entry_from_its_directory_and_keeps_it_recoverable() {
    let data_home = tempfile::TempDir::new().unwrap();
    // Safe in edition 2021, and this binary has no other test to race with.
    std::env::set_var("XDG_DATA_HOME", data_home.path());

    let dir = tempfile::TempDir::new().unwrap();
    let payload = "the user may want this back";
    fs::write(dir.path().join("doomed.txt"), payload).unwrap();
    fs::write(dir.path().join("keeper.txt"), "untouched").unwrap();
    let path = LocalFs::vfs_path(dir.path());

    LocalFs.trash(&path.child("doomed.txt")).unwrap();

    // Gone from where it was, and nothing else went with it.
    let names: BTreeSet<String> = LocalFs
        .read_dir(&path)
        .unwrap()
        .iter()
        .map(|entry| entry.name.clone())
        .collect();
    assert_eq!(names, BTreeSet::from(["keeper.txt".to_string()]));

    // Recoverable is the whole difference between trash and permanent delete,
    // so the test checks the file arrived rather than only that it vanished.
    let trashed: Vec<_> = fs::read_dir(data_home.path().join(TRASH_FILES))
        .expect("the trash directory was created")
        .map(|entry| entry.unwrap().path())
        .collect();
    assert_eq!(trashed.len(), 1);
    assert_eq!(fs::read_to_string(&trashed[0]).unwrap(), payload);
}
