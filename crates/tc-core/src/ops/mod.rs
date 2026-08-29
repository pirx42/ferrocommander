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
use std::time::UNIX_EPOCH;

use crate::archive::constants::PACKING_SUFFIX;
use crate::archive::{self, Packer};
use crate::vfs::constants::SEPARATOR;
use crate::vfs::{Attributes, VfsError, VfsPath, VirtualFs};

pub use cancel::CancelToken;
pub use conflict::{Answer, ApplyToAll, Conflict, ConflictResolver, Resolution};
pub use plan::{Item, Plan, Task};
pub use progress::{Outcome, Progress, ProgressSink, Report, Silent};
pub use queue::{ConflictRequest, JobHandle, JobQueue};

use conflict::{conflict_at, free_name_beside};
use constants::{COPY_BUFFER_BYTES, INTO_ITSELF, ONTO_ITSELF, PACK_CANCELLED};

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
    /// Shift+F4. An empty file, for an editor to open.
    CreateFile { path: VfsPath },
    /// Alt+F5. Packs `sources` into a new archive on the target backend.
    ///
    /// The format comes from `archive`'s own name, by the same rule that
    /// decides whether Enter walks into a file — one rule, so a name this
    /// packs into is a name that opens again.
    Pack {
        sources: Vec<VfsPath>,
        archive: VfsPath,
    },
}

