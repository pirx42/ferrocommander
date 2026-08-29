//! The archive backend: what a zip looks like as a directory, and what it
//! refuses to do.
//!
//! Most fixtures are built here with `zip`'s writer, which proves the index
//! and the decoders. It does not prove that anybody *else's* zip parses, so
//! one fixture — `fixtures/interop.zip` — was written by Info-ZIP's `zip(1)`
//! and is read as it is.

mod common;

use std::io::{Read, Write};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, UNIX_EPOCH};

use tc_core::archive::{format_for, ArchiveFs, Format};
use tc_core::vfs::{Attributes, Entry, EntryKind, LocalFs, VfsError, VfsPath, VirtualFs};
use tempfile::TempDir;

/// One entry to put in a test archive.
struct Item {
    name: String,
    contents: Vec<u8>,
    compressed: bool,
}

fn stored(name: &str, contents: &[u8]) -> Item {
    Item {
        name: name.to_string(),
        contents: contents.to_vec(),
        compressed: false,
    }
}

fn deflated(name: &str, contents: &[u8]) -> Item {
    Item {
        name: name.to_string(),
        contents: contents.to_vec(),
        compressed: true,
    }
}

/// The date every built entry carries, so a test can assert on one.
const STAMPED: (u16, u8, u8, u8, u8, u8) = (2024, 3, 4, 5, 6, 7);
/// The same moment as seconds since the epoch — 2024-03-04T05:06:**06**Z.
///
/// One second earlier than what was written, and not a rounding bug here: a
/// zip stores its seconds field halved, so odd seconds do not exist in the
/// format. Pinned as it is rather than papered over with an even second,
/// because a reader that silently invented the missing second would be
/// claiming to know something the file does not say.
const STAMPED_UNIX: u64 = 1_709_528_766;

