//! Ordering rules for a directory listing.

use std::cmp::Ordering;

use crate::vfs::Entry;

/// Which column the listing is ordered by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortKey {
    Name,
    Ext,
    Size,
    Modified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    Ascending,
    Descending,
}

impl SortOrder {
    /// The other direction — what clicking the same column header again does.
    pub fn flipped(self) -> Self {
        match self {
            SortOrder::Ascending => SortOrder::Descending,
            SortOrder::Descending => SortOrder::Ascending,
        }
    }
}

/// A column plus a direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sort {
    pub key: SortKey,
    pub order: SortOrder,
}

impl Sort {
    pub fn new(key: SortKey, order: SortOrder) -> Self {
        Sort { key, order }
    }

    /// Orders two entries.
    ///
    /// Directories are compared first and their grouping is **never**
    /// reversed: flipping the direction moves the newest file to the top, it
    /// does not bury the directories at the bottom. Every key ends in a name
    /// tiebreak, so the result is a total order over a directory's entries
    /// and `Descending` is the exact reverse of `Ascending` within a group.
    pub fn compare(&self, a: &Entry, b: &Entry) -> Ordering {
        let grouping = group_rank(a).cmp(&group_rank(b));
        if grouping != Ordering::Equal {
            return grouping;
        }

        let within_group = match self.key {
            SortKey::Name => compare_names(&a.name, &b.name),
            SortKey::Ext => compare_names(extension(&a.name), extension(&b.name))
                .then_with(|| compare_names(&a.name, &b.name)),
            SortKey::Size => a
                .size
                .cmp(&b.size)
                .then_with(|| compare_names(&a.name, &b.name)),
            SortKey::Modified => a
                .modified
                .cmp(&b.modified)
                .then_with(|| compare_names(&a.name, &b.name)),
        };

        match self.order {
            SortOrder::Ascending => within_group,
            SortOrder::Descending => within_group.reverse(),
        }
    }
}

/// Directories (and symlinks to them) sort ahead of files.
fn group_rank(entry: &Entry) -> u8 {
    if entry.is_dir() {
        0
    } else {
        1
    }
}

/// Case-insensitive comparison with a case-sensitive tiebreak.
///
/// Compares lowercased characters lazily rather than allocating two lowercase
/// `String`s per comparison — this runs O(n log n) times per directory. The
/// tiebreak keeps `A.txt` and `a.txt` in a stable, total order.
fn compare_names(a: &str, b: &str) -> Ordering {
    a.chars()
        .flat_map(char::to_lowercase)
        .cmp(b.chars().flat_map(char::to_lowercase))
        .then_with(|| a.cmp(b))
}

/// The part after the last dot, or `""` when there is none.
///
/// A leading dot does not start an extension: `.gitignore` is a name, not an
/// extension, which is both the Unix convention and Total Commander's.
fn extension(name: &str) -> &str {
    match name.rfind('.') {
        Some(index) if index > 0 => &name[index + 1..],
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_leading_dot_does_not_start_an_extension() {
        assert_eq!(extension("archive.tar.gz"), "gz");
        assert_eq!(extension("notes.txt"), "txt");
        assert_eq!(extension(".gitignore"), "");
        assert_eq!(extension("Makefile"), "");
        assert_eq!(extension("trailing."), "");
    }

    #[test]
    fn names_compare_case_insensitively_with_a_stable_tiebreak() {
        assert_eq!(compare_names("apple", "Banana"), Ordering::Less);
        assert_eq!(compare_names("Apple", "banana"), Ordering::Less);
        // Same letters, different case: never Equal, or the sort is unstable.
        assert_ne!(compare_names("a.txt", "A.txt"), Ordering::Equal);
        assert_eq!(compare_names("same", "same"), Ordering::Equal);
    }

    #[test]
    fn flipping_an_order_twice_returns_the_original() {
        for order in [SortOrder::Ascending, SortOrder::Descending] {
            assert_eq!(order.flipped().flipped(), order);
        }
    }
}
