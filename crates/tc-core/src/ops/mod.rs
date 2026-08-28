//! The file-operation engine: what F5, F6, F7 and F8 actually do.
//!
//! A job is scanned into tasks, then executed. Nothing here knows about
//! threads or widgets — conflicts are answered through a
//! [`ConflictResolver`], progress goes to a [`ProgressSink`], and stopping is
//! a [`CancelToken`]. The queue in `ops::queue` supplies channel-backed
//! versions of all three; every test in this crate supplies a `Vec` and a
//! script.

pub mod cancel;
pub mod conflict;
pub mod constants;
pub mod plan;
pub mod progress;
pub mod queue;

use std::io::{Read, Write};

use crate::vfs::{VfsError, VfsPath, VirtualFs};

pub use cancel::CancelToken;
pub use conflict::{Answer, ApplyToAll, Conflict, ConflictResolver, Resolution};
pub use plan::{Item, Plan, Task};
pub use progress::{Outcome, Progress, ProgressSink, Report, Silent};
pub use queue::{ConflictRequest, JobHandle, JobQueue};

use conflict::{conflict_at, free_name_beside};
use constants::COPY_BUFFER_BYTES;

/// Where a deleted entry goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeleteMode {
    /// Recoverable, and the default — the user can change their mind.
    Trash,
    /// Gone. Needs Shift plus a confirmation in the UI.
    Permanent,
}

/// Where the things a job moves are supposed to end up.
///
/// One type rather than separate "into a directory" and "to a name" jobs:
/// the F5 and F6 dialogs are the same dialog, and what the user typed is what
/// decides. Copying a file to a new name in its own directory (a duplicate)
/// and renaming one in place are then the same shape, not two special cases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Destination {
    /// Every source keeps its own name and lands inside this directory.
    Into(VfsPath),
    /// One source lands at exactly this path.
    Exact(VfsPath),
}

impl Destination {
    /// Where `source` lands.
    fn of(&self, source: &VfsPath) -> VfsPath {
        match self {
            Destination::Into(dir) => dir.child(source.file_name().unwrap_or_default()),
            Destination::Exact(path) => path.clone(),
        }
    }
}

/// One thing the user asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Job {
    /// F5.
    Copy {
        sources: Vec<VfsPath>,
        destination: Destination,
    },
    /// F6. A rename is a move whose destination is [`Destination::Exact`].
    Move {
        sources: Vec<VfsPath>,
        destination: Destination,
    },
    /// F8 / Del.
    Delete {
        paths: Vec<VfsPath>,
        mode: DeleteMode,
    },
    /// F7.
    CreateDir { path: VfsPath },
}

/// Runs a job to completion and reports what happened.
///
/// `source_fs` is read from, `target_fs` is written to; for a job that stays
/// on one side, pass the same backend twice.
///
/// **Move and Rename are single-store operations.** They try one `rename` for
/// a whole tree and fall back to copy + delete only on
/// [`VfsError::CrossDevice`]. Phase 2's UI has a single backend, so that
/// holds; phase 6, which introduces a second one, is where a cross-store move
/// has to be told apart.
pub fn run(
    job: &Job,
    source_fs: &dyn VirtualFs,
    target_fs: &dyn VirtualFs,
    resolver: &mut dyn ConflictResolver,
    progress: &mut dyn ProgressSink,
    cancel: &CancelToken,
) -> Report {
    let mut run = Run {
        source_fs,
        target_fs,
        resolver,
        progress,
        cancel,
        failures: Vec::new(),
        skips: 0,
        outcome: Outcome::Completed,
        redirects: Vec::new(),
    };
    run.job(job);
    Report {
        outcome: run.outcome,
        failures: run.failures,
    }
}

/// Whether the job carries on after a step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Flow {
    Continue,
    Stop,
}

/// Where a settled conflict leaves the task that hit it.
enum Landing {
    /// Go ahead at this path. `replacing` when something has to go first.
    Proceed { target: VfsPath, replacing: bool },
    /// The user chose to leave it alone.
    Skip,
    /// The user ended the job.
    Abort,
    /// Something went wrong and has already been reported.
    Failed,
}

