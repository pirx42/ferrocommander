//! Turning a stream of [`Progress`] events into what the window shows.
//!
//! Pure, so the arithmetic that a progress bar lives or dies by is tested
//! without a window — the same split `jobs.rs` and `navigation.rs` use.

use tc_core::ops::Progress;
use tc_core::vfs::{VfsError, VfsPath};

use crate::constants::{
    BYTE_DECIMALS, BYTE_STEP, BYTE_UNITS, FAILURES_MORE, FAILURES_SHOWN, FAILURE_FORMAT,
    PROGRESS_FORMAT, PROGRESS_SCANNING,
};

/// Where a job has got to.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Meter {
    total: u64,
    done: u64,
    /// Set once the scan has reported, which is when a total exists at all.
    scanned: bool,
    current: Option<String>,
}

impl Meter {
    /// Folds one event in.
    ///
    /// `Advanced` carries a delta, so this is a running sum and never has to
    /// know the event ordering.
    pub fn apply(&mut self, event: &Progress) {
        match event {
            Progress::Scanned { bytes, .. } => {
                self.total = *bytes;
                self.scanned = true;
            }
            Progress::Advanced { bytes } => self.done += bytes,
            Progress::Started { path } => {
                self.current = Some(path.to_string());
            }
            Progress::Finished { .. } | Progress::Failed { .. } => {}
        }
    }

    /// Whether this job is big enough to be worth a window.
    ///
    /// A `mkdir` moves nothing and should not put one up at all.
    pub fn has_work(&self) -> bool {
        self.total > 0
    }

    /// How far along, between 0 and 1.
    ///
    /// A job with nothing to move is complete rather than divided by zero,
    /// and a job that somehow reports more than its total is clamped instead
    /// of overflowing the bar.
    pub fn fraction(&self) -> f64 {
        if self.total == 0 {
            return 1.0;
        }
        (self.done as f64 / self.total as f64).clamp(0.0, 1.0)
    }

    /// The line under the bar.
    pub fn caption(&self) -> String {
        if !self.scanned {
            return PROGRESS_SCANNING.to_string();
        }
        PROGRESS_FORMAT
            .replace("{done}", &human_bytes(self.done))
            .replace("{total}", &human_bytes(self.total))
    }

    /// The path being worked on, empty before the first one starts.
    pub fn current(&self) -> &str {
        self.current.as_deref().unwrap_or_default()
    }
}

/// A byte count as a person reads it.
pub fn human_bytes(bytes: u64) -> String {
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= BYTE_STEP && unit + 1 < BYTE_UNITS.len() {
        size /= BYTE_STEP;
        unit += 1;
    }
    // Whole bytes are counted exactly; anything larger is an approximation
    // already, so a decimal is enough and a column of them lines up.
    if unit == 0 {
        return format!("{bytes} {}", BYTE_UNITS[0]);
    }
    format!("{size:.*} {}", BYTE_DECIMALS, BYTE_UNITS[unit])
}

/// The failure summary, as lines.
///
/// Long batches are cut off with a count rather than filling the screen: the
/// user needs to know that something failed and roughly what, not to read
/// four hundred identical permission errors.
pub fn failure_lines(failures: &[(VfsPath, VfsError)]) -> Vec<String> {
    let mut lines: Vec<String> = failures
        .iter()
        .take(FAILURES_SHOWN)
        .map(|(path, error)| {
            FAILURE_FORMAT
                .replace("{path}", path.as_str())
                .replace("{reason}", &error.to_string())
        })
        .collect();
    if failures.len() > FAILURES_SHOWN {
        let rest = failures.len() - FAILURES_SHOWN;
        lines.push(FAILURES_MORE.replace("{count}", &rest.to_string()));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path(name: &str) -> VfsPath {
        VfsPath::new(name)
    }

    #[test]
    fn a_job_shows_scanning_until_it_has_a_total() {
        let mut meter = Meter::default();
        assert_eq!(meter.caption(), PROGRESS_SCANNING);

        meter.apply(&Progress::Scanned {
            files: 2,
            bytes: 2048,
        });

        assert_ne!(meter.caption(), PROGRESS_SCANNING);
    }

    #[test]
    fn deltas_add_up_into_the_fraction() {
        // The bar's whole contract: the engine sends increments, never a
        // running total, so the window is the thing that sums them.
        let mut meter = Meter::default();
        meter.apply(&Progress::Scanned {
            files: 1,
            bytes: 100,
        });
        for _ in 0..4 {
            meter.apply(&Progress::Advanced { bytes: 25 });
        }
        assert_eq!(meter.fraction(), 1.0);
    }

    #[test]
    fn a_fraction_never_leaves_the_bar() {
        let mut meter = Meter::default();
        meter.apply(&Progress::Scanned {
            files: 1,
            bytes: 10,
        });
        assert_eq!(meter.fraction(), 0.0);

        // More than promised: possible if a file grew while it was copied.
        meter.apply(&Progress::Advanced { bytes: 999 });
        assert_eq!(meter.fraction(), 1.0);
    }

    #[test]
    fn a_job_that_moves_nothing_is_complete_rather_than_undefined() {
        // F7 creates a directory and moves no bytes at all.
        let meter = Meter::default();
        assert_eq!(meter.fraction(), 1.0);
        assert!(!meter.has_work(), "and it gets no progress window");
    }

    #[test]
    fn the_current_path_follows_the_started_events() {
        let mut meter = Meter::default();
        assert_eq!(meter.current(), "");

        meter.apply(&Progress::Started {
            path: path("/a/one.txt"),
        });
        assert_eq!(meter.current(), "/a/one.txt");

        meter.apply(&Progress::Started {
            path: path("/a/two.txt"),
        });
        assert_eq!(meter.current(), "/a/two.txt");
    }

    #[test]
    fn byte_counts_are_written_the_way_they_are_read() {
        let cases = [
            (0, "0 B"),
            (512, "512 B"),
            (1024, "1.0 KiB"),
            (1536, "1.5 KiB"),
            (1024 * 1024, "1.0 MiB"),
            (3 * 1024 * 1024 * 1024, "3.0 GiB"),
        ];
        for (bytes, expected) in cases {
            assert_eq!(human_bytes(bytes), expected, "{bytes}");
        }
    }

    #[test]
    fn the_largest_unit_is_the_last_one_rather_than_an_overflow() {
        // Guards the loop bound: without it the index walks off the array.
        let huge = u64::MAX;
        assert!(human_bytes(huge).ends_with(BYTE_UNITS[BYTE_UNITS.len() - 1]));
    }

    #[test]
    fn a_short_failure_list_is_shown_in_full() {
        let failures = vec![
            (path("/a.txt"), VfsError::PermissionDenied),
            (path("/b.txt"), VfsError::NotFound),
        ];
        let lines = failure_lines(&failures);
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("/a.txt") && lines[0].contains("permission denied"));
    }

    #[test]
    fn a_long_failure_list_is_cut_off_with_a_count() {
        // Four hundred identical permission errors are not information.
        let failures: Vec<_> = (0..FAILURES_SHOWN + 5)
            .map(|index| (path(&format!("/f{index}")), VfsError::PermissionDenied))
            .collect();

        let lines = failure_lines(&failures);

        assert_eq!(lines.len(), FAILURES_SHOWN + 1);
        assert!(lines.last().unwrap().contains('5'));
    }

    #[test]
    fn no_failures_produce_no_lines() {
        assert!(failure_lines(&[]).is_empty());
    }
}
