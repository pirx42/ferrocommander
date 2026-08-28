//! Where a navigation keystroke leads.
//!
//! Pure functions over a [`Listing`], so the decisions a pane makes when the
//! user presses Enter or Backspace are testable without a window.

use tc_core::listing::Listing;
use tc_core::vfs::VfsPath;

/// Where activating the cursor row leads, or `None` when it leads nowhere.
///
/// Files are `None`: opening one is F3/F4, which is phase 4. The `..` row
/// needs no special case — it is a directory like any other, and `VfsPath`
/// normalization makes its target the parent.
pub fn activation_target(listing: &Listing) -> Option<VfsPath> {
    if !listing.current()?.is_dir() {
        return None;
    }
    listing.current_path()
}

/// Where leaving the current directory leads, or `None` at the root.
pub fn parent_target(listing: &Listing) -> Option<VfsPath> {
    listing.dir().parent()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tc_core::vfs::{Entry, EntryKind, LocalFs, VfsPath};

    use super::*;

    fn entry(name: &str, kind: EntryKind) -> Entry {
        Entry {
            name: name.to_string(),
            kind,
            size: 0,
            modified: std::time::SystemTime::UNIX_EPOCH,
            hidden: false,
        }
    }

    #[test]
    fn activating_a_directory_leads_into_it() {
        let listing = Listing::new(
            VfsPath::new("/home/pirx"),
            vec![entry("projects", EntryKind::Dir)],
        );
        // Row 0 is `..`, row 1 is the directory.
        let mut listing = listing;
        listing.set_cursor(1);

        assert_eq!(
            activation_target(&listing),
            Some(VfsPath::new("/home/pirx/projects"))
        );
    }

    #[test]
    fn activating_the_parent_row_leads_out_of_the_directory() {
        let listing = Listing::new(VfsPath::new("/home/pirx"), Vec::new());
        assert_eq!(activation_target(&listing), Some(VfsPath::new("/home")));
    }

    #[test]
    fn activating_a_file_leads_nowhere() {
        let mut listing = Listing::new(
            VfsPath::new("/home/pirx"),
            vec![entry("notes.txt", EntryKind::File)],
        );
        listing.set_cursor(1);

        assert_eq!(activation_target(&listing), None);
    }

    #[test]
    fn activating_an_empty_listing_leads_nowhere() {
        let listing = Listing::new(VfsPath::root(), Vec::new());
        assert_eq!(activation_target(&listing), None);
    }

    #[test]
    fn the_root_has_nowhere_further_up() {
        let listing = Listing::new(VfsPath::root(), Vec::new());
        assert_eq!(parent_target(&listing), None);
    }

    #[test]
    fn activating_a_real_directory_yields_a_path_that_loads() {
        let dir = tempfile::TempDir::new().unwrap();
        fs::create_dir(dir.path().join("sub")).unwrap();
        fs::write(dir.path().join("sub/inner.txt"), "x").unwrap();
        let mut listing = Listing::load(&LocalFs, LocalFs::vfs_path(dir.path())).unwrap();
        listing.set_cursor(1);
        assert_eq!(listing.current().unwrap().name, "sub");

        let target = activation_target(&listing).expect("a directory is enterable");
        let entered = Listing::load(&LocalFs, target).unwrap();

        assert_eq!(entered.current().unwrap().name, "..");
        assert!(entered.iter().any(|entry| entry.name == "inner.txt"));
    }

    #[test]
    fn walking_into_a_directory_and_back_returns_to_the_start() {
        let dir = tempfile::TempDir::new().unwrap();
        fs::create_dir(dir.path().join("sub")).unwrap();
        let start = LocalFs::vfs_path(dir.path());
        let mut listing = Listing::load(&LocalFs, start.clone()).unwrap();
        listing.set_cursor(1);

        let down = Listing::load(&LocalFs, activation_target(&listing).unwrap()).unwrap();
        let up = parent_target(&down).unwrap();

        assert_eq!(up, start);
    }
}
