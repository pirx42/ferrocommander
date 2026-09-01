//! What to do when something is already at the destination.

use crate::listing::split_name;
use crate::vfs::{EntryKind, VfsError, VfsPath, VirtualFs};

use super::constants::{EXTENSION_SEPARATOR, KEEP_BOTH_FIRST_COUNTER, KEEP_BOTH_SUFFIX};

/// A destination that is already taken.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conflict {
    pub source: VfsPath,
    pub target: VfsPath,
    /// What is in the way, so the question can say whether the user is about
    /// to lose a file or a whole directory.
    pub existing: EntryKind,
}

/// What the user decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resolution {
    /// Replace what is there.
    Overwrite,
    /// Leave both the source and the destination alone.
    Skip,
    /// Keep the existing one and put the new one beside it under a free name
    /// the engine invents.
    ///
    /// The engine picks the name rather than the user typing one, which is
    /// what makes *apply to all* mean something here: one answer can serve a
    /// hundred collisions, and each still lands somewhere different.
    KeepBoth,
    /// Stop the whole job.
    Abort,
}

/// A resolution plus whether it should serve every later conflict too.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Answer {
    pub resolution: Resolution,
    pub apply_to_all: bool,
}

impl Answer {
    /// An answer for this conflict only.
    pub fn once(resolution: Resolution) -> Self {
        Answer {
            resolution,
            apply_to_all: false,
        }
    }

    /// An answer for this conflict and every later one.
    pub fn always(resolution: Resolution) -> Self {
        Answer {
            resolution,
            apply_to_all: true,
        }
    }
}

/// Whoever answers conflict questions — a dialog in the app, a scripted list
/// in a test.
pub trait ConflictResolver {
    fn resolve(&mut self, conflict: &Conflict) -> Answer;
}

/// Remembers an answer marked *apply to all* and gives it for everything
/// after, without asking again.
///
/// A wrapper rather than a flag inside the engine: the policy is then one
/// pure, testable thing, and the dialog only has to report what the user
/// ticked.
pub struct ApplyToAll<R> {
    inner: R,
    remembered: Option<Resolution>,
}

impl<R> ApplyToAll<R> {
    pub fn new(inner: R) -> Self {
        ApplyToAll {
            inner,
            remembered: None,
        }
    }

    /// Gives the wrapped resolver back, so a caller can see what it was
    /// actually asked.
    pub fn into_inner(self) -> R {
        self.inner
    }
}

impl<R: ConflictResolver> ConflictResolver for ApplyToAll<R> {
    fn resolve(&mut self, conflict: &Conflict) -> Answer {
        if let Some(resolution) = self.remembered {
            return Answer::once(resolution);
        }
        let answer = self.inner.resolve(conflict);
        if answer.apply_to_all {
            self.remembered = Some(answer.resolution);
        }
        answer
    }
}

/// Whether `target` blocks `source` from landing, and what is in the way.
///
/// **Two directories are a merge, not a conflict.** Copying `photos` onto an
/// existing `photos` means the files inside meet each other; asking about the
/// directory itself would be a question the user cannot act on, since neither
/// overwriting nor skipping the whole tree is what they meant.
pub fn conflict_at(
    fs: &dyn VirtualFs,
    source: &VfsPath,
    source_is_dir: bool,
    target: &VfsPath,
) -> Result<Option<Conflict>, VfsError> {
    let existing = match fs.stat(target) {
        Ok(entry) => entry,
        Err(VfsError::NotFound) => return Ok(None),
        Err(other) => return Err(other),
    };
    if source_is_dir && existing.is_dir() {
        return Ok(None);
    }
    Ok(Some(Conflict {
        source: source.clone(),
        target: target.clone(),
        existing: existing.kind,
    }))
}

/// A free path beside `target`, counting up until nothing is there.
///
/// `notes.txt` becomes `notes (2).txt`, then `notes (3).txt` — the counter
/// goes before the extension so the file keeps opening in the same program.
pub fn free_name_beside(fs: &dyn VirtualFs, target: &VfsPath) -> Result<VfsPath, VfsError> {
    let name = target.file_name().unwrap_or_default().to_string();
    let (stem, extension) = split_name(&name);
    let parent = target.parent().unwrap_or_else(VfsPath::root);

    let mut counter = KEEP_BOTH_FIRST_COUNTER;
    loop {
        let suffix = KEEP_BOTH_SUFFIX.replace("{counter}", &counter.to_string());
        let candidate = match extension {
            "" => format!("{stem}{suffix}"),
            ext => format!("{stem}{suffix}{EXTENSION_SEPARATOR}{ext}"),
        };
        let path = parent.child(&candidate);
        match fs.stat(&path) {
            Err(VfsError::NotFound) => return Ok(path),
            Err(other) => return Err(other),
            Ok(_) => counter += 1,
        }
    }
}
