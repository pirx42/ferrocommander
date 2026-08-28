// How a build number and a commit hash become the tail of the window title.
//
// A file of its own because two things compile it: `build.rs` pulls it in with
// `include!` to produce the stamp, and the crate compiles it under `cfg(test)`
// so the rule has ordinary unit tests. A build script's own tests are never
// run by `cargo test`, and the case that matters here — a build with no git to
// ask — is exactly the one that cannot be reproduced by building this crate a
// second way.
//
// Plain `//` rather than `//!`: an inner doc comment cannot appear where
// `include!` drops this, which is partway down another file.

/// The part of the title after the product name: ` #51 (62fddd9)`.
///
/// Empty when either value is missing, which is what a build from a source
/// tarball or an image without git produces. Empty rather than a placeholder:
/// a title reading `#0 (unknown)` looks like a build that exists, and the
/// point of putting it there at all is that it names one that does.
pub fn stamp(number: &str, hash: &str) -> String {
    if number.is_empty() || hash.is_empty() {
        return String::new();
    }
    format!(" #{number} ({hash})")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stamped_build_is_named_in_full() {
        assert_eq!(stamp("527", "ef8b326"), " #527 (ef8b326)");
    }

    #[test]
    fn a_build_with_no_repository_to_ask_adds_nothing() {
        for (number, hash) in [("", "ef8b326"), ("527", ""), ("", "")] {
            assert_eq!(stamp(number, hash), "", "{number:?} {hash:?}");
        }
    }
}
