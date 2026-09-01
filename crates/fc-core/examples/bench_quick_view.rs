//! What one step of the quick view costs, on the inputs that cannot flatter
//! it.
//!
//! Quick view follows the cursor: every arrow key asks what the row under it
//! holds ([`docs/viewer.md`]). For a file that is a `stat` and one windowed
//! read; for a *directory* — the owner's decision — it is the start of a
//! recursive walk, which is the number that decides whether a walk may begin
//! from the key handler at all or has to wait behind a pause.
//!
//! The layouts are part of the claim (skill 74), because a benchmark that
//! picks its own inputs picks kind ones:
//!
//! * `small file` — the ordinary row, and the floor.
//! * `large file` — a file far bigger than the 64 KiB window, proving the
//!   preview's cost does not follow the file's size.
//! * `deep directory` — the case the owner's decision introduced. Two
//!   numbers, because one of them is a trap: **starting** a walk and
//!   cancelling it the way the next arrow key does is what the *keystroke*
//!   pays, and it is nearly free — but a walk that has started keeps
//!   walking until it notices, so the honest question is how long a whole
//!   one takes. A per-keystroke benchmark that reported only the first
//!   number would say "no debounce needed" while an arrow key held down
//!   left a queue of walks churning the disk behind it.
//!
//! `cargo run --release -p fc-core --example bench_quick_view -- <dir>`
use std::time::Instant;

use fc_core::ops::CancelToken;
use fc_core::sizes;
use fc_core::vfs::{LocalFs, VfsPath};
use fc_core::viewer::View;

/// Enough turns that one scheduling hiccup does not become the answer.
const RUNS: usize = 20;

/// How many files the large file holds, in 64 KiB windows. Far more than the
/// preview reads, which is the point.
const LARGE_WINDOWS: usize = 256;

fn timed(label: &str, mut step: impl FnMut()) {
    let mut best = f64::MAX;
    for _ in 0..RUNS {
        let start = Instant::now();
        step();
        best = best.min(start.elapsed().as_secs_f64() * 1000.0);
    }
    println!("{label:>28}  {best:8.3} ms");
}

fn main() {
    let base = std::env::args().nth(1).expect("a directory to build in");
    let root = std::path::Path::new(&base);
    std::fs::create_dir_all(root).unwrap();

    let small = root.join("small.txt");
    std::fs::write(&small, "a line of text\n".repeat(40)).unwrap();

    let large = root.join("large.bin");
    std::fs::write(&large, "x".repeat(LARGE_WINDOWS * 64 * 1024)).unwrap();

    // A tree wide and deep enough that no walk of it finishes in a keystroke.
    let deep = root.join("deep");
    for outer in 0..40 {
        for inner in 0..25 {
            let dir = deep.join(format!("{outer}/{inner}"));
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("f.txt"), "x").unwrap();
        }
    }

    let preview = |path: &std::path::Path| {
        let view = View::open(&LocalFs, LocalFs::vfs_path(path)).unwrap();
        let _ = view.render(&LocalFs).unwrap();
    };

    println!("one quick-view step, best of {RUNS}:");
    timed("small file", || preview(&small));
    timed("large file", || preview(&large));

    // Starting a walk and cancelling it, which is what an arrow key passing
    // over a folder does. The receiver is dropped with the answers unread —
    // deliberately: the cost being measured is what the *keystroke* pays.
    // The cursor sits on `deep` itself, so the walk is over the whole tree
    // below it. Pointing it at one of `deep`'s *children* was the first
    // version of this benchmark and was wrong in the way this file's own
    // comment warns about: a child holds 26 entries, the folder a person
    // would pause on holds two thousand, and only the second is the case the
    // debounce question is about.
    let parent = VfsPath::new(&root.to_string_lossy());
    let folders = vec!["deep".to_string()];
    timed("starting a directory walk", || {
        let cancel = CancelToken::new();
        let answers = sizes::spawn(
            std::sync::Arc::new(LocalFs),
            parent.clone(),
            folders.clone(),
            cancel.clone(),
        );
        cancel.cancel();
        drop(answers);
    });

    // And what that started walk goes on to cost if nothing stops it — the
    // work a held-down arrow key would leave running behind itself, once per
    // repeat, if the preview asked on every keystroke.
    timed("...and running it to the end", || {
        let cancel = CancelToken::new();
        let answers = sizes::spawn(
            std::sync::Arc::new(LocalFs),
            parent.clone(),
            folders.clone(),
            cancel,
        );
        while answers.recv_blocking().is_ok() {}
    });
}
