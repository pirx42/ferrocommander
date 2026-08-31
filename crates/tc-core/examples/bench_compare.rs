//! Times the compare engine on the inputs that cannot flatter it.
//!
//! A benchmark that picks its own inputs picks flattering ones (skill 74),
//! so the layouts here are stated as part of the claim:
//!
//! * `identical` — every line equal: the best case, and the floor.
//! * `disjoint` — no line in common: Myers' worst case, where the line
//!   diff's cost is quadratic-ish in the change size. This is the number
//!   that decides whether the ceiling in `compare.rs` is honest.
//! * `every line changed at its far end` — the intra-line refinement's
//!   worst case: the char-level pass runs on every pair, and each line is
//!   long with its difference at the end, so the char diff walks the whole
//!   line to find it.
//! * `sprinkled` — one line in a hundred changed: the shape a person
//!   actually compares, so the common case has a number beside the corner
//!   cases.
//!
//! `cargo run --release -p tc-core --example bench_compare`
use std::time::Instant;

use tc_core::compare::rows;

/// Lines per side. docs/performance.md's listing rule calls 50 000 entries
/// not exotic; 10 000 lines is an ordinary source file's ceiling and keeps
/// the disjoint worst case measurable rather than absurd.
const LINES: usize = 10_000;
/// The intra-line case wants lines long enough that walking one costs
/// something.
const LONG_LINE_CHARS: usize = 200;
/// Enough runs that one scheduling hiccup does not become the answer.
const RUNS: usize = 5;

fn timed(label: &str, left: &str, right: &str) {
    let mut best = f64::MAX;
    let mut row_count = 0;
    for _ in 0..RUNS {
        let start = Instant::now();
        let rows = rows(left, right);
        best = best.min(start.elapsed().as_secs_f64() * 1000.0);
        row_count = rows.len();
    }
    println!("{label:>34}  {best:8.2} ms  ({row_count} rows)");
}

fn main() {
    let identical: String = (0..LINES).map(|i| format!("line number {i}\n")).collect();
    let disjoint: String = (0..LINES).map(|i| format!("other text {i}\n")).collect();

    let long_left: String = (0..LINES / 10)
        .map(|i| format!("{}{i} ends in A\n", "x".repeat(LONG_LINE_CHARS)))
        .collect();
    let long_right: String = (0..LINES / 10)
        .map(|i| format!("{}{i} ends in B\n", "x".repeat(LONG_LINE_CHARS)))
        .collect();

    let sprinkled: String = (0..LINES)
        .map(|i| match i % 100 {
            0 => format!("line number {i} but changed\n"),
            _ => format!("line number {i}\n"),
        })
        .collect();

    println!("compare::rows over {LINES} lines, best of {RUNS}:");
    timed("identical", &identical, &identical);
    timed("disjoint (line-diff worst case)", &identical, &disjoint);
    timed("every line changed at its far end", &long_left, &long_right);
    timed("one line in a hundred changed", &identical, &sprinkled);
}
