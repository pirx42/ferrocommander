//! Looking inside a file without holding it.
//!
//! The viewer never reads a file. It holds an **offset**, asks for a window
//! around it, and moves the offset — which is the only way a four-gigabyte
//! file opens instantly (`docs/performance.md`). A viewer that read a file to
//! show it could not open a disk image, and one that read the first megabyte
//! and stopped would be lying about what is in the file.
//!
//! In `tc-core` because none of it is a window: where the offset goes, what
//! the bytes say, and how a hex dump is laid out are arithmetic and are tested
//! as such.

use crate::vfs::{VfsError, VfsPath, VirtualFs};

/// How much is read for one screenful.
///
/// Comfortably more than fits on a screen, so paging is one read rather than
/// several, and small enough that reading it is never felt.
pub const WINDOW_BYTES: usize = 64 * 1024;

/// Bytes on one line of a hex dump. Sixteen is what every hex dump uses and
/// what a person's eye expects to count in.
pub const HEX_COLUMNS: usize = 16;

/// Shown in the text column of a hex dump for a byte that has no printable
/// form of its own.
pub const HEX_UNPRINTABLE: char = '.';

/// What a byte below this is: a control character, not text.
const FIRST_PRINTABLE: u8 = 0x20;
/// And above this, in Latin-1, the printable range resumes.
const LAST_ASCII_PRINTABLE: u8 = 0x7e;

/// How the bytes are shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    Text,
    Hex,
}

/// One file, and where in it the viewer is looking.
pub struct View {
    path: VfsPath,
    size: u64,
    offset: u64,
    mode: Mode,
}

impl View {
    /// Opens a view on a file, reading nothing but its size.
    pub fn open(fs: &dyn VirtualFs, path: VfsPath) -> Result<View, VfsError> {
        let size = fs.stat(&path)?.size;
        Ok(View {
            path,
            size,
            offset: 0,
            mode: Mode::default(),
        })
    }

    pub fn path(&self) -> &VfsPath {
        &self.path
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn offset(&self) -> u64 {
        self.offset
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
    }

    /// What to show now.
    ///
    /// In text mode the window starts just after a newline, so a page never
    /// opens mid-line — except at the very beginning, where there is nothing
    /// to snap to.
    pub fn render(&self, fs: &dyn VirtualFs) -> Result<String, VfsError> {
        let bytes = fs.read_at(&self.path, self.offset, WINDOW_BYTES)?;
        Ok(match self.mode {
            Mode::Text => decode(&bytes),
            Mode::Hex => hex_dump(self.offset, &bytes),
        })
    }

    /// Moves `lines` lines, forwards or back.
    ///
    /// By reading, not by an index: an index over four gigabytes is the thing
    /// being avoided. Forwards is a scan of the window from the offset;
    /// backwards is a scan of the window *before* it, which is why moving up
    /// through a file of very long lines costs the same as moving down.
    pub fn scroll_lines(&mut self, fs: &dyn VirtualFs, lines: i64) {
        for _ in 0..lines.unsigned_abs() {
            let moved = if lines > 0 {
                self.next_line(fs)
            } else {
                self.previous_line(fs)
            };
            if !moved {
                return;
            }
        }
    }

    /// Moves a windowful, forwards or back.
    pub fn scroll_window(&mut self, pages: i64) {
        let step = WINDOW_BYTES as i64 * pages;
        self.offset = self
            .offset
            .saturating_add_signed(step)
            .min(self.last_page());
    }

    pub fn to_start(&mut self) {
        self.offset = 0;
    }

    pub fn to_end(&mut self) {
        self.offset = self.last_page();
    }

    /// The furthest the offset may go: far enough that the last window still
    /// has something in it.
    fn last_page(&self) -> u64 {
        self.size.saturating_sub(WINDOW_BYTES as u64 / 2)
    }

    /// Forward to just after the next newline, if there is one in reach.
    fn next_line(&mut self, fs: &dyn VirtualFs) -> bool {
        let Ok(window) = fs.read_at(&self.path, self.offset, WINDOW_BYTES) else {
            return false;
        };
        match window.iter().position(|&byte| byte == b'\n') {
            Some(at) => {
                self.offset += at as u64 + 1;
                true
            }
            // No newline in a whole window: one long line, and moving by a
            // window is the only sense "a line down" can be given.
            None if !window.is_empty() => {
                self.offset += window.len() as u64;
                true
            }
            None => false,
        }
    }

    /// Back to just after the newline before the one this offset follows.
    fn previous_line(&mut self, fs: &dyn VirtualFs) -> bool {
        if self.offset == 0 {
            return false;
        }
        let back = WINDOW_BYTES.min(self.offset as usize);
        let start = self.offset - back as u64;
        let Ok(window) = fs.read_at(&self.path, start, back) else {
            return false;
        };
        // The window ends with the newline this offset is just past; the line
        // before it starts after the newline before that.
        let before = window.len().saturating_sub(1);
        self.offset = match window[..before].iter().rposition(|&byte| byte == b'\n') {
            Some(at) => start + at as u64 + 1,
            None => start,
        };
        true
    }
}

/// Bytes as text.
///
/// Valid UTF-8 is UTF-8: the sequences that make it valid do not happen by
/// accident, so the test *is* the detection. Anything else is read as Latin-1,
/// which maps every byte to some character and so cannot fail — the honest
/// answer for a file whose encoding nobody recorded.
///
/// UTF-16 comes out as mojibake, which is documented rather than pretended
/// away: telling it from Latin-1 needs a real detector, and that is a
/// dependency and a later decision (`docs/viewer.md`).
pub fn decode(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(text) => text.to_string(),
        Err(_) => bytes.iter().map(|&byte| byte as char).collect(),
    }
}

/// The classic three columns: offset, bytes, and what they say.
pub fn hex_dump(offset: u64, bytes: &[u8]) -> String {
    let mut dump = String::new();
    for (row, chunk) in bytes.chunks(HEX_COLUMNS).enumerate() {
        let at = offset + (row * HEX_COLUMNS) as u64;
        dump.push_str(&format!("{at:08x}  "));
        for column in 0..HEX_COLUMNS {
            match chunk.get(column) {
                Some(byte) => dump.push_str(&format!("{byte:02x} ")),
                // Padded, so the text column of a short last row still lines
                // up with every row above it.
                None => dump.push_str("   "),
            }
        }
        dump.push(' ');
        for &byte in chunk {
            dump.push(printable(byte));
        }
        dump.push('\n');
    }
    dump
}

/// A byte as one character, or a dot where it has no printable form.
fn printable(byte: u8) -> char {
    if (FIRST_PRINTABLE..=LAST_ASCII_PRINTABLE).contains(&byte) {
        return byte as char;
    }
    HEX_UNPRINTABLE
}
