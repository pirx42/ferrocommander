//! Behavioral tests for the directory model.
//!
//! Most run on `Listing::new`, which takes no filesystem at all — that is
//! itself the proof that sorting, filtering and cursor movement never touch
//! the disk. Only `load`/`reload` are exercised against a real tempdir.

use std::fs;
use std::time::{Duration, SystemTime};

use tc_core::listing::{Listing, Sort, SortKey, SortOrder};
use tc_core::vfs::{Entry, EntryKind, LocalFs, VfsPath, VirtualFs};

const ALL_KEYS: [SortKey; 4] = [
    SortKey::Name,
    SortKey::Ext,
    SortKey::Size,
    SortKey::Modified,
];
const ALL_ORDERS: [SortOrder; 2] = [SortOrder::Ascending, SortOrder::Descending];

fn at(seconds: u64) -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(seconds)
}

fn dir_entry(name: &str, modified: u64) -> Entry {
    Entry {
        name: name.to_string(),
        kind: EntryKind::Dir,
        size: 0,
        modified: at(modified),
        hidden: name.starts_with('.'),
    }
}

fn file_entry(name: &str, size: u64, modified: u64) -> Entry {
    Entry {
        name: name.to_string(),
        kind: EntryKind::File,
        size,
        modified: at(modified),
        hidden: name.starts_with('.'),
    }
}

/// Two directories and three files, chosen so that every sort key produces a
/// different order — otherwise a broken key could pass by looking like name.
fn fixture() -> Vec<Entry> {
    vec![
        file_entry("b.txt", 30, 3),
        dir_entry("zeta_dir", 4),
        file_entry("a.md", 10, 1),
        dir_entry("Alpha_dir", 5),
        file_entry("c.zip", 20, 2),
    ]
}

fn listing() -> Listing {
    Listing::new(VfsPath::new("/home/pirx"), fixture())
}

fn rows(listing: &Listing) -> Vec<String> {
    listing.iter().map(|entry| entry.name.clone()).collect()
}

fn sorted_rows(key: SortKey, order: SortOrder) -> Vec<String> {
    let mut listing = listing();
    listing.set_sort(Sort::new(key, order));
    rows(&listing)
}

#[test]
fn each_sort_key_and_direction_produces_its_documented_order() {
    let cases = [
        (
            SortKey::Name,
            SortOrder::Ascending,
            ["..", "Alpha_dir", "zeta_dir", "a.md", "b.txt", "c.zip"],
        ),
        (
            SortKey::Name,
            SortOrder::Descending,
            ["..", "zeta_dir", "Alpha_dir", "c.zip", "b.txt", "a.md"],
        ),
        (
            SortKey::Ext,
            SortOrder::Ascending,
            ["..", "Alpha_dir", "zeta_dir", "a.md", "b.txt", "c.zip"],
        ),
        (
            SortKey::Ext,
            SortOrder::Descending,
            ["..", "zeta_dir", "Alpha_dir", "c.zip", "b.txt", "a.md"],
        ),
        (
            SortKey::Size,
            SortOrder::Ascending,
            ["..", "Alpha_dir", "zeta_dir", "a.md", "c.zip", "b.txt"],
        ),
        (
            SortKey::Size,
            SortOrder::Descending,
            ["..", "zeta_dir", "Alpha_dir", "b.txt", "c.zip", "a.md"],
        ),
        (
            SortKey::Modified,
            SortOrder::Ascending,
            ["..", "zeta_dir", "Alpha_dir", "a.md", "c.zip", "b.txt"],
        ),
        (
            SortKey::Modified,
            SortOrder::Descending,
            ["..", "Alpha_dir", "zeta_dir", "b.txt", "c.zip", "a.md"],
        ),
    ];

    for (key, order, expected) in cases {
        assert_eq!(sorted_rows(key, order), expected, "{key:?} {order:?}");
    }
}

#[test]
fn directories_stay_above_files_under_every_key_and_direction() {
    for key in ALL_KEYS {
        for order in ALL_ORDERS {
            let mut listing = listing();
            listing.set_sort(Sort::new(key, order));
            let kinds: Vec<bool> = listing.iter().map(|entry| entry.is_dir()).collect();
            let first_file = kinds.iter().position(|&is_dir| !is_dir).unwrap();
            assert!(
                kinds[first_file..].iter().all(|&is_dir| !is_dir),
                "a directory sorted below a file for {key:?} {order:?}"
            );
        }
    }
}

