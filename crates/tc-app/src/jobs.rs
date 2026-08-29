//! What a file-operation keystroke actually asks for.
//!
//! Pure functions over a [`Listing`] and the text the user typed, so every
//! decision the dialogs make is testable without a window — the same split
//! `navigation.rs` uses for the cursor keys.

use tc_core::listing::{split_name, Listing};
use tc_core::ops::{DeleteMode, Destination};
use tc_core::vfs::constants::SEPARATOR;
use tc_core::vfs::VfsPath;

use crate::constants::{
    DELETE_PROMPT_PERMANENT, DELETE_PROMPT_TRASH, KIND_DIRECTORY, KIND_FILE,
    PACK_DEFAULT_EXTENSION, PACK_FALLBACK_NAME, QUOTE_CLOSE, QUOTE_OPEN, SELECTION_STATUS,
    SUBJECT_MANY,
};
use crate::progress::human_bytes;

/// What an operation would act on: everything marked, or the row under the
/// cursor when nothing is marked.
///
/// The fallback is what makes the marks optional rather than a mode — press
/// F5 on a file and it copies, mark ten and it copies ten, and there is no
/// third thing to learn. Empty when neither applies, which is the `..` row on
/// its own: a navigation control, not an entry, and copying or deleting "the
/// parent directory" from inside it is never what the user means.
pub fn sources(listing: &Listing) -> Vec<VfsPath> {
    let marked = listing.selected_paths();
    if !marked.is_empty() {
        return marked;
    }
    if listing.is_parent(listing.cursor()) {
        return Vec::new();
    }
    listing.current_path().into_iter().collect()
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

/// What the delete confirmation calls the things at stake.
///
/// One entry is named, because the name is the thing worth checking. Several
/// are counted, because a list of forty names is not a question anyone reads.
pub fn subject(listing: &Listing, count: usize) -> String {
    if count == 1 {
        let named = listing
            .selected_paths()
            .first()
            .and_then(|path| path.file_name().map(str::to_string))
            .or_else(|| listing.current().map(|entry| entry.name.clone()))
            .unwrap_or_default();
        let kind = match listing.current().is_some_and(|entry| entry.is_dir()) {
            true => KIND_DIRECTORY,
            false => KIND_FILE,
        };
        return format!("{kind} {QUOTE_OPEN}{named}{QUOTE_CLOSE}");
    }
    SUBJECT_MANY.replace("{count}", &count.to_string())
}

/// The question the delete confirmation asks.
///
/// Trash and permanent are different questions, not the same question with a
/// different flag, so they get different wording rather than a shared one
/// with a word swapped in.
pub fn delete_prompt(subject: &str, mode: DeleteMode) -> String {
    let template = match mode {
        DeleteMode::Trash => DELETE_PROMPT_TRASH,
        DeleteMode::Permanent => DELETE_PROMPT_PERMANENT,
    };
    template.replace("{subject}", subject)
}

/// What a pane's status line says about the marks.
pub fn selection_status(listing: &Listing) -> String {
    let marked = listing.selection_summary();
    let visible = listing.visible_summary();
    SELECTION_STATUS
        .replace("{marked}", &marked.count.to_string())
        .replace("{total}", &visible.count.to_string())
        .replace("{marked_bytes}", &human_bytes(marked.bytes))
        .replace("{total_bytes}", &human_bytes(visible.bytes))
}

/// Where the name typed into Alt+F5's field puts the archive.
///
/// A bare name lands beside the sources, exactly as it does for F5 and F6:
/// the field arrives prefilled with a path beside the *other* pane and
/// selected, so somebody who types over it with one word means "here", and a
/// program that read that as the root of the disk would be answering a
/// question nobody asked.
///
/// `None` for nothing at all.
pub fn packed_at(input: &str, source_dir: &VfsPath) -> Option<VfsPath> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(match trimmed.contains(SEPARATOR) {
        true => VfsPath::new(trimmed),
        false => source_dir.child(trimmed),
    })
}

