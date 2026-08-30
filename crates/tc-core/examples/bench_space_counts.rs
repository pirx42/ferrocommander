//! What a burst of `Space` presses costs, when each restarts the scan.
//!
//! `Space` counts the folder it marks, and a second press restarts the walk
//! over the folders that have not answered yet ([`docs/keymap.md`]). Anything
//! already counted is filtered out, so the loss is bounded by the folder being
//! walked at the moment of the next press — but "bounded" is a claim, and this
//! project's rule is that a speed decision carries a number.
//!
//! Measures the worst case honestly: one folder big enough that a walk of it
//! outlives a keystroke, marked first, with more folders marked after it. The
//! answer is how much work the restarts add over counting each folder once.
//!
//! `cargo run --release -p tc-core --example bench_space_counts -- <dir>`
use std::time::Instant;

use tc_core::ops::CancelToken;
use tc_core::sizes;
use tc_core::vfs::LocalFs;

/// Folders marked in the burst, the first of them the large one.
const FOLDERS: usize = 10;
/// Files in the large folder — enough that walking it is not instant.
const DEEP_FILES: usize = 20_000;

fn main() {
    let base = std::env::args().nth(1).expect("a directory to build in");
    let root = std::path::Path::new(&base).join("burst");
    std::fs::create_dir_all(&root).unwrap();
    for index in 0..FOLDERS {
        let dir = root.join(format!("d{index:02}"));
        std::fs::create_dir_all(&dir).unwrap();
        // The first folder is the one a keystroke can outrun.
        let files = if index == 0 { DEEP_FILES } else { 10 };
        for file in 0..files {
            std::fs::write(dir.join(format!("f{file:05}.txt")), b"x").unwrap();
        }
    }

    let vfs_root = LocalFs::vfs_path(&root);
    let names: Vec<String> = (0..FOLDERS).map(|index| format!("d{index:02}")).collect();

    // How long the one folder a keystroke might outrun actually takes. If it
    // finishes inside a key-repeat interval, the worst case below is a model
    // rather than something a person can provoke.
    let mut biggest = u128::MAX;
    for _ in 0..3 {
        let start = Instant::now();
        let measured = sizes::measure(&LocalFs, &vfs_root.child(&names[0]), &CancelToken::new());
        biggest = biggest.min(start.elapsed().as_millis());
        assert!(measured.complete);
    }
    println!("the one large folder, {DEEP_FILES} files    : {biggest} ms");

    // The floor: every folder counted exactly once, which is what a queue
    // would cost and what the restart is measured against.
    let mut once = u128::MAX;
    for _ in 0..3 {
        let start = Instant::now();
        for name in &names {
            let measured = sizes::measure(&LocalFs, &vfs_root.child(name), &CancelToken::new());
            assert!(measured.complete);
        }
        once = once.min(start.elapsed().as_millis());
    }

    // The restart: press k re-walks whatever the press before it had not
    // finished. Modelled as the worst case — the big folder never completes
    // until the last press, so it is walked once per press.
    let mut restarting = u128::MAX;
    for _ in 0..3 {
        let start = Instant::now();
        for press in 1..=FOLDERS {
            for name in names.iter().take(press) {
                let measured = sizes::measure(&LocalFs, &vfs_root.child(name), &CancelToken::new());
                assert!(measured.complete);
            }
        }
        restarting = restarting.min(start.elapsed().as_millis());
    }

    println!("counted once, {FOLDERS} folders           : {once} ms");
    println!("every press re-walks the unfinished ones : {restarting} ms");
    println!(
        "ratio                                    : {:.1}x",
        restarting as f64 / once as f64
    );
}
