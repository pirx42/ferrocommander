//! What a file-operation keystroke actually asks for.
//!
//! Pure functions over a [`Listing`] and the text the user typed, so every
//! decision the dialogs make is testable without a window — the same split
//! `navigation.rs` uses for the cursor keys.

use tc_core::listing::Listing;
use tc_core::ops::{DeleteMode, Destination};
use tc_core::vfs::constants::SEPARATOR;
use tc_core::vfs::VfsPath;

use crate::constants::{
    DELETE_PROMPT_PERMANENT, DELETE_PROMPT_TRASH, KIND_DIRECTORY, KIND_FILE, QUOTE_CLOSE,
    QUOTE_OPEN,
};

/// The path an operation would act on, or `None` when the cursor is
/// somewhere no operation makes sense.
///
/// The `..` row is the case that matters: it is a navigation control, not an
/// entry, and copying or deleting "the parent directory" from inside it is
/// never what the user means.
pub fn operable(listing: &Listing) -> Option<VfsPath> {
    if listing.is_parent(listing.cursor()) {
        return None;
    }
    listing.current_path()
}

/// How the F5/F6 target field is prefilled.
///
/// With a trailing separator, because that is what [`parse_destination`]
/// reads as "into this directory" — the field shows the rule rather than
/// hiding it, and deleting the slash is how the user says "under this name"
/// instead.
pub fn prefilled_target(dir: &VfsPath) -> String {
    if dir.is_root() {
        return dir.to_string();
    }
    format!("{dir}{SEPARATOR}")
}

/// Reads what the user left in the target field.
///
/// - `/home/pirx/photos/` — into that directory, each source keeping its name.
/// - `/home/pirx/photos` — to exactly that path.
/// - `holiday.txt` — to that name beside the source, which is what makes F6
///   a rename in place and F5 a duplicate.
pub fn parse_destination(input: &str, source_dir: &VfsPath) -> Option<Destination> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some(dir) = trimmed.strip_suffix(SEPARATOR) {
        return Some(Destination::Into(VfsPath::new(dir)));
    }
    if trimmed.contains(SEPARATOR) {
        return Some(Destination::Exact(VfsPath::new(trimmed)));
    }
    Some(Destination::Exact(source_dir.child(trimmed)))
}

/// The question the delete confirmation asks.
///
/// Trash and permanent are different questions, not the same question with a
/// different flag, so they get different wording rather than a shared one
/// with a word swapped in.
pub fn delete_prompt(name: &str, is_dir: bool, mode: DeleteMode) -> String {
    let kind = if is_dir { KIND_DIRECTORY } else { KIND_FILE };
    let template = match mode {
        DeleteMode::Trash => DELETE_PROMPT_TRASH,
        DeleteMode::Permanent => DELETE_PROMPT_PERMANENT,
    };
    template
        .replace("{kind}", kind)
        .replace("{name}", &format!("{QUOTE_OPEN}{name}{QUOTE_CLOSE}"))
}

#[cfg(test)]
mod tests {
    use tc_core::vfs::{Entry, EntryKind};

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

    fn listing_with_cursor_on(name: &str) -> Listing {
        let mut listing = Listing::new(
            VfsPath::new("/home/pirx"),
            vec![
                entry("notes.txt", EntryKind::File),
                entry("photos", EntryKind::Dir),
            ],
        );
        listing.focus_entry(name);
        listing
    }

    #[test]
    fn an_entry_under_the_cursor_is_what_an_operation_acts_on() {
        let listing = listing_with_cursor_on("notes.txt");
        assert_eq!(
            operable(&listing),
            Some(VfsPath::new("/home/pirx/notes.txt"))
        );
    }

    #[test]
    fn the_parent_row_is_not_something_to_copy_or_delete() {
        // Cursor starts on `..`, which is where a freshly loaded pane sits.
        let listing = Listing::new(VfsPath::new("/home/pirx"), Vec::new());
        assert_eq!(operable(&listing), None);
    }

    #[test]
    fn the_target_field_is_prefilled_with_a_trailing_separator() {
        assert_eq!(prefilled_target(&VfsPath::new("/home/pirx")), "/home/pirx/");
        // The root already ends in one; a second would be noise.
        assert_eq!(prefilled_target(&VfsPath::root()), "/");
    }

    #[test]
    fn a_trailing_separator_means_into_that_directory() {
        let here = VfsPath::new("/home/pirx");
        assert_eq!(
            parse_destination("/mnt/backup/", &here),
            Some(Destination::Into(VfsPath::new("/mnt/backup")))
        );
        assert_eq!(
            parse_destination("/", &here),
            Some(Destination::Into(VfsPath::root()))
        );
    }

    #[test]
    fn a_path_without_one_names_an_exact_destination() {
        let here = VfsPath::new("/home/pirx");
        assert_eq!(
            parse_destination("/mnt/backup/notes.txt", &here),
            Some(Destination::Exact(VfsPath::new("/mnt/backup/notes.txt")))
        );
    }

    #[test]
    fn a_bare_name_lands_beside_the_source() {
        // This is the whole of "F6 renames in place" and "F5 duplicates".
        let here = VfsPath::new("/home/pirx");
        assert_eq!(
            parse_destination("holiday.txt", &here),
            Some(Destination::Exact(VfsPath::new("/home/pirx/holiday.txt")))
        );
    }

    #[test]
    fn an_empty_target_asks_for_nothing() {
        let here = VfsPath::new("/home/pirx");
        for input in ["", "   ", "\t"] {
            assert_eq!(parse_destination(input, &here), None, "{input:?}");
        }
    }

    #[test]
    fn the_prefilled_target_round_trips_into_that_directory() {
        // The two functions are each other's inverse for the default case,
        // which is the one nearly every copy uses.
        let other = VfsPath::new("/mnt/backup");
        let here = VfsPath::new("/home/pirx");
        assert_eq!(
            parse_destination(&prefilled_target(&other), &here),
            Some(Destination::Into(other))
        );
    }

    #[test]
    fn trash_and_permanent_ask_visibly_different_questions() {
        let trash = delete_prompt("notes.txt", false, DeleteMode::Trash);
        let permanent = delete_prompt("notes.txt", false, DeleteMode::Permanent);

        assert_ne!(trash, permanent);
        assert!(trash.contains("notes.txt") && permanent.contains("notes.txt"));
        // The irreversible one has to say so.
        assert!(permanent.to_lowercase().contains("permanently"));
    }

    #[test]
    fn the_question_says_whether_a_directory_is_at_stake() {
        let file = delete_prompt("x", false, DeleteMode::Trash);
        let dir = delete_prompt("x", true, DeleteMode::Trash);
        assert_ne!(file, dir);
        assert!(dir.contains(KIND_DIRECTORY));
    }
}