/// The archive name Alt+F5 offers: beside `dir`, named after what is being
/// packed.
///
/// After the cursor row when that is the whole job, and after the directory
/// the files are in when several are — which is what somebody would have typed
/// themselves, and the reason the field arrives selected so they can type over
/// it instead.
pub fn prefilled_archive(dir: &VfsPath, listing: &Listing, count: usize) -> String {
    let subject = if count == 1 {
        listing
            .current()
            .map(|entry| split_name(&entry.name).0.to_string())
    } else {
        listing.dir().file_name().map(str::to_string)
    };
    let name = subject
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| PACK_FALLBACK_NAME.to_string());
    format!("{}{name}{PACK_DEFAULT_EXTENSION}", prefilled_target(dir))
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
            attributes: Default::default(),
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
        // The fallback that keeps marks optional rather than a mode.
        let listing = listing_with_cursor_on("notes.txt");
        assert_eq!(sources(&listing), [VfsPath::new("/home/pirx/notes.txt")]);
    }

    #[test]
    fn marks_win_over_the_cursor_wherever_the_cursor_is() {
        let mut listing = listing_with_cursor_on("notes.txt");
        listing.focus_entry("photos");
        listing.toggle_selected(listing.cursor());
        // Put the cursor back on the unmarked entry.
        listing.focus_entry("notes.txt");

        assert_eq!(sources(&listing), [VfsPath::new("/home/pirx/photos")]);
    }

    #[test]
    fn every_marked_entry_is_a_source() {
        let mut listing = listing_with_cursor_on("notes.txt");
        listing.select_all();

        assert_eq!(
            sources(&listing),
            [
                VfsPath::new("/home/pirx/photos"),
                VfsPath::new("/home/pirx/notes.txt"),
            ],
            "in the order they are shown"
        );
    }

    #[test]
    fn the_parent_row_alone_is_not_something_to_copy_or_delete() {
        // Cursor starts on `..`, which is where a freshly loaded pane sits,
        // and nothing is marked.
        let listing = Listing::new(VfsPath::new("/home/pirx"), Vec::new());
        assert!(sources(&listing).is_empty());
    }

    #[test]
    fn a_marked_entry_is_still_a_source_while_the_cursor_sits_on_the_parent_row() {
        // The `..` rule is about the *fallback*, not about the marks.
        let mut listing = listing_with_cursor_on("notes.txt");
        listing.toggle_selected(listing.cursor());
        listing.move_cursor_to_first();
        assert!(listing.is_parent(listing.cursor()));

        assert_eq!(sources(&listing), [VfsPath::new("/home/pirx/notes.txt")]);
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
    fn every_prompt_is_fully_rendered() {
        // The templates and the `.replace` calls name their placeholders
        // separately, so a rename on one side would leave `{subject}` sitting
        // in a dialog. Checking for a leftover brace catches all of them at
        // once rather than one assertion per placeholder.
        let listing = listing_with_cursor_on("notes.txt");
        for count in [1, 7] {
            for mode in [DeleteMode::Trash, DeleteMode::Permanent] {
                let rendered = delete_prompt(&subject(&listing, count), mode);
                assert!(
                    !rendered.contains('{'),
                    "unsubstituted placeholder in {rendered:?}"
                );
            }
        }
        assert!(!selection_status(&listing).contains('{'));
    }

    #[test]
    fn trash_and_permanent_ask_visibly_different_questions() {
        let listing = listing_with_cursor_on("notes.txt");
        let named = subject(&listing, 1);
        let trash = delete_prompt(&named, DeleteMode::Trash);
        let permanent = delete_prompt(&named, DeleteMode::Permanent);

        assert_ne!(trash, permanent);
        assert!(trash.contains("notes.txt") && permanent.contains("notes.txt"));
        // The irreversible one has to say so.
        assert!(permanent.to_lowercase().contains("permanently"));
    }

    #[test]
    fn one_entry_is_named_and_several_are_counted() {
        // A list of forty names is not a question anyone reads.
        let listing = listing_with_cursor_on("notes.txt");

        assert!(subject(&listing, 1).contains("notes.txt"));

        let many = subject(&listing, 40);
        assert!(many.contains("40"), "{many}");
        assert!(!many.contains("notes.txt"), "{many}");
    }

    #[test]
    fn the_question_says_whether_a_directory_is_at_stake() {
        let file = subject(&listing_with_cursor_on("notes.txt"), 1);
        let dir = subject(&listing_with_cursor_on("photos"), 1);
        assert_ne!(file, dir);
        assert!(dir.contains(KIND_DIRECTORY));
    }

    #[test]
    fn the_status_line_counts_the_marks_against_what_is_visible() {
        let mut listing = listing_with_cursor_on("notes.txt");
        assert!(selection_status(&listing).starts_with("0 of 2"));

        listing.toggle_selected(listing.cursor());
        assert!(selection_status(&listing).starts_with("1 of 2"));
    }
}
