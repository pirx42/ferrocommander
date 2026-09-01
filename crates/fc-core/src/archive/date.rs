//! Zip dates, both ways.
//!
//! Zip stores a local wall-clock time with no zone in it, so both directions
//! read and write UTC: being an hour or two out in a date column is a smaller
//! lie than refusing to show a date at all, and there is nothing in the file
//! to do better with. The format also stores its seconds halved, so odd
//! seconds do not exist in it and a reader that invented the missing one would
//! be claiming to know something the file does not say.
//!
//! Converted by hand rather than by adding a date library for six fields —
//! `zip`'s own conversion lives behind a feature that would add one. Both
//! halves are here together because they have to be exact inverses, and two
//! files is how that stops being true.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::constants::{
    DAYS_CIVIL_TO_EPOCH, DAYS_PER_ERA, MARCH_SHIFT_DENOMINATOR, MARCH_SHIFT_NUMERATOR,
    SECONDS_PER_DAY, SECONDS_PER_HOUR, SECONDS_PER_MINUTE, YEARS_PER_ERA, ZIP_EPOCH_YEAR,
};

/// A zip date as a `SystemTime`.
pub fn as_system_time(stamp: zip::DateTime) -> Option<SystemTime> {
    let days = days_from_civil(
        i64::from(stamp.year()),
        u32::from(stamp.month()),
        u32::from(stamp.day()),
    );
    let seconds = days * SECONDS_PER_DAY
        + i64::from(stamp.hour()) * SECONDS_PER_HOUR
        + i64::from(stamp.minute()) * SECONDS_PER_MINUTE
        + i64::from(stamp.second());
    u64::try_from(seconds)
        .ok()
        .map(|seconds| UNIX_EPOCH + Duration::from_secs(seconds))
}

/// Days from 1970-01-01 to a proleptic-Gregorian date, by Howard Hinnant's
/// `days_from_civil`. Shifting the year to start in March makes the leap day
/// the last day of the year, which is what removes every special case.
fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let year = year - i64::from(month <= 2);
    let era = year.div_euclid(YEARS_PER_ERA);
    let year_of_era = year.rem_euclid(YEARS_PER_ERA);
    let month = i64::from(month);
    let day_of_year = (MARCH_SHIFT_NUMERATOR * (month + if month > 2 { -3 } else { 9 }) + 2)
        / MARCH_SHIFT_DENOMINATOR
        + i64::from(day)
        - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * DAYS_PER_ERA + day_of_era - DAYS_CIVIL_TO_EPOCH
}

/// A `SystemTime` as a zip date, or `None` for one zip cannot express.
///
/// Zip counts from 1980, so a file dated earlier — or later than the format's
/// own end — gets no date rather than a wrong one.
pub fn zip_date(modified: SystemTime) -> Option<zip::DateTime> {
    let seconds = modified
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()?
        .as_secs() as i64;
    let days = seconds.div_euclid(SECONDS_PER_DAY);
    let rest = seconds.rem_euclid(SECONDS_PER_DAY);
    let (year, month, day) = civil_from_days(days);
    if year < ZIP_EPOCH_YEAR {
        return None;
    }
    zip::DateTime::from_date_and_time(
        u16::try_from(year).ok()?,
        u8::try_from(month).ok()?,
        u8::try_from(day).ok()?,
        (rest / SECONDS_PER_HOUR) as u8,
        (rest % SECONDS_PER_HOUR / SECONDS_PER_MINUTE) as u8,
        (rest % SECONDS_PER_MINUTE) as u8,
    )
    .ok()
}

/// The proleptic-Gregorian date `days` after 1970-01-01, by Howard Hinnant's
/// `civil_from_days` — the exact inverse of [`days_from_civil`], pinned as one
/// by a round-trip test.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let shifted = days + DAYS_CIVIL_TO_EPOCH;
    let era = shifted.div_euclid(DAYS_PER_ERA);
    let day_of_era = shifted.rem_euclid(DAYS_PER_ERA);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * YEARS_PER_ERA;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (MARCH_SHIFT_DENOMINATOR * day_of_year + 2) / MARCH_SHIFT_NUMERATOR;
    let day =
        day_of_year - (MARCH_SHIFT_NUMERATOR * shifted_month + 2) / MARCH_SHIFT_DENOMINATOR + 1;
    let month = shifted_month + if shifted_month < 10 { 3 } else { -9 };
    (year + i64::from(month <= 2), month as u32, day as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_two_date_conversions_are_each_other() {
        // The reader turns a calendar date into a day count and the writer
        // turns it back. A pair that disagreed anywhere would move every date
        // in an archive that was unpacked and repacked.
        //
        // Every day from 1901 to 2099, which covers every leap rule the
        // calendar has: the four-year one, the hundred-year exception, and the
        // four-hundred-year exception to that.
        for days in -25_000i64..47_000 {
            let (year, month, day) = civil_from_days(days);
            assert_eq!(
                days_from_civil(year, month, day),
                days,
                "{year}-{month}-{day}"
            );
        }
    }
}
