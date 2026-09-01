//! Finding files, and saying so as they are found.

use std::sync::Arc;
use std::time::{Duration, Instant};

use fc_core::ops::CancelToken;
use fc_core::search::{spawn, Criteria, Results, CONTENT_WINDOW};
use fc_core::vfs::{LocalFs, VfsError, VfsPath, VirtualFs};
use tempfile::TempDir;

mod common;

use common::delegate_vfs;

/// How long a search of a handful of files may take before something is wrong.
const PATIENCE: Duration = Duration::from_secs(10);

/// A tree of `(relative path, contents)`; directories are made as needed.
fn tree(files: &[(&str, &str)]) -> TempDir {
    let dir = TempDir::new().unwrap();
    for (path, contents) in files {
        let full = dir.path().join(path);
        std::fs::create_dir_all(full.parent().unwrap()).unwrap();
        std::fs::write(full, contents).unwrap();
    }
    dir
}

fn search(dir: &TempDir, name: &str, content: &str) -> Results {
    spawn(
        Arc::new(LocalFs) as Arc<dyn VirtualFs>,
        VfsPath::new(dir.path().to_str().unwrap()),
        Criteria {
            name: name.to_string(),
            content: content.to_string(),
        },
        CancelToken::new(),
    )
}

/// Everything a search found, once it has finished, as names and sorted.
///
/// Sorted because the order a tree is walked in is not something a search
/// promises — only which files come out of it.
fn collect(results: &Results) -> Vec<String> {
    let deadline = Instant::now() + PATIENCE;
    let mut found = Vec::new();
    loop {
        match results.try_recv() {
            Ok(path) => found.push(path.file_name().unwrap_or_default().to_string()),
            Err(error) if error.is_closed() => break,
            Err(_) => {
                assert!(Instant::now() < deadline, "the search never finished");
                std::thread::sleep(Duration::from_millis(5));
            }
        }
    }
    found.sort();
    found
}

#[test]
fn a_name_pattern_finds_every_match_and_nothing_else() {
    let dir = tree(&[
        ("notes.txt", "x"),
        ("data.bin", "x"),
        ("deep/more.txt", "x"),
        ("deep/deeper/deepest.txt", "x"),
        ("deep/other.md", "x"),
    ]);

    let found = collect(&search(&dir, "*.txt", ""));

    assert_eq!(found, ["deepest.txt", "more.txt", "notes.txt"]);
}

#[test]
fn a_search_walks_the_whole_tree_however_deep() {
    // Through a queue rather than recursion: a directory tree is user input,
    // and a deep enough one turns recursion into a stack overflow — which is
    // a crash in a file manager, over somebody else's directory layout.
    let mut path = String::new();
    for level in 0..200 {
        path.push_str(&format!("d{level}/"));
    }
    let dir = tree(&[(&format!("{path}buried.txt"), "x")]);

    let found = collect(&search(&dir, "*.txt", ""));

    assert_eq!(found, ["buried.txt"]);
}

#[test]
fn content_search_finds_only_the_files_that_say_it() {
    let dir = tree(&[
        ("yes.txt", "the needle is here"),
        ("no.txt", "nothing of interest"),
        ("deep/also.txt", "a needle, deeper"),
    ]);

    let found = collect(&search(&dir, "*", "needle"));

    assert_eq!(found, ["also.txt", "yes.txt"]);
}

#[test]
fn a_match_across_a_window_boundary_is_still_found() {
    // The bug the overlap exists to prevent, and the one nobody notices until
    // a search quietly misses things in big files.
    let needle = "SPANNING";
    let mut contents = "x".repeat(CONTENT_WINDOW - needle.len() / 2);
    contents.push_str(needle);
    contents.push_str(&"y".repeat(1000));
    let dir = tree(&[("big.txt", &contents)]);

    let found = collect(&search(&dir, "*", needle));

    assert_eq!(found, ["big.txt"]);
}

#[test]
fn both_criteria_have_to_agree() {
    let dir = tree(&[
        ("match.txt", "needle"),
        ("wrong-name.md", "needle"),
        ("wrong-content.txt", "nothing"),
    ]);

    let found = collect(&search(&dir, "*.txt", "needle"));

    assert_eq!(found, ["match.txt"]);
}

/// A filesystem that refuses one directory, whatever the permissions say.
///
/// A `chmod 000` would be the obvious way to write the test below and does not
/// work: these tests run as root often enough, and root reads what it likes.
/// Refusing in the layer above is the same refusal from the walker's side and
/// happens the same way every time.
struct Refusing {
    inner: LocalFs,
    refuse: VfsPath,
}

impl Refusing {
    fn read_dir_impl(&self, path: &VfsPath) -> Result<Vec<fc_core::vfs::Entry>, VfsError> {
        if *path == self.refuse {
            return Err(VfsError::PermissionDenied);
        }
        // Sorted, so the walk order is decided here rather than by whatever
        // the filesystem happened to hand back. The test below turns on which
        // directory is visited first, and an unspecified order would make it
        // pass or fail by luck.
        let mut entries = self.inner.read_dir(path)?;
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(entries)
    }
    fn open_read_impl(&self, path: &VfsPath) -> Result<Box<dyn std::io::Read + Send>, VfsError> {
        self.inner.open_read(path)
    }
    fn create_file_impl(&self, path: &VfsPath) -> Result<Box<dyn std::io::Write + Send>, VfsError> {
        self.inner.create_file(path)
    }
    fn rename_impl(&self, from: &VfsPath, to: &VfsPath) -> Result<(), VfsError> {
        self.inner.rename(from, to)
    }
    fn trash_impl(&self, path: &VfsPath) -> Result<(), VfsError> {
        self.inner.trash(path)
    }
}

