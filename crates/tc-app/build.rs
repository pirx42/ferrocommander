//! Stamps the build number and commit hash into the binary.
//!
//! The window title carries them, so a screenshot or a bug report says which
//! build it came from. That only works if the values describe *this* binary,
//! which is why they are read at build time from git rather than written into
//! a file somebody has to remember to bump.
//!
//! **A missing git is not a build failure.** Building from a source tarball,
//! or in an image with no git, has to work — so the values come out empty and
//! the title falls back to the bare product name. A build that refused over
//! its own decoration would be a worse trade than a title that says less.

use std::path::Path;
use std::process::Command;

/// The commit count, which is what makes a build *number* rather than another
/// hash: it goes up, so two builds can be told apart at a glance.
///
/// **`scripts/package-deb.sh` computes the same number the same way**, so the
/// package version and this title agree about which build somebody is
/// running. Written twice because neither can call the other — this runs in a
/// build script that must work on Windows with no shell — and the rule they
/// share is in `docs/packaging.md`.
const BUILD_NUMBER_ARGS: &[&str] = &["rev-list", "--count", "HEAD"];

/// Short enough to read off a title bar, long enough to find the commit.
const BUILD_HASH_ARGS: &[&str] = &["rev-parse", "--short", "HEAD"];

fn main() {
    watch_head();
    // One finished string rather than two values for the crate to assemble,
    // so the title is a `&'static str` constant and nothing is put together at
    // run time.
    //
    // Both calls need `HEAD` to resolve, so they answer or fail together —
    // there is no half-stamped build to describe. When they fail the tail is
    // empty and `concat!` yields the bare product name on its own, which is
    // why nothing here spells that case out.
    let stamp = match (git(BUILD_NUMBER_ARGS), git(BUILD_HASH_ARGS)) {
        (Some(number), Some(hash)) => format!(" #{number} ({hash})"),
        _ => String::new(),
    };
    println!("cargo:rustc-env=TC_BUILD_STAMP={stamp}");
}

/// Rebuilds when the commit changes, and only then.
///
/// Watching `.git/index` instead would rebuild after every `git add`, which is
/// a rebuild per staged file for a value that has not moved. Watching nothing
/// is worse the other way: a commit with no source change would leave the
/// title naming the commit before it.
fn watch_head() {
    println!("cargo:rerun-if-changed=build.rs");
    let Some(git_dir) = git(&["rev-parse", "--absolute-git-dir"]) else {
        return;
    };
    let head = Path::new(&git_dir).join("HEAD");
    if !head.exists() {
        return;
    }
    println!("cargo:rerun-if-changed={}", head.display());

    // HEAD names a branch rather than a commit, and it is the branch file that
    // moves when a commit lands. A packed ref has no file to watch; then only
    // HEAD itself is watched, which still catches a checkout.
    let Ok(contents) = std::fs::read_to_string(&head) else {
        return;
    };
    let Some(reference) = contents.trim().strip_prefix("ref: ") else {
        return;
    };
    let branch = Path::new(&git_dir).join(reference);
    if branch.exists() {
        println!("cargo:rerun-if-changed={}", branch.display());
    }
}

/// Runs git and returns its trimmed output, or `None` if anything at all went
/// wrong — no git, no repository, a detached state with no commits yet.
fn git(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?.trim().to_string();
    (!text.is_empty()).then_some(text)
}