fn zip_bytes(items: &[Item]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for item in items {
        let options: zip::write::FileOptions<'_, ()> = zip::write::FileOptions::default()
            .compression_method(if item.compressed {
                zip::CompressionMethod::Deflated
            } else {
                zip::CompressionMethod::Stored
            })
            .unix_permissions(0o644)
            .last_modified_time(
                zip::DateTime::from_date_and_time(
                    STAMPED.0, STAMPED.1, STAMPED.2, STAMPED.3, STAMPED.4, STAMPED.5,
                )
                .unwrap(),
            );
        writer.start_file(&item.name, options).unwrap();
        writer.write_all(&item.contents).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

/// How many entries the read-count tests build. Enough for the per-entry cost
/// to dominate the fixed one, small enough to stay instant.
const ENTRY_COUNT: usize = 200;
/// The budget for opening an archive: five container reads per two entries.
///
/// A deterministic bound, not a timing one. Measured at `ENTRY_COUNT`
/// entries: 403 reads buffered, 603 unbuffered — so this sits between them
/// with about a fifth of room either way, and a change in how the parse reads
/// fails loudly instead of quietly costing 30% of the opening time.
const READS_PER_ENTRIES: (usize, usize) = (5, 2);
/// How long the stored entry in the window test is: several container buffers,
/// so decoding its prefix would be visibly more than one read.
const STORED_BODY: usize = 512 * 1024;
/// What [`Counting`] calls its one file.
const CONTAINER: &str = "/a.zip";

/// A backend holding one archive in memory, counting the reads of it.
///
/// The reads are what the property tests are about, and a real file behind a
/// page cache would answer a different question — how fast the kernel is —
/// than the one being asked, which is how many times this layer asks at all.
#[derive(Clone)]
struct Counting {
    bytes: Arc<Vec<u8>>,
    reads: Arc<AtomicUsize>,
}

impl Counting {
    fn over(bytes: Vec<u8>) -> Arc<Counting> {
        Arc::new(Counting {
            bytes: Arc::new(bytes),
            reads: Arc::new(AtomicUsize::new(0)),
        })
    }

    fn reads(&self) -> usize {
        self.reads.load(Ordering::Relaxed)
    }
}

impl VirtualFs for Counting {
    fn read_at(&self, _path: &VfsPath, offset: u64, len: usize) -> Result<Vec<u8>, VfsError> {
        self.reads.fetch_add(1, Ordering::Relaxed);
        let from = (offset as usize).min(self.bytes.len());
        let to = from.saturating_add(len).min(self.bytes.len());
        Ok(self.bytes[from..to].to_vec())
    }

    fn stat(&self, path: &VfsPath) -> Result<Entry, VfsError> {
        Ok(Entry {
            name: path.file_name().unwrap_or_default().to_string(),
            kind: EntryKind::File,
            size: self.bytes.len() as u64,
            modified: UNIX_EPOCH,
            attributes: Default::default(),
            hidden: false,
        })
    }

    fn read_dir(&self, _path: &VfsPath) -> Result<Vec<Entry>, VfsError> {
        Err(VfsError::NotADirectory)
    }
    fn open_read(&self, _path: &VfsPath) -> Result<Box<dyn Read + Send>, VfsError> {
        Err(VfsError::NotFound)
    }
    fn create_dir(&self, _path: &VfsPath) -> Result<(), VfsError> {
        Err(VfsError::ReadOnly)
    }
    fn remove_dir(&self, _path: &VfsPath) -> Result<(), VfsError> {
        Err(VfsError::ReadOnly)
    }
    fn remove_file(&self, _path: &VfsPath) -> Result<(), VfsError> {
        Err(VfsError::ReadOnly)
    }
    fn rename(&self, _from: &VfsPath, _to: &VfsPath) -> Result<(), VfsError> {
        Err(VfsError::ReadOnly)
    }
    fn create_file(&self, _path: &VfsPath) -> Result<Box<dyn Write + Send>, VfsError> {
        Err(VfsError::ReadOnly)
    }
    fn set_modified(&self, _path: &VfsPath, _time: std::time::SystemTime) -> Result<(), VfsError> {
        Err(VfsError::ReadOnly)
    }
    fn set_attributes(&self, _path: &VfsPath, _attributes: Attributes) -> Result<(), VfsError> {
        Err(VfsError::ReadOnly)
    }
    fn trash(&self, _path: &VfsPath) -> Result<(), VfsError> {
        Err(VfsError::ReadOnly)
    }
}

/// Puts `bytes` in a tempdir under `name` and opens it as a filesystem.
///
/// Through `LocalFs` rather than from memory, because that is how the shell
/// will open one and because it exercises the `read_at`-based container.
fn open_bytes(bytes: Vec<u8>, name: &str) -> (TempDir, ArchiveFs) {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join(name), bytes).unwrap();
    let root = LocalFs::vfs_path(dir.path());
    let fs: Arc<dyn VirtualFs> = Arc::new(LocalFs);
    let archive = ArchiveFs::open(fs, &root.child(name)).unwrap();
    (dir, archive)
}

fn open(items: &[Item]) -> (TempDir, ArchiveFs) {
    open_bytes(zip_bytes(items), "test.zip")
}

fn names(fs: &ArchiveFs, at: &str) -> Vec<String> {
    let mut found: Vec<String> = fs
        .read_dir(&VfsPath::new(at))
        .unwrap()
        .into_iter()
        .map(|entry| entry.name)
        .collect();
    found.sort();
    found
}

fn read(fs: &ArchiveFs, path: &str) -> Result<Vec<u8>, VfsError> {
    let mut bytes = Vec::new();
    fs.open_read(&VfsPath::new(path))?
        .read_to_end(&mut bytes)
        .map_err(VfsError::from)?;
    Ok(bytes)
}

#[test]
fn a_zip_written_by_another_program_reads_as_a_directory() {
    // The one fixture this crate did not write. Everything else here proves
    // the index against the writer it was built with, which is a claim about
    // this crate and not about the format.
    let bytes = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/interop.zip"),
    )
    .unwrap();
    let (_dir, archive) = open_bytes(bytes, "interop.zip");

    assert_eq!(names(&archive, "/"), ["empty", "notes.txt", "sub"]);
    assert_eq!(names(&archive, "/sub"), ["inner.txt"]);
    assert!(names(&archive, "/empty").is_empty());
    assert_eq!(
        read(&archive, "/notes.txt").unwrap(),
        b"hello from a real zip\n"
    );
    assert_eq!(read(&archive, "/sub/inner.txt").unwrap(), b"deep\n");
}

