//! Behavioral tests for the operation engine.
//!
//! Assertions are conservation statements — file counts, byte sums, per-path
//! contents, what the source still holds — rather than checks on the one path
//! a call names. An engine that writes the right file and quietly loses a
//! sibling passes the narrow kind of test and fails these.

use std::io::{Read, Write};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use tc_core::ops::{
    self, Answer, ApplyToAll, CancelToken, ConflictResolver, DeleteMode, Destination, Job, Outcome,
    Progress, Report, Resolution, Silent,
};
use tc_core::vfs::{Entry, LocalFs, VfsError, VfsPath, VirtualFs};
use tempfile::TempDir;

mod common;

use common::{
    byte_sum, delegate_vfs, exists, file_count, fixture, snapshot, NoConflictsExpected, Scripted,
    Snapshot,
};

// -------------------------------------------------------------- decorators

/// Counts the work a job does, so a test can prove it did none.
struct CountingBackend<'a> {
    inner: &'a dyn VirtualFs,
    reads: AtomicUsize,
    walks: AtomicUsize,
}

impl<'a> CountingBackend<'a> {
    fn new(inner: &'a dyn VirtualFs) -> Self {
        CountingBackend {
            inner,
            reads: AtomicUsize::new(0),
            walks: AtomicUsize::new(0),
        }
    }
}

/// Reports every rename as crossing a filesystem, which is the only way to
/// exercise the copy+delete fallback on a one-device test box.
struct AlwaysCrossDevice<'a> {
    inner: &'a dyn VirtualFs,
}

/// Cancels the job partway through the first file it is asked to read, which
/// is the only way to land a cancel *inside* the copy loop rather than
/// between two tasks.
struct CancelsMidFile<'a> {
    inner: &'a dyn VirtualFs,
    token: CancelToken,
}

/// Pulls the cancel after a read that filled the whole buffer.
///
/// A full buffer means the file has more to come, so the destination is
/// genuinely half written at that moment. Cancelling after a *short* read
/// would stop on a file that was already complete, and the rollback could be
/// deleted entirely without any test noticing — which is what the first
/// version of this did.
struct CancelAfterFirstChunk {
    inner: Box<dyn Read + Send>,
    token: CancelToken,
}

impl Read for CancelAfterFirstChunk {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        let read = self.inner.read(buffer)?;
        if read == buffer.len() {
            self.token.cancel();
        }
        Ok(read)
    }
}

/// Fails a read partway through, to exercise the rollback that a cancel is
/// not the only way to reach.
struct FailsMidRead<'a> {
    inner: &'a dyn VirtualFs,
    /// Bytes handed over before the error.
    good: usize,
}

struct FailAfter {
    inner: Box<dyn Read + Send>,
    left: usize,
}

impl Read for FailAfter {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        if self.left == 0 {
            return Err(std::io::Error::other("the disk gave up"));
        }
        let room = buffer.len().min(self.left);
        let read = self.inner.read(&mut buffer[..room])?;
        self.left -= read;
        Ok(read)
    }
}

/// Refuses every trash request, so the failure path has a test.
struct TrashRefuses<'a> {
    inner: &'a dyn VirtualFs,
}

/// Records what was handed to the trash instead of really trashing it.
struct RecordingTrash<'a> {
    inner: &'a dyn VirtualFs,
    trashed: Mutex<Vec<VfsPath>>,
}

impl CountingBackend<'_> {
    fn create_file_impl(&self, path: &VfsPath) -> Result<Box<dyn Write + Send>, VfsError> {
        self.inner.create_file(path)
    }
    fn read_dir_impl(&self, path: &VfsPath) -> Result<Vec<Entry>, VfsError> {
        self.walks.fetch_add(1, Ordering::Relaxed);
        self.inner.read_dir(path)
    }
    fn rename_impl(&self, from: &VfsPath, to: &VfsPath) -> Result<(), VfsError> {
        self.inner.rename(from, to)
    }
    fn open_read_impl(&self, path: &VfsPath) -> Result<Box<dyn Read + Send>, VfsError> {
        self.reads.fetch_add(1, Ordering::Relaxed);
        self.inner.open_read(path)
    }
    fn trash_impl(&self, path: &VfsPath) -> Result<(), VfsError> {
        self.inner.trash(path)
    }
}

impl AlwaysCrossDevice<'_> {
    fn create_file_impl(&self, path: &VfsPath) -> Result<Box<dyn Write + Send>, VfsError> {
        self.inner.create_file(path)
    }
    fn read_dir_impl(&self, path: &VfsPath) -> Result<Vec<Entry>, VfsError> {
        self.inner.read_dir(path)
    }
    fn rename_impl(&self, _from: &VfsPath, _to: &VfsPath) -> Result<(), VfsError> {
        Err(VfsError::CrossDevice)
    }
    fn open_read_impl(&self, path: &VfsPath) -> Result<Box<dyn Read + Send>, VfsError> {
        self.inner.open_read(path)
    }
    fn trash_impl(&self, path: &VfsPath) -> Result<(), VfsError> {
        self.inner.trash(path)
    }
}

impl CancelsMidFile<'_> {
    fn create_file_impl(&self, path: &VfsPath) -> Result<Box<dyn Write + Send>, VfsError> {
        self.inner.create_file(path)
    }
    fn read_dir_impl(&self, path: &VfsPath) -> Result<Vec<Entry>, VfsError> {
        self.inner.read_dir(path)
    }
    fn rename_impl(&self, from: &VfsPath, to: &VfsPath) -> Result<(), VfsError> {
        self.inner.rename(from, to)
    }
    fn open_read_impl(&self, path: &VfsPath) -> Result<Box<dyn Read + Send>, VfsError> {
        Ok(Box::new(CancelAfterFirstChunk {
            inner: self.inner.open_read(path)?,
            token: self.token.clone(),
        }))
    }
    fn trash_impl(&self, path: &VfsPath) -> Result<(), VfsError> {
        self.inner.trash(path)
    }
}

