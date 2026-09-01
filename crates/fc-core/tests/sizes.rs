//! Counting what a folder holds, and the listing that carries the answer.
//!
//! Conservation again: the number a folder reports is the sum of the files
//! under it, checked against the same files added up another way. A scan that
//! counts a file twice, or misses a subtree, fails here rather than in
//! somebody's decision about what to delete.

mod common;

use common::delegate_vfs;

use std::io::{Read, Write};

use fc_core::listing::{Listing, Sort, SortKey, SortOrder};
use fc_core::ops::CancelToken;
use fc_core::sizes::{self, Measured};
use fc_core::vfs::{Entry, LocalFs, VfsError, VfsPath, VirtualFs};
use tempfile::TempDir;

fn tree() -> (TempDir, VfsPath) {
    let dir = TempDir::new().unwrap();
    let root = dir.path().join("tree");
    common::build_tree(&root);
    (dir, LocalFs::vfs_path(&root))
}

/// Everything under `root`, added up by walking it here rather than there.
fn total_below(root: &std::path::Path) -> u64 {
    let mut total = 0;
    for entry in std::fs::read_dir(root).unwrap() {
        let entry = entry.unwrap();
        let kind = entry.file_type().unwrap();
        if kind.is_dir() {
            total += total_below(&entry.path());
        } else {
            total += entry.metadata().unwrap().len();
        }
    }
    total
}

#[test]
fn a_folder_reports_the_bytes_of_everything_under_it() {
    // The conservation statement. Counted by this module against counted by
    // the test, two different walks over the same tree.
    let (dir, root) = tree();

    let measured = sizes::measure(&LocalFs, &root, &CancelToken::new());

    assert_eq!(measured.bytes, total_below(&dir.path().join("tree")));
    assert!(measured.complete);
}

#[test]
fn an_empty_folder_measures_zero_and_that_is_an_answer() {
    // The case that forces the `measured` flag: zero bytes is a real count,
    // and a row that fell back to `<DIR>` for it would be the one thing this
    // feature exists to get right.
    let (dir, root) = tree();

    let measured = sizes::measure(&LocalFs, &root.child("emptydir"), &CancelToken::new());

    assert_eq!(measured.bytes, 0);
    assert!(measured.complete);
    assert!(dir.path().join("tree/emptydir").is_dir());
}

#[test]
fn a_subtree_that_cannot_be_read_makes_the_answer_incomplete() {
    // A number that silently omitted an unreadable subtree is a number
    // nobody should trust — and trusting it is the whole reason somebody
    // pressed the key. Injected through a decorator rather than with
    // `chmod 000`, which stops nothing when the suite runs as root.
    let (_dir, root) = tree();
    let refusing = Closed {
        inner: LocalFs,
        closed: root.child("sub"),
    };

    let measured = sizes::measure(&refusing, &root, &CancelToken::new());

    assert!(!measured.complete, "the refusal was not reported");
    // What it *could* read is still counted, rather than the whole answer
    // being thrown away.
    let whole = sizes::measure(&LocalFs, &root, &CancelToken::new());
    assert!(measured.bytes < whole.bytes);
    assert!(measured.bytes > 0);
}

#[test]
fn a_cancelled_scan_says_its_answer_is_partial() {
    // The same rule as an unreadable subtree, for the same reason: the number
    // is a lower bound, and saying otherwise is the one lie this module
    // exists to avoid.
    let (_dir, root) = tree();
    let cancel = CancelToken::new();
    cancel.cancel();

    let measured = sizes::measure(&LocalFs, &root, &cancel);

    assert!(!measured.complete);
    assert_eq!(measured.bytes, 0);
}

#[test]
fn a_walk_cancelled_while_it_runs_delivers_nothing() {
    // Partial is what the *number* is; delivering it is a separate question,
    // and the answer is no. The walk was cancelled because somebody pressed
    // the key again, and the press that cancelled it is about to produce a
    // real answer — a lower bound landing on top of that is how a folder's
    // size came to change every time it was asked for.
    //
    // Cancelled **mid-walk**, which is the case that matters: a token already
    // cancelled before the walk starts is caught by the loop's own check
    // before anything is measured, so a test written that way passes with the
    // guard removed. This one cancels from inside the first `read_dir`.
    let (_dir, root) = tree();
    let cancel = CancelToken::new();
    let fs = std::sync::Arc::new(CancelsMidWalk {
        inner: LocalFs,
        cancel: cancel.clone(),
    });

    let answers = sizes::spawn(fs, root, vec!["sub".to_string()], cancel);

    assert!(
        answers.recv_blocking().is_err(),
        "a cancelled walk sent an answer nobody asked for"
    );
}

