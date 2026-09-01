//! Times the branch walk over a 20 000-file tree, and a plain listing of one
//! directory of the same size, so the two are comparable.
use fc_core::branch;
use fc_core::listing::Listing;
use fc_core::ops::CancelToken;
use fc_core::vfs::LocalFs;
use std::time::Instant;

fn main() {
    let base = std::env::args().nth(1).expect("a directory to build in");
    let tree = std::path::Path::new(&base).join("tree");
    let flat = std::path::Path::new(&base).join("flat");
    // 20 000 files, 200 directories of 100.
    std::fs::create_dir_all(&tree).unwrap();
    for d in 0..200 {
        let dir = tree.join(format!("d{d:03}"));
        std::fs::create_dir_all(&dir).unwrap();
        for f in 0..100 {
            std::fs::write(dir.join(format!("f{f:03}.txt")), b"x").unwrap();
        }
    }
    std::fs::create_dir_all(&flat).unwrap();
    for f in 0..20_000 {
        std::fs::write(flat.join(format!("f{f:05}.txt")), b"x").unwrap();
    }

    let root = LocalFs::vfs_path(&tree);
    let flat_root = LocalFs::vfs_path(&flat);
    let mut walk_best = u128::MAX;
    let mut list_best = u128::MAX;
    let mut count = 0;
    for _ in 0..5 {
        let start = Instant::now();
        let found = branch::walk(&LocalFs, &root, &CancelToken::new());
        let listing = Listing::branch(root.clone(), found);
        walk_best = walk_best.min(start.elapsed().as_millis());
        count = listing.len();

        let start = Instant::now();
        let plain = Listing::load(&LocalFs, flat_root.clone()).unwrap();
        list_best = list_best.min(start.elapsed().as_millis());
        assert!(!plain.is_empty());
    }
    // The folder-size scan over the same tree: the same read_dir walk with a
    // running total instead of a collected entry per file.
    let mut measure_best = u128::MAX;
    for _ in 0..5 {
        let start = Instant::now();
        let measured = fc_core::sizes::measure(&LocalFs, &root, &CancelToken::new());
        measure_best = measure_best.min(start.elapsed().as_millis());
        assert!(measured.complete && measured.bytes > 0);
    }

    println!("branch walk + listing of {count} rows: {walk_best} ms");
    println!("folder size of the same tree: {measure_best} ms");
    println!("plain listing of 20 000 in one directory: {list_best} ms");
}
