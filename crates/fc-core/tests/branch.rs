//! The branch view: every file below a directory, as one flat listing.
//!
//! Assertions are conservation statements where they can be — every file in
//! the tree appears exactly once, and their bytes add up to the tree's — for
//! the reason `ops.rs` gives: a walk that finds the right file and quietly
//! drops a sibling passes the narrow kind of test and fails these.

mod common;

use common::delegate_vfs;

use std::collections::BTreeSet;
use std::io::{Read, Write};

use fc_core::branch;
use fc_core::listing::{Listing, Sort, SortKey, SortOrder};
use fc_core::ops::CancelToken;
use fc_core::vfs::{Entry, LocalFs, VfsError, VfsPath, VirtualFs};
use tempfile::TempDir;

/// The tree `common::build_tree` builds, as relative names.
const EVERY_FILE: [&str; 5] = [
    "a.txt",
    "empty.txt",
    "Ünïcødé — ✓.md",
    "sub/b.bin",
    "sub/deep/c.txt",
];

fn tree() -> (TempDir, VfsPath) {
    let dir = TempDir::new().unwrap();
    let root = dir.path().join("tree");
    common::build_tree(&root);
    let path = LocalFs::vfs_path(&root);
    (dir, path)
}

fn names(listing: &Listing) -> Vec<String> {
    listing
        .iter()
        .map(|entry| entry.name.clone())
        .collect::<Vec<_>>()
}

#[test]
fn every_file_in_the_tree_appears_exactly_once() {
    // The conservation statement the whole feature rests on. A walk that
    // misses a directory, or descends one twice, fails here rather than in
    // somebody's copy of a tree.
    let (_dir, root) = tree();

    let found = branch::walk(&LocalFs, &root, &CancelToken::new());

    let names: Vec<String> = found.iter().map(|entry| entry.name.clone()).collect();
    let unique: BTreeSet<&String> = names.iter().collect();
    assert_eq!(unique.len(), names.len(), "something was walked twice");
    assert_eq!(
        unique,
        EVERY_FILE
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .iter()
            .collect(),
    );
}

#[test]
fn the_bytes_of_the_flat_list_are_the_bytes_of_the_tree() {
    // The other half of the conservation: names could all be there while the
    // entries behind them were somebody else's.
    let (_dir, root) = tree();

    let found = branch::walk(&LocalFs, &root, &CancelToken::new());

    let walked: u64 = found.iter().map(|entry| entry.size).sum();
    let expected: u64 = EVERY_FILE
        .iter()
        .map(|name| {
            LocalFs
                .stat(&root.child(name))
                .expect("the fixture's file")
                .size
        })
        .sum();
    assert_eq!(walked, expected);
}

#[test]
fn directories_are_walked_but_never_listed() {
    // A flat list is a list of leaves. `sub/deep` is descended — `c.txt` is
    // proof — and is not itself a row, because a directory row in a branch
    // view would be a row whose contents are also rows.
    let (_dir, root) = tree();

    let found = branch::walk(&LocalFs, &root, &CancelToken::new());

    assert!(found.iter().all(|entry| !entry.is_dir()));
    assert!(found.iter().any(|entry| entry.name == "sub/deep/c.txt"));
    // `emptydir` holds nothing, so it contributes nothing at all.
    assert!(!found.iter().any(|entry| entry.name.contains("emptydir")));
}

#[test]
fn a_row_is_named_by_its_path_and_still_resolves_to_the_file() {
    // The invariant this feature widens, checked from both ends: the name
    // carries the separator, and `dir.child(name)` is still the real path
    // because `VfsPath` normalises component by component.
    let (_dir, root) = tree();
    let listing = Listing::branch(
        root.clone(),
        branch::walk(&LocalFs, &root, &CancelToken::new()),
    );

    let deep = names(&listing)
        .into_iter()
        .find(|name| name.ends_with("c.txt"))
        .expect("the deep file");
    assert_eq!(deep, "sub/deep/c.txt");

    // `names` walks the visible rows, `..` included, so its index *is* the
    // row index.
    let index = names(&listing).iter().position(|n| *n == deep).unwrap();
    let path = listing.path_at(index).expect("a path");
    let mut contents = String::new();
    LocalFs
        .open_read(&path)
        .unwrap()
        .read_to_string(&mut contents)
        .unwrap();
    assert_eq!(contents, "deep");
}

