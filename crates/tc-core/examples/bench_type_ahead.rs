//! Times the type-ahead search over a large directory.
//!
//! Type-ahead runs a case-insensitive substring match over every visible row
//! on every keystroke, on the main loop ([`docs/performance.md`]). At 50 000
//! entries — which that document says is not exotic — the worst case is 50 000
//! comparisons between one letter and the next, and the project's rule is that
//! a speed decision carries a measurement rather than a conviction.
//!
//! No filesystem: a `Listing` is built straight from entries, which is the
//! point of it holding no backend, and it keeps the measurement about the
//! comparison rather than about the disk.
//!
//! `cargo run --release -p tc-core --example bench_type_ahead`
use std::time::{Instant, SystemTime};

use tc_core::listing::Listing;
use tc_core::vfs::{Attributes, Entry, EntryKind, VfsPath};

/// Big enough to be the case the rule is about.
const ENTRIES: usize = 50_000;
/// Enough turns that one scheduling hiccup does not become the answer.
const RUNS: usize = 20;

fn entry(name: String) -> Entry {
    Entry {
        name,
        kind: EntryKind::File,
        size: 0,
        modified: SystemTime::UNIX_EPOCH,
        attributes: Attributes::default(),
        hidden: false,
    }
}

fn best(label: &str, mut run: impl FnMut() -> Option<usize>) -> u128 {
    let mut best = u128::MAX;
    for _ in 0..RUNS {
        let start = Instant::now();
        let found = std::hint::black_box(run());
        best = best.min(start.elapsed().as_nanos());
        let _ = found;
    }
    println!("{label}: {:.1} µs", best as f64 / 1000.0);
    best
}

fn main() {
    // ASCII names, which is what the fast path in the comparison is for and
    // what a real directory almost always holds.
    let ascii: Vec<Entry> = (0..ENTRIES)
        .map(|index| entry(format!("report_{index:06}.txt")))
        .collect();
    // The same directory with one non-ASCII name, which is all it takes to
    // send every comparison down the full character rule.
    let mut awkward = ascii.clone();
    awkward[ENTRIES / 2].name = String::from("Bericht_grün.txt");

    let dir = VfsPath::new("/bench");
    let listing = Listing::new(dir.clone(), ascii);
    let mixed = Listing::new(dir.clone(), awkward.clone());

    println!("{} entries", listing.len());
    // The common case: the first letter of a name a few rows down.
    best("a hit near the cursor", || {
        listing.find_from(1, "report_000005")
    });
    // The worst case: no match anywhere, so the whole view is walked and
    // wrapped. This is the number the rule is about.
    best("a miss — the whole view", || {
        listing.find_from(1, "zzzzzz")
    });
    // The same miss where one entry is not ASCII.
    best("a miss, one non-ASCII name", || {
        mixed.find_from(1, "zzzzzz")
    });
    // What the quick filter pays for the same match on every keystroke, for
    // comparison: it rebuilds the view rather than stopping at the first hit.
    let mut filtered = Listing::new(dir, awkward);
    let mut filter_best = u128::MAX;
    for run in 0..RUNS {
        // A different needle each turn: re-applying one the listing already
        // has would measure the short-circuit rather than the match.
        let needle = format!("zzzz{run:02}");
        let start = Instant::now();
        filtered.set_filter(&needle);
        filter_best = filter_best.min(start.elapsed().as_nanos());
        assert!(filtered.is_empty() || filtered.len() == 1);
    }
    println!(
        "the quick filter, same miss: {:.1} µs",
        filter_best as f64 / 1000.0
    );
}
