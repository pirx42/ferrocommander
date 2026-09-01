//! Where a navigation keystroke leads.
//!
//! Pure functions over a [`Listing`], so the decisions a pane makes when the
//! user presses Enter or Backspace are testable without a window.

use fc_core::archive::format_for;
use fc_core::listing::Listing;
use fc_core::vfs::VfsPath;

/// What pressing Enter on the cursor row means.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    /// A directory on the backend the pane already has.
    Into(VfsPath),
    /// An archive, which needs a backend of its own opened over it.
    Enter(VfsPath),
    /// The `..` row at a root that has no parent on this backend — which
    /// means the root of an archive, and leads back out of it.
    Out,
}

/// Where activating the cursor row leads, or `None` when it leads nowhere.
///
/// An ordinary file is `None`: opening one is F3 or F4, not Enter. An archive
/// is not an ordinary file — walking into it is what Enter has always meant in
/// Total Commander, and what the [archive backend](fc_core::archive) exists
/// for. The `..` row needs no special case: it is a directory like any other,
/// and `VfsPath` normalization makes its target the parent.
pub fn activation_step(listing: &Listing) -> Option<Step> {
    let entry = listing.current()?;
    let path = listing.current_path()?;
    if entry.is_dir() {
        // `..` at a root leads to the root itself, because that is what
        // `VfsPath` normalization makes of it. A pane showing such a row is a
        // pane inside an archive — the listing was given the row precisely
        // because there is somewhere to go — and where it goes is out.
        if path == *listing.dir() {
            return Some(Step::Out);
        }
        return Some(Step::Into(path));
    }
    format_for(&entry.name).map(|_| Step::Enter(path))
}

/// Where leaving the current directory leads, or `None` at the root.
pub fn parent_target(listing: &Listing) -> Option<VfsPath> {
    listing.dir().parent()
}

/// The model cursor implied by the widget's selection index.
///
/// `None` when the widget reports no selection at all, which it does while a
/// pane is being repopulated.
pub fn adopted_cursor(selected: u32) -> Option<usize> {
    (selected != gtk::INVALID_LIST_POSITION).then_some(selected as usize)
}