#[test]
fn a_directory_the_archive_never_names_still_appears() {
    // A zip need not carry an entry for a directory whose files it holds. An
    // index that only filed what it was told would show an empty archive.
    let (_dir, archive) = open(&[deflated("a/b/c.txt", b"nested")]);

    assert_eq!(names(&archive, "/"), ["a"]);
    assert_eq!(names(&archive, "/a"), ["b"]);
    assert_eq!(names(&archive, "/a/b"), ["c.txt"]);
    assert_eq!(read(&archive, "/a/b/c.txt").unwrap(), b"nested");
}

#[test]
fn a_name_that_climbs_out_of_the_archive_is_filed_inside_it() {
    // The zip-slip failure: an entry called `../../etc/passwd` unpacked by a
    // program that trusts its names writes outside the destination. Sanitising
    // at index time means the archive cannot even express such a path — a
    // stronger claim than every unpacker remembering to check.
    let (_dir, archive) = open(&[
        stored("../../etc/passwd", b"escaped"),
        stored("/absolute.txt", b"rooted"),
        stored("windows\\made.txt", b"backslashed"),
    ]);

    assert_eq!(names(&archive, "/"), ["absolute.txt", "etc", "windows"]);
    assert_eq!(names(&archive, "/etc"), ["passwd"]);
    assert_eq!(names(&archive, "/windows"), ["made.txt"]);
    // And nothing anywhere is called `..`, at any depth.
    assert!(
        !every_path(&archive).iter().any(|path| path.contains("..")),
        "a climbing component survived: {:?}",
        every_path(&archive)
    );
}

#[test]
fn an_entry_whose_name_is_only_climbing_is_not_filed_at_all() {
    // Rather than under some fallback name, which would put a file in the
    // listing that the archive never described.
    let (_dir, archive) = open(&[stored("../..", b"nothing"), stored("kept.txt", b"kept")]);

    assert_eq!(names(&archive, "/"), ["kept.txt"]);
}

#[test]
fn every_window_of_an_entry_reads_the_same_bytes_the_stream_does() {
    // `read_at` and `open_read` are two different code paths — one decodes
    // from the entry's start and skips, the other streams — and the viewer
    // uses the first while a copy uses the second. They have to agree byte for
    // byte or a file looks different depending on how it was opened.
    let body: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz-the-quick-brown-fox";
    for item in [stored("f", body), deflated("f", body)] {
        let compressed = item.compressed;
        let (_dir, archive) = open(&[item]);
        let whole = read(&archive, "/f").unwrap();
        assert_eq!(whole, body, "compressed: {compressed}");

        for offset in 0..=body.len() as u64 {
            for len in [0usize, 1, 7, body.len()] {
                let window = archive.read_at(&VfsPath::new("/f"), offset, len).unwrap();
                let from = (offset as usize).min(body.len());
                let to = (from + len).min(body.len());
                assert_eq!(
                    window,
                    &body[from..to],
                    "compressed: {compressed}, offset {offset}, len {len}"
                );
            }
        }
    }
}

#[test]
fn a_read_entirely_past_the_end_comes_back_empty() {
    // What the trait promises of every backend, and what the viewer relies on
    // when it pages to the end.
    let (_dir, archive) = open(&[deflated("f", b"short")]);
    let window = archive.read_at(&VfsPath::new("/f"), 9_000, 64).unwrap();
    assert!(window.is_empty());
}

