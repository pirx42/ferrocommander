//! Turning a stream of [`Progress`] events into what the window shows.
//!
//! Pure, so the arithmetic that a progress bar lives or dies by is tested
//! without a window — the same split `jobs.rs` and `navigation.rs` use.

use std::collections::VecDeque;
use std::time::Duration;

use tc_core::ops::Progress;
use tc_core::vfs::{VfsError, VfsPath};

use crate::constants::{
    BYTE_DECIMALS, BYTE_STEP, BYTE_UNITS, FAILURES_MORE, FAILURES_SHOWN, FAILURE_FORMAT,
    MINUTES_PER_HOUR, PROGRESS_ETA_SUFFIX, PROGRESS_FORMAT, PROGRESS_RATE_DELAY,
    PROGRESS_RATE_SUFFIX, PROGRESS_RATE_WINDOW, PROGRESS_SCANNING, PROGRESS_SEPARATOR,
    SECONDS_PER_MINUTE,
};

/// Where a job has got to.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Meter {
    total: u64,
    done: u64,
    /// Set once the scan has reported, which is when a total exists at all.
    scanned: bool,
    current: Option<String>,
    /// How much was done and when, over the last [`PROGRESS_RATE_WINDOW`].
    ///
    /// A window rather than the whole job: a run of small files followed by
    /// one big one leaves a whole-job average saying something that stopped
    /// being true minutes ago, and an estimate built on it is wrong for the
    /// rest of the run.
    samples: VecDeque<(Duration, u64)>,
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

    /// Notes how much is done at `elapsed` since the job started.
    ///
    /// Separate from [`apply`](Self::apply) so that folding an event stays
    /// free of the clock: what a stream of events adds up to is arithmetic and
    /// tested as such, and how fast they arrived is the caller's observation.
    pub fn observe(&mut self, elapsed: Duration) {
        self.samples.push_back((elapsed, self.done));
        // Drop what has fallen out of the window, but never the last two:
        // events arrive when they arrive, and a job that reported nothing for
        // a while would otherwise be left with a single sample and no span to
        // measure over.
        let cutoff = elapsed.saturating_sub(PROGRESS_RATE_WINDOW);
        while self.samples.len() > 2 && self.samples[0].0 < cutoff {
            self.samples.pop_front();
        }
    }

    /// Bytes per second over the recent window, once there is enough to say.
    ///
    /// `None` until the job has been running long enough for the number to
    /// mean something, and when nothing has moved in the window — a rate of
    /// zero would give an infinite estimate, and "0 B/s" is not news anyone
    /// wants during a stall on a network mount.
    pub fn rate(&self) -> Option<f64> {
        let (first, done_then) = *self.samples.front()?;
        let (last, done_now) = *self.samples.back()?;
        let span = last.checked_sub(first)?;
        if last < PROGRESS_RATE_DELAY || span.is_zero() {
            return None;
        }
        let moved = done_now.checked_sub(done_then)?;
        if moved == 0 {
            return None;
        }
        Some(moved as f64 / span.as_secs_f64())
    }

    /// How long the rest is expected to take, at the recent rate.
    ///
    /// `None` before the scan — there is no total to be left of — and once the
    /// work is done. An estimate is a guess and says so by disappearing rather
    /// than counting down to a zero it may not reach.
    pub fn remaining(&self) -> Option<Duration> {
        if !self.scanned {
            return None;
        }
        let left = self.total.checked_sub(self.done).filter(|&left| left > 0)?;
        let rate = self.rate()?;
        Some(Duration::from_secs_f64(left as f64 / rate))
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

    /// The line under the bar: how much, how fast, how much longer.
    ///
    /// The rate and the estimate appear only once they are worth trusting, and
    /// each on its own terms — a rate with no estimate is what a finished-but-
    /// for-the-last-file job looks like.
    pub fn caption(&self) -> String {
        if !self.scanned {
            return PROGRESS_SCANNING.to_string();
        }
        let mut caption = PROGRESS_FORMAT
            .replace("{done}", &human_bytes(self.done))
            .replace("{total}", &human_bytes(self.total));
        if let Some(rate) = self.rate() {
            caption.push_str(PROGRESS_SEPARATOR);
            caption.push_str(&human_bytes(rate as u64));
            caption.push_str(PROGRESS_RATE_SUFFIX);
        }
        if let Some(left) = self.remaining() {
            caption.push_str(PROGRESS_SEPARATOR);
            caption.push_str(&human_duration(left));
            caption.push_str(PROGRESS_ETA_SUFFIX);
        }
        caption
    }

    /// The path being worked on, empty before the first one starts.
    pub fn current(&self) -> &str {
        self.current.as_deref().unwrap_or_default()
    }
}

