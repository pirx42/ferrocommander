//! Turning a `tc-core` [`Entry`] into the strings a pane row displays.
//!
//! Pure and GTK-widget-free on purpose: this is the only real logic in the
//! shell, so it is the part that gets tested. Everything around it is widget
//! assembly.

use std::time::{SystemTime, UNIX_EPOCH};

use gtk::glib;
use tc_core::listing::split_name;
use tc_core::vfs::Entry;

use crate::constants::{DATE_FORMAT, DIR_SIZE_LABEL, THOUSANDS_GROUP, THOUSANDS_SEPARATOR};

/// One rendered row: four column strings plus what the UI needs to style it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub name: String,
    pub ext: String,
    pub size: String,
    pub modified: String,
    pub attributes: String,
    pub is_dir: bool,
    /// Whether the user has marked this row. Rendered, not decided, here.
    pub selected: bool,
}

impl Row {
    /// Renders an entry. `is_parent` marks the synthetic `..` row.
    pub fn from_entry(entry: &Entry, is_parent: bool, selected: bool) -> Self {
        // A directory called `archive.tar.gz` is not a `.gz` file, so only
        // files get their name split across the name and ext columns.
        let (name, ext) = if entry.is_dir() {
            (entry.name.as_str(), "")
        } else {
            split_name(&entry.name)
        };

        Row {
            name: name.to_string(),
            ext: ext.to_string(),
            size: if entry.is_dir() {
                DIR_SIZE_LABEL.to_string()
            } else {
                group_digits(entry.size)
            },
            // `..` is a navigation control, not a file: it carries no real
            // timestamp, and printing the epoch would be a lie.
            modified: if is_parent {
                String::new()
            } else {
                format_modified(entry.modified)
            },
            // `..` is a navigation control and carries nobody's permissions.
            attributes: if is_parent {
                String::new()
            } else {
                tc_core::vfs::render_attributes(entry.attributes)
            },
            is_dir: entry.is_dir(),
            selected,
        }
    }
}

/// `1234567` becomes `1 234 567`.
fn group_digits(value: u64) -> String {
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

/// Formats a timestamp in the viewer's local time zone.
fn format_modified(modified: SystemTime) -> String {
    let seconds = modified
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_secs() as i64)
        .unwrap_or_default();
    glib::DateTime::from_unix_local(seconds)
        .as_ref()
        .map(format_date_time)
        .unwrap_or_default()
}

/// The formatting itself, separated from "which time zone" so it can be
/// tested against a fixed instant instead of wherever the machine happens
/// to be.
fn format_date_time(when: &glib::DateTime) -> String {
    when.format(DATE_FORMAT)
        .map(|text| text.to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tc_core::vfs::{EntryKind, SymlinkTarget};

    use super::*;

    fn file(name: &str, size: u64) -> Entry {
        Entry {
            name: name.to_string(),
            kind: EntryKind::File,
            size,
            modified: UNIX_EPOCH,
            attributes: Default::default(),
            hidden: false,
        }
    }

    fn directory(name: &str) -> Entry {
        Entry {
            name: name.to_string(),
            kind: EntryKind::Dir,
            size: 0,
            modified: UNIX_EPOCH,
            attributes: Default::default(),
            hidden: false,
        }
    }

    #[test]
    fn a_file_name_is_split_across_the_name_and_ext_columns() {
        let row = Row::from_entry(&file("report.txt", 0), false, false);
        assert_eq!(row.name, "report");
        assert_eq!(row.ext, "txt");
    }

    #[test]
    fn a_directory_keeps_its_whole_name_even_when_it_looks_like_a_file() {
        let row = Row::from_entry(&directory("archive.tar.gz"), false, false);
        assert_eq!(row.name, "archive.tar.gz");
        assert_eq!(row.ext, "");
    }

    #[test]
    fn directories_show_a_dir_marker_instead_of_a_byte_count() {
        assert_eq!(
            Row::from_entry(&directory("sub"), false, false).size,
            "<DIR>"
        );
    }

    #[test]
    fn a_symlink_to_a_directory_is_rendered_as_a_directory() {
        let mut entry = file("link", 4096);
        entry.kind = EntryKind::Symlink(SymlinkTarget::Dir);
        let row = Row::from_entry(&entry, false, false);
        assert_eq!(row.size, "<DIR>");
        assert!(row.is_dir);
    }

    #[test]
    fn file_sizes_are_grouped_in_threes() {
        let cases = [
            (0, "0"),
            (7, "7"),
            (999, "999"),
            (1000, "1 000"),
            (12345, "12 345"),
            (1234567, "1 234 567"),
            (u64::MAX, "18 446 744 073 709 551 615"),
        ];
        for (size, expected) in cases {
            assert_eq!(
                Row::from_entry(&file("f", size), false, false).size,
                expected
            );
        }
    }

    #[test]
    fn the_parent_row_shows_no_timestamp() {
        let row = Row::from_entry(&directory(".."), true, false);
        assert_eq!(row.name, "..");
        assert_eq!(row.modified, "");
        assert_eq!(row.size, "<DIR>");
    }

    #[test]
    fn timestamps_render_as_a_sortable_date_and_time() {
        let epoch = glib::DateTime::from_unix_utc(0).unwrap();
        assert_eq!(format_date_time(&epoch), "1970-01-01 00:00");

        let later = glib::DateTime::from_unix_utc(1_700_000_000).unwrap();
        assert_eq!(format_date_time(&later), "2023-11-14 22:13");
    }

    #[test]
    fn an_ordinary_file_gets_a_timestamp() {
        let mut entry = file("f", 1);
        entry.modified = UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        assert!(!Row::from_entry(&entry, false, false).modified.is_empty());
    }

    #[test]
    fn a_row_carries_whether_it_is_marked() {
        // Rendered here, decided in the listing: the cell factory reads this
        // to colour the row, and nothing else in the shell knows the rule.
        let entry = file("report.txt", 10);
        assert!(!Row::from_entry(&entry, false, false).selected);
        assert!(Row::from_entry(&entry, false, true).selected);
    }

    #[test]
    fn marking_changes_nothing_a_column_shows() {
        let entry = file("report.txt", 1234);
        let plain = Row::from_entry(&entry, false, false);
        let marked = Row::from_entry(&entry, false, true);

        assert_eq!(marked.name, plain.name);
        assert_eq!(marked.ext, plain.ext);
        assert_eq!(marked.size, plain.size);
        assert_eq!(marked.modified, plain.modified);
    }
}