#[test]
fn every_call_that_would_write_reports_that_it_cannot() {
    // One variant for the whole backend rather than a different error per
    // method: whether an archive can be written to is a property of the
    // archive, and the engine should be able to say so once.
    let (_dir, archive) = open(&[stored("f", b"x")]);
    let path = VfsPath::new("/f");
    let attempts: Vec<(&str, VfsError)> = vec![
        ("create_dir", archive.create_dir(&path).unwrap_err()),
        ("remove_dir", archive.remove_dir(&path).unwrap_err()),
        ("remove_file", archive.remove_file(&path).unwrap_err()),
        ("rename", archive.rename(&path, &path).unwrap_err()),
        (
            "create_file",
            archive.create_file(&path).err().expect("a refusal"),
        ),
        (
            "set_modified",
            archive.set_modified(&path, UNIX_EPOCH).unwrap_err(),
        ),
        (
            "set_attributes",
            archive
                .set_attributes(&path, Default::default())
                .unwrap_err(),
        ),
        ("trash", archive.trash(&path).unwrap_err()),
    ];
    for (call, error) in attempts {
        assert_eq!(error, VfsError::ReadOnly, "{call} did something else");
    }
}

#[test]
fn an_entry_that_does_not_match_its_checksum_fails_rather_than_returning_bytes() {
    // The failure this project's reliability requirement is actually about: a
    // copy that reports success over a file it got wrong. A stored entry is
    // used so the corrupted byte lands in the data rather than in a
    // compressed stream, where it would fail as a decode error instead and
    // prove nothing about the checksum.
    let mut bytes = zip_bytes(&[stored("f", b"the original bytes")]);
    let at = find(&bytes, b"the original bytes").expect("the stored data");
    bytes[at] = b'X';
    let (_dir, archive) = open_bytes(bytes, "test.zip");

    let error = read(&archive, "/f").unwrap_err();
    assert!(
        error.to_string().contains("checksum"),
        "read reported {error} instead of a checksum failure"
    );
}

#[test]
fn an_entry_compressed_by_a_method_this_build_cannot_read_says_which() {
    // An archive from the wild may use bzip2, zstd or lzma. The entry stays in
    // the listing — it is in the archive, and hiding it would be a listing of
    // something else — and the failure arrives when somebody reads it.
    const BZIP2: u16 = 12;
    let mut bytes = zip_bytes(&[stored("f", b"pretend this is bzip2")]);
    set_compression_method(&mut bytes, BZIP2);
    let (_dir, archive) = open_bytes(bytes, "test.zip");

    assert_eq!(names(&archive, "/"), ["f"]);
    let error = read(&archive, "/f").unwrap_err();
    assert!(
        error.to_string().contains("not supported"),
        "read reported {error} instead of an unsupported method"
    );
}

#[test]
fn an_entry_keeps_the_date_the_archive_recorded() {
    let (_dir, archive) = open(&[stored("f", b"x")]);
    let entry = archive.stat(&VfsPath::new("/f")).unwrap();
    assert_eq!(
        entry.modified,
        UNIX_EPOCH + Duration::from_secs(STAMPED_UNIX)
    );
}

#[test]
fn a_directory_the_archive_never_named_takes_the_date_of_what_implied_it() {
    // Something rather than 1970, which in half the rows of a listing reads as
    // a bug in the program rather than as an absence in the file. The entry
    // that implied the directory is the closest thing the archive has to a
    // date for it.
    let (_dir, archive) = open(&[stored("a/b.txt", b"x")]);

    let synthesised = archive.stat(&VfsPath::new("/a")).unwrap();
    assert_eq!(
        synthesised.modified,
        UNIX_EPOCH + Duration::from_secs(STAMPED_UNIX)
    );
}

