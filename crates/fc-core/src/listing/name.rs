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

/// Whether `haystack` contains `needle`, ignoring case.
///
/// What the quick filter asks of every loaded entry on every keystroke, so at
/// 50 000 entries it runs 50 000 times between one character and the next.
/// The ASCII path allocates nothing; the Unicode fallback lowercases both
/// sides, which is the honest way to compare them and rare enough not to
/// matter (`docs/performance.md`).
pub(super) fn contains_ignoring_case(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    if haystack.is_ascii() && needle.is_ascii() {
        return ascii_contains(haystack.as_bytes(), needle.as_bytes());
    }
    haystack.to_lowercase().contains(&needle.to_lowercase())
}

fn ascii_contains(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.len() > haystack.len() {
        return false;
    }
    haystack.windows(needle.len()).any(|window| {
        window
            .iter()
            .zip(needle)
            .all(|(a, b)| a.eq_ignore_ascii_case(b))
    })
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
    fn a_filter_matches_anywhere_in_the_name_and_ignores_case() {
        let cases = [
            ("report.txt", "rep", true),
            ("report.txt", "ORT", true),
            ("report.txt", ".txt", true),
            ("report.txt", "port.t", true),
            ("report.txt", "zzz", false),
            ("report.txt", "", true),
            ("", "a", false),
            ("", "", true),
            ("short", "a longer needle", false),
        ];
        for (haystack, needle, expected) in cases {
            assert_eq!(
                contains_ignoring_case(haystack, needle),
                expected,
                "{haystack:?} contains {needle:?}"
            );
        }
    }

    #[test]
    fn the_ascii_path_and_the_unicode_path_agree() {
        // The shortcut must be invisible, exactly as it is in the sort
        // comparison: same answer, whichever route the name takes.
        let cases = [
            ("MÜSLI.txt", "müsli", true),
            ("MÜSLI.txt", "MÜS", true),
            ("Ünïcødé — ✓.md", "cød", true),
            ("Ünïcødé — ✓.md", "xyz", false),
            ("plain.txt", "ü", false),
        ];
        for (haystack, needle, expected) in cases {
            assert_eq!(
                contains_ignoring_case(haystack, needle),
                expected,
                "{haystack:?} contains {needle:?}"
            );
            assert_eq!(
                haystack.to_lowercase().contains(&needle.to_lowercase()),
                expected,
                "reference disagrees for {haystack:?} / {needle:?}"
            );
        }
    }

    #[test]
    fn extension_is_the_second_half_of_the_split() {
        for name in ["a.txt", ".gitignore", "..", "no_dot", "x.tar.gz"] {
            assert_eq!(extension(name), split_name(name).1, "{name}");
        }
    }
}
