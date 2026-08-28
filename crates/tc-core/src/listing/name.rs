//! Splitting an entry name into the parts the UI shows in separate columns.

/// Splits `name` into its stem and its extension.
///
/// An extension needs a dot with something on both sides, which is what makes
/// the awkward cases fall out correctly: `.gitignore` is a name with no
/// extension (the Unix convention and Total Commander's), `trailing.` has
/// none either, and `..` stays intact rather than becoming `.` + `""`.
pub fn split_name(name: &str) -> (&str, &str) {
    match name.rfind('.') {
        Some(dot) if dot > 0 && dot + 1 < name.len() => (&name[..dot], &name[dot + 1..]),
        _ => (name, ""),
    }
}

/// The extension alone, or `""` when there is none.
pub(super) fn extension(name: &str) -> &str {
    split_name(name).1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_leading_dot_does_not_start_an_extension() {
        assert_eq!(split_name(".gitignore"), (".gitignore", ""));
        assert_eq!(split_name("Makefile"), ("Makefile", ""));
    }

    #[test]
    fn the_last_dot_wins() {
        assert_eq!(split_name("archive.tar.gz"), ("archive.tar", "gz"));
        assert_eq!(split_name("notes.txt"), ("notes", "txt"));
    }

    #[test]
    fn a_dot_with_nothing_after_it_is_not_an_extension() {
        assert_eq!(split_name("trailing."), ("trailing.", ""));
    }

    #[test]
    fn the_parent_row_name_survives_intact() {
        // Naively "everything after the last dot" would split `..` into
        // `.` + `""` and the pane would show a lone dot.
        assert_eq!(split_name(".."), ("..", ""));
    }

    #[test]
    fn extension_is_the_second_half_of_the_split() {
        for name in ["a.txt", ".gitignore", "..", "no_dot", "x.tar.gz"] {
            assert_eq!(extension(name), split_name(name).1, "{name}");
        }
    }
}
