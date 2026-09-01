//! The multi-rename rules: names in, names out, and nothing else.
//!
//! Pure on purpose. Renaming a hundred files by a rule nobody could check
//! first is exactly the operation the reliability requirement exists for
//! (`docs/reliability.md`), so the preview the user reads and the rename that
//! runs are **the same function**. A preview computed differently from the
//! thing it previews is worse than no preview at all.

use crate::listing::split_name;

/// What to build each new name out of.
#[derive(Debug, Clone)]
pub struct Rules {
    /// The name template, with placeholders — see [`preview`].
    pub template: String,
    /// Replaced everywhere it appears in the result. Empty does nothing.
    pub find: String,
    pub replace: String,
    /// What `[C]` counts from.
    pub counter_start: u64,
}

impl Default for Rules {
    fn default() -> Self {
        Rules {
            // The name and its extension, unchanged: a template that renames
            // nothing until the user says otherwise.
            template: format!("{NAME}.{EXT}"),
            find: String::new(),
            replace: String::new(),
            counter_start: 1,
        }
    }
}

/// The name without its extension.
pub const NAME: &str = "[N]";
/// The extension, without the dot.
pub const EXT: &str = "[E]";
/// A number, counting from [`Rules::counter_start`].
pub const COUNTER: &str = "[C]";

/// One row of the preview: what a file is called, and what it would be called.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Renamed {
    pub from: String,
    pub to: String,
    /// Why this row would not be renamed, if it would not be.
    pub refused: Option<Refusal>,
}

impl Renamed {
    /// Whether this row is a rename that would actually happen.
    pub fn changes_anything(&self) -> bool {
        self.refused.is_none() && self.from != self.to
    }
}

/// Why a row cannot be renamed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// The rules produced nothing at all. A file with no name is not a file.
    Empty,
    /// Another row in this same batch wants the name too.
    Collides,
    /// A name with a path separator in it is a move, and this tool renames.
    HasSeparator,
}

/// Applies `rules` to `names`, in order.
///
/// The counter follows the order given, which is the order the pane shows —
/// so `[C]` numbers files the way they are on screen, which is the only
/// numbering anybody can predict.
///
/// **Every input gets a row**, refused ones included. A preview that silently
/// dropped the files it would not touch would be a preview of something other
/// than what is about to happen.
pub fn preview(rules: &Rules, names: &[String]) -> Vec<Renamed> {
    let mut rows: Vec<Renamed> = Vec::with_capacity(names.len());
    for (index, from) in names.iter().enumerate() {
        let (stem, extension) = split_name(from);
        let counter = rules.counter_start.saturating_add(index as u64);
        let mut to = rules
            .template
            .replace(NAME, stem)
            .replace(EXT, extension)
            .replace(COUNTER, &counter.to_string());
        if !rules.find.is_empty() {
            to = to.replace(&rules.find, &rules.replace);
        }
        // A trailing dot is what `[N].[E]` leaves on a file that has no
        // extension, and it is not what anybody meant by it.
        let to = to.trim_end_matches('.').to_string();

        let refused = refuse(&to, &rows);
        rows.push(Renamed {
            from: from.clone(),
            to,
            refused,
        });
    }
    rows
}

/// Whether a produced name can be used, given the rows already decided.
fn refuse(to: &str, rows: &[Renamed]) -> Option<Refusal> {
    if to.is_empty() {
        return Some(Refusal::Empty);
    }
    if to.contains('/') || to.contains('\\') {
        return Some(Refusal::HasSeparator);
    }
    // Against the *whole* batch, refused rows included: two files both wanting
    // a name neither may have is still two files that cannot both have it, and
    // reporting only the second would suggest the first was fine.
    if rows.iter().any(|row| row.to == to) {
        return Some(Refusal::Collides);
    }
    None
}
