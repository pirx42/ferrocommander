//! The archive backend: what a zip looks like as a directory, and what it
//! refuses to do.
//!
//! Most fixtures are built here with `zip`'s writer, which proves the index
//! and the decoders. It does not prove that anybody *else's* zip parses, so
//! one fixture — `fixtures/interop.zip` — was written by Info-ZIP's `zip(1)`
//! and is read as it is.

mod common;

use common::delegate_vfs;

use std::io::{Read, Write};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, UNIX_EPOCH};

use tc_core::archive::{format_for, ArchiveFs, Format};
use tc_core::ops::{
    self, Answer, CancelToken, Conflict, ConflictResolver, DeleteMode, Destination, Job, Report,
    Resolution, Silent,
};
use tc_core::vfs::{Attributes, Entry, EntryKind, LocalFs, Store, VfsError, VfsPath, VirtualFs};
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
    fn store(&self) -> Store {
        Store::LOCAL
    }

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

/// The same items as a tar. Compression is the container's business here, so
/// `Item::compressed` is ignored.
fn tar_bytes(items: &[Item]) -> Vec<u8> {
    let mut builder = tar::Builder::new(Vec::new());
    for item in items {
        let mut header = tar::Header::new_gnu();
        header.set_size(item.contents.len() as u64);
        header.set_mode(0o644);
        header.set_mtime(STAMPED_UNIX);
        header.set_cksum();
        builder
            .append_data(&mut header, &item.name, item.contents.as_slice())
            .unwrap();
    }
    builder.into_inner().unwrap()
}

fn gzip(bytes: Vec<u8>) -> Vec<u8> {
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(&bytes).unwrap();
    encoder.finish().unwrap()
}

/// One of the archives written by another program, from `tests/fixtures/`.
fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name),
    )
    .unwrap()
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
    let (_dir, archive) = open_bytes(fixture("interop.zip"), "interop.zip");

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
fn a_tar_reads_as_a_directory_the_same_way_a_zip_does() {
    // A different format behind the same interface: the assertions are the
    // ones the zip tests make, because that is the whole claim.
    let (_dir, archive) = open_bytes(tar_bytes(&[stored("a/b/c.txt", b"nested")]), "t.tar");

    assert_eq!(names(&archive, "/"), ["a"]);
    assert_eq!(names(&archive, "/a/b"), ["c.txt"]);
    assert_eq!(read(&archive, "/a/b/c.txt").unwrap(), b"nested");
}

#[test]
fn a_tar_gz_written_by_another_program_reads_as_a_directory() {
    // GNU tar's output, gzipped, symlink and all. Its entries are named `./x`,
    // which is a shape this crate's own writer never produces.
    let (_dir, archive) = open_bytes(fixture("interop.tar.gz"), "interop.tar.gz");

    assert_eq!(names(&archive, "/"), ["empty", "notes.txt", "sub"]);
    assert_eq!(
        read(&archive, "/notes.txt").unwrap(),
        b"hello from a real zip\n"
    );
    assert_eq!(read(&archive, "/sub/inner.txt").unwrap(), b"deep\n");
}

#[test]
fn a_symlink_in_a_tar_is_passed_over_rather_than_unpacked_as_an_empty_file() {
    // Its recorded size is zero, so listing it as a file would unpack it as an
    // empty one — a copy that quietly got it wrong, which is the failure this
    // project's reliability requirement is about. The fixture holds
    // `link.txt -> notes.txt`.
    let (_dir, archive) = open_bytes(fixture("interop.tar.gz"), "interop.tar.gz");

    assert!(
        !names(&archive, "/").contains(&"link.txt".to_string()),
        "the symlink was listed: {:?}",
        names(&archive, "/")
    );
}