#[test]
fn a_file_under_a_hidden_directory_is_hidden_and_needs_no_second_walk() {
    // Decided during the walk and carried on the row, so `Ctrl+H` stays what
    // it is: a rearrangement of what is already loaded, costing no
    // filesystem access at all.
    let (dir, root) = tree();
    std::fs::create_dir(dir.path().join("tree/.private")).unwrap();
    std::fs::write(dir.path().join("tree/.private/secret.txt"), "shh").unwrap();

    let mut listing = Listing::branch(
        root.clone(),
        branch::walk(&LocalFs, &root, &CancelToken::new()),
    );

    assert!(
        !names(&listing).iter().any(|name| name.contains("secret")),
        "a file under a hidden directory was shown"
    );
    listing.toggle_hidden();
    assert!(
        names(&listing)
            .iter()
            .any(|name| name == ".private/secret.txt"),
        "revealing hidden files did not reveal it: {:?}",
        names(&listing)
    );
}

#[test]
fn sorting_by_name_keeps_each_directory_together() {
    // What naming a row by its path buys: the files of one directory are
    // adjacent, because their names share a prefix.
    let (_dir, root) = tree();
    let mut listing = Listing::branch(
        root.clone(),
        branch::walk(&LocalFs, &root, &CancelToken::new()),
    );
    listing.set_sort(Sort::new(SortKey::Name, SortOrder::Ascending));

    let under_sub: Vec<usize> = names(&listing)
        .iter()
        .enumerate()
        .filter(|(_, name)| name.starts_with("sub/"))
        .map(|(index, _)| index)
        .collect();
    assert_eq!(under_sub.len(), 2);
    assert_eq!(under_sub[1], under_sub[0] + 1, "sub/ was split apart");
}

#[test]
fn the_filter_matches_across_the_separator() {
    // A decision, not an accident: the filter matches what is on screen, and
    // what is on screen is the path. That is what makes "filter to one
    // directory, mark, copy" work.
    let (_dir, root) = tree();
    let mut listing = Listing::branch(
        root.clone(),
        branch::walk(&LocalFs, &root, &CancelToken::new()),
    );

    listing.set_filter("sub");

    // The `..` row is a navigation control and no filter takes it away.
    let shown: Vec<String> = names(&listing)
        .into_iter()
        .filter(|name| name != "..")
        .collect();
    assert_eq!(shown.len(), 2, "{shown:?}");
    assert!(shown.iter().all(|name| name.starts_with("sub/")));
}

#[test]
fn marks_are_kept_by_name_so_they_survive_a_fresh_walk() {
    // The same rule a reload follows, and it works better here than in a
    // plain listing: a relative path is unique where a bare name would not
    // be, so two files called `c.txt` in different directories keep their
    // own marks.
    let (_dir, root) = tree();
    let mut listing = Listing::branch(
        root.clone(),
        branch::walk(&LocalFs, &root, &CancelToken::new()),
    );
    let index = names(&listing)
        .iter()
        .position(|name| name == "sub/deep/c.txt")
        .expect("the deep file");
    listing.set_selected(index, true);
    let marked = listing.selected_names();
    assert_eq!(marked, ["sub/deep/c.txt"]);

    let mut walked_again = Listing::branch(
        root.clone(),
        branch::walk(&LocalFs, &root, &CancelToken::new()),
    );
    walked_again.set_selected_names(&marked);

    assert_eq!(walked_again.selected_names(), marked);
    assert_eq!(
        walked_again.selection_summary().count,
        1,
        "the mark spread to a namesake"
    );
}