#[test]
fn the_root_of_an_archive_is_dated_from_the_archive_file() {
    // Nothing inside implies the root, so it falls back to the container —
    // which is the only date an archive has for itself.
    let (dir, archive) = open(&[stored("a/b.txt", b"x")]);
    let container = std::fs::metadata(dir.path().join("test.zip"))
        .unwrap()
        .modified()
        .unwrap();

    assert_eq!(archive.stat(&VfsPath::root()).unwrap().modified, container);
}

#[test]
fn a_container_that_is_not_an_archive_reports_rather_than_panicking() {
    // Every byte of an archive is somebody else's input, opening included.
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("not.zip"), vec![0xABu8; 4096]).unwrap();
    let root = LocalFs::vfs_path(dir.path());

    let error = opening_fails(&root.child("not.zip"));
    assert!(matches!(error, VfsError::Io(_)), "reported {error}");
}

#[test]
fn a_truncated_archive_reports_rather_than_panicking() {
    let mut bytes = zip_bytes(&[deflated("a.txt", b"something long enough to matter")]);
    bytes.truncate(bytes.len() / 2);
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("cut.zip"), bytes).unwrap();
    let root = LocalFs::vfs_path(dir.path());

    assert!(ArchiveFs::open(Arc::new(LocalFs), &root.child("cut.zip")).is_err());
}

#[test]
fn a_file_that_is_not_an_archive_by_name_is_not_opened_as_one() {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("notes.txt"),
        zip_bytes(&[stored("f", b"x")]),
    )
    .unwrap();
    let root = LocalFs::vfs_path(dir.path());

    assert_eq!(
        opening_fails(&root.child("notes.txt")),
        VfsError::NotAnArchive
    );
}

#[test]
fn the_extension_decides_which_format_and_case_does_not_matter() {
    for name in ["a.zip", "a.ZIP", "a.Zip", "some.name.zip"] {
        assert_eq!(format_for(name), Some(Format::Zip), "{name}");
    }
    for name in ["a.txt", "zip", "a.zipper", ""] {
        assert_eq!(format_for(name), None, "{name}");
    }
}

#[test]
fn listing_a_file_and_reading_a_directory_report_what_they_are() {
    let (_dir, archive) = open(&[stored("a/b.txt", b"x")]);

    assert_eq!(
        archive.read_dir(&VfsPath::new("/a/b.txt")).unwrap_err(),
        VfsError::NotADirectory
    );
    assert_eq!(
        archive.read_dir(&VfsPath::new("/nowhere")).unwrap_err(),
        VfsError::NotFound
    );
    assert_eq!(read(&archive, "/a").unwrap_err(), VfsError::IsADirectory);
}

#[test]
fn the_root_of_an_archive_is_a_directory() {
    // The listing layer stats the directory it is about to show, so an
    // archive whose root did not exist would open as an error.
    let (_dir, archive) = open(&[stored("f", b"x")]);
    assert!(archive.stat(&VfsPath::root()).unwrap().is_dir());
}

#[test]
fn opening_an_archive_does_not_read_the_container_once_per_entry() {
    // The parse seeks constantly — the end-of-directory record, then every
    // entry's local header — and each of those is a `read_at` on whatever
    // holds the archive. Buffering collapses them; unbuffered, opening a
    // 10 000-entry zip took 63 ms instead of 49 (docs/performance.md).
    //
    // Pinned as a count rather than as a time, because a timing test is a
    // flake and "fewer reads than entries" is not.
    let items: Vec<Item> = (0..ENTRY_COUNT)
        .map(|index| stored(&format!("file{index}"), b"y"))
        .collect();
    let counted = Counting::over(zip_bytes(&items));

    let archive = ArchiveFs::open(counted.clone(), &VfsPath::new(CONTAINER)).unwrap();
    assert_eq!(
        archive.read_dir(&VfsPath::root()).unwrap().len(),
        ENTRY_COUNT
    );

    let reads = counted.reads();
    let (allowed, per) = READS_PER_ENTRIES;
    assert!(
        reads * per < ENTRY_COUNT * allowed,
        "opening {ENTRY_COUNT} entries took {reads} reads of the container"
    );
}

