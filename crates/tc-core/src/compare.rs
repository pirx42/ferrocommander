//! Compare by content: two files in, paired rows out.
//!
//! The engine answers one of two ways. Text files inside the ceiling become
//! **rows** — each row `Same`, `Changed`, or one-sided — with the differing
//! character spans marked inside every changed pair, which is everything a
//! side-by-side view needs and nothing GTK. Binary or oversize files get a
//! **verdict** instead: identical, or the offset of the first differing
//! byte, from a streaming comparison that never holds either file whole.
//!
//! The split is where the viewer's never-read-the-file rule
//! (`docs/viewer.md`) meets reality: a line diff needs both files in
//! memory, so the ceiling below is the honest boundary of that need.

use std::ops::Range;

use similar::{capture_diff_slices, Algorithm, DiffOp};

use crate::vfs::{VfsError, VfsPath, VirtualFs};

/// The most that is read whole for a line diff, per file.
///
/// Above it the answer degrades to a verdict rather than to a stall: two
/// files this size diff in seconds at worst (`docs/performance.md` carries
/// the measured numbers), and a bigger pair is compared byte-wise in
/// constant memory instead.
pub const CEILING_BYTES: u64 = 64 * 1024 * 1024;

/// How much of a file's head is sniffed for the NUL that marks it binary.
///
/// The viewer's encoding sniff makes the same wager over the same span: a
/// text file with a NUL in its first pages is rarer than a binary without
/// one.
const SNIFF_BYTES: usize = 8 * 1024;

/// Read size for the streaming byte comparison.
const CHUNK_BYTES: usize = 1024 * 1024;

/// What comparing two files produced.
pub enum Comparison {
    /// Line-diffed rows, ready to render side by side.
    Rows(Vec<Row>),
    /// The files were binary or over the ceiling; only bytes were compared.
    Verdict(Verdict),
}

/// The byte-wise answer for files the row view will not take.
#[derive(Debug, PartialEq, Eq)]
pub enum Verdict {
    Identical,
    /// The offset of the first byte where the two disagree. When one file
    /// is a prefix of the other, that is the shorter one's length: the
    /// first position where one file has a byte and the other has ended.
    Differ {
        first_difference: u64,
    },
}

/// One line of the side-by-side view.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum RowKind {
    Same,
    Changed,
    LeftOnly,
    RightOnly,
}

/// A row: its kind, and the text each side shows.
///
/// `LeftOnly` rows have no right side and `RightOnly` no left; the widget
/// renders the absent side as an empty filler line, which is what keeps the
/// two columns aligned.
#[derive(Debug, PartialEq, Eq)]
pub struct Row {
    pub kind: RowKind,
    pub left: Option<Side>,
    pub right: Option<Side>,
}

/// One side of a row: the line's text, and the byte ranges of it that
/// differ from the other side.
///
/// The ranges are ascending, non-overlapping, aligned to `char` boundaries
/// by construction — they come from a diff over `char`s, mapped back
/// through `char_indices` — and empty everywhere except inside a `Changed`
/// pair. Char-aligned matters: a range cut through a UTF-8 sequence is a
/// panic waiting in whatever text buffer receives it.
#[derive(Debug, PartialEq, Eq)]
pub struct Side {
    pub text: String,
    pub changed: Vec<Range<usize>>,
}

impl Side {
    fn plain(text: &str) -> Side {
        Side {
            text: text.to_string(),
            changed: Vec::new(),
        }
    }
}

/// Compares two files, each through its own filesystem.
///
/// Two filesystems rather than one, because the panes may not share one: a
/// file inside an archive compares against a file on disk exactly this way.
pub fn compare(
    left_fs: &dyn VirtualFs,
    left: &VfsPath,
    right_fs: &dyn VirtualFs,
    right: &VfsPath,
) -> Result<Comparison, VfsError> {
    let left_size = left_fs.stat(left)?.size;
    let right_size = right_fs.stat(right)?.size;

    let oversize = left_size > CEILING_BYTES || right_size > CEILING_BYTES;
    if oversize || sniffs_binary(left_fs, left)? || sniffs_binary(right_fs, right)? {
        let verdict = byte_verdict(left_fs, left, left_size, right_fs, right, right_size)?;
        return Ok(Comparison::Verdict(verdict));
    }

    let left_text = read_whole(left_fs, left, left_size)?;
    let right_text = read_whole(right_fs, right, right_size)?;
    Ok(Comparison::Rows(rows(&left_text, &right_text)))
}