/// Which entry the cursor should land on after moving from `from` to `to`.
///
/// Stepping **up** lands on the directory just left: someone who pressed
/// Backspace is looking for where they were, not for the top of the list.
/// Every other move — descending, or jumping somewhere unrelated — starts at
/// the top, since there is no previous position to restore.
pub fn focus_after_move(from: &VfsPath, to: &VfsPath) -> Option<String> {
    if from.parent().as_ref() != Some(to) {
        return None;
    }
    from.file_name().map(str::to_string)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use fc_core::vfs::{Entry, EntryKind, LocalFs, VfsPath};

    use super::*;

    fn entry(name: &str, kind: EntryKind) -> Entry {
        Entry {
            name: name.to_string(),
            kind,
            size: 0,
            modified: std::time::SystemTime::UNIX_EPOCH,
            attributes: Default::default(),
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
            activation_step(&listing),
            Some(Step::Into(VfsPath::new("/home/pirx/projects")))
        );
    }

    #[test]
    fn activating_the_parent_row_leads_out_of_the_directory() {
        let listing = Listing::new(VfsPath::new("/home/pirx"), Vec::new());
        assert_eq!(
            activation_step(&listing),
            Some(Step::Into(VfsPath::new("/home")))
        );
    }

    #[test]
    fn activating_a_file_leads_nowhere() {
        let mut listing = Listing::new(
            VfsPath::new("/home/pirx"),
            vec![entry("notes.txt", EntryKind::File)],
        );
        listing.set_cursor(1);

        assert_eq!(activation_step(&listing), None);
    }

    #[test]
    fn activating_an_empty_listing_leads_nowhere() {
        let listing = Listing::new(VfsPath::root(), Vec::new());
        assert_eq!(activation_step(&listing), None);
    }

    #[test]
    fn activating_an_archive_leads_into_it_rather_than_nowhere() {
        // The one file Enter does something with, and the whole point of the
        // archive backend.
        let mut listing = Listing::new(
            VfsPath::new("/home/pirx"),
            vec![
                entry("backup.tar.gz", EntryKind::File),
                entry("notes.txt", EntryKind::File),
            ],
        );
        listing.focus_entry("backup.tar.gz");
        assert_eq!(
            activation_step(&listing),
            Some(Step::Enter(VfsPath::new("/home/pirx/backup.tar.gz")))
        );

        listing.focus_entry("notes.txt");
        assert_eq!(activation_step(&listing), None);
    }

    #[test]
    fn a_widget_selection_becomes_the_model_cursor() {
        assert_eq!(adopted_cursor(0), Some(0));
        assert_eq!(adopted_cursor(7), Some(7));
    }

    #[test]
    fn no_widget_selection_means_no_cursor_to_adopt() {
        assert_eq!(adopted_cursor(gtk::INVALID_LIST_POSITION), None);
    }

    #[test]
    fn stepping_up_lands_on_the_directory_just_left() {
        let from = VfsPath::new("/home/pirx/projects");
        assert_eq!(
            focus_after_move(&from, &VfsPath::new("/home/pirx")),
            Some("projects".to_string())
        );
    }

    #[test]
    fn stepping_down_starts_at_the_top() {
        let from = VfsPath::new("/home/pirx");
        assert_eq!(
            focus_after_move(&from, &VfsPath::new("/home/pirx/projects")),
            None
        );
    }

    #[test]
    fn an_unrelated_jump_starts_at_the_top() {
        let from = VfsPath::new("/home/pirx/projects");
        // A sibling, and a grandparent: neither is the directory we left.
        assert_eq!(
            focus_after_move(&from, &VfsPath::new("/home/pirx/music")),
            None
        );
        assert_eq!(focus_after_move(&from, &VfsPath::new("/home")), None);
    }

    #[test]
    fn there_is_nothing_to_focus_when_leaving_the_root() {
        assert_eq!(focus_after_move(&VfsPath::root(), &VfsPath::root()), None);
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

        let target = into(&listing).expect("a directory is enterable");
        let entered = Listing::load(&LocalFs, target).unwrap();

        assert_eq!(entered.current().unwrap().name, "..");
        assert!(entered.iter().any(|entry| entry.name == "inner.txt"));
    }

    /// Where the cursor row leads when it leads into an ordinary directory.
    fn into(listing: &Listing) -> Option<VfsPath> {
        match activation_step(listing)? {
            Step::Into(path) => Some(path),
            other => panic!("{other:?} is not an ordinary directory"),
        }
    }

    /// Walks down and back up exactly the way `PaneView::navigate_to` does.
    fn step(listing: &Listing, target: VfsPath) -> Listing {
        let focus = focus_after_move(listing.dir(), &target);
        let mut moved = Listing::load(&LocalFs, target).unwrap();
        if let Some(name) = focus {
            moved.focus_entry(&name);
        }
        moved
    }

    #[test]
    fn stepping_back_up_puts_the_cursor_on_the_directory_just_left() {
        let dir = tempfile::TempDir::new().unwrap();
        for name in ["alpha", "beta", "gamma"] {
            fs::create_dir(dir.path().join(name)).unwrap();
        }
        let mut listing = Listing::load(&LocalFs, LocalFs::vfs_path(dir.path())).unwrap();
        // Descend from a row that is neither the first nor the last.
        listing.focus_entry("beta");

        let inside = step(&listing, into(&listing).unwrap());
        let back = step(&inside, parent_target(&inside).unwrap());

        assert_eq!(back.current().unwrap().name, "beta");
    }

    #[test]
    fn without_the_focus_step_going_up_would_land_on_the_parent_row() {
        // Pins the bug this exists to prevent: a plain reload of the parent
        // starts its cursor at the top, which is `..`, not where the user was.
        let dir = tempfile::TempDir::new().unwrap();
        fs::create_dir(dir.path().join("alpha")).unwrap();
        let mut listing = Listing::load(&LocalFs, LocalFs::vfs_path(dir.path())).unwrap();
        listing.focus_entry("alpha");
        let inside = step(&listing, into(&listing).unwrap());

        let plain = Listing::load(&LocalFs, parent_target(&inside).unwrap()).unwrap();

        assert_eq!(plain.current().unwrap().name, "..");
    }

    #[test]
    fn walking_into_a_directory_and_back_returns_to_the_start() {
        let dir = tempfile::TempDir::new().unwrap();
        fs::create_dir(dir.path().join("sub")).unwrap();
        let start = LocalFs::vfs_path(dir.path());
        let mut listing = Listing::load(&LocalFs, start.clone()).unwrap();
        listing.set_cursor(1);

        let down = Listing::load(&LocalFs, into(&listing).unwrap()).unwrap();
        let up = parent_target(&down).unwrap();

        assert_eq!(up, start);
    }
}
