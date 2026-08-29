//! What an archive contains, read once when it is opened.
//!
//! Format-independent on purpose: a zip and a tar disagree about almost
//! everything except that they hold named blobs at known offsets, so the
//! parsers fill this in and everything above it stops caring which one it was.

use std::collections::HashMap;
use std::time::SystemTime;

use crate::vfs::constants::SEPARATOR;

use super::constants::WINDOWS_SEPARATOR;
use crate::vfs::{Attributes, Entry, EntryKind, VfsPath};

/// How one entry's bytes are stored in the container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Method {
    /// Written as they are; a window can be read straight out of the container.
    Stored,
    Deflated,
    /// Named rather than swallowed, so the failure says which method it was
    /// and not merely that something went wrong.
    Unsupported(String),
    /// Readable only with a password, which this program has nowhere to keep.
    Encrypted,
}

/// Where an entry's bytes are and what has to be done to them.
#[derive(Debug, Clone)]
pub struct Bytes {
    /// Offset of the first byte of data in the container.
    pub start: u64,
    /// How many bytes are stored there.
    pub stored: u64,
    pub method: Method,
    /// What the archive says the decoded bytes add up to, or `None` where the
    /// format records no checksum.
    pub crc32: Option<u32>,
}

/// One entry: what it looks like in a listing, and where its bytes are.
#[derive(Debug, Clone)]
pub struct Node {
    pub entry: Entry,
    /// `None` for a directory, real or synthesised.
    pub bytes: Option<Bytes>,
}

/// Every path in an archive, and every directory's children.
#[derive(Debug, Default)]
pub struct Index {
    nodes: HashMap<VfsPath, Node>,
    children: HashMap<VfsPath, Vec<String>>,
}

impl Index {
    /// Starts an index whose root is dated `modified`.
    ///
    /// The container's own date rather than the epoch: nothing inside the
    /// archive implies its root, and 1970 in the date column reads as a bug in
    /// the program rather than as an absence in the file. A directory further
    /// down takes the date of the entry that implied it, which is the closest
    /// thing the archive has to one.
    pub fn new(modified: SystemTime) -> Index {
        let mut index = Index::default();
        index.children.insert(VfsPath::root(), Vec::new());
        index.nodes.insert(
            VfsPath::root(),
            Node {
                entry: directory_entry(String::new(), modified),
                bytes: None,
            },
        );
        index
    }

    /// Files an entry under its sanitised path, creating the directories above
    /// it. Ignores an entry whose name sanitises away to nothing.
    ///
    /// `size` and `modified` describe the entry as it will be listed;
    /// `bytes` is `None` for a directory.
    pub fn insert(
        &mut self,
        raw_name: &str,
        is_dir: bool,
        size: u64,
        modified: SystemTime,
        attributes: Attributes,
        bytes: Option<Bytes>,
    ) {
        let Some(filed) = sanitise(raw_name) else {
            return;
        };
        let components: Vec<String> = filed.components().map(str::to_string).collect();
        let mut path = VfsPath::root();
        let last = components.len() - 1;
        for (position, component) in components.into_iter().enumerate() {
            let parent = path.clone();
            path = path.child(&component);
            // A name is listed under its parent once. An archive may hold the
            // same path twice — two entries, or a directory both named and
            // implied — and a listing that showed it twice would let a copy
            // of that directory run over itself.
            if self.nodes.contains_key(&path) {
                // Unless the archive contradicts itself: `a` as a file and
                // `a/b` as another entry cannot both be true, and hanging
                // children off a file would make `read_dir` answer for
                // something that is not a directory. The first claim wins and
                // the rest of this name is dropped.
                if !self.children.contains_key(&path) {
                    return;
                }
                continue;
            }
            self.children
                .entry(parent)
                .or_default()
                .push(component.clone());

            let node = if position == last && !is_dir {
                Node {
                    entry: Entry {
                        // Hidden is decided from the name: an archive carries
                        // no platform flag to ask, and the leading dot is the
                        // only rule that means the same thing on both targets.
                        hidden: component.starts_with('.'),
                        name: component,
                        kind: EntryKind::File,
                        size,
                        modified,
                        attributes,
                    },
                    bytes: bytes.clone(),
                }
            } else {
                // Only a directory gets a children list, so `is_dir` can be
                // answered by asking whether it has one.
                self.children.entry(path.clone()).or_default();
                Node {
                    entry: directory_entry(component, modified),
                    bytes: None,
                }
            };
            self.nodes.insert(path.clone(), node);
        }
    }

    pub fn node(&self, path: &VfsPath) -> Option<&Node> {
        self.nodes.get(path)
    }

    /// The entries directly inside `path`, or `None` if it is not a directory
    /// this archive has.
    pub fn read_dir(&self, path: &VfsPath) -> Option<Vec<Entry>> {
        let names = self.children.get(path)?;
        Some(
            names
                .iter()
                .filter_map(|name| self.nodes.get(&path.child(name)))
                .map(|node| node.entry.clone())
                .collect(),
        )
    }
}

fn directory_entry(name: String, modified: SystemTime) -> Entry {
    Entry {
        name,
        kind: EntryKind::Dir,
        size: 0,
        modified,
        attributes: Attributes::default(),
        hidden: false,
    }
}

/// Where an archive's entry name lands, or `None` if it lands nowhere.
///
/// An archive name is written by whoever made the archive, so this is the one
/// place that decides an entry cannot point outside it. It decides by handing
/// the name to [`VfsPath::new`], which is total by construction: a leading
/// separator, `.`, and `..` beyond the root all collapse away, so `/`-rooted
/// is the only thing a `VfsPath` can be. The archive therefore **cannot
/// express** a path outside itself — a stronger claim than "every unpacker
/// remembers to check", and one made in one place rather than two.
///
/// What is left to do here is the backslash. Archives written on Windows
/// separate with it, and `VfsPath` does not: read as an ordinary character it
/// would turn `sub\file` into one entry whose *name* contains a separator,
/// which every path this program builds out of it would then get wrong.
///
/// `None` when nothing survives — an entry called `../..` is the archive's
/// own root, which is not a place to file anything, so it is not filed.
pub fn sanitise(raw_name: &str) -> Option<VfsPath> {
    let path = VfsPath::new(&raw_name.replace(WINDOWS_SEPARATOR, &SEPARATOR.to_string()));
    (!path.is_root()).then_some(path)
}
