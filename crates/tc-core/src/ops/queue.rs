//! Running jobs off the caller's thread.
//!
//! One worker thread takes jobs in the order they were submitted. Everything
//! the caller needs while a job runs — progress, conflict questions, the
//! cancel switch, the final report — hangs off the [`JobHandle`] the
//! submission returns.
//!
//! The engine itself knows none of this: it is handed a [`ConflictResolver`]
//! and a [`ProgressSink`], and this module supplies channel-backed versions.
//! That is why the whole of [`super`] is testable without a thread.

use std::sync::Arc;
use std::thread;

use async_channel::{bounded, unbounded, Receiver, Sender};

use crate::vfs::VirtualFs;

use super::{
    run, Answer, CancelToken, Conflict, ConflictResolver, Job, Progress, ProgressSink, Report,
    Resolution,
};

/// A conflict question with the way to answer it.
///
/// Answering consumes the request, so a question cannot be answered twice —
/// and dropping it unanswered is a well-defined outcome rather than a
/// deadlock.
pub struct ConflictRequest {
    pub conflict: Conflict,
    reply: Sender<Answer>,
}

impl ConflictRequest {
    pub fn answer(self, answer: Answer) {
        // The worker may already be gone if the job was cancelled meanwhile.
        let _ = self.reply.send_blocking(answer);
    }
}

/// Everything a caller needs while one job runs.
pub struct JobHandle {
    pub progress: Receiver<Progress>,
    pub conflicts: Receiver<ConflictRequest>,
    /// Yields exactly one [`Report`] when the job ends.
    pub report: Receiver<Report>,
    pub cancel: CancelToken,
}

/// Jobs, run one at a time in submission order.
pub struct JobQueue {
    submissions: Sender<Submission>,
    worker: Option<thread::JoinHandle<()>>,
}

struct Submission {
    job: Job,
    source_fs: Arc<dyn VirtualFs>,
    target_fs: Arc<dyn VirtualFs>,
    progress: Sender<Progress>,
    conflicts: Sender<ConflictRequest>,
    report: Sender<Report>,
    cancel: CancelToken,
}

impl JobQueue {
    pub fn new() -> Self {
        let (submissions, incoming) = unbounded::<Submission>();
        let worker = thread::spawn(move || {
            while let Ok(submission) = incoming.recv_blocking() {
                submission.execute();
            }
        });
        JobQueue {
            submissions,
            worker: Some(worker),
        }
    }

    /// Hands a job to the worker and returns the handle to watch it with.
    ///
    /// For a job that stays on one backend, pass the same handle twice.
    pub fn submit(
        &self,
        job: Job,
        source_fs: Arc<dyn VirtualFs>,
        target_fs: Arc<dyn VirtualFs>,
    ) -> JobHandle {
        // Progress is unbounded so a busy caller slows down the display
        // rather than the copy; a bounded channel here would let a stalled
        // UI throttle the disk.
        let (progress_tx, progress) = unbounded();
        let (conflicts_tx, conflicts) = unbounded();
        let (report_tx, report) = bounded(1);
        let cancel = CancelToken::new();

        let submission = Submission {
            job,
            source_fs,
            target_fs,
            progress: progress_tx,
            conflicts: conflicts_tx,
            report: report_tx,
            cancel: cancel.clone(),
        };
        // Unbounded, and the worker outlives the queue's own drop, so this
        // cannot fail while the queue exists.
        let _ = self.submissions.send_blocking(submission);

        JobHandle {
            progress,
            conflicts,
            report,
            cancel,
        }
    }
}

impl Default for JobQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for JobQueue {
    /// Lets the worker finish what it has and waits for it.
    ///
    /// Waiting cannot hang: a job blocked on a conflict wakes as soon as the
    /// caller drops the question, which is exactly what dropping the handles
    /// does.
    fn drop(&mut self) {
        self.submissions.close();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

impl Submission {
    fn execute(self) {
        let mut resolver = ChannelResolver {
            requests: self.conflicts,
        };
        let mut progress = ChannelSink {
            events: self.progress,
        };
        let report = run(
            &self.job,
            self.source_fs.as_ref(),
            self.target_fs.as_ref(),
            &mut resolver,
            &mut progress,
            &self.cancel,
        );
        let _ = self.report.send_blocking(report);
    }
}

struct ChannelSink {
    events: Sender<Progress>,
}

impl ProgressSink for ChannelSink {
    fn emit(&mut self, event: Progress) {
        // A caller that stopped listening is not a reason to stop working.
        let _ = self.events.send_blocking(event);
    }
}

struct ChannelResolver {
    requests: Sender<ConflictRequest>,
}

impl ConflictResolver for ChannelResolver {
    /// Asks, and waits.
    ///
    /// **No answer means abort.** If the caller is gone, or a dialog is
    /// dismissed without deciding, the worker must not sit on a channel
    /// forever holding a half-copied tree. Abort rather than skip, because
    /// silence is not consent to overwrite anything, and aborting is the
    /// outcome a user can always recover from by starting again.
    fn resolve(&mut self, conflict: &Conflict) -> Answer {
        let (reply, answer) = bounded(1);
        let request = ConflictRequest {
            conflict: conflict.clone(),
            reply,
        };
        if self.requests.send_blocking(request).is_err() {
            return Answer::once(Resolution::Abort);
        }
        answer
            .recv_blocking()
            .unwrap_or(Answer::once(Resolution::Abort))
    }
}
