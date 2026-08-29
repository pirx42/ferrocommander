//! The whole tree below a directory, as one flat list of files.
//!
//! Total Commander's `Ctrl+B`. What comes back is an ordinary [`Listing`]
//! whose rows happen to be files from several directories, so the sort, the
//! quick filter, the marks and every file operation go on meaning what they
//! meant — which is the whole point, and the reason this is a walk feeding
//! the existing model rather than a mode of its own.
//!
//! Here rather than on `Listing`, so that the directory model never names a
//! walker: [`listing::spawn`] supplies the thread and this supplies what to do
//! on it — the arrangement [`crate::archive::spawn_enter`] uses for the same
//! reason.

use std::sync::Arc;

use crate::listing::{self, Listing, Loading};
use crate::ops::CancelToken;
use crate::vfs::constants::SEPARATOR;
use crate::vfs::{Entry, VfsPath, VirtualFs};

/// Walks everything below `root` on a worker thread.
///
/// The pane keeps showing what it has until this lands, exactly as it does for
/// a slow directory read — and `cancel` is why `Ctrl+B` on a huge tree is not
/// a trap.
pub fn spawn(fs: Arc<dyn VirtualFs>, root: VfsPath, cancel: CancelToken) -> Loading {
    listing::spawn(move || {
        let entries = walk(fs.as_ref(), &root, &cancel);
        let listing = Listing::branch(root, entries);
        Ok((fs, listing))
    })
}

/// Every file below `root`, named by its path relative to it.
///
/// **Breadth-first through a queue rather than recursively**, for the reason
/// [`crate::search`] gives: a directory tree is user input, and a deep enough
/// one turns recursion into a stack overflow — a crash in a file manager,
/// over somebody else's directory layout.
///
/// **Unreadable is skipped, not fatal.** One subdirectory nobody may enter
/// must not cost the view, which is the rule the copy engine keeps about a
/// tree it cannot fully read.
///
/// **A cancel yields what was found so far** rather than nothing. The caller
/// asked to stop, and the caller decides whether a partial answer is worth
/// showing; throwing the work away here would take that decision from it.
pub fn walk(fs: &dyn VirtualFs, root: &VfsPath, cancel: &CancelToken) -> Vec<Entry> {
    let mut found = Vec::new();
    // The relative prefix travels with the directory, and so does whether
    // anything above it was hidden: a file in a hidden directory is hidden,
    // and deciding that during the walk is what keeps `Ctrl+H` free of a
    // second one.
    let mut pending = vec![(root.clone(), String::new(), false)];

    while let Some((directory, prefix, buried)) = pending.pop() {
        if cancel.is_cancelled() {
            return found;
        }
        let Ok(entries) = fs.read_dir(&directory) else {
            continue;
        };
        for mut entry in entries {
            let relative = match prefix.is_empty() {
                true => entry.name.clone(),
                false => format!("{prefix}{SEPARATOR}{}", entry.name),
            };
            let hidden = buried || entry.hidden;
            if entry.is_dir() {
                pending.push((directory.child(&entry.name), relative, hidden));
                continue;
            }
            // The row's name *is* the relative path. Every reader of it goes
            // on working — see `Entry::name` — and it is what makes a sort by
            // name keep each directory's files together.
            entry.name = relative;
            entry.hidden = hidden;
            found.push(entry);
        }
    }
    found
}
