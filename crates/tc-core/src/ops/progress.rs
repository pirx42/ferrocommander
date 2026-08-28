//! What a running job tells whoever is watching.

use crate::vfs::{VfsError, VfsPath};

/// One thing that happened while a job ran.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Progress {
    /// The scan finished. Everything after this can be measured against
    /// these totals — which is the reason the scan exists: a progress bar
    /// that has to guess its total is a progress bar that lies.
    Scanned { files: usize, bytes: u64 },
    /// Work on one path began.
    Started { path: VfsPath },
    /// Bytes moved **since the last event**.
    ///
    /// A delta rather than a running total, so no consumer has to know the
    /// event ordering to add it up, and a dropped event costs a little
    /// accuracy instead of a bar that jumps backwards.
    Advanced { bytes: u64 },
    /// Work on one path finished successfully.
    Finished { path: VfsPath },
    /// One path failed. The job carries on — a batch that stops at the first
    /// unreadable file is worse than useless on a big tree.
    Failed { path: VfsPath, error: VfsError },
}

/// Where progress events go.
///
/// A trait rather than a channel, so the engine has no opinion about
/// threading: the queue sends events across one, and every test in this
/// module collects them in a `Vec`.
pub trait ProgressSink {
    fn emit(&mut self, event: Progress);
}

/// Discards everything. For callers that only want the job done.
pub struct Silent;

impl ProgressSink for Silent {
    fn emit(&mut self, _event: Progress) {}
}

impl ProgressSink for Vec<Progress> {
    fn emit(&mut self, event: Progress) {
        self.push(event);
    }
}

/// How a job ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Every task was attempted. Individual files may still have failed —
    /// see [`Report::failures`].
    Completed,
    /// The user cancelled it.
    Cancelled,
    /// A conflict was answered with [`super::Resolution::Abort`].
    Aborted,
}

/// What a finished job leaves behind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub outcome: Outcome,
    /// Every path that failed, with its reason. Collected instead of thrown,
    /// so a batch that hit six unreadable files needs one summary rather
    /// than six dialogs.
    pub failures: Vec<(VfsPath, VfsError)>,
}

impl Report {
    pub fn is_clean(&self) -> bool {
        self.outcome == Outcome::Completed && self.failures.is_empty()
    }
}
