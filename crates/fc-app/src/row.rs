//! Turning a `fc-core` [`Entry`] into the strings a pane row displays.
//!
//! Pure and GTK-widget-free on purpose: this is the only real logic in the
//! shell, so it is the part that gets tested. Everything around it is widget
//! assembly.

use std::time::{SystemTime, UNIX_EPOCH};

use fc_core::listing::split_name;
use fc_core::vfs::Entry;
use gtk::gio;
use gtk::glib;

use crate::constants::{
    DATE_FORMAT, DIRECTORY_CONTENT_TYPE, DIR_NAME_CLOSE, DIR_NAME_OPEN, DIR_SIZE_LABEL,
    SIZE_PARTIAL_MARKER,
};
use crate::format::group_digits;

/// One rendered row: four column strings plus what the UI needs to style it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub name: String,
    pub ext: String,
    /// The name as the filesystem spells it, undivided.
    ///
    /// The name and ext columns are a presentation split; a rename needs the
    /// whole thing back, and reassembling it from the two halves would have to
    /// know when to put the dot back and when not to.
    pub full_name: String,
    pub size: String,
    pub modified: String,
    pub attributes: String,
    pub is_dir: bool,
    /// Whether the user has marked this row. Rendered, not decided, here.
    pub selected: bool,
    /// Whether this row is the one being renamed in place, so its name cell
    /// shows an editable field instead of a label.
    pub renaming: bool,
}

impl Row {
    /// Renders an entry. `is_parent` marks the synthetic `..` row.
    /// `measured` is `None` when nobody has counted this directory, and
    /// `Some(complete)` when somebody has — `false` meaning the count is a
    /// lower bound. Files ignore it: they have always known their size.
    pub fn from_entry(
        entry: &Entry,
        is_parent: bool,
        selected: bool,
        measured: Option<bool>,
    ) -> Self {
        // A directory called `archive.tar.gz` is not a `.gz` file, so only
        // files get their name split across the name and ext columns.
        let (name, ext) = if entry.is_dir() {
            (entry.name.as_str(), "")
        } else {
            split_name(&entry.name)
        };

        Row {
            // Directories are bracketed, Total Commander's own spelling, and
            // the parent row with them. **Display only** — `full_name` below
            // stays what the filesystem calls it, because renaming, the
            // type-ahead, the marks, the pack prefill and every job read that
            // field. A bracket that reached a `VfsPath` would be an operation
            // on a name that does not exist.
            name: match entry.is_dir() {
                true => format!("{DIR_NAME_OPEN}{name}{DIR_NAME_CLOSE}"),
                false => name.to_string(),
            },
            ext: ext.to_string(),
            full_name: entry.name.clone(),
            size: match (entry.is_dir(), measured) {
                // Counted. The `+` says the number is a lower bound — a
                // subdirectory refused to be read, or the scan was stopped —
                // because a size nobody can trust is worse than none.
                (true, Some(true)) => group_digits(entry.size),
                (true, Some(false)) => format!("{}{SIZE_PARTIAL_MARKER}", group_digits(entry.size)),
                // Nobody has counted it, which is what `<DIR>` means.
                (true, None) => DIR_SIZE_LABEL.to_string(),
                (false, _) => group_digits(entry.size),
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
                fc_core::vfs::render_attributes(entry.attributes)
            },
            renaming: false,
            is_dir: entry.is_dir(),
            selected,
        }
    }
}

impl Row {
    /// The content type whose icon this row shows.
    ///
    /// A *content type* rather than an icon name, because GIO turns one into
    /// the theme's whole fallback chain: a theme with no `text-plain` still
    /// finds `text-x-generic`, and a theme with neither still finds the
    /// generic file. Asking for a name directly would be asking for one
    /// rung of that ladder and falling off it.
    ///
    /// **Guessed from the name, never from the file's contents.** Reading the
    /// first bytes of every row to identify it is what the viewer's
    /// never-read-the-file rule forbids one key over, and a directory of
    /// fifty thousand entries would be fifty thousand opens
    /// ([`docs/performance.md`]).
    pub fn content_type(&self) -> String {
        if self.is_dir {
            return DIRECTORY_CONTENT_TYPE.to_string();
        }
        // The `bool` is "and I am only guessing", which is true of every
        // answer here — the alternative is opening the file.
        gio::content_type_guess(Some(self.full_name.as_str()), None)
            .0
            .into()
    }
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