#[test]
fn the_same_tree_measures_the_same_twice() {
    // The conservation invariant behind the report: a folder's size is a
    // property of the folder, so asking twice is asking once. What broke it
    // was never the arithmetic — it was a cancelled walk's lower bound
    // arriving after a complete one.
    let (_dir, root) = tree();
    let full = CancelToken::new();

    let first = sizes::measure(&LocalFs, &root, &full);
    let second = sizes::measure(&LocalFs, &root, &full);

    assert_eq!(first, second);
    assert!(first.complete);
}

#[test]
fn a_partial_answer_never_replaces_a_complete_one() {
    // The second guard, and it is not the same as the first: a walk cancelled
    // a keystroke ago can already be past its own check and on its way here.
    // A folder whose size is known must not go back to being a guess.
    let (_dir, root) = tree();
    let mut listing = Listing::load(&LocalFs, root).unwrap();
    let row = listing.set_measured("sub", 4096, true).expect("the row");

    assert_eq!(
        listing.set_measured("sub", 12, false),
        None,
        "a lower bound overwrote a counted size"
    );
    assert_eq!(listing.get(row).expect("an entry").size, 4096);
    assert_eq!(listing.measured_at(row), Some(true));
}

#[test]
fn a_partial_answer_still_lands_where_nothing_is_known_yet() {
    // The guard is about not going backwards, not about refusing lower
    // bounds: an unreadable subtree is the honest `+` and has to show.
    let (_dir, root) = tree();
    let mut listing = Listing::load(&LocalFs, root).unwrap();

    let row = listing.set_measured("sub", 12, false).expect("the row");

    assert_eq!(listing.measured_at(row), Some(false));
}

#[test]
fn a_measured_size_lands_on_the_row_of_that_name() {
    // Sizes arrive by name, the rule the marks already follow, so an answer
    // finds its row even after a re-sort has moved it.
    let (_dir, root) = tree();
    let mut listing = Listing::load(&LocalFs, root).unwrap();

    let row = listing.set_measured("sub", 4096, true).expect("the row");

    assert!(listing.is_measured(row));
    assert_eq!(listing.get(row).expect("an entry").size, 4096);
}

#[test]
fn a_measured_folder_counts_towards_the_marked_total() {
    // The whole reason the answer goes into `Entry::size`: the accumulated
    // total the request asked for is the status line's, already rendered.
    let (_dir, root) = tree();
    let mut listing = Listing::load(&LocalFs, root).unwrap();
    let row = listing.set_measured("sub", 4096, true).expect("the row");
    listing.set_selected(row, true);

    assert_eq!(listing.selection_summary().bytes, 4096);
    assert_eq!(listing.selection_summary().count, 1);
}

#[test]
fn an_answer_finds_its_row_after_the_view_was_re_sorted() {
    // Answers arrive one at a time and a person can re-sort between two of
    // them. Keying by name rather than by row is what makes the second answer
    // still land on its own folder — the same rule the marks follow.
    let (_dir, root) = tree();
    let mut listing = Listing::load(&LocalFs, root).unwrap();
    listing.set_measured("sub", 4096, true);

    // Descending by size now puts `sub` at the top of the directories, so the
    // next answer's row is deliberately **not** the first one — an answer
    // that went by position rather than by name would land on `sub`.
    listing.set_sort(Sort::new(SortKey::Size, SortOrder::Descending));

    let row = listing.set_measured("emptydir", 7, true).expect("the row");

    assert_eq!(
        listing.get(row).expect("an entry").name,
        "emptydir",
        "the answer landed on somebody else's row"
    );
    assert_eq!(listing.get(row).expect("an entry").size, 7);
    assert_eq!(
        listing
            .iter()
            .find(|entry| entry.name == "sub")
            .expect("sub")
            .size,
        4096,
        "the earlier answer was overwritten"
    );
}