impl FailsMidRead<'_> {
    fn create_file_impl(&self, path: &VfsPath) -> Result<Box<dyn Write + Send>, VfsError> {
        self.inner.create_file(path)
    }
    fn read_dir_impl(&self, path: &VfsPath) -> Result<Vec<Entry>, VfsError> {
        self.inner.read_dir(path)
    }
    fn rename_impl(&self, from: &VfsPath, to: &VfsPath) -> Result<(), VfsError> {
        self.inner.rename(from, to)
    }
    fn open_read_impl(&self, path: &VfsPath) -> Result<Box<dyn Read + Send>, VfsError> {
        Ok(Box::new(FailAfter {
            inner: self.inner.open_read(path)?,
            left: self.good,
        }))
    }
    fn trash_impl(&self, path: &VfsPath) -> Result<(), VfsError> {
        self.inner.trash(path)
    }
}

impl TrashRefuses<'_> {
    fn create_file_impl(&self, path: &VfsPath) -> Result<Box<dyn Write + Send>, VfsError> {
        self.inner.create_file(path)
    }
    fn read_dir_impl(&self, path: &VfsPath) -> Result<Vec<Entry>, VfsError> {
        self.inner.read_dir(path)
    }
    fn rename_impl(&self, from: &VfsPath, to: &VfsPath) -> Result<(), VfsError> {
        self.inner.rename(from, to)
    }
    fn open_read_impl(&self, path: &VfsPath) -> Result<Box<dyn Read + Send>, VfsError> {
        self.inner.open_read(path)
    }
    fn trash_impl(&self, _path: &VfsPath) -> Result<(), VfsError> {
        Err(VfsError::PermissionDenied)
    }
}

impl RecordingTrash<'_> {
    fn create_file_impl(&self, path: &VfsPath) -> Result<Box<dyn Write + Send>, VfsError> {
        self.inner.create_file(path)
    }
    fn read_dir_impl(&self, path: &VfsPath) -> Result<Vec<Entry>, VfsError> {
        self.inner.read_dir(path)
    }
    fn rename_impl(&self, from: &VfsPath, to: &VfsPath) -> Result<(), VfsError> {
        self.inner.rename(from, to)
    }
    fn open_read_impl(&self, path: &VfsPath) -> Result<Box<dyn Read + Send>, VfsError> {
        self.inner.open_read(path)
    }
    fn trash_impl(&self, path: &VfsPath) -> Result<(), VfsError> {
        self.trashed.lock().unwrap().push(path.clone());
        Ok(())
    }
}

