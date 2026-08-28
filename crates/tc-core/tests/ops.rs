//! Behavioral tests for the operation engine.
//!
//! Assertions are conservation statements — file counts, byte sums, per-path
//! contents, what the source still holds — rather than checks on the one path
//! a call names. An engine that writes the right file and quietly loses a
//! sibling passes the narrow kind of test and fails these.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::SystemTime;

use tc_core::ops::{
    self, Answer, ApplyToAll, CancelToken, Conflict, ConflictResolver, DeleteMode, Destination,
    Job, Outcome, Progress, Report, Resolution, Silent,
};
use tc_core::vfs::{Entry, LocalFs, VfsError, VfsPath, VirtualFs};
use tempfile::TempDir;

// ---------------------------------------------------------------- fixtures

/// A tree with nesting, an empty file, an empty directory and a unicode name,
/// so every walk has something awkward in it.
fn tree(root: &std::path::Path) {
    use std::fs;
    fs::create_dir_all(root.join("sub/deep")).unwrap();
    fs::create_dir(root.join("emptydir")).unwrap();
    fs::write(root.join("a.txt"), "hello world").unwrap();
    fs::write(root.join("empty.txt"), "").unwrap();
    fs::write(root.join("Ünïcødé — ✓.md"), "unicode").unwrap();
    fs::write(root.join("sub/b.bin"), vec![7u8; 200_000]).unwrap();
    fs::write(root.join("sub/deep/c.txt"), "deep").unwrap();
}

fn fixture() -> (TempDir, VfsPath) {
    let dir = TempDir::new().unwrap();
    tree(&dir.path().join("tree"));
    let root = LocalFs::vfs_path(dir.path());
    (dir, root)
}

/// A whole tree as relative path to contents, `None` for a directory.
type Snapshot = BTreeMap<String, Option<Vec<u8>>>;

/// Every file below `root`, as relative path to contents.
///
/// The comparison unit for every transfer test: two trees are the same tree
/// exactly when their snapshots are equal.
fn snapshot(fs: &dyn VirtualFs, root: &VfsPath) -> Snapshot {
    let mut found = BTreeMap::new();
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
            // Directories appear too, so an empty one is not silently lost —
            // as `None`, because an empty file is not an empty directory.
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

fn file_count(snapshot: &Snapshot) -> usize {
    snapshot.values().flatten().count()
}

fn byte_sum(snapshot: &Snapshot) -> usize {
    snapshot.values().flatten().map(Vec::len).sum()
}

fn exists(fs: &dyn VirtualFs, path: &VfsPath) -> bool {
    fs.stat(path).is_ok()
}

// --------------------------------------------------------------- resolvers

/// Answers whatever it was handed, in order, and counts the questions.
struct Scripted {
    answers: Vec<Answer>,
    asked: Vec<Conflict>,
}

impl Scripted {
    fn new(answers: Vec<Answer>) -> Self {
        Scripted {
            answers,
            asked: Vec::new(),
        }
    }

    fn always(answer: Answer) -> Self {
        Scripted::new(vec![answer])
    }
}

impl ConflictResolver for Scripted {
    fn resolve(&mut self, conflict: &Conflict) -> Answer {
        self.asked.push(conflict.clone());
        let index = (self.asked.len() - 1).min(self.answers.len() - 1);
        self.answers[index]
    }
}

/// Refuses to be asked. Any conflict at all fails the test that used it.
struct NoConflictsExpected;

impl ConflictResolver for NoConflictsExpected {
    fn resolve(&mut self, conflict: &Conflict) -> Answer {
        panic!("unexpected conflict at {}", conflict.target);
    }
}

// -------------------------------------------------------------- decorators

/// Counts reads, so a test can prove a move never touched the bytes.
struct Counting<'a> {
    inner: &'a dyn VirtualFs,
    reads: AtomicUsize,
}

impl<'a> Counting<'a> {
    fn new(inner: &'a dyn VirtualFs) -> Self {
        Counting {
            inner,
            reads: AtomicUsize::new(0),
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

/// Records what was handed to the trash instead of really trashing it.
struct RecordingTrash<'a> {
    inner: &'a dyn VirtualFs,
    trashed: Mutex<Vec<VfsPath>>,
}

macro_rules! delegate {
    ($target:ty) => {
        impl VirtualFs for $target {
            fn read_dir(&self, path: &VfsPath) -> Result<Vec<Entry>, VfsError> {
                self.inner.read_dir(path)
            }
            fn stat(&self, path: &VfsPath) -> Result<Entry, VfsError> {
                self.inner.stat(path)
            }
            fn create_dir(&self, path: &VfsPath) -> Result<(), VfsError> {
                self.inner.create_dir(path)
            }
            fn remove_dir(&self, path: &VfsPath) -> Result<(), VfsError> {
                self.inner.remove_dir(path)
            }
            fn remove_file(&self, path: &VfsPath) -> Result<(), VfsError> {
                self.inner.remove_file(path)
            }
            fn rename(&self, from: &VfsPath, to: &VfsPath) -> Result<(), VfsError> {
                self.rename_impl(from, to)
            }
            fn open_read(&self, path: &VfsPath) -> Result<Box<dyn Read + Send>, VfsError> {
                self.open_read_impl(path)
            }
            fn create_file(&self, path: &VfsPath) -> Result<Box<dyn Write + Send>, VfsError> {
                self.inner.create_file(path)
            }
            fn set_modified(&self, path: &VfsPath, time: SystemTime) -> Result<(), VfsError> {
                self.inner.set_modified(path, time)
            }
            fn trash(&self, path: &VfsPath) -> Result<(), VfsError> {
                self.trash_impl(path)
            }
        }
    };
}

impl Counting<'_> {
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

impl RecordingTrash<'_> {
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

delegate!(Counting<'_>);
delegate!(AlwaysCrossDevice<'_>);
delegate!(CancelsMidFile<'_>);
delegate!(RecordingTrash<'_>);

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
    LocalFs.create_dir(&target_dir).unwrap();
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
    LocalFs.create_dir(&target_dir).unwrap();

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
    LocalFs.create_dir(&target_dir).unwrap();
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
fn a_move_within_one_filesystem_reads_no_bytes() {
    // The invariant that catches a rename silently degrading into a copy.
    // No value assertion would notice: the resulting tree is identical.
    let (_dir, root) = fixture();
    let counting = Counting::new(&LocalFs);
    let target_dir = root.child("into");
    counting.create_dir(&target_dir).unwrap();

    run_clean(
        &Job::Move {
            sources: vec![root.child("tree")],
            destination: Destination::Into(target_dir.clone()),
        },
        &counting,
    );

    assert_eq!(counting.reads.load(Ordering::Relaxed), 0);
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
    fs.create_dir(&target_dir).unwrap();
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
        Snapshot::from([("/keeper.txt".to_string(), Some(b"untouched".to_vec()))])
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
    LocalFs.create_dir(&target_dir).unwrap();
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
    LocalFs.create_dir(&target_dir).unwrap();
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
    assert_eq!(resolver.into_inner().asked.len(), 1);
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
    LocalFs.create_dir(&target_dir).unwrap();

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
    LocalFs.create_dir(&target_dir).unwrap();

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
        for path in [&target_dir, &outside] {
            LocalFs.create_dir(path).unwrap();
        }
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
        for path in [&target_dir, &outside] {
            LocalFs.create_dir(path).unwrap();
        }
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
        LocalFs.create_dir(&target_dir).unwrap();
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
