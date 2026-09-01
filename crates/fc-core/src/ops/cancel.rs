//! Asking a running job to stop.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// A shared "stop what you are doing" flag.
///
/// Cloning shares the flag rather than copying it, which is what lets the UI
/// keep a handle while the worker thread owns the job.
#[derive(Clone, Default)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    pub fn new() -> Self {
        Self::default()
    }

    /// Asks the job to stop at its next checkpoint. Idempotent.
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_clone_observes_a_cancel_on_the_original() {
        // The whole point: the UI holds one handle, the worker another.
        let token = CancelToken::new();
        let worker = token.clone();
        assert!(!worker.is_cancelled());

        token.cancel();

        assert!(worker.is_cancelled());
    }

    #[test]
    fn cancelling_twice_changes_nothing() {
        let token = CancelToken::new();
        token.cancel();
        token.cancel();
        assert!(token.is_cancelled());
    }
}