struct Run<'a> {
    source_fs: &'a dyn VirtualFs,
    target_fs: &'a dyn VirtualFs,
    resolver: &'a mut dyn ConflictResolver,
    progress: &'a mut dyn ProgressSink,
    cancel: &'a CancelToken,
    failures: Vec<(VfsPath, VfsError)>,
    /// How many destinations the user chose to leave alone. Counted, not
    /// just logged: a move may only delete a source whose every file
    /// actually arrived, and a skip is not an arrival.
    skips: usize,
    outcome: Outcome,
    /// Destinations that moved after the plan was made, because a conflict
    /// was resolved by keeping both. Applied as a path prefix, so redirecting
    /// a directory carries everything planned inside it along.
    redirects: Vec<(VfsPath, VfsPath)>,
}

impl Run<'_> {
    fn job(&mut self, job: &Job) {
        match job {
            Job::CreateDir { path } => self.create_dir(path),
            Job::Copy {
                sources,
                destination,
            } => {
                let plan =
                    plan::plan_transfer(self.source_fs, sources, |source| destination.of(source));
                self.transfer(plan, false);
            }
            Job::Move {
                sources,
                destination,
            } => {
                let plan =
                    plan::plan_transfer(self.source_fs, sources, |source| destination.of(source));
                self.transfer(plan, true);
            }
            Job::Delete { paths, mode } => {
                let plan = plan::plan_removal(self.source_fs, paths, *mode == DeleteMode::Trash);
                self.remove(plan);
            }
        }
    }

    fn create_dir(&mut self, path: &VfsPath) {
        self.progress.emit(Progress::Started { path: path.clone() });
        match self.target_fs.create_dir(path) {
            Ok(()) => self
                .progress
                .emit(Progress::Finished { path: path.clone() }),
            Err(error) => self.fail(path, error),
        }
    }

    fn transfer(&mut self, plan: Plan, moving: bool) {
        self.announce(&plan);
        for item in &plan.items {
            self.redirects.clear();
            if self.stopped() || self.item(item, moving) == Flow::Stop {
                return;
            }
        }
    }

    fn remove(&mut self, plan: Plan) {
        self.announce(&plan);
        for item in &plan.items {
            for (path, error) in &item.failures {
                self.fail(path, error.clone());
            }
            for task in &item.tasks {
                if self.stopped() || self.task(task) == Flow::Stop {
                    return;
                }
            }
        }
    }

    /// Reports the totals and anything the scan already could not read.
    fn announce(&mut self, plan: &Plan) {
        self.progress.emit(Progress::Scanned {
            files: plan.total_files,
            bytes: plan.total_bytes,
        });
        for (path, error) in &plan.failures {
            self.fail(path, error.clone());
        }
    }

    /// One top-level source, copied or moved.
    fn item(&mut self, item: &Item, moving: bool) -> Flow {
        if moving && self.renamed_whole(item) {
            return Flow::Continue;
        }

        let failures_before = self.failures.len();
        let skips_before = self.skips;
        // What the scan could not turn into a task is reported here rather
        // than at scan time, so it counts against this item's completeness
        // and a move will not delete a source it could not fully copy.
        for (path, error) in &item.failures {
            self.fail(path, error.clone());
        }
        for task in &item.tasks {
            if self.task(task) == Flow::Stop {
                return Flow::Stop;
            }
        }

        // Deleting the source is only safe when everything arrived. A move
        // that skipped or failed one file must not delete that file — the
        // copy is the only reason the original is expendable.
        let complete = self.failures.len() == failures_before && self.skips == skips_before;
        if moving && complete {
            let removal =
                plan::plan_removal(self.source_fs, std::slice::from_ref(&item.source), false);
            for task in removal.items.iter().flat_map(|item| &item.tasks) {
                if self.task(task) == Flow::Stop {
                    return Flow::Stop;
                }
            }
        }
        Flow::Continue
    }

    /// Tries to move a whole tree with one `rename`, returning whether the
    /// item is now dealt with.
    ///
    /// Only attempted when the destination is free: a rename cannot merge
    /// into an existing directory, and it would silently replace an existing
    /// file without asking.
    fn renamed_whole(&mut self, item: &Item) -> bool {
        if self.target_fs.stat(&item.target) != Err(VfsError::NotFound) {
            return false;
        }
        self.progress.emit(Progress::Started {
            path: item.source.clone(),
        });
        match self.target_fs.rename(&item.source, &item.target) {
            Ok(()) => {
                self.progress.emit(Progress::Advanced { bytes: item.bytes });
                self.progress.emit(Progress::Finished {
                    path: item.source.clone(),
                });
                true
            }
            // The one error that means "possible, just not in one step".
            Err(VfsError::CrossDevice) => false,
            Err(error) => {
                self.fail(&item.source, error);
                true
            }
        }
    }

    fn task(&mut self, task: &Task) -> Flow {
        if self.stopped() {
            return Flow::Stop;
        }
        match task {
            Task::MakeDir { source, target } => self.make_dir(source, target),
            Task::CopyFile {
                source,
                target,
                size,
                modified,
            } => self.copy_file(source, target, *size, *modified),
            Task::RemoveFile { path } => self.simple(path, |fs, p| fs.remove_file(p)),
            Task::RemoveDir { path } => self.simple(path, |fs, p| fs.remove_dir(p)),
            Task::Trash { path } => self.simple(path, |fs, p| fs.trash(p)),
        }
    }

    /// A task that is one call on the source backend and has no conflict.
    fn simple(
        &mut self,
        path: &VfsPath,
        call: impl Fn(&dyn VirtualFs, &VfsPath) -> Result<(), VfsError>,
    ) -> Flow {
        self.progress.emit(Progress::Started { path: path.clone() });
        match call(self.source_fs, path) {
            Ok(()) => {
                self.progress
                    .emit(Progress::Finished { path: path.clone() });
            }
            Err(error) => self.fail(path, error),
        }
        Flow::Continue
    }

    /// What settling a conflict left the caller to do.
    ///
    /// The four resolutions mean the same thing wherever a conflict is met;
    /// only the work afterwards differs. Deciding once and acting twice is
    /// what keeps `make_dir` and `copy_file` from being two copies of the
    /// same match (skill 44).
    fn settle(&mut self, source: &VfsPath, is_dir: bool, target: VfsPath) -> Landing {
        let conflict = match conflict_at(self.target_fs, source, is_dir, &target) {
            // Nothing there, or an existing directory to merge into.
            Ok(None) => {
                return Landing::Proceed {
                    target,
                    replacing: false,
                }
            }
            Ok(Some(conflict)) => conflict,
            Err(error) => {
                self.fail(&target, error);
                return Landing::Failed;
            }
        };

        match self.ask(&conflict) {
            Resolution::Overwrite => Landing::Proceed {
                target,
                replacing: true,
            },
            Resolution::Skip => {
                self.skips += 1;
                Landing::Skip
            }
            Resolution::KeepBoth => match free_name_beside(self.target_fs, &target) {
                Ok(free) => {
                    // A directory's contents were planned under the old name,
                    // so the redirect has to carry them along, however deep.
                    // A file has no contents and needs no entry — which also
                    // keeps the redirect list short enough to scan per task.
                    if is_dir {
                        self.redirects.push((target, free.clone()));
                    }
                    Landing::Proceed {
                        target: free,
                        replacing: false,
                    }
                }
                Err(error) => {
                    self.fail(&target, error);
                    Landing::Failed
                }
            },
            Resolution::Abort => {
                self.outcome = Outcome::Aborted;
                Landing::Abort
            }
        }
    }

    fn make_dir(&mut self, source: &VfsPath, target: &VfsPath) -> Flow {
        let target = self.redirected(target);
        let (target, replacing) = match self.settle(source, true, target) {
            Landing::Proceed { target, replacing } => (target, replacing),
            Landing::Skip | Landing::Failed => return Flow::Continue,
            Landing::Abort => return Flow::Stop,
        };

        if replacing {
            if let Err(error) = self.target_fs.remove_file(&target) {
                self.fail(&target, error);
                return Flow::Continue;
            }
        }
        // Merging into a directory that is already there needs no create.
        if self.target_fs.stat(&target) == Err(VfsError::NotFound) {
            if let Err(error) = self.target_fs.create_dir(&target) {
                self.fail(&target, error);
            }
        }
        Flow::Continue
    }

    fn copy_file(
        &mut self,
        source: &VfsPath,
        target: &VfsPath,
        size: u64,
        modified: std::time::SystemTime,
    ) -> Flow {
        let target = self.redirected(target);
        let (target, overwriting) = match self.settle(source, false, target) {
            Landing::Proceed { target, replacing } => (target, replacing),
            Landing::Skip => {
                // Count the bytes anyway, so the bar still reaches the total
                // the scan promised.
                self.progress.emit(Progress::Advanced { bytes: size });
                return Flow::Continue;
            }
            Landing::Failed => return Flow::Continue,
            Landing::Abort => return Flow::Stop,
        };

        self.progress.emit(Progress::Started {
            path: source.clone(),
        });
        match self.stream(source, &target) {
            Ok(true) => {
                // A copy that keeps the original's date is a copy of the
                // file, not a new file with the same bytes.
                if let Err(error) = self.target_fs.set_modified(&target, modified) {
                    self.fail(&target, error);
                }
                self.progress.emit(Progress::Finished {
                    path: source.clone(),
                });
                Flow::Continue
            }
            // Cancelled mid-file.
            Ok(false) => {
                self.discard_partial(&target, overwriting);
                self.outcome = Outcome::Cancelled;
                Flow::Stop
            }
            Err(error) => {
                self.discard_partial(&target, overwriting);
                self.fail(source, error);
                Flow::Continue
            }
        }
    }

    /// Copies the bytes across. `Ok(false)` means a cancel interrupted it.
    fn stream(&mut self, source: &VfsPath, target: &VfsPath) -> Result<bool, VfsError> {
        let mut reader = self.source_fs.open_read(source)?;
        let mut writer = self.target_fs.create_file(target)?;
        let mut buffer = vec![0u8; COPY_BUFFER_BYTES];
        loop {
            if self.cancel.is_cancelled() {
                return Ok(false);
            }
            let read = reader.read(&mut buffer)?;
            if read == 0 {
                return Ok(true);
            }
            writer.write_all(&buffer[..read])?;
            self.progress
                .emit(Progress::Advanced { bytes: read as u64 });
        }
    }

    /// Removes a half-written destination, so the target never holds a
    /// truncated file.
    ///
    /// Not when overwriting: the original was already gone the moment the
    /// file was truncated, so deleting would leave the user with neither
    /// copy instead of one damaged one.
    fn discard_partial(&mut self, target: &VfsPath, overwriting: bool) {
        if !overwriting {
            let _ = self.target_fs.remove_file(target);
        }
    }

    fn ask(&mut self, conflict: &Conflict) -> Resolution {
        self.resolver.resolve(conflict).resolution
    }

    /// Applies any destination redirect a resolved conflict introduced.
    fn redirected(&self, target: &VfsPath) -> VfsPath {
        let mut result = target.clone();
        for (from, to) in &self.redirects {
            if let Some(rest) = result.as_str().strip_prefix(from.as_str()) {
                if rest.is_empty() || rest.starts_with('/') {
                    result = VfsPath::new(&format!("{to}{rest}"));
                }
            }
        }
        result
    }

    fn fail(&mut self, path: &VfsPath, error: VfsError) {
        self.progress.emit(Progress::Failed {
            path: path.clone(),
            error: error.clone(),
        });
        self.failures.push((path.clone(), error));
    }

    fn stopped(&mut self) -> bool {
        if self.cancel.is_cancelled() {
            self.outcome = Outcome::Cancelled;
            return true;
        }
        self.outcome != Outcome::Completed
    }
}
