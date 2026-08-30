//! The clipboard's encoding, which is a wire format other programs read.

use tc_core::clipboard::{
    decode_gnome, decode_uri_list, encode_gnome, encode_uri_list, from_uri, to_uri, Clipped,
};
use tc_core::vfs::VfsPath;

fn paths(names: &[&str]) -> Vec<VfsPath> {
    names.iter().map(|name| VfsPath::new(name)).collect()
}

#[test]
fn a_copy_round_trips_through_the_gnome_format() {
    let clipped = Clipped {
        paths: paths(&["/home/pirx/notes.txt", "/home/pirx/src"]),
        cut: false,
    };

    let payload = encode_gnome(&clipped);
    assert_eq!(decode_gnome(&payload), Some(clipped));
}

#[test]
fn a_cut_round_trips_and_keeps_being_a_cut() {
    // The one thing this format carries that `text/uri-list` cannot, and the
    // one whose loss would silently turn a move into a copy.
    let clipped = Clipped {
        paths: paths(&["/home/pirx/notes.txt"]),
        cut: true,
    };

    let payload = encode_gnome(&clipped);
    assert!(payload.starts_with("cut\n"), "the verb: {payload:?}");
    assert_eq!(decode_gnome(&payload), Some(clipped));
}

#[test]
fn names_with_spaces_and_unicode_survive_the_round_trip() {
    // A file called `my notes.txt` is ordinary, and a URI that carried the
    // space raw would be one nothing else can read.
    let clipped = Clipped {
        paths: paths(&["/home/pirx/my notes.txt", "/home/pirx/Grüße & co/#1.txt"]),
        cut: false,
    };

    let payload = encode_gnome(&clipped);
    assert!(payload.contains("my%20notes.txt"), "encoded: {payload}");
    assert_eq!(decode_gnome(&payload), Some(clipped));
}

#[test]
fn the_separator_is_not_encoded_away() {
    // Encoding the whole path at once turns every `/` into `%2F` and names a
    // single file with slashes in it. Encoding per component is the fix, and
    // this is what says so.
    let uri = to_uri(&VfsPath::new("/home/pirx/deep/inner.txt"));

    assert_eq!(uri, "file:///home/pirx/deep/inner.txt");
    assert!(!uri.contains("%2F"), "the separators were encoded: {uri}");
}

#[test]
fn a_uri_that_is_not_a_local_file_is_refused() {
    // Not turned into a local path with the scheme chopped off: that would
    // act on a file of that name which happens to exist here.
    assert_eq!(from_uri("trash:///expunged/notes.txt"), None);
    assert_eq!(from_uri("smb://server/share/notes.txt"), None);
    assert_eq!(from_uri("https://example.com/notes.txt"), None);
    assert_eq!(from_uri("/home/pirx/notes.txt"), None, "no scheme at all");
    assert_eq!(from_uri(""), None);
    // `file://host/path` is another machine's file, not this one's.
    assert_eq!(from_uri("file://elsewhere/home/notes.txt"), None);
}

#[test]
fn a_payload_with_no_verb_is_refused_rather_than_guessed_at() {
    // Guessing wrong in one direction deletes the source, so a payload whose
    // intent cannot be read yields nothing at all.
    assert_eq!(decode_gnome("file:///home/pirx/notes.txt"), None);
    assert_eq!(decode_gnome(""), None);
    assert_eq!(decode_gnome("move\nfile:///home/pirx/notes.txt"), None);
}

#[test]
fn a_uri_list_round_trips_and_says_nothing_about_cutting() {
    let wanted = paths(&["/home/pirx/a.txt", "/home/pirx/b.txt"]);

    let payload = encode_uri_list(&wanted);
    assert!(payload.contains("\r\n"), "RFC 2483 separates with CRLF");
    assert_eq!(decode_uri_list(&payload), wanted);
}

#[test]
fn a_uri_list_ignores_its_comments() {
    let payload = "# a comment\r\nfile:///home/pirx/a.txt\r\n";

    assert_eq!(decode_uri_list(payload), paths(&["/home/pirx/a.txt"]));
}

#[test]
fn a_list_with_something_unreachable_in_it_keeps_the_rest() {
    // One entry nobody can act on must not cost the others — the same rule
    // the operation engine follows for a source it cannot read.
    let payload = "copy\nfile:///home/pirx/a.txt\ntrash:///gone\nfile:///home/pirx/b.txt";

    let clipped = decode_gnome(payload).expect("a verb it understands");
    assert_eq!(
        clipped.paths,
        paths(&["/home/pirx/a.txt", "/home/pirx/b.txt"])
    );
}

#[test]
fn the_text_form_is_one_plain_path_per_line() {
    // What a terminal or an editor pastes: no scheme, no encoding, because
    // what they want is something to type, not something to parse.
    let text = tc_core::clipboard::encode_text(&paths(&["/home/pirx/a.txt", "/home/pirx/b.txt"]));

    assert_eq!(text, "/home/pirx/a.txt\n/home/pirx/b.txt");
}