delegate_vfs!(CountingBackend<'_>);
delegate_vfs!(AlwaysCrossDevice<'_>);
delegate_vfs!(CancelsMidFile<'_>);
delegate_vfs!(FailsMidRead<'_>);
delegate_vfs!(TrashRefuses<'_>);
delegate_vfs!(RecordingTrash<'_>);

// ------------------------------------------------------------------ driver

fn run_on(
    job: &Job,
    fs: &dyn VirtualFs,
    resolver: &mut dyn ConflictResolver,
    cancel: &CancelToken,
) -> (Report, Vec<Progress>) {
    let mut events = Vec::new();
    let report = ops::run(job, fs, fs, resolver, &mut events, cancel);
    (report, events)
}

fn run_clean(job: &Job, fs: &dyn VirtualFs) -> Report {
    let report = ops::run(
        job,
        fs,
        fs,
        &mut NoConflictsExpected,
        &mut Silent,
        &CancelToken::new(),
    );
    assert!(report.is_clean(), "job did not run cleanly: {report:?}");
    report
}

// ------------------------------------------------------------------- tests

#[test]
fn copying_conserves_every_file_byte_and_name() {
    let (_dir, root) = fixture();
    let source = root.child("tree");
    let target_dir = root.child("into");
    let before = snapshot(&LocalFs, &source);

    run_clean(
        &Job::Copy {
            sources: vec![source.clone()],
            destination: Destination::Into(target_dir.clone()),
        },
        &LocalFs,
    );

    let copied = snapshot(&LocalFs, &target_dir.child("tree"));
    assert_eq!(copied, before, "the copy is not the same tree");
    assert_eq!(
        snapshot(&LocalFs, &source),
        before,
        "a copy must leave its source untouched"
    );
    assert_eq!(file_count(&copied), file_count(&before));
    assert_eq!(byte_sum(&copied), byte_sum(&before));
}

#[test]
fn a_copied_file_keeps_the_original_date() {
    // Otherwise every copied file claims to have been written just now, and
    // the date column stops meaning anything after a backup.
    let (_dir, root) = fixture();
    let target_dir = root.child("into");

    run_clean(
        &Job::Copy {
            sources: vec![root.child("tree")],
            destination: Destination::Into(target_dir.clone()),
        },
        &LocalFs,
    );

    let original = LocalFs.stat(&root.child("tree").child("a.txt")).unwrap();
    let copy = LocalFs
        .stat(&target_dir.child("tree").child("a.txt"))
        .unwrap();
    assert_eq!(copy.modified, original.modified);
}

#[test]
fn moving_conserves_the_union_of_both_sides() {
    let (_dir, root) = fixture();
    let source = root.child("tree");
    let target_dir = root.child("into");
    let before = snapshot(&LocalFs, &source);

    run_clean(
        &Job::Move {
            sources: vec![source.clone()],
            destination: Destination::Into(target_dir.clone()),
        },
        &LocalFs,
    );

    assert!(!exists(&LocalFs, &source), "the source must be gone");
    let moved = snapshot(&LocalFs, &target_dir.child("tree"));
    assert_eq!(moved, before, "a move must not change what it moved");
}

#[test]
fn a_move_within_one_filesystem_neither_reads_nor_walks() {
    // The invariant that catches a rename silently degrading into a copy, or
    // into a scan followed by a rename. No value assertion would notice
    // either: the resulting tree is identical.
    let (_dir, root) = fixture();
    let counting = CountingBackend::new(&LocalFs);
    let target_dir = root.child("into");

    run_clean(
        &Job::Move {
            sources: vec![root.child("tree")],
            destination: Destination::Into(target_dir.clone()),
        },
        &counting,
    );

    assert_eq!(counting.reads.load(Ordering::Relaxed), 0, "it copied bytes");
    // And it never walked the tree either. Scanning first would cost a full
    // directory walk before an operation that is otherwise a single syscall —
    // 57 ms against 6 us on a tree of 20 000 files.
    assert_eq!(
        counting.walks.load(Ordering::Relaxed),
        0,
        "it scanned the tree"
    );
    assert!(exists(&LocalFs, &target_dir.child("tree").child("a.txt")));
}

#[test]
fn the_cross_device_fallback_conserves_the_same_things() {
    // A one-device test box cannot produce a real CrossDevice, so the
    // fallback is driven directly. The behavior is covered; the trigger is
    // covered by the error-kind table in the vfs tests.
    let (_dir, root) = fixture();
    let fs = AlwaysCrossDevice { inner: &LocalFs };
    let source = root.child("tree");
    let target_dir = root.child("into");
    let before = snapshot(&LocalFs, &source);

    run_clean(
        &Job::Move {
            sources: vec![source.clone()],
            destination: Destination::Into(target_dir.clone()),
        },
        &fs,
    );

    assert!(!exists(&LocalFs, &source), "copy+delete must delete");
    assert_eq!(snapshot(&LocalFs, &target_dir.child("tree")), before);
}

#[test]
fn deleting_touches_nothing_but_its_targets() {
    let (_dir, root) = fixture();
    let tree = root.child("tree");
    let sibling = root.child("keeper.txt");
    LocalFs
        .create_file(&sibling)
        .unwrap()
        .write_all(b"untouched")
        .unwrap();

    run_clean(
        &Job::Delete {
            paths: vec![tree.clone()],
            mode: DeleteMode::Permanent,
        },
        &LocalFs,
    );

    assert!(!exists(&LocalFs, &tree), "the whole tree must be gone");
    assert_eq!(
        snapshot(&LocalFs, &root),
        Snapshot::from([
            ("/keeper.txt".to_string(), Some(b"untouched".to_vec())),
            // The shared fixture's empty landing directory, untouched.
            ("/into".to_string(), None),
        ])
    );
}

#[test]
fn trashing_hands_over_whole_paths_instead_of_walking_them() {
    // Recursion would be wrong here as well as slow: the trash takes a tree
    // in one call, and one call is what keeps the operation undoable.
    let (_dir, root) = fixture();
    let fs = RecordingTrash {
        inner: &LocalFs,
        trashed: Mutex::new(Vec::new()),
    };
    let tree = root.child("tree");

    run_clean(
        &Job::Delete {
            paths: vec![tree.clone()],
            mode: DeleteMode::Trash,
        },
        &fs,
    );

    assert_eq!(*fs.trashed.lock().unwrap(), vec![tree]);
}

#[test]
fn a_cancel_never_leaves_a_truncated_file() {
    // The invariant: a destination that did not exist before holds either a
    // complete copy or nothing. Never half a file that looks whole.
    //
    // The cancel lands *inside* the copy loop, not between two tasks — a
    // cancel that arrives before any byte moves would pass this test without
    // the rollback existing at all.
    let (_dir, root) = fixture();
    let target_dir = root.child("into");
    let cancel = CancelToken::new();
    let fs = CancelsMidFile {
        inner: &LocalFs,
        token: cancel.clone(),
    };

    let (report, _) = run_on(
        &Job::Copy {
            sources: vec![root.child("tree")],
            destination: Destination::Into(target_dir.clone()),
        },
        &fs,
        &mut NoConflictsExpected,
        &cancel,
    );

    assert_eq!(report.outcome, Outcome::Cancelled);
    // And a cancel is not a failure. Nothing went wrong — the user asked it
    // to stop — so an interrupted copy that also reported the file it was in
    // the middle of would put a name in front of them that needs no action.
    assert!(
        report.failures.is_empty(),
        "the cancel was reported as a failure: {:?}",
        report.failures
    );
    let source = snapshot(&LocalFs, &root.child("tree"));
    assert!(
        !snapshot(&LocalFs, &target_dir.child("tree")).is_empty(),
        "the cancel must land after some copying, or this proves nothing"
    );
    for (relative, bytes) in snapshot(&LocalFs, &target_dir.child("tree")) {
        let original = source
            .get(&relative)
            .unwrap_or_else(|| panic!("{relative} was invented by the cancel"));
        assert_eq!(&bytes, original, "{relative} is a partial copy");
    }
}

#[test]
fn a_cancel_leaves_the_source_untouched() {
    let (_dir, root) = fixture();
    let source = root.child("tree");
    let target_dir = root.child("into");
    let before = snapshot(&LocalFs, &source);
    let cancel = CancelToken::new();
    cancel.cancel();

    run_on(
        &Job::Move {
            sources: vec![source.clone()],
            destination: Destination::Into(target_dir),
        },
        &AlwaysCrossDevice { inner: &LocalFs },
        &mut NoConflictsExpected,
        &cancel,
    );

    assert_eq!(snapshot(&LocalFs, &source), before);
}

// ----------------------------------------------------------- conflict table

/// A target directory that already holds a file called `a.txt`.
fn collision() -> (TempDir, VfsPath, VfsPath) {
    let dir = TempDir::new().unwrap();
    let root = LocalFs::vfs_path(dir.path());
    let source_dir = root.child("from");
    let target_dir = root.child("to");
    for path in [&source_dir, &target_dir] {
        LocalFs.create_dir(path).unwrap();
    }
    LocalFs
        .create_file(&source_dir.child("a.txt"))
        .unwrap()
        .write_all(b"new")
        .unwrap();
    LocalFs
        .create_file(&target_dir.child("a.txt"))
        .unwrap()
        .write_all(b"old")
        .unwrap();
    (dir, source_dir, target_dir)
}

fn copy_one(source_dir: &VfsPath, target_dir: &VfsPath, answer: Answer) -> Report {
    let job = Job::Copy {
        sources: vec![source_dir.child("a.txt")],
        destination: Destination::Into(target_dir.clone()),
    };
    ops::run(
        &job,
        &LocalFs,
        &LocalFs,
        &mut Scripted::always(answer),
        &mut Silent,
        &CancelToken::new(),
    )
}

#[test]
fn overwrite_replaces_the_existing_bytes() {
    let (_dir, source_dir, target_dir) = collision();
    copy_one(
        &source_dir,
        &target_dir,
        Answer::once(Resolution::Overwrite),
    );
    assert_eq!(
        snapshot(&LocalFs, &target_dir),
        Snapshot::from([("/a.txt".to_string(), Some(b"new".to_vec()))])
    );
}

#[test]
fn skip_leaves_both_sides_exactly_as_they_were() {
    let (_dir, source_dir, target_dir) = collision();
    copy_one(&source_dir, &target_dir, Answer::once(Resolution::Skip));
    assert_eq!(
        snapshot(&LocalFs, &target_dir),
        Snapshot::from([("/a.txt".to_string(), Some(b"old".to_vec()))])
    );
    assert_eq!(
        snapshot(&LocalFs, &source_dir),
        Snapshot::from([("/a.txt".to_string(), Some(b"new".to_vec()))])
    );
}

#[test]
fn keep_both_conserves_the_old_one_and_adds_the_new_one() {
    let (_dir, source_dir, target_dir) = collision();
    copy_one(&source_dir, &target_dir, Answer::once(Resolution::KeepBoth));
    assert_eq!(
        snapshot(&LocalFs, &target_dir),
        Snapshot::from([
            ("/a.txt".to_string(), Some(b"old".to_vec())),
            ("/a (2).txt".to_string(), Some(b"new".to_vec())),
        ]),
        "the counter goes before the extension, so the file still opens"
    );
}

#[test]
fn abort_stops_the_job_and_changes_nothing() {
    let (_dir, source_dir, target_dir) = collision();
    let report = copy_one(&source_dir, &target_dir, Answer::once(Resolution::Abort));
    assert_eq!(report.outcome, Outcome::Aborted);
    assert_eq!(
        snapshot(&LocalFs, &target_dir),
        Snapshot::from([("/a.txt".to_string(), Some(b"old".to_vec()))])
    );
}

#[test]
fn apply_to_all_asks_exactly_once_however_many_collide() {
    let (dir, source_dir, target_dir) = collision();
    let names = ["b.txt", "c.txt", "d.txt", "e.txt"];
    for name in names {
        for parent in [&source_dir, &target_dir] {
            std::fs::write(dir.path().join(parent.file_name().unwrap()).join(name), "x").unwrap();
        }
    }
    let sources: Vec<_> = std::iter::once("a.txt")
        .chain(names)
        .map(|name| source_dir.child(name))
        .collect();
    let mut resolver = ApplyToAll::new(Scripted::always(Answer::always(Resolution::Skip)));

    ops::run(
        &Job::Copy {
            sources,
            destination: Destination::Into(target_dir),
        },
        &LocalFs,
        &LocalFs,
        &mut resolver,
        &mut Silent,
        &CancelToken::new(),
    );

    // Five collisions, one question. Without the wrapper this is five.
    assert_eq!(resolver.into_inner().questions(), 1);
}

#[test]
fn a_move_that_skipped_a_file_does_not_delete_it() {
    // The data-loss invariant. Deleting the source is only earned by the
    // copy having arrived; a skip is not an arrival.
    let (_dir, source_dir, target_dir) = collision();
    let job = Job::Move {
        sources: vec![source_dir.child("a.txt")],
        destination: Destination::Into(target_dir),
    };

    ops::run(
        &job,
        &LocalFs,
        &LocalFs,
        &mut Scripted::always(Answer::once(Resolution::Skip)),
        &mut Silent,
        &CancelToken::new(),
    );

    assert_eq!(
        snapshot(&LocalFs, &source_dir),
        Snapshot::from([("/a.txt".to_string(), Some(b"new".to_vec()))]),
        "the skipped file is the only copy left, so it must survive"
    );
}

// -------------------------------------------------------------- other jobs

#[test]
fn progress_deltas_add_up_to_what_the_scan_promised() {
    let (_dir, root) = fixture();
    let target_dir = root.child("into");

    let (_, events) = run_on(
        &Job::Copy {
            sources: vec![root.child("tree")],
            destination: Destination::Into(target_dir),
        },
        &LocalFs,
        &mut NoConflictsExpected,
        &CancelToken::new(),
    );

    let scanned = events
        .iter()
        .find_map(|event| match event {
            Progress::Scanned { bytes, .. } => Some(*bytes),
            _ => None,
        })
        .expect("a job announces its totals");
    let advanced: u64 = events
        .iter()
        .filter_map(|event| match event {
            Progress::Advanced { bytes } => Some(*bytes),
            _ => None,
        })
        .sum();
    assert_eq!(advanced, scanned);
    assert!(scanned > 0, "the fixture has bytes in it");
}

#[test]
fn creating_a_directory_reports_the_reason_when_it_cannot() {
    let (_dir, root) = fixture();
    let report = ops::run(
        &Job::CreateDir {
            path: root.child("tree"),
        },
        &LocalFs,
        &LocalFs,
        &mut NoConflictsExpected,
        &mut Silent,
        &CancelToken::new(),
    );

    assert_eq!(report.outcome, Outcome::Completed);
    assert_eq!(report.failures.len(), 1);
    assert_eq!(report.failures[0].1, VfsError::AlreadyExists);
}

#[test]
fn renaming_moves_a_path_to_an_exact_new_name() {
    let (_dir, root) = fixture();
    let before = snapshot(&LocalFs, &root.child("tree"));

    run_clean(
        &Job::Move {
            sources: vec![root.child("tree")],
            destination: Destination::Exact(root.child("renamed")),
        },
        &LocalFs,
    );

    assert!(!exists(&LocalFs, &root.child("tree")));
    assert_eq!(snapshot(&LocalFs, &root.child("renamed")), before);
}

#[test]
fn one_unreadable_source_does_not_cost_the_others() {
    let (_dir, root) = fixture();
    let target_dir = root.child("into");

    let report = ops::run(
        &Job::Copy {
            sources: vec![root.child("not-there"), root.child("tree")],
            destination: Destination::Into(target_dir.clone()),
        },
        &LocalFs,
        &LocalFs,
        &mut NoConflictsExpected,
        &mut Silent,
        &CancelToken::new(),
    );

    assert_eq!(report.outcome, Outcome::Completed);
    assert_eq!(report.failures.len(), 1);
    assert_eq!(report.failures[0].1, VfsError::NotFound);
    assert_eq!(
        snapshot(&LocalFs, &target_dir.child("tree")),
        snapshot(&LocalFs, &root.child("tree")),
        "the readable source still arrived in full"
    );
}

#[cfg(unix)]
mod symlinks {
    use super::*;

    #[test]
    fn deleting_a_link_to_a_directory_does_not_delete_what_it_points_at() {
        // Descending into the link would reach outside the tree the user
        // selected and destroy someone else's files.
        let (_dir, root) = fixture();
        let outside = root.child("tree");
        let doomed = root.child("doomed");
        std::fs::create_dir(doomed.as_str()).unwrap();
        std::os::unix::fs::symlink(outside.as_str(), format!("{doomed}/link")).unwrap();
        let before = snapshot(&LocalFs, &outside);

        run_clean(
            &Job::Delete {
                paths: vec![doomed.clone()],
                mode: DeleteMode::Permanent,
            },
            &LocalFs,
        );

        assert!(!exists(&LocalFs, &doomed));
        assert_eq!(snapshot(&LocalFs, &outside), before);
    }

    #[test]
    fn a_link_inside_a_tree_costs_only_itself() {
        // Regression. The scan used to return the refusal as an error for the
        // whole source, so one symlink anywhere inside a tree discarded every
        // task already collected and copied nothing at all. The unit test
        // below missed it because it copies the link on its own; only a real
        // run against a real tree showed an empty target directory.
        let (_dir, root) = fixture();
        let target_dir = root.child("into");
        let outside = root.child("outside");
        LocalFs.create_dir(&outside).unwrap();
        // Pointed away from the copy target: a link into it would make the
        // snapshot walk follow the copy back into itself.
        std::os::unix::fs::symlink(
            outside.as_str(),
            format!("{}/link-to-dir", root.child("tree")),
        )
        .unwrap();

        let report = ops::run(
            &Job::Copy {
                sources: vec![root.child("tree")],
                destination: Destination::Into(target_dir.clone()),
            },
            &LocalFs,
            &LocalFs,
            &mut NoConflictsExpected,
            &mut Silent,
            &CancelToken::new(),
        );

        assert_eq!(report.failures.len(), 1, "the link, and nothing else");
        let copied = snapshot(&LocalFs, &target_dir.child("tree"));
        assert!(
            copied.contains_key("/a.txt") && copied.contains_key("/sub/b.bin"),
            "everything the link is not must still arrive: {copied:?}"
        );
        assert_eq!(
            file_count(&copied),
            file_count(&snapshot(&LocalFs, &root.child("tree")))
        );
    }

    #[test]
    fn a_move_whose_tree_holds_a_link_keeps_the_source() {
        // The other half: an item that could not be copied in full is not
        // one a move may delete.
        let (_dir, root) = fixture();
        let source = root.child("tree");
        let target_dir = root.child("into");
        let outside = root.child("outside");
        LocalFs.create_dir(&outside).unwrap();
        std::os::unix::fs::symlink(outside.as_str(), format!("{source}/link-to-dir")).unwrap();
        let before = snapshot(&LocalFs, &source);

        // The decorator has to sit on the *target* side too: the fast path
        // asks the target backend to rename, and on one real filesystem that
        // succeeds and moves the link along with everything else — which is
        // correct, and not the path this test is about.
        let fs = AlwaysCrossDevice { inner: &LocalFs };
        ops::run(
            &Job::Move {
                sources: vec![source.clone()],
                destination: Destination::Into(target_dir),
            },
            &fs,
            &fs,
            &mut NoConflictsExpected,
            &mut Silent,
            &CancelToken::new(),
        );

        assert_eq!(snapshot(&LocalFs, &source), before);
    }

    #[test]
    fn copying_a_link_to_a_directory_is_refused_rather_than_followed() {
        // Following it loops forever on a cycle, and the interface cannot
        // recreate the link instead. Refusing says so out loud.
        let (_dir, root) = fixture();
        let target_dir = root.child("into");
        std::os::unix::fs::symlink(root.child("tree").as_str(), format!("{root}/link")).unwrap();

        let report = ops::run(
            &Job::Copy {
                sources: vec![root.child("link")],
                destination: Destination::Into(target_dir),
            },
            &LocalFs,
            &LocalFs,
            &mut NoConflictsExpected,
            &mut Silent,
            &CancelToken::new(),
        );

        assert_eq!(report.failures.len(), 1);
    }
}

/// Cases that are unlikely right up until the moment they cost someone their
/// files. Operations have to be reliable before they are anything else, so
/// the awkward corners get tests rather than the benefit of the doubt.
mod awkward_corners {
    use super::*;

    /// A file with contents worth noticing the loss of.
    fn lone_file(name: &str, contents: &[u8]) -> (TempDir, VfsPath, VfsPath) {
        let dir = TempDir::new().unwrap();
        let root = LocalFs::vfs_path(dir.path());
        let file = root.child(name);
        LocalFs
            .create_file(&file)
            .unwrap()
            .write_all(contents)
            .unwrap();
        (dir, root, file)
    }

    #[test]
    fn copying_a_file_onto_itself_is_refused_and_the_file_survives() {
        // This destroyed the file before it was caught: the destination is
        // opened for writing while the source handle is still open, so the
        // copy read back an empty file and reported success. Reachable with
        // both panes in one directory and the prefilled target accepted.
        let (_dir, _root, file) = lone_file("precious.txt", b"seventeen bytes!!");

        let report = ops::run(
            &Job::Copy {
                sources: vec![file.clone()],
                destination: Destination::Exact(file.clone()),
            },
            &LocalFs,
            &LocalFs,
            &mut Scripted::always(Answer::once(Resolution::Overwrite)),
            &mut Silent,
            &CancelToken::new(),
        );

        assert_eq!(report.failures.len(), 1, "it must say why, not just cope");
        assert_eq!(LocalFs.stat(&file).unwrap().size, 17);
    }

    #[test]
    fn copying_into_the_directory_a_file_is_already_in_is_refused() {
        // The same thing by the route a user actually takes: F5 with both
        // panes showing one directory.
        let (_dir, root, file) = lone_file("precious.txt", b"seventeen bytes!!");

        let report = ops::run(
            &Job::Copy {
                sources: vec![file.clone()],
                destination: Destination::Into(root),
            },
            &LocalFs,
            &LocalFs,
            &mut Scripted::always(Answer::once(Resolution::Overwrite)),
            &mut Silent,
            &CancelToken::new(),
        );

        assert_eq!(report.failures.len(), 1);
        assert_eq!(LocalFs.stat(&file).unwrap().size, 17);
    }

    #[test]
    fn moving_a_file_onto_itself_is_refused_and_the_file_survives() {
        let (_dir, _root, file) = lone_file("precious.txt", b"seventeen bytes!!");

        let report = ops::run(
            &Job::Move {
                sources: vec![file.clone()],
                destination: Destination::Exact(file.clone()),
            },
            &LocalFs,
            &LocalFs,
            &mut NoConflictsExpected,
            &mut Silent,
            &CancelToken::new(),
        );

        assert_eq!(report.failures.len(), 1);
        assert_eq!(LocalFs.stat(&file).unwrap().size, 17);
    }

    #[test]
    fn copying_a_directory_into_its_own_subtree_is_refused() {
        // Following this would never terminate: the walk keeps finding what
        // it has just written.
        let (_dir, root) = fixture();
        let source = root.child("tree");
        let before = snapshot(&LocalFs, &source);

        let report = ops::run(
            &Job::Copy {
                sources: vec![source.clone()],
                destination: Destination::Into(source.child("sub")),
            },
            &LocalFs,
            &LocalFs,
            &mut NoConflictsExpected,
            &mut Silent,
            &CancelToken::new(),
        );

        assert_eq!(report.failures.len(), 1);
        assert_eq!(snapshot(&LocalFs, &source), before, "nothing may be added");
    }

    #[test]
    fn moving_a_directory_into_its_own_subtree_is_refused() {
        let (_dir, root) = fixture();
        let source = root.child("tree");
        let before = snapshot(&LocalFs, &source);

        let report = ops::run(
            &Job::Move {
                sources: vec![source.clone()],
                destination: Destination::Into(source.child("sub")),
            },
            &LocalFs,
            &LocalFs,
            &mut NoConflictsExpected,
            &mut Silent,
            &CancelToken::new(),
        );

        assert_eq!(report.failures.len(), 1);
        assert_eq!(snapshot(&LocalFs, &source), before);
    }

    #[test]
    fn a_sibling_directory_with_a_similar_name_is_not_inside_anything() {
        // The refusal compares whole components. `/x/treeish` starts with the
        // characters of `/x/tree` and is not inside it; a prefix test on the
        // raw string would refuse a perfectly ordinary copy.
        let (_dir, root) = fixture();
        let alongside = root.child("treeish");
        LocalFs.create_dir(&alongside).unwrap();

        run_clean(
            &Job::Copy {
                sources: vec![root.child("tree")],
                destination: Destination::Into(alongside.clone()),
            },
            &LocalFs,
        );

        assert_eq!(
            snapshot(&LocalFs, &alongside.child("tree")),
            snapshot(&LocalFs, &root.child("tree"))
        );
    }

    #[test]
    fn a_refused_source_does_not_stop_the_others() {
        let (_dir, root) = fixture();
        let target_dir = root.child("into");

        let report = ops::run(
            &Job::Copy {
                // The first would copy into itself; the second is ordinary.
                sources: vec![target_dir.clone(), root.child("tree")],
                destination: Destination::Into(target_dir.clone()),
            },
            &LocalFs,
            &LocalFs,
            &mut NoConflictsExpected,
            &mut Silent,
            &CancelToken::new(),
        );

        assert_eq!(report.failures.len(), 1);
        assert_eq!(
            snapshot(&LocalFs, &target_dir.child("tree")),
            snapshot(&LocalFs, &root.child("tree"))
        );
    }

    #[test]
    fn overwriting_a_large_file_with_a_small_one_leaves_no_tail() {
        // A destination opened without truncation would keep the tail of what
        // was there, and the result would look like a successful copy.
        let (dir, root) = fixture();
        let target_dir = root.child("into");
        std::fs::create_dir(dir.path().join("into/tree")).unwrap();
        std::fs::write(dir.path().join("into/tree/a.txt"), vec![b'Z'; 5000]).unwrap();

        ops::run(
            &Job::Copy {
                sources: vec![root.child("tree")],
                destination: Destination::Into(target_dir.clone()),
            },
            &LocalFs,
            &LocalFs,
            &mut Scripted::always(Answer::always(Resolution::Overwrite)),
            &mut Silent,
            &CancelToken::new(),
        );

        assert_eq!(
            snapshot(&LocalFs, &target_dir.child("tree")),
            snapshot(&LocalFs, &root.child("tree")),
            "an overwritten file must be the source, not a mixture"
        );
    }

    #[test]
    fn keep_both_walks_past_an_alternative_name_that_is_taken() {
        let (dir, root) = fixture();
        let target_dir = root.child("into");
        std::fs::create_dir(dir.path().join("into/tree")).unwrap();
        std::fs::write(dir.path().join("into/tree/a.txt"), "old").unwrap();
        std::fs::write(dir.path().join("into/tree/a (2).txt"), "older").unwrap();

        ops::run(
            &Job::Copy {
                sources: vec![root.child("tree/a.txt")],
                destination: Destination::Into(target_dir.child("tree")),
            },
            &LocalFs,
            &LocalFs,
            &mut Scripted::always(Answer::once(Resolution::KeepBoth)),
            &mut Silent,
            &CancelToken::new(),
        );

        let landed = snapshot(&LocalFs, &target_dir.child("tree"));
        assert_eq!(
            landed.get("/a (3).txt"),
            Some(&Some(b"hello world".to_vec()))
        );
        assert_eq!(landed.get("/a.txt"), Some(&Some(b"old".to_vec())));
        assert_eq!(landed.get("/a (2).txt"), Some(&Some(b"older".to_vec())));
    }

    #[test]
    fn a_read_error_partway_through_leaves_no_truncated_file() {
        // Cancelling is not the only way to stop mid-file, and the invariant
        // has to hold for the other way too: a destination that did not exist
        // holds a whole copy or nothing.
        let (_dir, root) = fixture();
        let target_dir = root.child("into");
        let fs = FailsMidRead {
            inner: &LocalFs,
            good: 1024,
        };

        let report = ops::run(
            &Job::Copy {
                sources: vec![root.child("tree/sub/b.bin")],
                destination: Destination::Into(target_dir.clone()),
            },
            &fs,
            &LocalFs,
            &mut NoConflictsExpected,
            &mut Silent,
            &CancelToken::new(),
        );

        assert_eq!(report.failures.len(), 1, "the error must be reported");
        assert!(
            !exists(&LocalFs, &target_dir.child("b.bin")),
            "a half-written file was left behind"
        );
    }

    #[test]
    fn overwriting_a_directory_with_a_file_fails_only_that_path() {
        let (dir, root) = fixture();
        let target_dir = root.child("into");
        std::fs::create_dir(dir.path().join("into/tree")).unwrap();
        // A directory standing exactly where a file wants to go.
        std::fs::create_dir(dir.path().join("into/tree/a.txt")).unwrap();

        let report = ops::run(
            &Job::Copy {
                sources: vec![root.child("tree")],
                destination: Destination::Into(target_dir.clone()),
            },
            &LocalFs,
            &LocalFs,
            &mut Scripted::always(Answer::always(Resolution::Overwrite)),
            &mut Silent,
            &CancelToken::new(),
        );

        assert_eq!(report.outcome, Outcome::Completed);
        assert_eq!(report.failures.len(), 1);
        // Everything that was not in the way still arrived.
        assert_eq!(
            snapshot(&LocalFs, &target_dir.child("tree").child("sub")),
            snapshot(&LocalFs, &root.child("tree").child("sub"))
        );
    }

    #[test]
    fn aborting_on_the_first_of_two_sources_never_reaches_the_second() {
        let (dir, root) = fixture();
        let target_dir = root.child("into");
        std::fs::write(dir.path().join("into/a.txt"), "old").unwrap();

        let report = ops::run(
            &Job::Copy {
                // Order is kept, so the second source is provably untouched.
                sources: vec![root.child("tree/a.txt"), root.child("tree/sub")],
                destination: Destination::Into(target_dir.clone()),
            },
            &LocalFs,
            &LocalFs,
            &mut Scripted::always(Answer::once(Resolution::Abort)),
            &mut Silent,
            &CancelToken::new(),
        );

        assert_eq!(report.outcome, Outcome::Aborted);
        assert_eq!(
            snapshot(&LocalFs, &target_dir),
            Snapshot::from([("/a.txt".to_string(), Some(b"old".to_vec()))])
        );
    }

    #[test]
    fn the_same_source_listed_twice_arrives_once_and_asks_once() {
        // The second copy collides with what the first one just wrote, which
        // is a conflict like any other rather than a surprise.
        let (_dir, root) = fixture();
        let target_dir = root.child("into");
        let source = root.child("tree/a.txt");
        let mut resolver = Scripted::always(Answer::once(Resolution::Skip));

        ops::run(
            &Job::Copy {
                sources: vec![source.clone(), source.clone()],
                destination: Destination::Into(target_dir.clone()),
            },
            &LocalFs,
            &LocalFs,
            &mut resolver,
            &mut Silent,
            &CancelToken::new(),
        );

        assert_eq!(resolver.questions(), 1);
        assert_eq!(
            snapshot(&LocalFs, &target_dir),
            Snapshot::from([("/a.txt".to_string(), Some(b"hello world".to_vec()))])
        );
    }

    #[test]
    fn deleting_the_same_path_twice_reports_the_second_and_carries_on() {
        let (_dir, root) = fixture();
        let doomed = root.child("tree/a.txt");

        let report = ops::run(
            &Job::Delete {
                paths: vec![doomed.clone(), doomed.clone()],
                mode: DeleteMode::Permanent,
            },
            &LocalFs,
            &LocalFs,
            &mut NoConflictsExpected,
            &mut Silent,
            &CancelToken::new(),
        );

        assert_eq!(report.outcome, Outcome::Completed);
        assert_eq!(report.failures.len(), 1);
        assert_eq!(report.failures[0].1, VfsError::NotFound);
        assert!(!exists(&LocalFs, &doomed));
    }

    #[test]
    fn copying_into_a_directory_that_is_not_there_fails_without_creating_it() {
        let (_dir, root) = fixture();
        let missing = root.child("nowhere");

        let report = ops::run(
            &Job::Copy {
                sources: vec![root.child("tree/a.txt")],
                destination: Destination::Into(missing.clone()),
            },
            &LocalFs,
            &LocalFs,
            &mut NoConflictsExpected,
            &mut Silent,
            &CancelToken::new(),
        );

        assert_eq!(report.failures.len(), 1);
        assert!(!exists(&LocalFs, &missing), "it must not invent the parent");
    }

    #[test]
    fn a_trash_that_refuses_reports_it_and_keeps_the_file() {
        // A delete that quietly fails is the worst possible outcome: the user
        // believes the file is gone and stops looking after it.
        let (_dir, root) = fixture();
        let fs = TrashRefuses { inner: &LocalFs };
        let doomed = root.child("tree/a.txt");

        let report = ops::run(
            &Job::Delete {
                paths: vec![doomed.clone()],
                mode: DeleteMode::Trash,
            },
            &fs,
            &fs,
            &mut NoConflictsExpected,
            &mut Silent,
            &CancelToken::new(),
        );

        assert_eq!(report.failures.len(), 1);
        assert_eq!(report.failures[0].1, VfsError::PermissionDenied);
        assert!(exists(&LocalFs, &doomed));
    }

    #[test]
    fn a_job_with_no_sources_finishes_cleanly_and_changes_nothing() {
        let (_dir, root) = fixture();
        let before = snapshot(&LocalFs, &root);

        run_clean(
            &Job::Copy {
                sources: Vec::new(),
                destination: Destination::Into(root.child("into")),
            },
            &LocalFs,
        );

        assert_eq!(snapshot(&LocalFs, &root), before);
    }
}

/// Copies that keep the original's permissions.
///
/// An executable that arrives without its `+x` is a broken copy that looks
/// exactly like a working one, which is why this is a reliability concern and
/// not a column (`docs/reliability.md`).
#[cfg(unix)]
mod permissions {
    use std::os::unix::fs::PermissionsExt;

    use super::*;

    fn mode_of(path: &VfsPath) -> u32 {
        std::fs::metadata(path.as_str())
            .unwrap()
            .permissions()
            .mode()
            & 0o7777
    }

    #[test]
    fn a_copied_executable_is_still_executable() {
        let (dir, root) = fixture();
        let script = dir.path().join("tree/script.sh");
        std::fs::write(&script, "#!/bin/sh\necho hi\n").unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        let target_dir = root.child("into");

        run_clean(
            &Job::Copy {
                sources: vec![root.child("tree")],
                destination: Destination::Into(target_dir.clone()),
            },
            &LocalFs,
        );

        assert_eq!(mode_of(&target_dir.child("tree/script.sh")), 0o755);
    }

    #[test]
    fn a_copy_does_not_hand_out_permissions_the_original_did_not_have() {
        // The other direction, which a blanket chmod would get wrong: a
        // private file must not arrive world-readable.
        let (dir, root) = fixture();
        let secret = dir.path().join("tree/secret.txt");
        std::fs::write(&secret, "private").unwrap();
        std::fs::set_permissions(&secret, std::fs::Permissions::from_mode(0o600)).unwrap();
        let target_dir = root.child("into");

        run_clean(
            &Job::Copy {
                sources: vec![root.child("tree")],
                destination: Destination::Into(target_dir.clone()),
            },
            &LocalFs,
        );

        assert_eq!(mode_of(&target_dir.child("tree/secret.txt")), 0o600);
    }

    #[test]
    fn permissions_are_restored_after_the_timestamp_not_before() {
        // A read-only file has to end up read-only *and* keep its date.
        // Setting the permissions first would take write access away and the
        // timestamp would never be set.
        let (dir, root) = fixture();
        let locked = dir.path().join("tree/locked.txt");
        std::fs::write(&locked, "read only").unwrap();
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o444)).unwrap();
        let original = LocalFs.stat(&root.child("tree/locked.txt")).unwrap();
        let target_dir = root.child("into");

        run_clean(
            &Job::Copy {
                sources: vec![root.child("tree")],
                destination: Destination::Into(target_dir.clone()),
            },
            &LocalFs,
        );

        let copy = LocalFs.stat(&target_dir.child("tree/locked.txt")).unwrap();
        assert_eq!(mode_of(&target_dir.child("tree/locked.txt")), 0o444);
        assert_eq!(copy.modified, original.modified);
    }
}

/// `Shift+F4`: an empty file for an editor to open.
mod create_file {
    use super::*;

    #[test]
    fn an_empty_file_appears_where_it_was_asked_for() {
        let (dir, root) = fixture();
        let path = root.child("into").child("notes.txt");

        let report = run_clean(&Job::CreateFile { path: path.clone() }, &LocalFs);

        assert!(report.failures.is_empty(), "{:?}", report.failures);
        let written = dir.path().join("into/notes.txt");
        assert!(written.exists(), "the file was not created");
        assert_eq!(std::fs::read(&written).unwrap().len(), 0, "not empty");
    }

    #[test]
    fn a_name_that_is_taken_is_refused_rather_than_emptied() {
        // The whole reason this is not a bare `create_file` on the VFS: that
        // call truncates, so Shift+F4 on an existing name would empty the very
        // file the user meant to open — and then hand it to an editor, which
        // would save the emptiness back.
        let (dir, root) = fixture();
        let existing = dir.path().join("into/keep.txt");
        std::fs::write(&existing, "precious").unwrap();

        let (report, _) = run_on(
            &Job::CreateFile {
                path: root.child("into").child("keep.txt"),
            },
            &LocalFs,
            &mut NoConflictsExpected,
            &CancelToken::new(),
        );

        assert_eq!(report.failures.len(), 1, "it was allowed through");
        assert_eq!(
            std::fs::read_to_string(&existing).unwrap(),
            "precious",
            "the file was emptied"
        );
    }

    #[test]
    fn a_directory_that_is_not_there_is_a_failure_not_a_panic() {
        let (_dir, root) = fixture();

        let (report, _) = run_on(
            &Job::CreateFile {
                path: root.child("nowhere").child("notes.txt"),
            },
            &LocalFs,
            &mut NoConflictsExpected,
            &CancelToken::new(),
        );

        assert_eq!(report.failures.len(), 1);
    }
}
