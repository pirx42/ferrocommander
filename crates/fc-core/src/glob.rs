//! Wildcard matching for filenames.
//!
//! `*` matches any run of characters including none, `?` matches exactly one.
//! Nothing else is special — no character classes, no brace expansion — because
//! that is what Total Commander's select-by-pattern accepts and what a person
//! types into it.
//!
//! Lives here rather than in `listing` because the search needs the same
//! matcher, and two implementations of "does this name match" is one too many.

/// Any run of characters, including none.
const ANY_RUN: char = '*';

/// Exactly one character.
const ANY_ONE: char = '?';

/// Whether `name` matches `pattern`, ignoring case.
///
/// Case-insensitive because that is what the pattern dialog is for: someone
/// typing `*.TXT` means the same thing as `*.txt`. Unlike the sort comparison
/// this runs once per entry rather than O(n log n) times, so it compares
/// characters directly instead of carrying an ASCII fast path around.
pub fn matches(pattern: &str, name: &str) -> bool {
    let pattern: Vec<char> = pattern.chars().collect();
    let name: Vec<char> = name.chars().collect();

    // Two cursors, with the position to resume from when a `*` has to give
    // back what it swallowed. Backtracking rather than recursion: the depth
    // would otherwise be the length of the name.
    let (mut p, mut n) = (0, 0);
    let (mut star, mut resume) = (None, 0);

    while n < name.len() {
        match pattern.get(p) {
            Some(&ANY_RUN) => {
                // Remember where to come back to, and start by matching none.
                star = Some(p);
                resume = n;
                p += 1;
            }
            Some(&ANY_ONE) => {
                p += 1;
                n += 1;
            }
            Some(&literal) if equal_ignoring_case(literal, name[n]) => {
                p += 1;
                n += 1;
            }
            // No match here: let the last `*` swallow one more character.
            _ => match star {
                Some(position) => {
                    p = position + 1;
                    resume += 1;
                    n = resume;
                }
                None => return false,
            },
        }
    }

    // Whatever is left of the pattern must be able to match nothing at all.
    pattern[p..].iter().all(|&c| c == ANY_RUN)
}

fn equal_ignoring_case(a: char, b: char) -> bool {
    a == b || a.to_lowercase().eq(b.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pattern_without_wildcards_is_an_exact_name() {
        assert!(matches("notes.txt", "notes.txt"));
        assert!(!matches("notes.txt", "notes.txt.bak"));
        assert!(!matches("notes.txt", "note.txt"));
    }

    #[test]
    fn the_common_patterns_do_what_they_look_like() {
        let cases = [
            ("*.txt", "notes.txt", true),
            ("*.txt", "notes.txt.bak", false),
            ("*.txt", ".txt", true),
            ("*", "anything at all", true),
            ("*", "", true),
            ("a?c", "abc", true),
            ("a?c", "ac", false),
            ("a?c", "abbc", false),
            ("*.tar.*", "archive.tar.gz", true),
            ("data*", "data", true),
            ("*x*", "axb", true),
            ("*x*", "ab", false),
        ];
        for (pattern, name, expected) in cases {
            assert_eq!(matches(pattern, name), expected, "{pattern:?} vs {name:?}");
        }
    }

    #[test]
    fn matching_ignores_case_on_both_sides() {
        assert!(matches("*.TXT", "notes.txt"));
        assert!(matches("*.txt", "NOTES.TXT"));
        assert!(matches("RE*ME", "readme"));
        assert!(matches("*ü*", "MÜSLI.txt"));
    }

    #[test]
    fn several_stars_in_a_row_are_no_worse_than_one() {
        assert!(matches("***", "anything"));
        assert!(matches("a***b", "ab"));
        assert!(matches("a***b", "axxxb"));
        assert!(!matches("a***b", "axxx"));
    }

    #[test]
    fn an_empty_pattern_matches_only_an_empty_name() {
        assert!(matches("", ""));
        assert!(!matches("", "a"));
    }

    #[test]
    fn backtracking_finds_a_match_that_a_greedy_star_would_miss() {
        // A `*` that swallowed everything would fail these; it has to give
        // characters back until the tail lines up.
        assert!(matches("*.txt", "a.txt.txt"));
        assert!(matches("*ab", "aab"));
        assert!(matches("*a*b", "xxaxxb"));
    }

    #[test]
    fn a_star_can_be_asked_to_match_nothing_at_the_end() {
        assert!(matches("notes*", "notes"));
        assert!(matches("notes.txt*", "notes.txt"));
    }
}
