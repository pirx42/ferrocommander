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

use std::io::{Read, Write};

use crate::vfs::{VfsError, VfsPath, VirtualFs};

pub use cancel::CancelToken;
pub use conflict::{Answer, ApplyToAll, Conflict, ConflictResolver, Resolution};
pub use plan::{Item, Plan, Task};
pub use progress::{Outcome, Progress, ProgressSink, Report, Silent};

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

/// One thing the user asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Job {
    /// F5: copy each source into `target_dir` under its own name.
    Copy {
        sources: Vec<VfsPath>,
        target_dir: VfsPath,
    },
    /// F6 with a directory as the target: move each source into it.
    Move {
        sources: Vec<VfsPath>,
        target_dir: VfsPath,
    },
    /// F6 with the target edited down to a name: move one path to an exact
    /// new path. Distinct from `Move` because it is a distinct intent, not
    /// because it executes differently.
    Rename { source: VfsPath, target: VfsPath },
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
                target_dir,
            } => {
                let plan = plan::plan_transfer(self.source_fs, sources, |source| {
                    target_dir.child(source.file_name().unwrap_or_default())
                });
                self.transfer(plan, false);
            }
            Job::Move {
                sources,
                target_dir,
            } => {
                let plan = plan::plan_transfer(self.source_fs, sources, |source| {
                    target_dir.child(source.file_name().unwrap_or_default())
                });
                self.transfer(plan, true);
            }
            Job::Rename { source, target } => {
                let plan =
                    plan::plan_transfer(self.source_fs, std::slice::from_ref(source), |_| {
                        target.clone()
                    });
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

    fn make_dir(&mut self, source: &VfsPath, target: &VfsPath) -> Flow {
        let target = self.redirected(target);
        match conflict_at(self.target_fs, source, true, &target) {
            // Nothing there, or an existing directory to merge into.
            Ok(None) => {}
            Ok(Some(conflict)) => match self.ask(&conflict) {
                Resolution::Overwrite => {
                    if let Err(error) = self.target_fs.remove_file(&target) {
                        self.fail(&target, error);
                        return Flow::Continue;
                    }
                }
                Resolution::Skip => {
                    self.skips += 1;
                    return Flow::Continue;
                }
                Resolution::KeepBoth => match free_name_beside(self.target_fs, &target) {
                    Ok(free) => {
                        // Redirecting the directory carries everything the
                        // plan put inside it along, however deep.
                        self.redirects.push((target.clone(), free));
                        return self.make_dir(source, &target);
                    }
                    Err(error) => {
                        self.fail(&target, error);
                        return Flow::Continue;
                    }
                },
                Resolution::Abort => {
                    self.outcome = Outcome::Aborted;
                    return Flow::Stop;
                }
            },
            Err(error) => {
                self.fail(&target, error);
                return Flow::Continue;
            }
        }
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
        let mut target = self.redirected(target);
        let mut overwriting = false;

        match conflict_at(self.target_fs, source, false, &target) {
            Ok(None) => {}
            Ok(Some(conflict)) => match self.ask(&conflict) {
                Resolution::Overwrite => overwriting = true,
                Resolution::Skip => {
                    self.skips += 1;
                    // Count the bytes anyway, so the bar still reaches the
                    // total the scan promised.
                    self.progress.emit(Progress::Advanced { bytes: size });
                    return Flow::Continue;
                }
                Resolution::KeepBoth => match free_name_beside(self.target_fs, &target) {
                    Ok(free) => target = free,
                    Err(error) => {
                        self.fail(source, error);
                        return Flow::Continue;
                    }
                },
                Resolution::Abort => {
                    self.outcome = Outcome::Aborted;
                    return Flow::Stop;
                }
            },
            Err(error) => {
                self.fail(source, error);
                return Flow::Continue;
            }
        }

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
