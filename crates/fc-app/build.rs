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

/// The resource script, the icon it names, and what `windres` makes of them.
const ICON_SCRIPT: &str = "ferrocommander.rc";
const ICON_FILE: &str = "st.rose.Ferrocommander.ico";
const ICON_OBJECT: &str = "ferrocommander-icon.o";

/// `windres` under the two names it goes by: MSYS2 installs it plain, a
/// cross toolchain on Linux prefixes it with the target triple.
const WINDRES: &str = "windres";
const CROSS_WINDRES: &str = "x86_64-w64-mingw32-windres";

/// The object format a Windows linker reads.
const COFF: &str = "coff";

fn main() {
    watch_head();
    embed_icon();
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

/// The Windows icon resource: what the taskbar and `Alt+Tab` draw.
///
/// **Windows takes an application's icon from the executable**, not from
/// the toolkit — there is no GTK call that supplies one, and without this
/// the program shows the shell's default icon everywhere it appears
/// (`docs/windows.md`). `windres` turns `packaging/ferrocommander.rc` and
/// the `.ico` beside it into an object file, and the linker puts it in the
/// binary as `RT_ICON` and `RT_GROUP_ICON`.
///
/// Keyed on the **target**, not on the host: a build script runs on the
/// machine doing the building, and `cfg!(windows)` there would answer for
/// the wrong one.
///
/// No fallback when `windres` is missing. It ships with the
/// `mingw-w64-x86_64-toolchain` that a Windows build already requires, so
/// its absence means the toolchain is wrong — and a build that quietly
/// produced an iconless binary would hide exactly the bug this fixes.
fn embed_icon() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let packaging = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("the crate sits two levels under the workspace root")
        .join("packaging");
    let script = packaging.join(ICON_SCRIPT);
    let icon = packaging.join(ICON_FILE);
    println!("cargo:rerun-if-changed={}", script.display());
    println!("cargo:rerun-if-changed={}", icon.display());

    let object =
        Path::new(&std::env::var("OUT_DIR").expect("cargo sets OUT_DIR")).join(ICON_OBJECT);
    // Named differently by the MSYS2 toolchain a Windows build uses and by
    // the cross toolchain a Linux box installs, and the same program either
    // way.
    let tool = [WINDRES, CROSS_WINDRES]
        .into_iter()
        .find(|name| which(name))
        .unwrap_or_else(|| {
            panic!("neither {WINDRES} nor {CROSS_WINDRES} is on PATH; see docs/windows.md")
        });
    let status = Command::new(tool)
        // The `.rc` names the icon beside it, so the search path is its own
        // directory — nothing here depends on the working directory cargo
        // happens to run a build script in.
        .arg("--include-dir")
        .arg(&packaging)
        .arg(&script)
        .arg("-O")
        .arg(COFF)
        .arg("-o")
        .arg(&object)
        .status()
        .unwrap_or_else(|reason| panic!("{tool}: {reason}"));
    assert!(
        status.success(),
        "{tool} could not compile {}",
        script.display()
    );
    // `-bins`, not `-link-arg`: the object belongs in the program, and a
    // test binary that linked it would carry a copy for nothing.
    println!("cargo:rustc-link-arg-bins={}", object.display());
}

/// Whether a program is on `PATH`.
fn which(name: &str) -> bool {
    Command::new(name)
        .arg("--version")
        .output()
        .is_ok_and(|answer| answer.status.success())
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