/// A duration as a person reads a time left: `0:07`, `2:35`, `1:02:35`.
///
/// Minutes and seconds, with hours only when there are any — a job with four
/// seconds to go should not say `0:00:04`.
pub fn human_duration(left: Duration) -> String {
    let seconds = left.as_secs();
    let (minutes, seconds) = (seconds / SECONDS_PER_MINUTE, seconds % SECONDS_PER_MINUTE);
    let (hours, minutes) = (minutes / MINUTES_PER_HOUR, minutes % MINUTES_PER_HOUR);
    if hours > 0 {
        return format!("{hours}:{minutes:02}:{seconds:02}");
    }
    format!("{minutes}:{seconds:02}")
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
    fn every_rendered_line_is_fully_substituted() {
        // Same reasoning as the prompt test in `jobs.rs`: one check for a
        // leftover brace covers every placeholder in every template.
        let mut meter = Meter::default();
        meter.apply(&Progress::Scanned {
            files: 1,
            bytes: 2048,
        });
        assert!(!meter.caption().contains('{'), "{}", meter.caption());

        let lines = failure_lines(&[(path("/a.txt"), VfsError::NotFound)]);
        assert!(!lines[0].contains('{'), "{}", lines[0]);
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

    /// A meter that has scanned `total` and moved `done` by `at`.
    fn moving(total: u64, done: u64, at: Duration) -> Meter {
        let mut meter = Meter::default();
        meter.apply(&Progress::Scanned {
            files: 1,
            bytes: total,
        });
        meter.observe(Duration::ZERO);
        meter.apply(&Progress::Advanced { bytes: done });
        meter.observe(at);
        meter
    }

    #[test]
    fn the_rate_is_what_moved_over_how_long_it_took() {
        // Ten seconds, ten megabytes: one megabyte a second, and no rounding
        // games in between.
        let meter = moving(100 << 20, 10 << 20, Duration::from_secs(10));

        let rate = meter.rate().expect("a rate after ten seconds");
        assert!(
            (rate - (1 << 20) as f64).abs() < 1.0,
            "{rate} is not a MiB per second"
        );
    }

    #[test]
    fn nothing_is_said_until_there_is_something_to_say() {
        // A number computed from the first fifty milliseconds is noise, and
        // one that appears and then halves reads as a program that does not
        // know what it is doing.
        let meter = moving(100 << 20, 1 << 20, Duration::from_millis(100));

        assert_eq!(meter.rate(), None);
        assert_eq!(meter.remaining(), None);
        assert!(!meter.caption().contains(PROGRESS_RATE_SUFFIX));
    }

    #[test]
    fn a_stall_reports_no_rate_rather_than_zero() {
        // A rate of zero divides into an infinite estimate, and "0 B/s" is not
        // news anybody wants during a pause on a network mount.
        let mut meter = moving(100 << 20, 10 << 20, Duration::from_secs(1));
        // Long enough that every sample carrying progress has fallen out of
        // the window.
        for second in 2..12 {
            meter.observe(Duration::from_secs(second));
        }

        assert_eq!(meter.rate(), None, "a stall reported a rate");
        assert_eq!(meter.remaining(), None);
    }

    #[test]
    fn the_estimate_is_what_is_left_at_the_current_rate() {
        // 90 MiB left at 1 MiB/s is a minute and a half.
        let meter = moving(100 << 20, 10 << 20, Duration::from_secs(10));

        let left = meter.remaining().expect("an estimate");
        assert!(
            (left.as_secs_f64() - 90.0).abs() < 1.0,
            "{left:?} is not about ninety seconds"
        );
    }

    #[test]
    fn a_job_with_nothing_left_offers_no_estimate() {
        // An estimate is a guess and says so by disappearing, rather than
        // counting down to a zero it may not reach.
        let meter = moving(10 << 20, 10 << 20, Duration::from_secs(10));

        assert_eq!(meter.remaining(), None);
        assert!(!meter.caption().contains(PROGRESS_ETA_SUFFIX));
    }

    #[test]
    fn the_rate_follows_what_is_happening_now() {
        // The reason for a window rather than a whole-job average: a run of
        // small files and then one big one. A meter that averaged everything
        // would still be reporting the slow start long after it ended.
        let mut meter = Meter::default();
        meter.apply(&Progress::Scanned {
            files: 2,
            bytes: 1000 << 20,
        });
        meter.observe(Duration::ZERO);
        // Ten seconds crawling.
        meter.apply(&Progress::Advanced { bytes: 1 << 20 });
        meter.observe(Duration::from_secs(10));
        // Then a second at a hundred times that.
        meter.apply(&Progress::Advanced { bytes: 100 << 20 });
        meter.observe(Duration::from_secs(11));

        let rate = meter.rate().expect("a rate");
        let whole_job = (101 << 20) as f64 / 11.0;
        assert!(
            rate > whole_job * 5.0,
            "{rate} is still averaging in the slow start ({whole_job})"
        );
    }

    #[test]
    fn a_time_left_reads_the_way_a_person_writes_one() {
        for (seconds, written) in [
            (7u64, "0:07"),
            (67, "1:07"),
            (155, "2:35"),
            (3600, "1:00:00"),
            (3755, "1:02:35"),
        ] {
            assert_eq!(human_duration(Duration::from_secs(seconds)), written);
        }
    }

    #[test]
    fn the_caption_carries_all_three_once_it_can() {
        let meter = moving(100 << 20, 10 << 20, Duration::from_secs(10));

        let caption = meter.caption();
        for part in [
            "10.0 MiB",
            "100.0 MiB",
            PROGRESS_RATE_SUFFIX,
            PROGRESS_ETA_SUFFIX,
        ] {
            assert!(caption.contains(part), "{caption:?} is missing {part:?}");
        }
    }
}
