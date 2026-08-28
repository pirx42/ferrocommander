//! Behavioral tests for the background job queue.
//!
//! No GTK and no main loop: the queue is a worker thread and four channels,
//! and every one of those is drivable from a test thread. What is checked
//! here is the wrapper — ordering, the conflict round trip, cancelling from
//! outside — not the engine, which `ops.rs` already pins.

use std::collections::BTreeMap;
use std::io::Read;
use std::sync::Arc;

use tc_core::ops::{Answer, DeleteMode, Job, JobQueue, Outcome, Progress, Report, Resolution};
use tc_core::vfs::{LocalFs, VfsPath, VirtualFs};
use tempfile::TempDir;

fn backend() -> Arc<dyn VirtualFs> {
    Arc::new(LocalFs)
}

fn fixture() -> (TempDir, VfsPath) {
    let dir = TempDir::new().unwrap();
    let root = LocalFs::vfs_path(dir.path());
    std::fs::create_dir(dir.path().join("tree")).unwrap();
    std::fs::write(dir.path().join("tree/a.txt"), "hello world").unwrap();
    std::fs::write(dir.path().join("tree/big.bin"), vec![3u8; 200_000]).unwrap();
    std::fs::create_dir(dir.path().join("into")).unwrap();
    (dir, root)
}

fn snapshot(root: &VfsPath) -> BTreeMap<String, Vec<u8>> {
    let mut found = BTreeMap::new();
    let mut stack = vec![root.clone()];
    while let Some(at) = stack.pop() {
        for entry in LocalFs.read_dir(&at).unwrap_or_default() {
            let path = at.child(&entry.name);
            let relative = path.as_str().trim_start_matches(root.as_str()).to_string();
            if entry.is_dir() {
                stack.push(path);
            } else {
                let mut bytes = Vec::new();
                LocalFs
                    .open_read(&path)
                    .unwrap()
                    .read_to_end(&mut bytes)
                    .unwrap();
                found.insert(relative, bytes);
            }
        }
    }
    found
}

/// The report a finished job leaves. Blocks until the job is done, which is
/// what makes these tests deterministic rather than timing-dependent.
fn wait(report: &async_channel::Receiver<Report>) -> Report {
    report.recv_blocking().expect("every job reports")
}

#[test]
fn a_job_through_the_queue_produces_the_same_tree_as_a_direct_run() {
    // The concurrency wrapper changes timing, not results.
    let (_dir, root) = fixture();
    let queue = JobQueue::new();
    let before = snapshot(&root.child("tree"));

    let handle = queue.submit(
        Job::Copy {
            sources: vec![root.child("tree")],
            target_dir: root.child("into"),
        },
        backend(),
        backend(),
    );
    let report = wait(&handle.report);

    assert!(report.is_clean(), "{report:?}");
    assert_eq!(snapshot(&root.child("into").child("tree")), before);
}

#[test]
fn progress_deltas_still_add_up_when_they_arrive_over_a_channel() {
    let (_dir, root) = fixture();
    let queue = JobQueue::new();

    let handle = queue.submit(
        Job::Copy {
            sources: vec![root.child("tree")],
            target_dir: root.child("into"),
        },
        backend(),
        backend(),
    );
    wait(&handle.report);

    let mut scanned = 0;
    let mut advanced = 0;
    while let Ok(event) = handle.progress.try_recv() {
        match event {
            Progress::Scanned { bytes, .. } => scanned = bytes,
            Progress::Advanced { bytes } => advanced += bytes,
            _ => {}
        }
    }
    assert_eq!(advanced, scanned);
    assert!(scanned > 0);
}

#[test]
fn a_conflict_is_answered_over_the_channel_and_the_job_carries_on() {
    let (dir, root) = fixture();
    std::fs::create_dir(dir.path().join("into/tree")).unwrap();
    std::fs::write(dir.path().join("into/tree/a.txt"), "old").unwrap();
    let queue = JobQueue::new();

    let handle = queue.submit(
        Job::Copy {
            sources: vec![root.child("tree")],
            target_dir: root.child("into"),
        },
        backend(),
        backend(),
    );

    let request = handle.conflicts.recv_blocking().expect("a.txt collides");
    assert_eq!(request.conflict.target, root.child("into/tree/a.txt"));
    request.answer(Answer::once(Resolution::Overwrite));

    let report = wait(&handle.report);
    assert!(report.is_clean(), "{report:?}");
    assert_eq!(
        snapshot(&root.child("into").child("tree")),
        snapshot(&root.child("tree"))
    );
}

