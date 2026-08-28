//! Being told a directory changed.
//!
//! These tests wait on real filesystem events, so they have deadlines rather
//! than sleeps: a fixed sleep is either flaky or slow, and on a loaded machine
//! it is both.

use std::time::{Duration, Instant};

use tc_core::vfs::VfsPath;
use tc_core::watch::{Changes, Watch, QUIET_PERIOD};
use tempfile::TempDir;

/// How long a nudge may take to arrive. Generous: it is the quiet period plus
/// however long the platform took to notice.
const NUDGE_TIMEOUT: Duration = Duration::from_secs(5);

fn path_of(dir: &TempDir) -> VfsPath {
    VfsPath::new(dir.path().to_str().unwrap())
}

/// Waits for one nudge, or says how long it waited in vain.
fn await_nudge(changes: &Changes) {
    let deadline = Instant::now() + NUDGE_TIMEOUT;
    while Instant::now() < deadline {
        if changes.try_recv().is_ok() {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("no nudge arrived within {NUDGE_TIMEOUT:?}");
}

/// Whether any nudge arrives within `patience`. For the negative cases.
fn nudged_within(changes: &Changes, patience: Duration) -> bool {
    let deadline = Instant::now() + patience;
    while Instant::now() < deadline {
        if changes.try_recv().is_ok() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    false
}

#[test]
fn a_file_appearing_nudges_the_watcher() {
    let dir = TempDir::new().unwrap();
    let watch = Watch::start(&path_of(&dir)).expect("the directory can be watched");
    let changes = watch.changes();

    std::fs::write(dir.path().join("appeared.txt"), "hello").unwrap();

    await_nudge(&changes);
}

#[test]
fn a_file_vanishing_nudges_the_watcher() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("doomed.txt"), "hello").unwrap();
    let watch = Watch::start(&path_of(&dir)).expect("the directory can be watched");
    let changes = watch.changes();

    std::fs::remove_file(dir.path().join("doomed.txt")).unwrap();

    await_nudge(&changes);
}

#[test]
fn a_quiet_directory_says_nothing() {
    // A watcher that nudged on its own would have every pane re-reading
    // fifty thousand entries for no reason at all.
    let dir = TempDir::new().unwrap();
    let watch = Watch::start(&path_of(&dir)).expect("the directory can be watched");

    assert!(
        !nudged_within(&watch.changes(), QUIET_PERIOD * 4),
        "a nudge arrived with nothing to report"
    );
}

#[test]
fn a_burst_of_changes_is_one_nudge_and_not_a_hundred() {
    // The property the whole design turns on: an unpacking archive fires an
    // event per file, and a re-read of a large directory per event would make
    // the program unusable exactly when it is busiest.
    let dir = TempDir::new().unwrap();
    let watch = Watch::start(&path_of(&dir)).expect("the directory can be watched");
    let changes = watch.changes();

    for index in 0..200 {
        std::fs::write(dir.path().join(format!("file-{index}")), "x").unwrap();
    }

    await_nudge(&changes);
    // Whatever else the burst produced, it is not a nudge per file: the
    // channel holds one, and the burst is over.
    assert!(
        !nudged_within(&changes, QUIET_PERIOD * 4),
        "the burst produced more than one settled nudge"
    );
}

#[test]
fn dropping_the_watch_stops_the_watching() {
    // A pane replaces its watch on every navigation, so one that outlived its
    // directory would leave a thread and an inotify slot behind per step.
    let dir = TempDir::new().unwrap();
    let changes = {
        let watch = Watch::start(&path_of(&dir)).expect("the directory can be watched");
        watch.changes()
    };

    std::fs::write(dir.path().join("after.txt"), "hello").unwrap();

    assert!(
        !nudged_within(&changes, QUIET_PERIOD * 4),
        "a dropped watch was still watching"
    );
}

#[test]
fn a_directory_that_is_not_there_is_not_a_failure() {
    // A pane that cannot be watched is one that does not refresh itself, not
    // one that fails to open.
    assert!(Watch::start(&VfsPath::new("/no/such/directory/anywhere")).is_none());
}