delegate_vfs!(Refusing);

#[test]
fn a_directory_that_cannot_be_read_does_not_stop_the_walk() {
    // Half a home directory is unreadable on any real machine, and a search
    // that stops at the first refusal is a search nobody can use.
    // `later` sorts before `locked`, and the walker takes directories off a
    // stack — so `locked` is pushed last and visited *first*, and the file in
    // `later` is only found if the refusal was stepped over rather than
    // returned from. Arranged that way on purpose: with the refusal visited
    // last, a walker that gave up on it would produce exactly the same results
    // as one that carried on, and the test would be worthless.
    let dir = tree(&[("locked/inside.txt", "x"), ("later/found.txt", "x")]);
    let root = VfsPath::new(dir.path().to_str().unwrap());
    let fs = Refusing {
        inner: LocalFs,
        refuse: root.child("locked"),
    };

    let results = spawn(
        Arc::new(fs) as Arc<dyn VirtualFs>,
        root,
        Criteria {
            name: "*.txt".to_string(),
            content: String::new(),
        },
        CancelToken::new(),
    );

    assert_eq!(collect(&results), ["found.txt"]);
}

/// A filesystem that pulls the cancel the moment a directory is listed.
///
/// Cancelling from the test is a race: the walker may not have started, and
/// then any walker at all looks cancelled. This puts the cancel exactly where
/// the check being tested is — after a directory was read, while its entries
/// are being handed out one at a time.
struct CancelsMidDirectory {
    inner: LocalFs,
    cancel: CancelToken,
}

impl CancelsMidDirectory {
    fn read_dir_impl(&self, path: &VfsPath) -> Result<Vec<fc_core::vfs::Entry>, VfsError> {
        let entries = self.inner.read_dir(path)?;
        self.cancel.cancel();
        Ok(entries)
    }
    fn open_read_impl(&self, path: &VfsPath) -> Result<Box<dyn std::io::Read + Send>, VfsError> {
        self.inner.open_read(path)
    }
    fn create_file_impl(&self, path: &VfsPath) -> Result<Box<dyn std::io::Write + Send>, VfsError> {
        self.inner.create_file(path)
    }
    fn rename_impl(&self, from: &VfsPath, to: &VfsPath) -> Result<(), VfsError> {
        self.inner.rename(from, to)
    }
    fn trash_impl(&self, path: &VfsPath) -> Result<(), VfsError> {
        self.inner.trash(path)
    }
}

delegate_vfs!(CancelsMidDirectory);

#[test]
fn cancelling_stops_it_inside_a_directory_and_not_only_between_them() {
    // The check that matters: one directory with a hundred thousand entries is
    // exactly where a search feels stuck, and a walker that only looked at the
    // token between directories would finish that one first.
    let mut files: Vec<(String, String)> = Vec::new();
    for index in 0..500 {
        files.push((format!("f{index:03}.txt"), "x".to_string()));
    }
    let borrowed: Vec<(&str, &str)> = files
        .iter()
        .map(|(path, body)| (path.as_str(), body.as_str()))
        .collect();
    let dir = tree(&borrowed);

    let cancel = CancelToken::new();
    let fs = CancelsMidDirectory {
        inner: LocalFs,
        cancel: cancel.clone(),
    };
    let results = spawn(
        Arc::new(fs) as Arc<dyn VirtualFs>,
        VfsPath::new(dir.path().to_str().unwrap()),
        Criteria {
            name: "*".to_string(),
            content: String::new(),
        },
        cancel,
    );

    assert!(
        collect(&results).is_empty(),
        "it handed out the directory it had already been told to stop for"
    );
}

#[test]
fn cancelling_stops_the_walk() {
    // Not "eventually": a search abandoned is a search over, or the slow case
    // stays unbearable however quickly the window closes.
    // One flat directory on purpose. Spread over subdirectories, a walker that
    // only checked between directories would pass this too — and the check
    // that matters is the one inside a directory, because a single one with a
    // hundred thousand entries in it is exactly where a search feels stuck.
    let mut files: Vec<(String, String)> = Vec::new();
    for index in 0..2000 {
        files.push((format!("f{index}.txt"), "x".to_string()));
    }
    let borrowed: Vec<(&str, &str)> = files
        .iter()
        .map(|(path, body)| (path.as_str(), body.as_str()))
        .collect();
    let dir = tree(&borrowed);

    let cancel = CancelToken::new();
    let results = spawn(
        Arc::new(LocalFs) as Arc<dyn VirtualFs>,
        VfsPath::new(dir.path().to_str().unwrap()),
        Criteria {
            name: "*".to_string(),
            content: String::new(),
        },
        cancel.clone(),
    );
    cancel.cancel();

    // The channel closes because the walker returned, not because it ran out
    // of tree: two thousand files would not all fit in the queue.
    let deadline = Instant::now() + PATIENCE;
    let mut seen = 0;
    loop {
        match results.try_recv() {
            Ok(_) => seen += 1,
            Err(error) if error.is_closed() => break,
            Err(_) => {
                assert!(Instant::now() < deadline, "cancelling did not stop it");
                std::thread::sleep(Duration::from_millis(5));
            }
        }
    }
    assert!(seen < 2000, "it finished the whole tree anyway ({seen})");
}

#[test]
fn an_empty_tree_finds_nothing_and_ends() {
    let dir = tree(&[]);

    assert!(collect(&search(&dir, "*", "")).is_empty());
}