#[test]
fn browsing_an_open_archive_does_not_read_the_container_at_all() {
    // Why walking around inside an archive is instant: the index is complete
    // when the archive opens, so a listing is a map lookup. An implementation
    // that went back to the container per directory would be re-parsing it
    // once per keystroke.
    let items: Vec<Item> = (0..ENTRY_COUNT)
        .map(|index| stored(&format!("dir{}/file{index}", index % 10), b"y"))
        .collect();
    let counted = Counting::over(zip_bytes(&items));
    let archive = ArchiveFs::open(counted.clone(), &VfsPath::new(CONTAINER)).unwrap();

    let opened = counted.reads();
    for index in 0..10 {
        let at = VfsPath::new(&format!("/dir{index}"));
        assert_eq!(archive.read_dir(&at).unwrap().len(), ENTRY_COUNT / 10);
        archive.stat(&at).unwrap();
    }
    assert_eq!(
        counted.reads(),
        opened,
        "listing an already-open archive went back to the container"
    );
}

#[test]
fn a_window_of_a_stored_entry_does_not_read_everything_before_it() {
    // The one case where a window of an entry is a window of the file. A
    // compressed entry has to be decoded from its start — the honest cost of a
    // format with no seek — but paying it for an entry that is not compressed
    // would make a jpeg inside a zip page like a compressed one.
    let counted = Counting::over(zip_bytes(&[stored("big", &vec![b'z'; STORED_BODY])]));
    let archive = ArchiveFs::open(counted.clone(), &VfsPath::new(CONTAINER)).unwrap();

    let before = counted.reads();
    archive
        .read_at(&VfsPath::new("/big"), (STORED_BODY - 16) as u64, 16)
        .unwrap();
    assert_eq!(
        counted.reads() - before,
        1,
        "a window near the end of a stored entry read the whole entry"
    );
}

/// The error from opening something that will not open.
///
/// `ArchiveFs` is not `Debug` — it holds an index of somebody's whole archive,
/// and printing that on a failed assertion helps nobody — so `unwrap_err` is
/// not available and the `Ok` side is discarded by hand.
fn opening_fails(path: &VfsPath) -> VfsError {
    match ArchiveFs::open(Arc::new(LocalFs), path) {
        Ok(_) => panic!("{path:?} opened as an archive"),
        Err(error) => error,
    }
}

/// Every path in the archive, for an assertion about all of them at once.
fn every_path(fs: &ArchiveFs) -> Vec<String> {
    let mut found = Vec::new();
    let mut queue = vec![VfsPath::root()];
    while let Some(at) = queue.pop() {
        for entry in fs.read_dir(&at).unwrap_or_default() {
            let path = at.child(&entry.name);
            found.push(path.as_str().to_string());
            if entry.is_dir() {
                queue.push(path);
            }
        }
    }
    found
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// Rewrites the compression method in both places a zip records it: the local
/// file header and the central directory record.
///
/// Hand-patched because this build cannot *write* bzip2 — the whole point of
/// the test is an archive made by something that can.
fn set_compression_method(bytes: &mut [u8], method: u16) {
    const LOCAL_HEADER: &[u8] = b"PK\x03\x04";
    const CENTRAL_RECORD: &[u8] = b"PK\x01\x02";
    /// Offset of the method field inside each record.
    const IN_LOCAL: usize = 8;
    const IN_CENTRAL: usize = 10;

    for (signature, offset) in [(LOCAL_HEADER, IN_LOCAL), (CENTRAL_RECORD, IN_CENTRAL)] {
        let at = find(bytes, signature).expect("a zip record") + offset;
        bytes[at..at + 2].copy_from_slice(&method.to_le_bytes());
    }
}
