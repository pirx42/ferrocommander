//! Turning a job's sources into a flat list of things to do, with totals.
//!
//! Scanning before executing costs one directory walk and buys three things:
//! an honest total for the progress bar, a first cheap place for a cancel to
//! take effect, and an execution loop that never has to ask "what is this
//! path" again.

use std::time::SystemTime;

use crate::vfs::{Entry, EntryKind, SymlinkTarget, VfsError, VfsPath, VirtualFs};

/// One step of a job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Task {
    /// Create a directory at the destination before its contents arrive.
    MakeDir { source: VfsPath, target: VfsPath },
    CopyFile {
        source: VfsPath,
        target: VfsPath,
        size: u64,
        /// Stamped onto the copy, so a copied file keeps its date.
        modified: SystemTime,
    },
    /// Remove a file, or a symlink of any kind — never followed.
    RemoveFile { path: VfsPath },
    /// Remove a directory that the tasks before it have already emptied.
    RemoveDir { path: VfsPath },
    /// Hand a whole path to the platform's trash in one call.
    Trash { path: VfsPath },
}

/// Everything one top-level source turned into.
///
/// Grouping by source rather than flattening straight away is what lets a
/// move try a single `rename` for a whole tree and, when that works, skip the
/// group's tasks while still counting its bytes as done.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub source: VfsPath,
    pub target: VfsPath,
    pub source_is_dir: bool,
    pub tasks: Vec<Task>,
    /// Paths inside this item the scan could not turn into a task — an
    /// unreadable subdirectory, a symlink that cannot be recreated.
    ///
    /// Collected rather than returned, because one bad entry deep inside a
    /// tree must not cost the whole tree. The executor reports them and the
    /// rest of the item still runs; a move treats the item as incomplete and
    /// leaves the source alone.
    pub failures: Vec<(VfsPath, VfsError)>,
    pub files: usize,
    pub bytes: u64,
}

/// A whole job, ready to execute.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Plan {
    pub items: Vec<Item>,
    /// Paths the scan could not read. Reported as failures rather than
    /// aborting: one unreadable subdirectory must not cost the other ten.
    pub failures: Vec<(VfsPath, VfsError)>,
    pub total_files: usize,
    pub total_bytes: u64,
}

impl Plan {
    fn push(&mut self, item: Item) {
        self.total_files += item.files;
        self.total_bytes += item.bytes;
        self.items.push(item);
    }
}

/// Plans copying or moving `sources` to the destinations `destination` picks.
pub fn plan_transfer(
    fs: &dyn VirtualFs,
    sources: &[VfsPath],
    destination: impl Fn(&VfsPath) -> VfsPath,
) -> Plan {
    let mut plan = Plan::default();
    for source in sources {
        let target = destination(source);
        match scan_transfer(fs, source, &target) {
            Ok(item) => plan.push(item),
            Err(error) => plan.failures.push((source.clone(), error)),
        }
    }
    plan
}

/// Plans removing `paths`, either into the trash or for good.
pub fn plan_removal(fs: &dyn VirtualFs, paths: &[VfsPath], to_trash: bool) -> Plan {
    let mut plan = Plan::default();
    for path in paths {
        let scanned = if to_trash {
            // The trash takes a whole tree in one call, so there is nothing
            // to walk and nothing a cancel could interrupt half way.
            fs.stat(path).map(|entry| Item {
                source: path.clone(),
                target: path.clone(),
                source_is_dir: entry.is_dir(),
                tasks: vec![Task::Trash { path: path.clone() }],
                failures: Vec::new(),
                files: 1,
                bytes: entry.size,
            })
        } else {
            scan_removal(fs, path)
        };
        match scanned {
            Ok(item) => plan.push(item),
            Err(error) => plan.failures.push((path.clone(), error)),
        }
    }
    plan
}

/// Walks one source into copy tasks, parents before children.
fn scan_transfer(fs: &dyn VirtualFs, source: &VfsPath, target: &VfsPath) -> Result<Item, VfsError> {
    let entry = fs.stat(source)?;
    let mut item = Item {
        source: source.clone(),
        target: target.clone(),
        source_is_dir: entry.kind == EntryKind::Dir,
        tasks: Vec::new(),
        failures: Vec::new(),
        files: 0,
        bytes: 0,
    };
    collect_transfer(fs, source, target, &entry, &mut item);
    Ok(item)
}

fn collect_transfer(
    fs: &dyn VirtualFs,
    source: &VfsPath,
    target: &VfsPath,
    entry: &Entry,
    item: &mut Item,
) {
    match entry.kind {
        EntryKind::Dir => {
            item.tasks.push(Task::MakeDir {
                source: source.clone(),
                target: target.clone(),
            });
            let children = match fs.read_dir(source) {
                Ok(children) => children,
                Err(error) => {
                    // The directory itself is still created; only what is
                    // inside it is lost, and the rest of the tree is not.
                    item.failures.push((source.clone(), error));
                    return;
                }
            };
            for child in children {
                let child_source = source.child(&child.name);
                let child_target = target.child(&child.name);
                collect_transfer(fs, &child_source, &child_target, &child, item);
            }
        }
        // A link to a file is copied as the file it points at, which is what
        // reading it gives. A link to a directory is not descended into: the
        // copy would follow it, which loops forever on a cycle, and the
        // interface has no way to recreate the link instead.
        EntryKind::File | EntryKind::Symlink(SymlinkTarget::File) => {
            item.files += 1;
            item.bytes += entry.size;
            item.tasks.push(Task::CopyFile {
                source: source.clone(),
                target: target.clone(),
                size: entry.size,
                modified: entry.modified,
            });
        }
        EntryKind::Symlink(SymlinkTarget::Dir | SymlinkTarget::Broken) => item.failures.push((
            source.clone(),
            VfsError::Io(super::constants::SYMLINK_NOT_COPIED.to_string()),
        )),
    }
}

/// Walks one path into removal tasks, children before parents.
///
/// A symlink is removed as a file whatever it points at. Descending into one
/// would delete the *target's* contents, which is not what deleting a link
/// means and would reach outside the tree the user selected.
fn scan_removal(fs: &dyn VirtualFs, path: &VfsPath) -> Result<Item, VfsError> {
    let entry = fs.stat(path)?;
    let mut item = Item {
        source: path.clone(),
        target: path.clone(),
        source_is_dir: entry.kind == EntryKind::Dir,
        tasks: Vec::new(),
        failures: Vec::new(),
        files: 0,
        bytes: 0,
    };
    collect_removal(fs, path, &entry, &mut item);
    Ok(item)
}

fn collect_removal(fs: &dyn VirtualFs, path: &VfsPath, entry: &Entry, item: &mut Item) {
    if entry.kind == EntryKind::Dir {
        match fs.read_dir(path) {
            Ok(children) => {
                for child in children {
                    let child_path = path.child(&child.name);
                    collect_removal(fs, &child_path, &child, item);
                }
                item.tasks.push(Task::RemoveDir { path: path.clone() });
            }
            // Without knowing what is inside, the directory cannot be
            // emptied, so removing it would fail anyway. Say so here.
            Err(error) => item.failures.push((path.clone(), error)),
        }
        return;
    }
    item.files += 1;
    item.bytes += entry.size;
    item.tasks.push(Task::RemoveFile { path: path.clone() });
}
