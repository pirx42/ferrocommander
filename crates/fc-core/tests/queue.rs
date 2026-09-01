//! Behavioral tests for the background job queue.
//!
//! No GTK and no main loop: the queue is a worker thread and four channels,
//! and every one of those is drivable from a test thread. What is checked
//! here is the wrapper — ordering, the conflict round trip, cancelling from
//! outside — not the engine, which `ops.rs` already pins.

use std::sync::Arc;

use fc_core::ops::{
    Answer, DeleteMode, Destination, Job, JobQueue, Outcome, Progress, Report, Resolution,
};
use fc_core::vfs::{LocalFs, VirtualFs};

mod common;

use common::{fixture, snapshot};

fn backend() -> Arc<dyn VirtualFs> {
    Arc::new(LocalFs)
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
    let before = snapshot(&LocalFs, &root.child("tree"));

    let handle = queue.submit(
        Job::Copy {
            sources: vec![root.child("tree")],
            destination: Destination::Into(root.child("into")),
        },
        backend(),
        backend(),
    );
    let report = wait(&handle.report);

    assert!(report.is_clean(), "{report:?}");
    assert_eq!(
        snapshot(&LocalFs, &root.child("into").child("tree")),
        before
    );
}

#[test]
fn progress_deltas_still_add_up_when_they_arrive_over_a_channel() {
    let (_dir, root) = fixture();
    let queue = JobQueue::new();

    let handle = queue.submit(
        Job::Copy {
            sources: vec![root.child("tree")],
            destination: Destination::Into(root.child("into")),
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
            destination: Destination::Into(root.child("into")),
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
        snapshot(&LocalFs, &root.child("into").child("tree")),
        snapshot(&LocalFs, &root.child("tree"))
    );
}

#[test]
fn a_conflict_question_dropped_unanswered_aborts_instead_of_hanging() {
    // If the caller is gone, or a dialog is dismissed without deciding, the
    // worker must not sit on a channel forever holding a half-copied tree.
    // Abort, not skip: silence is not consent to overwrite anything.
    //
    // Two sources, the conflicting one first. `plan_transfer` keeps the order
    // it was given, so the second source is provably untouched work — where
    // relying on the order `read_dir` happens to return would not be.
    let (dir, root) = fixture();
    std::fs::write(dir.path().join("into/a.txt"), "old").unwrap();
    let queue = JobQueue::new();

    let handle = queue.submit(
        Job::Copy {
            sources: vec![root.child("tree/a.txt"), root.child("tree/sub")],
            destination: Destination::Into(root.child("into")),
        },
        backend(),
        backend(),
    );

    drop(handle.conflicts.recv_blocking().expect("a.txt collides"));

    assert_eq!(wait(&handle.report).outcome, Outcome::Aborted);
    assert_eq!(
        snapshot(&LocalFs, &root.child("into")),
        common::Snapshot::from([("/a.txt".to_string(), Some(b"old".to_vec()))]),
        "an abort must leave the target as it was and never reach the rest"
    );
}

#[test]
fn a_cancel_from_another_thread_stops_a_running_job() {
    // Deterministic by construction twice over: the worker is parked on a
    // conflict question, so the cancel reaches it mid-job rather than racing
    // the end of one, and the second source proves there was still work left
    // to stop.
    let (dir, root) = fixture();
    std::fs::write(dir.path().join("into/a.txt"), "old").unwrap();
    let queue = JobQueue::new();

    let handle = queue.submit(
        Job::Copy {
            sources: vec![root.child("tree/a.txt"), root.child("tree/sub")],
            destination: Destination::Into(root.child("into")),
        },
        backend(),
        backend(),
    );

    let request = handle.conflicts.recv_blocking().expect("a.txt collides");
    handle.cancel.cancel();
    request.answer(Answer::once(Resolution::Skip));

    assert_eq!(wait(&handle.report).outcome, Outcome::Cancelled);
    assert!(
        LocalFs.stat(&root.child("into/sub")).is_err(),
        "the work after the cancel must not have happened"
    );
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
            destination: Destination::Into(root.child("into")),
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