    use fc_core::vfs::{EntryKind, SymlinkTarget};

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
        let row = Row::from_entry(&file("report.txt", 0), false, false, None);
        assert_eq!(row.name, "report");
        assert_eq!(row.ext, "txt");
    }

    #[test]
    fn a_directory_keeps_its_whole_name_even_when_it_looks_like_a_file() {
        // The assertion moved from `archive.tar.gz` to `[archive.tar.gz]` on
        // 2026-09-01, when directory names started being bracketed. What it
        // is about is unchanged and still the point: the name is *not* split
        // across the two columns, because a directory called `.tar.gz` has no
        // extension.
        let row = Row::from_entry(&directory("archive.tar.gz"), false, false, None);
        assert_eq!(row.name, "[archive.tar.gz]");
        assert_eq!(row.ext, "");
        assert_eq!(
            row.full_name, "archive.tar.gz",
            "the brackets reached the name a rename would use"
        );
    }

    #[test]
    fn a_directory_asks_for_the_folder_icon_whatever_it_is_called() {
        // Including one whose name looks like a file's, which is the case a
        // guess from the name would get wrong.
        assert_eq!(
            Row::from_entry(&directory("archive.tar.gz"), false, false, None).content_type(),
            "inode/directory"
        );
    }

    #[test]
    fn a_file_asks_for_the_type_its_name_claims() {
        assert_eq!(
            Row::from_entry(&file("notes.txt", 0), false, false, None).content_type(),
            "text/plain"
        );
    }

    #[test]
    fn a_file_with_no_extension_still_asks_for_something() {
        // GIO answers `application/octet-stream` or the platform's equivalent
        // — what matters is that it is never empty, because an empty content
        // type looks up no icon at all and the column would go ragged.
        let asked = Row::from_entry(&file("LICENSE", 0), false, false, None).content_type();
        assert!(!asked.is_empty(), "a nameless type shows no icon");
    }

    #[test]
    fn a_directory_is_bracketed_and_a_file_is_not() {
        assert_eq!(
            Row::from_entry(&directory("Documents"), false, false, None).name,
            "[Documents]"
        );
        assert_eq!(
            Row::from_entry(&file("notes.txt", 0), false, false, None).name,
            "notes"
        );
    }

    #[test]
    fn the_brackets_never_reach_the_name_anything_acts_on() {
        // Five things read a row's name to *do* something with it — rename,
        // type-ahead, the marks, the pack prefill and every job — and all of
        // them read `full_name`. A bracket reaching a `VfsPath` would be an
        // operation on a file that does not exist.
        let row = Row::from_entry(&directory("Documents"), false, false, None);
        assert_eq!(row.full_name, "Documents");
    }

    #[test]
    fn directories_show_a_dir_marker_instead_of_a_byte_count() {
        assert_eq!(
            Row::from_entry(&directory("sub"), false, false, None).size,
            "<DIR>"
        );
    }

    #[test]
    fn a_symlink_to_a_directory_is_rendered_as_a_directory() {
        let mut entry = file("link", 4096);
        entry.kind = EntryKind::Symlink(SymlinkTarget::Dir);
        let row = Row::from_entry(&entry, false, false, None);
        assert_eq!(row.size, "<DIR>");
        assert!(row.is_dir);
    }

    #[test]
    fn a_counted_directory_shows_its_size_instead_of_the_marker() {
        // The whole feature, at the one place it is visible.
        let mut entry = directory("build");
        entry.size = 1234567;

        let row = Row::from_entry(&entry, false, false, Some(true));

        assert_eq!(row.size, "1 234 567");
        assert!(row.is_dir, "it is still a directory");
    }

    #[test]
    fn a_counted_directory_of_nothing_shows_zero_not_the_marker() {
        // Zero is a real answer for an empty folder, and the case that makes
        // "counted" a flag rather than a non-zero size.
        let row = Row::from_entry(&directory("empty"), false, false, Some(true));

        assert_eq!(row.size, "0");
    }

    #[test]
    fn a_partial_count_is_marked_as_a_lower_bound() {
        let mut entry = directory("half-read");
        entry.size = 4096;

        let row = Row::from_entry(&entry, false, false, Some(false));

        assert_eq!(row.size, "4 096+");
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
                Row::from_entry(&file("f", size), false, false, None).size,
                expected
            );
        }
    }

    #[test]
    fn the_parent_row_shows_no_timestamp() {
        let row = Row::from_entry(&directory(".."), true, false, None);
        // Bracketed like any other directory since 2026-09-01, which is what
        // the screenshot of Total Commander shows; the rest of this test is
        // unchanged and is what it is about.
        assert_eq!(row.name, "[..]");
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
        assert!(!Row::from_entry(&entry, false, false, None)
            .modified
            .is_empty());
    }

    #[test]
    fn a_row_carries_whether_it_is_marked() {
        // Rendered here, decided in the listing: the cell factory reads this
        // to colour the row, and nothing else in the shell knows the rule.
        let entry = file("report.txt", 10);
        assert!(!Row::from_entry(&entry, false, false, None).selected);
        assert!(Row::from_entry(&entry, false, true, None).selected);
    }

    #[test]
    fn marking_changes_nothing_a_column_shows() {
        let entry = file("report.txt", 1234);
        let plain = Row::from_entry(&entry, false, false, None);
        let marked = Row::from_entry(&entry, false, true, None);

        assert_eq!(marked.name, plain.name);
        assert_eq!(marked.ext, plain.ext);
        assert_eq!(marked.size, plain.size);
        assert_eq!(marked.modified, plain.modified);
    }
}