#[test]
fn descending_is_the_exact_reverse_of_ascending_within_each_group() {
    for key in ALL_KEYS {
        let ascending = sorted_rows(key, SortOrder::Ascending);
        let descending = sorted_rows(key, SortOrder::Descending);

        // Row 0 is `..`, then two directories, then three files.
        let ascending_dirs: Vec<_> = ascending[1..3].iter().rev().cloned().collect();
        let ascending_files: Vec<_> = ascending[3..].iter().rev().cloned().collect();
        assert_eq!(descending[1..3].to_vec(), ascending_dirs, "{key:?} dirs");
        assert_eq!(descending[3..].to_vec(), ascending_files, "{key:?} files");
    }
}

#[test]
fn the_parent_row_comes_first_under_every_key_and_direction() {
    for key in ALL_KEYS {
        for order in ALL_ORDERS {
            let listing = {
                let mut listing = listing();
                listing.set_sort(Sort::new(key, order));
                listing
            };
            assert!(listing.is_parent(0), "{key:?} {order:?}");
            assert_eq!(listing.get(0).unwrap().name, "..");
        }
    }
}

#[test]
fn the_root_has_no_parent_row() {
    let listing = Listing::new(VfsPath::root(), fixture());

    assert!(!listing.is_parent(0));
    assert_eq!(listing.len(), fixture().len());
    assert!(!rows(&listing).contains(&"..".to_string()));
}

#[test]
fn activating_the_parent_row_leads_to_the_parent_directory() {
    let listing = listing();
    assert_eq!(listing.path_at(0), VfsPath::new("/home/pirx").parent());
    assert_eq!(listing.path_at(0), Some(VfsPath::new("/home")));
}

#[test]
fn activating_a_row_leads_to_that_entry() {
    let mut listing = listing();
    listing.set_cursor(1);
    assert_eq!(
        listing.current_path(),
        Some(VfsPath::new("/home/pirx/Alpha_dir"))
    );
}

#[test]
fn showing_hidden_entries_adds_exactly_the_hidden_ones() {
    let mut entries = fixture();
    entries.push(file_entry(".secret", 5, 9));
    entries.push(dir_entry(".config", 9));
    let hidden_count = entries.iter().filter(|entry| entry.hidden).count();
    let mut listing = Listing::new(VfsPath::new("/home/pirx"), entries);

    let visible = listing.len();
    listing.toggle_hidden();
    let with_hidden = listing.len();

    assert_eq!(with_hidden, visible + hidden_count);
    assert!(listing.show_hidden());
}

#[test]
fn toggling_hidden_twice_restores_the_exact_view() {
    let mut entries = fixture();
    entries.push(file_entry(".secret", 5, 9));
    let mut listing = Listing::new(VfsPath::new("/home/pirx"), entries);

    let before = rows(&listing);
    listing.toggle_hidden();
    listing.toggle_hidden();

    assert_eq!(rows(&listing), before);
    assert!(!listing.show_hidden());
}

#[test]
fn the_cursor_clamps_when_the_row_it_sits_on_becomes_hidden() {
    let mut entries = fixture();
    entries.push(file_entry(".secret", 5, 9));
    let mut listing = Listing::new(VfsPath::new("/home/pirx"), entries);
    // Descending puts the dot-file last — under ascending a leading dot sorts
    // it to the front, which would not exercise the clamp at all.
    listing.set_sort(Sort::new(SortKey::Name, SortOrder::Descending));
    listing.toggle_hidden();
    listing.move_cursor_to_last();
    assert_eq!(listing.current().unwrap().name, ".secret");
    let last_row = listing.cursor();

    listing.toggle_hidden();

    // The focused entry is gone and its index is now past the end.
    assert!(last_row >= listing.len());
    assert!(listing.cursor() < listing.len());
    assert!(listing.current().is_some(), "cursor fell off the listing");
}

#[test]
fn the_cursor_follows_an_entry_that_survives_the_hidden_toggle() {
    let mut entries = fixture();
    entries.push(file_entry(".secret", 5, 9));
    let mut listing = Listing::new(VfsPath::new("/home/pirx"), entries);
    listing.toggle_hidden();
    let secret = (0..listing.len())
        .find(|&index| listing.get(index).unwrap().name == ".secret")
        .expect("hidden entry is visible while show_hidden is on");
    listing.set_cursor(secret + 1);
    let neighbour = listing.current().unwrap().name.clone();

    listing.toggle_hidden();

    assert_eq!(listing.current().unwrap().name, neighbour);
}