#[test]
fn a_subdirectory_that_cannot_be_read_costs_only_itself() {
    // One directory nobody may enter must not cost the rest of the tree — the
    // rule the copy engine keeps about a tree it cannot fully read.
    //
    // Injected through a decorator rather than with `chmod 000`: these tests
    // run as root often enough that a mode of zero stops nothing.
    //
    // **Which** directory is closed is not arbitrary. The walk holds pending
    // directories on a stack, so the one the backend lists *last* is reached
    // *first* — and only closing that one leaves siblings still pending when
    // the failure happens. Closing any other, a walk that gave up on the
    // failure would look exactly like one that skipped it. The test asks the
    // backend for the order rather than guessing at it.
    let (dir, root) = tree();
    for name in ["one", "two"] {
        std::fs::create_dir(dir.path().join("tree").join(name)).unwrap();
        std::fs::write(
            dir.path().join("tree").join(name).join("in-there.txt"),
            name,
        )
        .unwrap();
    }
    let reached_first = LocalFs
        .read_dir(&root)
        .unwrap()
        .into_iter()
        .rfind(|entry| entry.is_dir())
        .expect("the fixture's directories")
        .name;

    let refusing = Closed {
        inner: LocalFs,
        closed: root.child(&reached_first),
    };
    let found = branch::walk(&refusing, &root, &CancelToken::new());

    let names: Vec<String> = found.iter().map(|entry| entry.name.clone()).collect();
    assert!(
        !names
            .iter()
            .any(|name| name.starts_with(&format!("{reached_first}/"))),
        "the closed directory was read anyway: {names:?}"
    );
    // Everything else, exactly: the tree's own files plus the two added ones,
    // less whatever lived under the directory that refused. Stated as the set
    // rather than as a count, since which directory refuses depends on the
    // backend's order.
    let expected: BTreeSet<String> = EVERY_FILE
        .iter()
        .map(|name| name.to_string())
        .chain([
            "one/in-there.txt".to_string(),
            "two/in-there.txt".to_string(),
        ])
        .filter(|name| !name.starts_with(&format!("{reached_first}/")))
        .collect();
    assert_eq!(
        names.into_iter().collect::<BTreeSet<String>>(),
        expected,
        "the failure cost more than the one directory"
    );
}

/// A backend that refuses to list one directory, and is `LocalFs` otherwise.
struct Closed {
    inner: LocalFs,
    closed: VfsPath,
}

impl Closed {
    fn read_dir_impl(&self, path: &VfsPath) -> Result<Vec<Entry>, VfsError> {
        if *path == self.closed {
            return Err(VfsError::PermissionDenied);
        }
        self.inner.read_dir(path)
    }

    fn rename_impl(&self, from: &VfsPath, to: &VfsPath) -> Result<(), VfsError> {
        self.inner.rename(from, to)
    }

    fn open_read_impl(&self, path: &VfsPath) -> Result<Box<dyn Read + Send>, VfsError> {
        self.inner.open_read(path)
    }

    fn create_file_impl(&self, path: &VfsPath) -> Result<Box<dyn Write + Send>, VfsError> {
        self.inner.create_file(path)
    }

    fn trash_impl(&self, path: &VfsPath) -> Result<(), VfsError> {
        self.inner.trash(path)
    }
}

delegate_vfs!(Closed);

#[test]
fn a_cancelled_walk_hands_back_what_it_had() {
    // Cancelled before it starts is the extreme of the same rule: the caller
    // asked to stop, and what to do with a partial answer is the caller's
    // decision, not the walk's.
    let (_dir, root) = tree();
    let cancel = CancelToken::new();
    cancel.cancel();

    let found = branch::walk(&LocalFs, &root, &cancel);

    assert!(found.is_empty());
}

#[test]
fn a_branch_listing_says_it_is_one() {
    // What a caller asks before reaching for `reload`, which re-reads one
    // directory and would silently turn this back into a plain listing.
    let (_dir, root) = tree();

    let walked = Listing::branch(
        root.clone(),
        branch::walk(&LocalFs, &root, &CancelToken::new()),
    );
    let plain = Listing::load(&LocalFs, root).unwrap();

    assert!(walked.is_branch());
    assert!(!plain.is_branch());
}
