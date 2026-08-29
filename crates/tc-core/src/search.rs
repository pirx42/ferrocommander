//! Finding files, and saying so as they are found.
//!
//! A search over a home directory finds its first hit in milliseconds and its
//! last in minutes. Two things follow, and both are the speed requirement
//! rather than polish (`docs/performance.md`):
//!
//! * **It streams.** Waiting for the whole walk before showing anything makes
//!   the fast case feel like the slow one.
//! * **It stops.** Cancellation is checked between entries, so a search
//!   abandoned is a search over, not one that finishes the tree first.
//!
//! A directory that cannot be read is skipped rather than fatal. Half a home
//! directory is unreadable on any real machine, and a search that stops at the
//! first refusal is a search nobody can use.

use std::sync::Arc;

use crate::glob;
use crate::ops::CancelToken;
use crate::vfs::{VfsPath, VirtualFs};

/// How much of a file is read at a time when looking inside it.
///
/// The same reasoning as the viewer's window: a search must not hold a file to
/// look in it, or searching a directory of disk images is searching until the
/// allocator gives up.
pub const CONTENT_WINDOW: usize = 64 * 1024;

/// What to look for.
#[derive(Debug, Clone)]
pub struct Criteria {
    /// A wildcard against the file's name. `*` matches everything.
    pub name: String,
    /// Text that must appear in the file. Empty means the name is the whole
    /// question and nothing is opened.
    pub content: String,
}

/// Where the streamed results arrive.
pub type Results = async_channel::Receiver<VfsPath>;

/// Walks `root` and sends every match, until it runs out or is cancelled.
///
/// A queue rather than recursion, unreadable skipped: the walk shape this
/// crate keeps, and why its three walks are not one function, is in
/// `tc-core/src/CLAUDE.md`. What is particular to this one is that the cancel
/// is checked **per entry** as well — the per-entry work here may open and
/// read the whole file, which neither of the others does.
pub fn spawn(
    fs: Arc<dyn VirtualFs>,
    root: VfsPath,
    criteria: Criteria,
    cancel: CancelToken,
) -> Results {
    // Bounded, so a search that finds faster than the window can show cannot
    // grow without limit; the walker simply waits, which is the right kind of
    // slow.
    let (sender, receiver) = async_channel::bounded(RESULT_QUEUE);
    std::thread::spawn(move || {
        let mut pending = vec![root];
        while let Some(directory) = pending.pop() {
            if cancel.is_cancelled() {
                return;
            }
            // Unreadable is skipped, not fatal.
            let Ok(entries) = fs.read_dir(&directory) else {
                continue;
            };
            for entry in entries {
                if cancel.is_cancelled() {
                    return;
                }
                let path = directory.child(&entry.name);
                if entry.is_dir() {
                    pending.push(path);
                    continue;
                }
                if !matches(fs.as_ref(), &path, &entry.name, &criteria) {
                    continue;
                }
                // A closed channel is the window having gone; there is nobody
                // left to tell.
                if sender.send_blocking(path).is_err() {
                    return;
                }
            }
        }
    });
    receiver
}

/// How many results may wait unread.
const RESULT_QUEUE: usize = 256;

/// Whether one file answers the criteria.
fn matches(fs: &dyn VirtualFs, path: &VfsPath, name: &str, criteria: &Criteria) -> bool {
    if !glob::matches(&criteria.name, name) {
        return false;
    }
    criteria.content.is_empty() || contains(fs, path, &criteria.content)
}

/// Whether a file contains `needle`, without holding the file.
///
/// Windows overlap by the length of the needle less one byte, so a match
/// lying across a window boundary is still found — the bug this arrangement
/// exists to avoid, and the one a test pins.
fn contains(fs: &dyn VirtualFs, path: &VfsPath, needle: &str) -> bool {
    let needle = needle.as_bytes();
    if needle.is_empty() {
        return true;
    }
    let overlap = needle.len() - 1;
    let mut offset = 0u64;
    loop {
        let Ok(window) = fs.read_at(path, offset, CONTENT_WINDOW) else {
            return false;
        };
        if window.len() < needle.len() {
            return false;
        }
        if window.windows(needle.len()).any(|slice| slice == needle) {
            return true;
        }
        // Step by the window less the overlap, so nothing between two windows
        // is ever unexamined.
        offset += (window.len() - overlap) as u64;
    }
}