#[test]
fn a_conflict_question_dropped_unanswered_aborts_instead_of_hanging() {
    // If the caller is gone, or a dialog is dismissed without deciding, the
    // worker must not sit on a channel forever holding a half-copied tree.
    // Abort, not skip: silence is not consent to overwrite anything.
    let (dir, root) = fixture();
    std::fs::create_dir(dir.path().join("into/tree")).unwrap();
    std::fs::write(dir.path().join("into/tree/a.txt"), "old").unwrap();
    let queue = JobQueue::new();

    let handle = queue.submit(
        Job::Copy {
            sources: vec![root.child("tree")],
            target_dir: root.child("into"),
        },
        backend(),
        backend(),
    );

    drop(handle.conflicts.recv_blocking().expect("a.txt collides"));

    assert_eq!(wait(&handle.report).outcome, Outcome::Aborted);
    assert_eq!(
        snapshot(&root.child("into").child("tree")),
        BTreeMap::from([("/a.txt".to_string(), b"old".to_vec())]),
        "an abort must leave the target exactly as it was"
    );
}

#[test]
fn a_cancel_from_another_thread_stops_a_running_job() {
    // Deterministic by construction: the worker is parked on a conflict
    // question, so the cancel is guaranteed to reach it mid-job rather than
    // racing the end of one.
    let (dir, root) = fixture();
    std::fs::create_dir(dir.path().join("into/tree")).unwrap();
    std::fs::write(dir.path().join("into/tree/a.txt"), "old").unwrap();
    let queue = JobQueue::new();

    let handle = queue.submit(
        Job::Copy {
            sources: vec![root.child("tree")],
            target_dir: root.child("into"),
        },
        backend(),
        backend(),
    );

    let request = handle.conflicts.recv_blocking().expect("a.txt collides");
    handle.cancel.cancel();
    request.answer(Answer::once(Resolution::Skip));

    assert_eq!(wait(&handle.report).outcome, Outcome::Cancelled);
}

#[test]
fn queued_jobs_run_in_the_order_they_were_submitted() {
    // Checked by effect rather than by timestamp: the second job can only
    // succeed if the first one already ran.
    let (_dir, root) = fixture();
    let queue = JobQueue::new();

    let first = queue.submit(
        Job::CreateDir {
            path: root.child("outer"),
        },
        backend(),
        backend(),
    );
    let second = queue.submit(
        Job::CreateDir {
            path: root.child("outer/inner"),
        },
        backend(),
        backend(),
    );

    assert!(wait(&first.report).is_clean());
    assert!(
        wait(&second.report).is_clean(),
        "the nested directory proves the first job finished first"
    );
}

#[test]
fn one_jobs_events_never_reach_another_jobs_handle() {
    let (_dir, root) = fixture();
    let queue = JobQueue::new();

    let copy = queue.submit(
        Job::Copy {
            sources: vec![root.child("tree")],
            target_dir: root.child("into"),
        },
        backend(),
        backend(),
    );
    let mkdir = queue.submit(
        Job::CreateDir {
            path: root.child("solo"),
        },
        backend(),
        backend(),
    );
    wait(&copy.report);
    wait(&mkdir.report);

    let mkdir_events: Vec<_> = std::iter::from_fn(|| mkdir.progress.try_recv().ok()).collect();
    assert!(
        mkdir_events
            .iter()
            .all(|event| !matches!(event, Progress::Advanced { .. })),
        "a mkdir moves no bytes, so it cannot have copy progress in it"
    );
    assert!(!mkdir_events.is_empty(), "it still reported something");
}

#[test]
fn dropping_the_queue_waits_for_the_work_it_accepted() {
    let (_dir, root) = fixture();
    let queue = JobQueue::new();
    let handle = queue.submit(
        Job::Delete {
            paths: vec![root.child("tree")],
            mode: DeleteMode::Permanent,
        },
        backend(),
        backend(),
    );

    drop(queue);

    assert!(wait(&handle.report).is_clean());
    assert!(LocalFs.stat(&root.child("tree")).is_err());
}
