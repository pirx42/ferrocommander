//! Telling a pane that its directory changed under it.
//!
//! In `tc-core` because watching a directory is a filesystem concern and the
//! UI does not reach past its own layer. The platform difference — inotify on
//! Linux, `ReadDirectoryChangesW` on Windows — is behind one interface, which
//! is the shape every platform difference in this project takes.
//!
//! **Coalesced, not streamed.** An unpacking archive fires an event per file,
//! and a re-read of a fifty-thousand-entry directory per event would make the
//! program unusable exactly when it is busiest (`docs/performance.md`). The
//! watcher sends *one* nudge per quiet period and says nothing about what
//! changed: the pane re-reads the whole directory anyway, so which file moved
//! is not information it has any use for.

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use notify::{RecursiveMode, Watcher as _};

use crate::vfs::VfsPath;

/// How long the directory must be quiet before a nudge is sent.
///
/// Long enough that unpacking an archive costs a handful of re-reads rather
/// than one per file; short enough that a file appearing feels immediate. The
/// timer restarts on every event, so a directory under continuous change is
/// re-read when it settles rather than never.
pub const QUIET_PERIOD: Duration = Duration::from_millis(250);

/// Where a watcher's nudges arrive.
///
/// Named here so the shell can hold one without naming the channel crate,
/// which is `tc-core`'s dependency and not the UI's.
pub type Changes = async_channel::Receiver<()>;

/// Watches one directory and nudges when it settles after a change.
///
/// Dropping it stops the watching: the pane owns one of these per directory it
/// is showing, and replaces it on every navigation.
pub struct Watch {
    /// Held only to be dropped. `notify` stops watching when its watcher goes,
    /// and the worker thread ends when the channel it reads closes with it.
    _watcher: notify::RecommendedWatcher,
    /// Told to stop when this `Watch` is dropped, so the worker does not
    /// outlive the pane it speaks to.
    changes: Changes,
}

impl Watch {
    /// Starts watching `directory`, or returns `None` if it cannot be watched.
    ///
    /// `None` rather than an error: a directory that cannot be watched — a
    /// network mount, a filesystem the platform does not cover, a permission
    /// that is not there — is a pane that does not refresh itself, not a pane
    /// that fails to open. `Ctrl+R` still works, which is most of why it
    /// exists.
    pub fn start(directory: &VfsPath) -> Option<Watch> {
        let (raw_sender, raw_receiver) = mpsc::channel();
        let mut watcher =
            notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
                // The event itself is discarded: the pane re-reads the whole
                // directory, so what changed is of no use to it.
                let _ = raw_sender.send(event.is_ok());
            })
            .ok()?;
        watcher
            .watch(
                &crate::vfs::to_std_path(directory),
                // The directory itself, not the tree below it: a pane shows
                // one directory, and watching a whole subtree would wake it
                // for changes it is not displaying.
                RecursiveMode::NonRecursive,
            )
            .ok()?;

        let (sender, changes) = async_channel::bounded(1);
        thread::spawn(move || coalesce(&raw_receiver, &sender));

        Some(Watch {
            _watcher: watcher,
            changes,
        })
    }

    /// Resolves each time the directory has settled after changing.
    pub fn changes(&self) -> Changes {
        self.changes.clone()
    }
}

/// Turns a burst of events into one nudge, once the burst stops.
///
/// Waits for the first event, then keeps waiting while more arrive within
/// [`QUIET_PERIOD`]. Ends when the watcher is dropped and its channel closes.
fn coalesce(events: &mpsc::Receiver<bool>, sender: &async_channel::Sender<()>) {
    while events.recv().is_ok() {
        // Drain whatever else is already queued, then wait out the quiet
        // period; anything arriving inside it restarts the wait.
        while events.recv_timeout(QUIET_PERIOD).is_ok() {}
        // A full channel is a nudge the pane has not read yet, and one nudge
        // is as good as two — it re-reads everything either way.
        if sender.try_send(()).is_err() && sender.is_closed() {
            return;
        }
    }
}
