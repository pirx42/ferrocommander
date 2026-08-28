//! Absolute, `/`-separated paths used by every backend.

use std::fmt;

use super::constants::{CURRENT, PARENT, ROOT, SEPARATOR};

/// An always-absolute, always-normalized VFS path.
///
/// Normalization is *lexical*: `..` pops the previous component without
/// consulting the filesystem. That is deliberate — it means going up from
/// `/link/sub` returns to `/link`, which is where the user believes they are,
/// rather than to the symlink target's parent. It also makes the type a pure
/// value with no I/O and no error case.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VfsPath {
    /// Starts with `/`; no trailing separator except for the root itself.
    inner: String,
}

impl VfsPath {
    /// The root of the filesystem.
    pub fn root() -> Self {
        VfsPath {
            inner: ROOT.to_string(),
        }
    }

    /// Interprets `input` as an absolute path and normalizes it.
    ///
    /// Total by construction: empty components, `.`, and `..` beyond the root
    /// all collapse away, so there is no such thing as an invalid `VfsPath`.
    pub fn new(input: &str) -> Self {
        let mut components: Vec<&str> = Vec::new();
        for component in input.split(SEPARATOR) {
            match component {
                "" | CURRENT => {}
                PARENT => {
                    components.pop();
                }
                name => components.push(name),
            }
        }
        VfsPath {
            inner: format!("{ROOT}{}", components.join(&SEPARATOR.to_string())),
        }
    }

    /// The path of an entry inside this directory.
    pub fn child(&self, name: &str) -> Self {
        VfsPath::new(&format!("{}{SEPARATOR}{name}", self.inner))
    }

    /// The containing directory, or `None` at the root.
    pub fn parent(&self) -> Option<Self> {
        if self.is_root() {
            return None;
        }
        Some(VfsPath::new(&format!("{}{SEPARATOR}{PARENT}", self.inner)))
    }

    /// Final component, or `None` at the root.
    pub fn file_name(&self) -> Option<&str> {
        if self.is_root() {
            return None;
        }
        self.inner.rsplit(SEPARATOR).next()
    }

    /// Components between the root separators, empty at the root.
    pub fn components(&self) -> impl Iterator<Item = &str> {
        self.inner.split(SEPARATOR).filter(|part| !part.is_empty())
    }

    pub fn is_root(&self) -> bool {
        self.inner == ROOT
    }

    pub fn as_str(&self) -> &str {
        &self.inner
    }

    /// Whether this path lies strictly below `directory`.
    ///
    /// Compared by whole components, not as text: `/home/pirx2` is not inside
    /// `/home/pirx`, and reading it as one is how a copy comes to refuse a
    /// perfectly good target — or, worse, to accept a job that writes into its
    /// own source. A path is not inside itself.
    ///
    /// The component walk also settles the root, which a prefix test gets
    /// wrong in the dangerous direction: stripping `/` off `/home` leaves
    /// `home`, which starts with no separator, so `/home` read as *not* inside
    /// `/`. Every path on Unix is under the root, and the mount rule asks
    /// exactly that question about every path there is.
    pub fn is_inside(&self, directory: &VfsPath) -> bool {
        let mut mine = self.components();
        // An empty `all` is true, which is the root having no components to
        // match — and a path equal to `directory` runs `mine` out, so nothing
        // is inside itself.
        directory.components().all(|part| mine.next() == Some(part)) && mine.next().is_some()
    }
}

impl fmt::Display for VfsPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn everything_is_inside_the_root_except_the_root() {
        // A prefix test gets this wrong in the dangerous direction, and the
        // mount rule asks it about every path there is.
        for path in ["/home", "/home/pirx", "/mnt/backup/2026"] {
            assert!(
                VfsPath::new(path).is_inside(&VfsPath::root()),
                "{path} is not under /"
            );
        }
        assert!(!VfsPath::root().is_inside(&VfsPath::root()));
    }

    #[test]
    fn a_neighbour_sharing_a_name_prefix_is_not_inside() {
        // Whole components, not text. Reading it as text is how a copy comes
        // to refuse a perfectly good target, or accept one that eats itself.
        assert!(!VfsPath::new("/home/pirx2").is_inside(&VfsPath::new("/home/pirx")));
        assert!(VfsPath::new("/home/pirx/notes").is_inside(&VfsPath::new("/home/pirx")));
    }

    #[test]
    fn nothing_is_inside_itself_or_its_own_children() {
        let path = VfsPath::new("/home/pirx");
        assert!(!path.is_inside(&path));
        assert!(!path.is_inside(&VfsPath::new("/home/pirx/notes")));
    }

    #[test]
    fn every_input_normalizes_to_an_absolute_path() {
        for input in ["a/b", "/a/b", "//a//b", "/a/./b", "/a/c/../b"] {
            assert_eq!(VfsPath::new(input).as_str(), "/a/b", "input {input}");
        }
    }

    #[test]
    fn parent_of_root_is_none_and_dotdot_cannot_escape_root() {
        assert_eq!(VfsPath::root().parent(), None);
        assert_eq!(VfsPath::new("/../../etc"), VfsPath::new("/etc"));
        assert_eq!(VfsPath::new("/.."), VfsPath::root());
    }

    #[test]
    fn child_and_parent_are_inverse_for_a_plain_name() {
        let dir = VfsPath::new("/home/pirx");
        assert_eq!(dir.child("notes.txt").parent(), Some(dir.clone()));
        assert_eq!(dir.child("notes.txt").as_str(), "/home/pirx/notes.txt");
    }

    #[test]
    fn file_name_is_the_last_component_and_absent_at_root() {
        assert_eq!(VfsPath::new("/a/b.txt").file_name(), Some("b.txt"));
        assert_eq!(VfsPath::root().file_name(), None);
    }

    #[test]
    fn components_of_root_are_empty() {
        assert_eq!(VfsPath::root().components().count(), 0);
        let path = VfsPath::new("/a/b/c");
        assert_eq!(path.components().collect::<Vec<_>>(), vec!["a", "b", "c"]);
    }

    #[test]
    fn names_with_spaces_and_unicode_survive_a_round_trip() {
        for name in ["my file.txt", "Ünïcødé — ✓.md", "trailing space .txt"] {
            let path = VfsPath::root().child(name);
            assert_eq!(path.as_str(), format!("/{name}"));
        }
    }
}