#[test]
fn a_partial_count_says_so_on_its_row() {
    // A lower bound has to look like one. The row keeps the number — it is
    // the best answer there is — and carries that it is not the whole story.
    let (_dir, root) = tree();
    let mut listing = Listing::load(&LocalFs, root).unwrap();

    let row = listing.set_measured("sub", 4096, false).expect("the row");

    assert!(listing.is_measured(row));
    assert_eq!(listing.measured_at(row), Some(false));
}

#[test]
fn nothing_is_measured_until_something_measures_it() {
    let (_dir, root) = tree();
    let listing = Listing::load(&LocalFs, root).unwrap();

    assert!((0..listing.len()).all(|index| !listing.is_measured(index)));
}

#[test]
fn a_re_read_forgets_the_sizes_but_keeps_the_marks() {
    // Two rules in one assertion because they are two halves of the same
    // decision: what a re-read is entitled to carry over. A mark is the
    // user's own and survives; a count is the filesystem's and does not.
    let (_dir, root) = tree();
    let mut listing = Listing::load(&LocalFs, root.clone()).unwrap();
    let row = listing.set_measured("sub", 4096, true).expect("the row");
    listing.set_selected(row, true);

    listing.reload(&LocalFs).unwrap();

    let row = (0..listing.len())
        .find(|&index| listing.get(index).is_some_and(|entry| entry.name == "sub"))
        .expect("sub is still there");
    assert!(
        !listing.is_measured(row),
        "a stale size survived the re-read"
    );
    assert!(listing.is_selected(row), "the mark did not survive");
}

#[test]
fn only_directories_are_offered_to_the_scan() {
    // Files already know their size, and `..` is a navigation control rather
    // than an entry. Scanning either would be work for an answer already in
    // hand.
    let (_dir, root) = tree();
    let listing = Listing::load(&LocalFs, root).unwrap();

    let names = listing.directory_names(false);

    assert!(names.contains(&"sub".to_string()));
    assert!(names.contains(&"emptydir".to_string()));
    assert!(!names.iter().any(|name| name.ends_with(".txt")));
    assert!(!names.iter().any(|name| name == ".."));
}

#[test]
fn the_marks_narrow_what_is_offered() {
    let (_dir, root) = tree();
    let mut listing = Listing::load(&LocalFs, root).unwrap();
    let row = (0..listing.len())
        .find(|&index| listing.get(index).is_some_and(|entry| entry.name == "sub"))
        .expect("sub");
    listing.set_selected(row, true);

    assert_eq!(listing.directory_names(true), ["sub"]);
}

#[test]
fn every_answer_reaches_the_receiver() {
    // The streaming half: one send per folder asked for, in the order they
    // were asked for, so a pane can splice each row as it lands.
    let (_dir, root) = tree();
    let cancel = CancelToken::new();

    let answers = sizes::spawn(
        std::sync::Arc::new(LocalFs),
        root,
        vec!["sub".to_string(), "emptydir".to_string()],
        cancel,
    );

    let mut got: Vec<(String, Measured)> = Vec::new();
    while let Ok(answer) = answers.recv_blocking() {
        got.push(answer);
    }

    assert_eq!(got.len(), 2, "{got:?}");
    assert_eq!(got[0].0, "sub");
    assert!(got[0].1.bytes > 0);
    assert_eq!(got[1].0, "emptydir");
    assert_eq!(got[1].1.bytes, 0);
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

/// A filesystem that cancels the walk reading it, from inside the read.
///
/// The only way to reach the case the report is about: a walk that has
/// already started, is part way through, and is stopped by the next
/// keystroke.
struct CancelsMidWalk {
    inner: LocalFs,
    cancel: CancelToken,
}

impl CancelsMidWalk {
    fn read_dir_impl(&self, path: &VfsPath) -> Result<Vec<Entry>, VfsError> {
        let entries = self.inner.read_dir(path);
        self.cancel.cancel();
        entries
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

delegate_vfs!(CancelsMidWalk);
