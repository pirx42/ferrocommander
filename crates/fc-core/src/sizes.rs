//! What a folder actually holds, counted.
//!
//! Total Commander's `Alt+Shift+Enter`. A directory row says `<DIR>` because
//! nobody has counted it; this counts it, and the answer goes where a size
//! belongs — see [`Listing::set_measured`](crate::listing::Listing::set_measured).
//!
//! **One folder at a time, each answer sent as it lands**, because the whole
//! point of marking ten folders is not to wait for the slowest before seeing
//! any of them.

use std::sync::Arc;

use crate::ops::CancelToken;
use crate::vfs::{VfsPath, VirtualFs};

/// What a folder holds, and whether that is the whole story.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Measured {
    pub bytes: u64,
    /// `false` when a subdirectory could not be read.
    ///
    /// Carried rather than dropped, because a size that silently omits an
    /// unreadable subtree is a number nobody should trust — and trusting it
    /// is the whole reason somebody pressed the key. What the pane does with
    /// it is the pane's business; what this owes is the truth about it.
    pub complete: bool,
}

/// A folder's name, and what it holds.
///
/// The **name** rather than the path, because that is what the listing knows
/// a row by — and the same thing the marks are kept by, so an answer finds
/// its row even after a re-sort has moved it.
pub type Sizes = async_channel::Receiver<(String, Measured)>;

/// Measures each of `folders` inside `dir`, sending each answer as it lands.
///
/// Stops early on a cancel, keeping whatever was already sent: each folder's
/// number is its own and complete, so a partial set of answers is not a
/// misleading one.
pub fn spawn(
    fs: Arc<dyn VirtualFs>,
    dir: VfsPath,
    folders: Vec<String>,
    cancel: CancelToken,
) -> Sizes {
    // Bounded like the search's, so a scan that finishes folders faster than
    // the window can draw them cannot grow without limit; the worker simply
    // waits, which is the right kind of slow.
    let (sender, receiver) = async_channel::bounded(ANSWER_QUEUE);
    std::thread::spawn(move || {
        for name in folders {
            if cancel.is_cancelled() {
                return;
            }
            let measured = measure(fs.as_ref(), &dir.child(&name), &cancel);
            // A cancelled walk says nothing at all. It has a number — every
            // byte it managed to add up — but that number is a lower bound
            // for a question nobody is asking any more, and sending it is how
            // a folder's size came to change every time the key was pressed:
            // the *next* press cancels this walk, and its partial answer then
            // lands on top of the good one the new walk is about to produce
            // (2026-09-01, the second testing round).
            if cancel.is_cancelled() {
                return;
            }
            // A closed channel is the pane having moved on; there is nobody
            // left to tell.
            if sender.send_blocking((name, measured)).is_err() {
                return;
            }
        }
    });
    receiver
}

/// Everything below `root`, added up.
///
/// A queue rather than recursion, unreadable skipped, the cancel checked per
/// directory: the walk shape this crate keeps, and why the three walks are
/// not one function, is in `fc-core/src/CLAUDE.md`. This is the smallest of
/// them — it keeps no per-entry output at all, only a running total, which is
/// also why it costs less than a branch view of the same tree
/// (`docs/performance.md`).
///
/// What is particular to this one: a refusal and a cancel both make the
/// answer **incomplete** rather than merely smaller.
pub fn measure(fs: &dyn VirtualFs, root: &VfsPath, cancel: &CancelToken) -> Measured {
    let mut measured = Measured {
        bytes: 0,
        complete: true,
    };
    let mut pending = vec![root.clone()];

    while let Some(directory) = pending.pop() {
        if cancel.is_cancelled() {
            // Cancelled is not counted: the number so far is a lower bound
            // like any other partial answer, and saying otherwise would be
            // the one lie this module exists to avoid.
            measured.complete = false;
            return measured;
        }
        let Ok(entries) = fs.read_dir(&directory) else {
            measured.complete = false;
            continue;
        };
        for entry in entries {
            if entry.is_dir() {
                pending.push(directory.child(&entry.name));
                continue;
            }
            measured.bytes += entry.size;
        }
    }
    measured
}

/// How many answers may wait unread.
const ANSWER_QUEUE: usize = 64;