impl Job {
    /// What this job would create or replace, for a refusal that has to name
    /// something.
    ///
    /// The destination rather than the sources: a job refused because it
    /// cannot write is refused about where it was writing.
    fn writes_to(&self) -> Vec<VfsPath> {
        match self {
            Job::Copy {
                sources,
                destination,
            }
            | Job::Move {
                sources,
                destination,
            } => sources
                .iter()
                .map(|source| destination.of(source))
                .collect(),
            Job::Delete { paths, .. } => paths.clone(),
            Job::CreateDir { path } | Job::CreateFile { path } => vec![path.clone()],
            Job::Pack { archive, .. } => vec![archive.clone()],
        }
    }
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

/// Whether `path` lies below `directory`.
///
/// Compared on whole components — `/a/bc` is not inside `/a/b`, however much
/// the strings look alike.
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
        // A backend that cannot be written to says so once rather than a
        // thousand times: a copy of a large tree into an archive would
        // otherwise be a failure list nobody can read, with the same reason on
        // every line.
        //
        // A **delete** asks the source, because that is the backend it removes
        // from. A move asks the target, and only the target: moving *out of*
        // an archive is a copy that works followed by a delete that cannot,
        // which is worth doing and worth reporting per entry.
        let writing_to = match job {
            Job::Delete { .. } => self.source_fs,
            _ => self.target_fs,
        };
        if writing_to.read_only() {
            for path in job.writes_to() {
                self.fail(&path, VfsError::ReadOnly);
            }
            return;
        }
        match job {
            Job::CreateDir { path } => self.create_dir(path),
            Job::CreateFile { path } => self.create_file(path),
            Job::Copy {
                sources,
                destination,
            } => {
                let sources = self.refuse_self_targets(sources, destination);
                let plan =
                    plan::plan_transfer(self.source_fs, &sources, |source| destination.of(source));
                self.transfer(plan, false);
            }
            Job::Move {
                sources,
                destination,
            } => {
                // Try the rename before scanning anything. A rename moves a
                // whole tree in one call, so walking that tree first would be
                // the entire cost of an operation that is otherwise instant —
                // and on a large tree the walk is what the user would feel.
                let sources = self.refuse_self_targets(sources, destination);
                // The shortcut is a single `rename`, which only means anything
                // within one store. Across two, the source path handed to the
                // target backend addresses a different file that happens to be
                // spelled the same — `/packed.txt` inside an archive naming a
                // file at the root of the disk — so it is skipped entirely and
                // the move is a copy followed by a delete.
                let remaining = if self.one_store() {
                    let remaining = self.rename_what_it_can(&sources, destination);
                    if remaining.is_empty() {
                        return;
                    }
                    remaining
                } else {
                    sources
                };
                let plan = plan::plan_transfer(self.source_fs, &remaining, |source| {
                    destination.of(source)
                });
                self.transfer(plan, true);
            }
            Job::Pack { sources, archive } => self.pack(sources, archive),
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

    /// Creates an empty file, and refuses to touch one that is already there.
    ///
    /// The refusal is the whole point: `create_file` on the VFS truncates,
    /// and `Shift+F4` on a name that exists would then empty a file the user
    /// meant to open. Reported as a failure rather than asked about, because
    /// there is no version of this the user wants — the file they named is
    /// already there and already has something in it.
    fn create_file(&mut self, path: &VfsPath) {
        self.progress.emit(Progress::Started { path: path.clone() });
        if self.target_fs.stat(path).is_ok() {
            self.fail(path, VfsError::AlreadyExists);
            return;
        }
        match self.target_fs.create_file(path) {
            Ok(_) => self
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

    /// Packs `sources` into a new archive at `archive` on the target backend.
    ///
    /// The scan, the progress, the cancel and the failure list are the ones
    /// every other job uses; only "write these bytes at the destination" is
    /// different, and that is a [`Packer`](crate::archive::Packer). Nothing
    /// about a format reaches this module.
    ///
    /// The bytes go to a temporary name beside the archive and are renamed
    /// into place at the end, so an interrupted pack leaves nothing that looks
    /// like a finished archive. A rename within one directory is atomic, so
    /// there is no moment where the name exists holding half an archive.
    fn pack(&mut self, sources: &[VfsPath], archive: &VfsPath) {
        let Some(format) = archive.file_name().and_then(archive::format_for) else {
            self.fail(archive, VfsError::NotAnArchive);
            return;
        };
        let target = match self.settle(archive, false, archive.clone()) {
            Landing::Proceed { target, .. } => target,
            Landing::Skip | Landing::Failed => return,
            Landing::Abort => return,
        };

        // Named from the final name, so two packs into one directory cannot
        // collide on the temporary either.
        let partial = beside(&target, PACKING_SUFFIX);
        let plan = plan::plan_transfer(self.source_fs, sources, |source| {
            VfsPath::root().child(source.file_name().unwrap_or_default())
        });
        self.announce(&plan);

        let sink = match self.target_fs.create_file(&partial) {
            Ok(sink) => sink,
            Err(error) => return self.fail(&partial, error),
        };
        let outcome = self.fill(archive::packer(format, sink), &plan);

        // A pack that did not finish leaves no archive at all — not an empty
        // one, and not a half-written one under the name somebody will later
        // open.
        if outcome.is_err() || self.stopped() {
            let _ = self.target_fs.remove_file(&partial);
            if let Err(error) = outcome {
                self.fail(archive, error);
            }
            return;
        }
        if let Err(error) = self.target_fs.rename(&partial, &target) {
            let _ = self.target_fs.remove_file(&partial);
            self.fail(&target, error);
        }
    }

    /// Feeds a plan's tasks to a packer, stopping at the first refusal.
    ///
    /// Stopping rather than carrying on: a copy that skips one file leaves a
    /// tree missing a file, which the failure list explains. An archive that
    /// skipped one is a single file somebody will keep — so a pack is all or
    /// nothing.
    fn fill(&mut self, mut packer: Box<dyn Packer>, plan: &Plan) -> Result<(), VfsError> {
        for item in &plan.items {
            for (path, error) in &item.failures {
                self.fail(path, error.clone());
            }
            for task in &item.tasks {
                if self.stopped() {
                    return Ok(());
                }
                match task {
                    Task::MakeDir { source, target } => {
                        // A directory's date and mode are not on the task —
                        // no copy can restore a directory's date, so nothing
                        // needed them until now — and an archive can hold
                        // them, so they are worth one stat each.
                        let about = self.source_fs.stat(source).ok();
                        packer.add_dir(
                            &inside(target),
                            about.as_ref().map_or(UNIX_EPOCH, |entry| entry.modified),
                            about.map(|entry| entry.attributes).unwrap_or_default(),
                        )?
                    }
                    Task::CopyFile {
                        source,
                        target,
                        size,
                        modified,
                        attributes,
                    } => {
                        self.progress.emit(Progress::Started {
                            path: source.clone(),
                        });
                        let mut reader = self.source_fs.open_read(source)?;
                        let mut metered = Metered {
                            inner: &mut reader,
                            progress: self.progress,
                            cancel: self.cancel,
                        };
                        packer.add_file(
                            &inside(target),
                            *size,
                            *modified,
                            *attributes,
                            &mut metered,
                        )?;
                        self.progress.emit(Progress::Finished {
                            path: source.clone(),
                        });
                    }
                    // A pack builds; it never removes.
                    Task::RemoveFile { .. } | Task::RemoveDir { .. } | Task::Trash { .. } => {}
                }
            }
        }
        packer.finish()
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

    /// Drops the sources whose destination is themselves, or inside
    /// themselves, reporting each as a failure.
    ///
    /// Checked before anything else touches the disk, because both cases
    /// destroy data rather than merely failing. Copying a file onto itself
    /// truncates it before it is read, and the job would report success over
    /// an empty file; copying a directory into its own subtree never
    /// terminates, because the walk keeps finding what it just wrote.
    ///
    /// Neither is contrived. Both panes may be showing the same directory,
    /// and then the prefilled target *is* the source.
    fn refuse_self_targets(
        &mut self,
        sources: &[VfsPath],
        destination: &Destination,
    ) -> Vec<VfsPath> {
        // Between two stores the question does not arise: the paths look
        // alike and address different files, so `/a` in an archive copied to
        // `/a` on the disk is not a copy onto itself and refusing it would
        // refuse a perfectly ordinary unpack.
        if !self.one_store() {
            return sources.to_vec();
        }
        let mut allowed = Vec::new();
        for source in sources {
            let target = destination.of(source);
            let refusal = if target == *source {
                Some(ONTO_ITSELF)
            } else if target.is_inside(source) {
                Some(INTO_ITSELF)
            } else {
                None
            };
            match refusal {
                Some(reason) => self.fail(source, VfsError::Io(reason.to_string())),
                None => allowed.push(source.clone()),
            }
        }
        allowed
    }

    /// Whether both sides of this job address the same storage.
    fn one_store(&self) -> bool {
        self.source_fs.store() == self.target_fs.store()
    }

    /// Moves what a single `rename` can move, and reports what is left.
    ///
    /// Only where the destination is free: a rename cannot merge into an
    /// existing directory, and it would silently replace an existing file
    /// without asking. Everything it declines — an occupied destination, or a
    /// [`VfsError::CrossDevice`] that says "possible, just not in one
    /// step" — comes back to be scanned and copied.
    fn rename_what_it_can(
        &mut self,
        sources: &[VfsPath],
        destination: &Destination,
    ) -> Vec<VfsPath> {
        let mut remaining = Vec::new();
        for source in sources {
            if self.stopped() {
                break;
            }
            let target = destination.of(source);
            if self.target_fs.stat(&target) != Err(VfsError::NotFound) {
                remaining.push(source.clone());
                continue;
            }
            self.progress.emit(Progress::Started {
                path: source.clone(),
            });
            match self.target_fs.rename(source, &target) {
                Ok(()) => self.progress.emit(Progress::Finished {
                    path: source.clone(),
                }),
                Err(VfsError::CrossDevice) => remaining.push(source.clone()),
                Err(error) => self.fail(source, error),
            }
        }
        remaining
    }

    /// One top-level source, copied or moved.
    fn item(&mut self, item: &Item, moving: bool) -> Flow {
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
                attributes,
            } => self.copy_file(source, target, *size, *modified, *attributes),
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
        attributes: Attributes,
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
                // Permissions last: taking write permission away first would
                // stop the timestamp being set at all. Reported like any
                // other per-path failure — a copy that silently loses its
                // `+x` is a broken copy that looks fine.
                if let Err(error) = self.target_fs.set_attributes(&target, attributes) {
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

/// A sibling of `path` with `suffix` on the end of its name.
fn beside(path: &VfsPath, suffix: &str) -> VfsPath {
    match (path.parent(), path.file_name()) {
        (Some(parent), Some(name)) => parent.child(&format!("{name}{suffix}")),
        _ => path.clone(),
    }
}

/// An entry's name inside an archive: the planned target path without its
/// leading separator, which is what both formats store.
fn inside(target: &VfsPath) -> String {
    target.as_str().trim_start_matches(SEPARATOR).to_string()
}

/// A reader that counts what passes through it and stops on a cancel.
///
/// The packer is handed this rather than a path, so the counting and the
/// cancelling stay here — one implementation for every format, instead of one
/// per format that each has to remember.
struct Metered<'a> {
    inner: &'a mut Box<dyn std::io::Read + Send>,
    progress: &'a mut dyn ProgressSink,
    cancel: &'a CancelToken,
}

impl std::io::Read for Metered<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        if self.cancel.is_cancelled() {
            // The message is never read: the caller asks the token, which is
            // the only thing that can tell a cancel from a disk failure.
            return Err(std::io::Error::other(PACK_CANCELLED));
        }
        let read = self.inner.read(buffer)?;
        self.progress
            .emit(Progress::Advanced { bytes: read as u64 });
        Ok(read)
    }
}
