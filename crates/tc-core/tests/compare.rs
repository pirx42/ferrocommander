//! Compare-by-content over a real filesystem: the routing between the row
//! view and the byte verdict, which the unit tests inside the module cannot
//! reach because it starts at `stat` and `read_at`.

use std::fs;

use tc_core::compare::{compare, Comparison, Verdict};
use tc_core::vfs::LocalFs;
use tempfile::TempDir;

fn written(dir: &TempDir, name: &str, bytes: &[u8]) -> tc_core::vfs::VfsPath {
    let path = dir.path().join(name);
    fs::write(&path, bytes).unwrap();
    LocalFs::vfs_path(&path)
}

#[test]
fn two_text_files_come_back_as_rows() {
    let dir = TempDir::new().unwrap();
    let left = written(&dir, "a.txt", b"one\ntwo\n");
    let right = written(&dir, "b.txt", b"one\nthree\n");
    match compare(&LocalFs, &left, &LocalFs, &right).unwrap() {
        Comparison::Rows(rows) => assert_eq!(rows.len(), 2),
        Comparison::Verdict(_) => panic!("text files answered with a byte verdict"),
    }
}

#[test]
fn a_nul_byte_routes_to_the_verdict_and_names_the_first_difference() {
    let dir = TempDir::new().unwrap();
    // The NUL sits past the difference, so a verdict that stopped sniffing
    // where the files diverge would still be wrong to call this text.
    let left = written(&dir, "a.bin", b"same-then-A\0tail");
    let right = written(&dir, "b.bin", b"same-then-B\0tail");
    match compare(&LocalFs, &left, &LocalFs, &right).unwrap() {
        Comparison::Verdict(verdict) => assert_eq!(
            verdict,
            Verdict::Differ {
                first_difference: 10
            }
        ),
        Comparison::Rows(_) => panic!("a NUL byte was diffed as text"),
    }
}

#[test]
fn identical_binaries_are_identical() {
    let dir = TempDir::new().unwrap();
    let bytes = [b"x\0y".as_slice(), &[7u8; 4096]].concat();
    let left = written(&dir, "a.bin", &bytes);
    let right = written(&dir, "b.bin", &bytes);
    match compare(&LocalFs, &left, &LocalFs, &right).unwrap() {
        Comparison::Verdict(verdict) => assert_eq!(verdict, Verdict::Identical),
        Comparison::Rows(_) => panic!("a NUL byte was diffed as text"),
    }
}

#[test]
fn a_prefix_differs_where_the_shorter_file_ends() {
    let dir = TempDir::new().unwrap();
    let left = written(&dir, "a.bin", b"\0abc");
    let right = written(&dir, "b.bin", b"\0abcdef");
    match compare(&LocalFs, &left, &LocalFs, &right).unwrap() {
        Comparison::Verdict(verdict) => assert_eq!(
            verdict,
            Verdict::Differ {
                first_difference: 4
            }
        ),
        Comparison::Rows(_) => panic!("a NUL byte was diffed as text"),
    }
}

#[test]
fn text_over_the_ceiling_gets_a_verdict_rather_than_a_stall() {
    let dir = TempDir::new().unwrap();
    // Pure text, one byte over the ceiling: only the size can be what
    // routes this to the verdict. Comparing the file against itself keeps
    // the test about the routing, not about a difference.
    let big = "a".repeat(tc_core::compare::CEILING_BYTES as usize + 1);
    let left = written(&dir, "a.txt", big.as_bytes());
    let right = written(&dir, "b.txt", big.as_bytes());
    match compare(&LocalFs, &left, &LocalFs, &right).unwrap() {
        Comparison::Verdict(verdict) => assert_eq!(verdict, Verdict::Identical),
        Comparison::Rows(_) => panic!("an oversize file was read whole for a line diff"),
    }
}