/// [`compare`] on a worker thread, answered through a channel.
///
/// The row path reads both files whole — up to twice the ceiling — and the
/// prime directive does not let the main loop wait on that
/// (`docs/performance.md`). Threads and channels live in this crate rather
/// than the shell, per the standing rule: the shell only awaits.
pub fn spawn(
    left_fs: std::sync::Arc<dyn VirtualFs>,
    left: VfsPath,
    right_fs: std::sync::Arc<dyn VirtualFs>,
    right: VfsPath,
) -> async_channel::Receiver<Result<Comparison, VfsError>> {
    let (sender, receiver) = async_channel::bounded(1);
    std::thread::spawn(move || {
        let _ = sender.send_blocking(compare(left_fs.as_ref(), &left, right_fs.as_ref(), &right));
    });
    receiver
}

fn sniffs_binary(fs: &dyn VirtualFs, path: &VfsPath) -> Result<bool, VfsError> {
    Ok(fs.read_at(path, 0, SNIFF_BYTES)?.contains(&0))
}

fn read_whole(fs: &dyn VirtualFs, path: &VfsPath, size: u64) -> Result<String, VfsError> {
    let bytes = fs.read_at(path, 0, size as usize)?;
    // Lossy on purpose: the sniff has already routed real binaries to the
    // verdict path, so what reaches here is text in some encoding, and a
    // replacement character in a stray byte beats refusing to diff the
    // file at all. The rows exist to be *shown*, and GTK takes only UTF-8.
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn byte_verdict(
    left_fs: &dyn VirtualFs,
    left: &VfsPath,
    left_size: u64,
    right_fs: &dyn VirtualFs,
    right: &VfsPath,
    right_size: u64,
) -> Result<Verdict, VfsError> {
    let shared = left_size.min(right_size);
    let mut offset = 0u64;
    while offset < shared {
        let want = CHUNK_BYTES.min((shared - offset) as usize);
        let a = left_fs.read_at(left, offset, want)?;
        let b = right_fs.read_at(right, offset, want)?;
        // Compare only what both reads returned: a filesystem may hand back
        // less than asked near its own boundaries, and a short read is not
        // a difference.
        let got = a.len().min(b.len());
        if let Some(at) = (0..got).find(|&i| a[i] != b[i]) {
            return Ok(Verdict::Differ {
                first_difference: offset + at as u64,
            });
        }
        if got == 0 {
            // Neither side yields bytes below the size it reported: stop
            // rather than loop forever on a file that shrank mid-read.
            break;
        }
        offset += got as u64;
    }
    match left_size == right_size {
        true => Ok(Verdict::Identical),
        false => Ok(Verdict::Differ {
            first_difference: shared,
        }),
    }
}

/// Line-diffs two texts into paired rows. Pure, and the part the tests pin.
pub fn rows(left_text: &str, right_text: &str) -> Vec<Row> {
    let left_lines: Vec<&str> = left_text.lines().collect();
    let right_lines: Vec<&str> = right_text.lines().collect();
    // Over `Vec<&str>` of our own rather than similar's line tokenizer, so
    // every index in the ops is an index into these two vectors and nothing
    // has to agree about trailing newlines.
    let ops = capture_diff_slices(Algorithm::Myers, &left_lines, &right_lines);

    let mut out = Vec::new();
    for op in ops {
        match op {
            DiffOp::Equal {
                old_index,
                new_index,
                len,
            } => {
                for i in 0..len {
                    out.push(Row {
                        kind: RowKind::Same,
                        left: Some(Side::plain(left_lines[old_index + i])),
                        right: Some(Side::plain(right_lines[new_index + i])),
                    });
                }
            }
            DiffOp::Delete {
                old_index, old_len, ..
            } => {
                for i in 0..old_len {
                    out.push(left_only(left_lines[old_index + i]));
                }
            }
            DiffOp::Insert {
                new_index, new_len, ..
            } => {
                for i in 0..new_len {
                    out.push(right_only(right_lines[new_index + i]));
                }
            }
            DiffOp::Replace {
                old_index,
                old_len,
                new_index,
                new_len,
            } => {
                // Pair the replaced lines positionally and mark what
                // differs inside each pair; whatever one side has over
                // is its own one-sided rows. Positional is the simple
                // answer a `Replace` op has already earned — the line
                // diff put these lines opposite each other.
                let paired = old_len.min(new_len);
                for i in 0..paired {
                    let (left, right) =
                        changed_pair(left_lines[old_index + i], right_lines[new_index + i]);
                    out.push(Row {
                        kind: RowKind::Changed,
                        left: Some(left),
                        right: Some(right),
                    });
                }
                for i in paired..old_len {
                    out.push(left_only(left_lines[old_index + i]));
                }
                for i in paired..new_len {
                    out.push(right_only(right_lines[new_index + i]));
                }
            }
        }
    }
    out
}

fn left_only(text: &str) -> Row {
    Row {
        kind: RowKind::LeftOnly,
        left: Some(Side::plain(text)),
        right: None,
    }
}

fn right_only(text: &str) -> Row {
    Row {
        kind: RowKind::RightOnly,
        left: None,
        right: Some(Side::plain(text)),
    }
}

/// Char-diffs one changed pair into its two sides with spans.
///
/// A second, finer pass over just this pair (`docs/compare.md`): the
/// char-level diff is quadratic-ish in line length, and running it only on
/// lines the line diff already put opposite each other is what keeps the
/// refinement off the hot path.
fn changed_pair(left_text: &str, right_text: &str) -> (Side, Side) {
    let left_chars: Vec<char> = left_text.chars().collect();
    let right_chars: Vec<char> = right_text.chars().collect();
    let ops = capture_diff_slices(Algorithm::Myers, &left_chars, &right_chars);

    let mut left_spans = Vec::new();
    let mut right_spans = Vec::new();
    for op in ops {
        match op {
            DiffOp::Equal { .. } => {}
            DiffOp::Delete {
                old_index, old_len, ..
            } => push_span(&mut left_spans, byte_range(left_text, old_index, old_len)),
            DiffOp::Insert {
                new_index, new_len, ..
            } => push_span(&mut right_spans, byte_range(right_text, new_index, new_len)),
            DiffOp::Replace {
                old_index,
                old_len,
                new_index,
                new_len,
            } => {
                push_span(&mut left_spans, byte_range(left_text, old_index, old_len));
                push_span(&mut right_spans, byte_range(right_text, new_index, new_len));
            }
        }
    }
    (
        Side {
            text: left_text.to_string(),
            changed: left_spans,
        },
        Side {
            text: right_text.to_string(),
            changed: right_spans,
        },
    )
}

/// A char range mapped back to the byte range a text buffer wants.
fn byte_range(text: &str, char_index: usize, char_len: usize) -> Range<usize> {
    let mut indices = text.char_indices().map(|(byte, _)| byte);
    let start = indices.nth(char_index).unwrap_or(text.len());
    let end = match char_len {
        0 => start,
        _ => text
            .char_indices()
            .map(|(byte, _)| byte)
            .nth(char_index + char_len)
            .unwrap_or(text.len()),
    };
    start..end
}

/// Appends a span, merging it into the previous one when they touch — two
/// adjacent ops (a `Delete` against an `Insert`, say) otherwise leave a
/// seam in what reads as one changed run.
fn push_span(spans: &mut Vec<Range<usize>>, span: Range<usize>) {
    if span.is_empty() {
        return;
    }
    if let Some(last) = spans.last_mut() {
        if last.end == span.start {
            last.end = span.end;
            return;
        }
    }
    spans.push(span);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Rebuilds one side's text from the rows, skipping the rows where that
    /// side is absent.
    fn reconstruct(rows: &[Row], side: fn(&Row) -> &Option<Side>) -> Vec<String> {
        rows.iter()
            .filter_map(|row| side(row).as_ref().map(|s| s.text.clone()))
            .collect()
    }

    fn with_spans_removed(side: &Side) -> String {
        let mut text = String::new();
        let mut next = 0;
        for span in &side.changed {
            text.push_str(&side.text[next..span.start]);
            next = span.end;
        }
        text.push_str(&side.text[next..]);
        text
    }

    // The conservation invariant (skill 52): a diff that loses or invents a
    // line would survive any number of looks-right assertions, and cannot
    // survive this one.
    #[test]
    fn each_side_reconstructs_its_own_file_exactly() {
        let left = "shared\nleft only\nchanged here\ntail\n";
        let right = "shared\nchanged HERE\nright only\ntail\n";
        let rows = rows(left, right);
        assert_eq!(
            reconstruct(&rows, |r| &r.left),
            left.lines().collect::<Vec<_>>()
        );
        assert_eq!(
            reconstruct(&rows, |r| &r.right),
            right.lines().collect::<Vec<_>>()
        );
    }

    // The same invariant one level down: the changed spans of a pair are
    // exactly what keeps the two texts from being equal, so removing them
    // from both must leave equal strings.
    #[test]
    fn a_changed_pairs_texts_agree_once_their_spans_are_removed() {
        let rows = rows("the quick brown fox\n", "the slow brown ox\n");
        let row = &rows[0];
        assert_eq!(row.kind, RowKind::Changed);
        let (left, right) = (row.left.as_ref().unwrap(), row.right.as_ref().unwrap());
        assert!(!left.changed.is_empty());
        assert_eq!(with_spans_removed(left), with_spans_removed(right));
    }

    #[test]
    fn identical_texts_are_all_same_rows_with_no_spans() {
        let text = "one\ntwo\nthree\n";
        let rows = rows(text, text);
        assert_eq!(rows.len(), 3);
        assert!(rows.iter().all(|r| r.kind == RowKind::Same));
        assert!(rows
            .iter()
            .all(|r| r.left.as_ref().unwrap().changed.is_empty()));
    }

    #[test]
    fn an_added_and_a_removed_line_become_one_sided_rows() {
        let rows = rows("a\nb\n", "b\nc\n");
        let kinds: Vec<RowKind> = rows.iter().map(|r| r.kind).collect();
        assert_eq!(
            kinds,
            [RowKind::LeftOnly, RowKind::Same, RowKind::RightOnly]
        );
        assert!(rows[0].right.is_none());
        assert!(rows[2].left.is_none());
    }

    // The UTF-8 hazard named in the plan: a span cut through a multi-byte
    // sequence panics in whatever buffer receives it. Every boundary this
    // input offers is multi-byte, so a span off by a byte cannot slice it.
    #[test]
    fn spans_land_on_char_boundaries_in_multibyte_text() {
        let rows = rows("ein grünes Fass\n", "ein grönes Faß\n");
        let row = &rows[0];
        assert_eq!(row.kind, RowKind::Changed);
        for side in [row.left.as_ref().unwrap(), row.right.as_ref().unwrap()] {
            for span in &side.changed {
                assert!(side.text.is_char_boundary(span.start));
                assert!(side.text.is_char_boundary(span.end));
                // And the span selects the difference, not an empty spot.
                assert!(span.start < span.end);
            }
        }
        assert_eq!(
            with_spans_removed(row.left.as_ref().unwrap()),
            with_spans_removed(row.right.as_ref().unwrap())
        );
    }

    #[test]
    fn spans_are_ascending_and_disjoint() {
        let rows = rows("a1b2c3d\n", "aXbYcZd\n");
        let left = rows[0].left.as_ref().unwrap();
        for pair in left.changed.windows(2) {
            assert!(
                pair[0].end < pair[1].start,
                "merged or out of order: {pair:?}"
            );
        }
    }

    #[test]
    fn a_replace_of_unequal_size_pairs_what_it_can_and_singles_the_rest() {
        let rows = rows("x\ny\n", "p\nq\nr\n");
        let kinds: Vec<RowKind> = rows.iter().map(|r| r.kind).collect();
        assert_eq!(
            kinds,
            [RowKind::Changed, RowKind::Changed, RowKind::RightOnly]
        );
    }
}