#[test]
fn the_cursor_follows_its_entry_when_the_sort_changes() {
    let mut listing = listing();
    listing.set_cursor(3); // "a.md" under the default name/ascending order
    assert_eq!(listing.current().unwrap().name, "a.md");

    listing.set_sort(Sort::new(SortKey::Size, SortOrder::Descending));

    assert_eq!(listing.current().unwrap().name, "a.md");
    assert_ne!(listing.cursor(), 3, "the fixture should have moved it");
}

#[test]
fn focusing_an_entry_by_name_moves_the_cursor_to_it() {
    let mut listing = listing();
    assert_eq!(listing.cursor(), 0);

    listing.focus_entry("c.zip");

    assert_eq!(listing.current().unwrap().name, "c.zip");
}

#[test]
fn focusing_an_entry_that_is_not_visible_leaves_the_cursor_alone() {
    let mut entries = fixture();
    entries.push(file_entry(".secret", 5, 9));
    let mut listing = Listing::new(VfsPath::new("/home/pirx"), entries);
    listing.set_cursor(2);
    let before = listing.cursor();

    // Hidden while `show_hidden` is off, and a name that does not exist.
    listing.focus_entry(".secret");
    listing.focus_entry("nothing_here");

    assert_eq!(listing.cursor(), before);
}

#[test]
fn the_cursor_never_leaves_the_row_range() {
    let mut listing = listing();

    listing.move_cursor_by(-5);
    assert_eq!(listing.cursor(), 0);

    listing.move_cursor_by(1000);
    assert_eq!(listing.cursor(), listing.len() - 1);

    listing.move_cursor_to_first();
    assert_eq!(listing.cursor(), 0);

    listing.set_cursor(usize::MAX);
    assert_eq!(listing.cursor(), listing.len() - 1);
}

#[test]
fn an_empty_listing_has_no_current_row() {
    let listing = Listing::new(VfsPath::root(), Vec::new());

    assert!(listing.is_empty());
    assert_eq!(listing.cursor(), 0);
    assert_eq!(listing.current(), None);
    assert_eq!(listing.current_path(), None);
}

#[test]
fn loading_a_real_directory_lists_its_entries_below_the_parent_row() {
    let dir = tempfile::TempDir::new().unwrap();
    fs::write(dir.path().join("one.txt"), "1").unwrap();
    fs::create_dir(dir.path().join("sub")).unwrap();
    let path = LocalFs::vfs_path(dir.path());

    let listing = Listing::load(&LocalFs, path).unwrap();

    assert_eq!(rows(&listing), ["..", "sub", "one.txt"]);
}

#[test]
fn reloading_keeps_the_cursor_on_the_same_entry() {
    let dir = tempfile::TempDir::new().unwrap();
    for name in ["a.txt", "b.txt", "c.txt"] {
        fs::write(dir.path().join(name), name).unwrap();
    }
    let path = LocalFs::vfs_path(dir.path());
    let mut listing = Listing::load(&LocalFs, path).unwrap();
    listing.move_cursor_to_last();
    assert_eq!(listing.current().unwrap().name, "c.txt");

    // A new entry sorts in front of the focused one and shifts every index.
    fs::write(dir.path().join("aa.txt"), "aa").unwrap();
    listing.reload(&LocalFs).unwrap();

    assert_eq!(listing.current().unwrap().name, "c.txt");
    assert_eq!(listing.len(), 5);
}

#[test]
fn reloading_clamps_the_cursor_when_the_focused_entry_is_gone() {
    let dir = tempfile::TempDir::new().unwrap();
    for name in ["a.txt", "b.txt", "c.txt"] {
        fs::write(dir.path().join(name), name).unwrap();
    }
    let path = LocalFs::vfs_path(dir.path());
    let mut listing = Listing::load(&LocalFs, path).unwrap();
    listing.move_cursor_to_last();

    for name in ["b.txt", "c.txt"] {
        fs::remove_file(dir.path().join(name)).unwrap();
    }
    listing.reload(&LocalFs).unwrap();

    assert!(listing.cursor() < listing.len());
    assert!(listing.current().is_some());
}

#[test]
fn a_listing_can_be_built_from_a_trait_object_backend() {
    let fs: Box<dyn VirtualFs> = Box::new(LocalFs);
    assert!(Listing::load(fs.as_ref(), VfsPath::root()).is_ok());
}
