//! How a number, a duration and a failure are written for a person to read.
//!
//! Split out of `progress.rs`, which held both the throughput arithmetic and
//! the rendering: two of the three callers of these want no meter at all.
//! Pure functions over plain values, so every one of them is tested without a
//! window. A row's *timestamp* is not here: rendering one needs
//! `glib::DateTime` for the local time zone, and this module is glib-free.

use std::time::Duration;

use tc_core::vfs::{VfsError, VfsPath};

use crate::constants::{
    BYTE_DECIMALS, BYTE_STEP, BYTE_UNITS, FAILURES_MORE, FAILURES_SHOWN, FAILURE_FORMAT,
    MINUTES_PER_HOUR, SECONDS_PER_MINUTE, THOUSANDS_GROUP, THOUSANDS_SEPARATOR,
};

/// The size column: `1234567` becomes `1 234 567`.
///
/// Exact digits rather than [`human_bytes`], because a listing is where
/// somebody compares two files and `1.2 MiB` twice does not: Total Commander
/// shows the count and the units belong to the progress caption.
pub fn group_digits(value: u64) -> String {
    let digits = value.to_string();
    let mut grouped = String::with_capacity(digits.len() + digits.len() / THOUSANDS_GROUP);
    for (position, digit) in digits.chars().enumerate() {
        if position > 0 && (digits.len() - position).is_multiple_of(THOUSANDS_GROUP) {
            grouped.push(THOUSANDS_SEPARATOR);
        }
        grouped.push(digit);
    }
    grouped
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
    fn every_rendered_failure_is_fully_substituted() {
        // Same reasoning as the prompt test in `jobs.rs`: one check for a
        // leftover brace covers every placeholder in every template.
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
}
