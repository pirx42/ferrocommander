//! What `Ctrl+C`, `Ctrl+X` and `Ctrl+V` put on the clipboard and take off it.
//!
//! Pure: encoding and decoding only. Which clipboard, and when, is the
//! shell's business — this module knows nothing about GTK, and everything
//! here is testable without one.
//!
//! **The freedesktop convention, not an invention of ours.** Nautilus, Nemo,
//! Thunar and Caja all read and write `x-special/gnome-copied-files`, whose
//! payload is the word `copy` or `cut`, then one `file://` URI per line. That
//! word is the only place a cut is recorded: `text/uri-list` carries the same
//! paths with no way to say what should happen to them, which is why a
//! program that offers only the list is offering a copy.
//!
//! Windows has none of this — the clipboard there is `CF_HDROP` and a
//! separate preferred-drop-effect — so this module is freedesktop-shaped and
//! says so rather than pretending to be portable.

use crate::vfs::VfsPath;

/// The format that carries the verb as well as the paths.
pub const GNOME_COPIED_FILES: &str = "x-special/gnome-copied-files";

/// The format everything else understands, which cannot say "cut".
pub const URI_LIST: &str = "text/uri-list";

/// What the payload's first line says.
const VERB_COPY: &str = "copy";
const VERB_CUT: &str = "cut";

/// The scheme a path can come back from. Anything else names a file this
/// program has no way to reach.
const FILE_SCHEME: &str = "file://";

/// Lines of a `text/uri-list` are separated by CRLF, per RFC 2483.
const URI_LIST_SEPARATOR: &str = "\r\n";

/// Paths taken from a clipboard, and what was meant to happen to them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Clipped {
    pub paths: Vec<VfsPath>,
    /// `true` when the source asked for a move. The paths are still only
    /// copied first — deleting the source is the *engine's* job, after it has
    /// a complete copy, and never this module's.
    pub cut: bool,
}

/// Writes the payload of [`GNOME_COPIED_FILES`].
pub fn encode_gnome(clipped: &Clipped) -> String {
    let verb = match clipped.cut {
        true => VERB_CUT,
        false => VERB_COPY,
    };
    std::iter::once(verb.to_string())
        .chain(clipped.paths.iter().map(to_uri))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Reads the payload of [`GNOME_COPIED_FILES`].
///
/// `None` when the first line is not a verb this understands: a payload
/// whose intent cannot be read is not one to guess at, since guessing wrong
/// in one direction deletes the source.
pub fn decode_gnome(payload: &str) -> Option<Clipped> {
    let mut lines = payload.lines();
    let cut = match lines.next()?.trim() {
        VERB_CUT => true,
        VERB_COPY => false,
        _ => return None,
    };
    Some(Clipped {
        paths: lines.filter_map(from_uri).collect(),
        cut,
    })
}

/// Writes the payload of [`URI_LIST`], which is always a copy.
pub fn encode_uri_list(paths: &[VfsPath]) -> String {
    paths
        .iter()
        .map(to_uri)
        .collect::<Vec<_>>()
        .join(URI_LIST_SEPARATOR)
}

/// Reads a `text/uri-list`, ignoring its comment lines.
pub fn decode_uri_list(payload: &str) -> Vec<VfsPath> {
    payload
        .lines()
        .filter(|line| !line.starts_with('#'))
        .filter_map(from_uri)
        .collect()
}

/// One path as a `file://` URI.
///
/// Encoded per component, because `/` is the one reserved character that has
/// to survive: encoding the whole path at once turns every separator into
/// `%2F` and produces a URI naming a single file with slashes in its name.
pub fn to_uri(path: &VfsPath) -> String {
    let encoded = path
        .as_str()
        .split('/')
        .map(|component| urlencoding::encode(component).into_owned())
        .collect::<Vec<_>>()
        .join("/");
    format!("{FILE_SCHEME}{encoded}")
}

/// One `file://` URI back to a path.
///
/// `None` for anything else — `trash://`, `smb://`, a bare path, a blank
/// line. A URI this program cannot reach is not a local path with the scheme
/// chopped off, and treating it as one would act on a file of that name that
/// happens to exist.
pub fn from_uri(uri: &str) -> Option<VfsPath> {
    let rest = uri.trim().strip_prefix(FILE_SCHEME)?;
    // A `file://host/path` names somebody else's machine. Only an empty
    // authority — `file:///path` — is this one.
    let decoded = urlencoding::decode(rest).ok()?;
    decoded.starts_with('/').then(|| VfsPath::new(&decoded))
}