#[test]
fn every_window_of_a_gzipped_entry_reads_the_same_bytes_the_stream_does() {
    // The wrapper is the second thing between a window and the container, and
    // the offsets it works in are the decompressed ones. Getting that wrong
    // reads the neighbouring entry, which is a copy of the wrong file.
    let body: &[u8] = b"the second entry, whose offset is not zero";
    let (_dir, archive) = open_bytes(
        gzip(tar_bytes(&[
            stored("first", b"padding padding"),
            stored("second", body),
        ])),
        "t.tar.gz",
    );

    assert_eq!(read(&archive, "/second").unwrap(), body);
    for offset in [0u64, 1, 17, body.len() as u64] {
        let window = archive
            .read_at(&VfsPath::new("/second"), offset, 8)
            .unwrap();
        let from = (offset as usize).min(body.len());
        let to = (from + 8).min(body.len());
        assert_eq!(window, &body[from..to], "offset {offset}");
    }
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
    let cases = [
        ("a.zip", Some(Format::Zip)),
        ("a.ZIP", Some(Format::Zip)),
        ("some.name.zip", Some(Format::Zip)),
        ("a.tar", Some(Format::Tar)),
        ("a.TAR", Some(Format::Tar)),
        ("a.tgz", Some(Format::TarGz)),
        // Before the single extension, or this reads as a gzip of something
        // unknown rather than as the tar it is.
        ("a.tar.gz", Some(Format::TarGz)),
        ("A.Tar.Gz", Some(Format::TarGz)),
        ("a.txt", None),
        ("a.gz", None),
        ("zip", None),
        ("a.zipper", None),
        ("", None),
    ];
    for (name, expected) in cases {
        assert_eq!(format_for(name), expected, "{name}");
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

#[test]
fn unpacking_an_archive_conserves_every_file_byte_and_name() {
    // The pack-and-unpack roundtrip, with the copy engine doing the unpacking
    // and no idea that it is: `ops::run` reads one backend and writes another,
    // which is the whole reason it takes two.
    let dir = TempDir::new().unwrap();
    let tree = dir.path().join("tree");
    common::build_tree(&tree);
    let into = dir.path().join("into");
    std::fs::create_dir(&into).unwrap();
    std::fs::write(dir.path().join("t.zip"), zip_of_tree(&tree)).unwrap();

    let archive = open_container(dir.path(), "t.zip");
    unpack(&archive, &LocalFs::vfs_path(&into));

    assert_eq!(
        common::snapshot(&LocalFs, &LocalFs::vfs_path(&into)),
        common::snapshot(&LocalFs, &LocalFs::vfs_path(&tree)),
        "what came out is not what went in"
    );
}

#[test]
fn moving_out_of_an_archive_takes_nothing_out_of_it() {
    // A move is a copy and then a delete, and an archive cannot be deleted
    // from. What must not happen is the copy succeeding and the entry
    // vanishing anyway, or the failure being swallowed so the user believes
    // the archive was emptied.
    let dir = TempDir::new().unwrap();
    let tree = dir.path().join("tree");
    common::build_tree(&tree);
    let into = dir.path().join("into");
    std::fs::create_dir(&into).unwrap();
    std::fs::write(dir.path().join("t.zip"), zip_of_tree(&tree)).unwrap();

    let archive = open_container(dir.path(), "t.zip");
    let before = common::snapshot(&archive, &VfsPath::root());
    let report = transfer(&archive, &LocalFs::vfs_path(&into), true);

    assert_eq!(
        common::snapshot(&archive, &VfsPath::root()),
        before,
        "the archive lost something"
    );
    assert_eq!(
        common::snapshot(&LocalFs, &LocalFs::vfs_path(&into)),
        common::snapshot(&LocalFs, &LocalFs::vfs_path(&tree)),
        "the copy half of the move did not happen"
    );
    assert!(
        report
            .failures
            .iter()
            .any(|(_, error)| *error == VfsError::ReadOnly),
        "the archive silently accepted a delete: {report:?}"
    );
}

#[test]
fn a_move_between_two_stores_never_renames() {
    // The failure this guards against is not slowness. A move's fast path is
    // one `rename`, and running it across two backends hands the *source's*
    // path to the *target* — `/a.txt` inside an archive naming a file at the
    // root of the disk. The paths look alike and address different files, so
    // the shortcut has to be skipped rather than merely fail.
    let dir = TempDir::new().unwrap();
    let tree = dir.path().join("tree");
    common::build_tree(&tree);
    let into = dir.path().join("into");
    std::fs::create_dir(&into).unwrap();
    std::fs::write(dir.path().join("t.zip"), zip_of_tree(&tree)).unwrap();

    let archive = open_container(dir.path(), "t.zip");
    let watched = Renames::over(LocalFs);
    let sources = top_level(&archive);
    ops::run(
        &Job::Move {
            sources,
            destination: Destination::Into(LocalFs::vfs_path(&into)),
        },
        &archive,
        &watched,
        &mut Refuse,
        &mut Silent,
        &CancelToken::new(),
    );

    assert_eq!(
        watched.attempts(),
        0,
        "a path from the archive was handed to the local filesystem"
    );
}

#[test]
fn a_move_within_one_store_still_takes_the_shortcut() {
    // The control for the test above: the guard must not have turned the fast
    // path off for the case it exists for.
    let dir = TempDir::new().unwrap();
    common::build_tree(&dir.path().join("tree"));
    let into = dir.path().join("into");
    std::fs::create_dir(&into).unwrap();
    let watched = Renames::over(LocalFs);

    ops::run(
        &Job::Move {
            sources: vec![LocalFs::vfs_path(&dir.path().join("tree"))],
            destination: Destination::Into(LocalFs::vfs_path(&into)),
        },
        &watched,
        &watched,
        &mut Refuse,
        &mut Silent,
        &CancelToken::new(),
    );

    assert_eq!(watched.attempts(), 1, "the whole tree was copied instead");
}

#[test]
fn unpacking_into_a_root_is_not_mistaken_for_a_copy_onto_itself() {
    // The engine refuses a copy whose target is its own source. Between two
    // stores that rule has nothing to decide: `/notes.txt` in an archive and
    // `/notes.txt` on the destination are two different files that happen to
    // be spelled the same, and applying the rule anyway would refuse an
    // ordinary unpack into the root of somewhere.
    let dir = TempDir::new().unwrap();
    let tree = dir.path().join("tree");
    std::fs::create_dir(&tree).unwrap();
    std::fs::write(tree.join("notes.txt"), "packed").unwrap();
    let into = dir.path().join("into");
    std::fs::create_dir(&into).unwrap();
    std::fs::write(dir.path().join("t.zip"), zip_of_tree(&tree)).unwrap();

    let archive = open_container(dir.path(), "t.zip");
    let target = common::Rooted::over(LocalFs::vfs_path(&into));
    let report = ops::run(
        &Job::Copy {
            sources: top_level(&archive),
            // The destination *is* the source's own path, spelled the same.
            destination: Destination::Into(VfsPath::root()),
        },
        &archive,
        &target,
        &mut Refuse,
        &mut Silent,
        &CancelToken::new(),
    );

    assert!(report.is_clean(), "the unpack was refused: {report:?}");
    assert_eq!(
        std::fs::read_to_string(into.join("notes.txt")).unwrap(),
        "packed"
    );
}

#[test]
fn packing_then_unpacking_gives_back_the_same_tree() {
    // The invariant this phase exists to keep. Not "the names look right":
    // the same files, the same bytes, the same directories, including an
    // empty one, an empty file and a non-ASCII name — for every format this
    // build can write.
    for name in ["out.zip", "out.tar", "out.tar.gz"] {
        let dir = TempDir::new().unwrap();
        let tree = dir.path().join("tree");
        common::build_tree(&tree);
        let into = dir.path().join("into");
        std::fs::create_dir(&into).unwrap();

        pack(dir.path(), &[LocalFs::vfs_path(&tree)], name);
        let archive = open_container(dir.path(), name);
        unpack(&archive, &LocalFs::vfs_path(&into));

        assert_eq!(
            common::snapshot(&LocalFs, &LocalFs::vfs_path(&into.join("tree"))),
            common::snapshot(&LocalFs, &LocalFs::vfs_path(&tree)),
            "{name} did not give back what it was given"
        );
    }
}

#[test]
fn a_packed_file_keeps_its_date_and_its_executable_bit() {
    // An executable that comes back without its `+x` is a broken copy, which
    // makes this a reliability property rather than a nicety — and the date is
    // what a backup is sorted by.
    for name in ["out.zip", "out.tar"] {
        let dir = TempDir::new().unwrap();
        let tree = dir.path().join("tree");
        std::fs::create_dir(&tree).unwrap();
        let script = tree.join("run.sh");
        std::fs::write(&script, "#!/bin/sh\necho hello\n").unwrap();
        let stamp = std::time::UNIX_EPOCH + std::time::Duration::from_secs(STAMPED_UNIX);
        LocalFs
            .set_modified(&LocalFs::vfs_path(&script), stamp)
            .unwrap();
        LocalFs
            .set_attributes(
                &LocalFs::vfs_path(&script),
                tc_core::vfs::attributes_from_unix_mode(0o755),
            )
            .unwrap();

        pack(dir.path(), &[LocalFs::vfs_path(&script)], name);
        let archive = open_container(dir.path(), name);
        let packed = archive.stat(&VfsPath::new("/run.sh")).unwrap();

        assert_eq!(packed.modified, stamp, "{name} lost the date");
        assert_eq!(
            tc_core::vfs::render_attributes(packed.attributes),
            tc_core::vfs::render_attributes(tc_core::vfs::attributes_from_unix_mode(0o755)),
            "{name} lost the mode"
        );
    }
}

#[test]
fn a_pack_that_is_cancelled_leaves_no_archive_at_all() {
    // Not an empty one, and not a half-written one under the name somebody
    // will later open: an archive that exists is an archive that finished.
    let dir = TempDir::new().unwrap();
    let tree = dir.path().join("tree");
    common::build_tree(&tree);
    let cancel = CancelToken::new();
    cancel.cancel();

    ops::run(
        &Job::Pack {
            sources: vec![LocalFs::vfs_path(&tree)],
            archive: LocalFs::vfs_path(dir.path()).child("out.zip"),
        },
        &LocalFs,
        &LocalFs,
        &mut Refuse,
        &mut Silent,
        &cancel,
    );

    let left: Vec<String> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(left, ["tree"], "a cancelled pack left something behind");
}

#[test]
fn packing_into_a_name_that_is_not_an_archive_is_refused() {
    // One rule decides what opens as an archive and what packs into one, so a
    // name this writes is a name that opens again.
    let dir = TempDir::new().unwrap();
    let tree = dir.path().join("tree");
    common::build_tree(&tree);

    let report = ops::run(
        &Job::Pack {
            sources: vec![LocalFs::vfs_path(&tree)],
            archive: LocalFs::vfs_path(dir.path()).child("out.rar"),
        },
        &LocalFs,
        &LocalFs,
        &mut Refuse,
        &mut Silent,
        &CancelToken::new(),
    );

    assert!(
        report
            .failures
            .iter()
            .any(|(_, error)| *error == VfsError::NotAnArchive),
        "packing into a .rar was allowed: {report:?}"
    );
    assert!(!dir.path().join("out.rar").exists());
}

#[test]
fn packing_a_second_time_asks_before_replacing() {
    // The archive is a destination like any other, so an existing one asks
    // the question every other destination asks rather than being overwritten
    // where a user cannot see it.
    let dir = TempDir::new().unwrap();
    let tree = dir.path().join("tree");
    common::build_tree(&tree);
    let archive = LocalFs::vfs_path(dir.path()).child("out.zip");
    std::fs::write(dir.path().join("out.zip"), b"not really an archive").unwrap();

    let mut asked = Asked::new(Answer::once(Resolution::Skip));
    ops::run(
        &Job::Pack {
            sources: vec![LocalFs::vfs_path(&tree)],
            archive,
        },
        &LocalFs,
        &LocalFs,
        &mut asked,
        &mut Silent,
        &CancelToken::new(),
    );

    assert_eq!(asked.questions, 1, "nothing was asked");
    assert_eq!(
        std::fs::read(dir.path().join("out.zip")).unwrap(),
        b"not really an archive",
        "the answer was ignored"
    );
}

/// Packs `sources` into `name` beside them, and insists it went cleanly.
fn pack(directory: &std::path::Path, sources: &[VfsPath], name: &str) {
    let report = ops::run(
        &Job::Pack {
            sources: sources.to_vec(),
            archive: LocalFs::vfs_path(directory).child(name),
        },
        &LocalFs,
        &LocalFs,
        &mut Refuse,
        &mut Silent,
        &CancelToken::new(),
    );
    assert!(
        report.is_clean(),
        "packing {name} was not clean: {report:?}"
    );
}

/// A resolver that answers once and counts the questions.
struct Asked {
    answer: Answer,
    questions: usize,
}

impl Asked {
    fn new(answer: Answer) -> Asked {
        Asked {
            answer,
            questions: 0,
        }
    }
}

impl ConflictResolver for Asked {
    fn resolve(&mut self, _conflict: &Conflict) -> Answer {
        self.questions += 1;
        self.answer
    }
}

#[test]
fn a_job_that_would_write_into_an_archive_is_refused_once() {
    // Once, not once per file. A copy of a large tree into an archive would
    // otherwise be a failure list with a thousand identical lines, which is
    // the same as no failure list at all.
    let dir = TempDir::new().unwrap();
    let tree = dir.path().join("tree");
    common::build_tree(&tree);
    std::fs::write(dir.path().join("t.zip"), zip_of_tree(&tree)).unwrap();
    let archive = open_container(dir.path(), "t.zip");

    let report = ops::run(
        &Job::Copy {
            sources: vec![LocalFs::vfs_path(&tree)],
            destination: Destination::Into(VfsPath::root()),
        },
        &LocalFs,
        &archive,
        &mut Refuse,
        &mut Silent,
        &CancelToken::new(),
    );

    assert_eq!(report.failures.len(), 1, "not one refusal: {report:?}");
    assert_eq!(report.failures[0].1, VfsError::ReadOnly);
}

#[test]
fn creating_and_deleting_inside_an_archive_are_refused_too() {
    // Every job that writes, not only the transfers. A delete asks the
    // *source* backend, because that is the one it removes from.
    let dir = TempDir::new().unwrap();
    let tree = dir.path().join("tree");
    common::build_tree(&tree);
    std::fs::write(dir.path().join("t.zip"), zip_of_tree(&tree)).unwrap();
    let archive = open_container(dir.path(), "t.zip");

    let jobs = [
        Job::CreateDir {
            path: VfsPath::new("/made"),
        },
        Job::CreateFile {
            path: VfsPath::new("/made.txt"),
        },
        Job::Delete {
            paths: vec![VfsPath::new("/a.txt")],
            mode: DeleteMode::Permanent,
        },
    ];
    for job in jobs {
        let report = ops::run(
            &job,
            &archive,
            &archive,
            &mut Refuse,
            &mut Silent,
            &CancelToken::new(),
        );
        assert_eq!(
            report.failures.iter().map(|(_, e)| e).collect::<Vec<_>>(),
            [&VfsError::ReadOnly],
            "{job:?} was not refused"
        );
    }
    // And the archive still says what it always said.
    assert!(archive.stat(&VfsPath::new("/a.txt")).is_ok());
}

/// The top-level entries of an archive, as the paths a job takes.
fn top_level(archive: &ArchiveFs) -> Vec<VfsPath> {
    archive
        .read_dir(&VfsPath::root())
        .unwrap()
        .into_iter()
        .map(|entry| VfsPath::root().child(&entry.name))
        .collect()
}

/// Copies everything in `archive` into `into`, and insists it went cleanly.
fn unpack(archive: &ArchiveFs, into: &VfsPath) {
    let report = transfer(archive, into, false);
    assert!(
        report.is_clean(),
        "the unpack did not run cleanly: {report:?}"
    );
}

fn transfer(archive: &ArchiveFs, into: &VfsPath, moving: bool) -> Report {
    let sources = top_level(archive);
    let destination = Destination::Into(into.clone());
    let job = if moving {
        Job::Move {
            sources,
            destination,
        }
    } else {
        Job::Copy {
            sources,
            destination,
        }
    };
    ops::run(
        &job,
        archive,
        &LocalFs,
        &mut Refuse,
        &mut Silent,
        &CancelToken::new(),
    )
}

/// A resolver for jobs that must not meet a conflict at all.
struct Refuse;

impl ConflictResolver for Refuse {
    fn resolve(&mut self, conflict: &Conflict) -> Answer {
        panic!("unexpected conflict: {conflict:?}");
    }
}

/// A `LocalFs` that counts the renames attempted through it.
struct Renames {
    inner: LocalFs,
    attempts: AtomicUsize,
}

impl Renames {
    fn over(inner: LocalFs) -> Renames {
        Renames {
            inner,
            attempts: AtomicUsize::new(0),
        }
    }

    fn attempts(&self) -> usize {
        self.attempts.load(Ordering::Relaxed)
    }

    fn rename_impl(&self, from: &VfsPath, to: &VfsPath) -> Result<(), VfsError> {
        self.attempts.fetch_add(1, Ordering::Relaxed);
        self.inner.rename(from, to)
    }

    fn read_dir_impl(&self, path: &VfsPath) -> Result<Vec<Entry>, VfsError> {
        self.inner.read_dir(path)
    }

    fn open_read_impl(&self, path: &VfsPath) -> Result<Box<dyn Read + Send>, VfsError> {
        self.inner.open_read(path)
    }

    fn create_file_impl(&self, path: &VfsPath) -> Result<Box<dyn Write + Send>, VfsError> {
        self.inner.create_file(path)
    }

    fn trash_impl(&self, path: &VfsPath) -> Result<(), VfsError> {
        self.inner.trash(path)
    }
}

delegate_vfs!(Renames);

/// A zip of everything below `root`, written by the `zip` crate rather than by
/// this project — which has no packer yet, and which is the point: the tree
/// that comes out has to be the tree that went in whoever put it there.
fn zip_of_tree(root: &std::path::Path) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options: zip::write::FileOptions<'_, ()> = zip::write::FileOptions::default();
    let mut queue = vec![(root.to_path_buf(), String::new())];
    while let Some((directory, prefix)) = queue.pop() {
        for entry in std::fs::read_dir(&directory).unwrap() {
            let entry = entry.unwrap();
            let name = format!("{prefix}{}", entry.file_name().to_string_lossy());
            if entry.file_type().unwrap().is_dir() {
                writer.add_directory(&name, options).unwrap();
                queue.push((entry.path(), format!("{name}/")));
            } else {
                writer.start_file(&name, options).unwrap();
                writer
                    .write_all(&std::fs::read(entry.path()).unwrap())
                    .unwrap();
            }
        }
    }
    writer.finish().unwrap().into_inner()
}

/// Opens `name` in `directory` as an archive.
fn open_container(directory: &std::path::Path, name: &str) -> ArchiveFs {
    ArchiveFs::open(Arc::new(LocalFs), &LocalFs::vfs_path(directory).child(name)).unwrap()
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
