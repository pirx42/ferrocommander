//! What a row's icon costs, with the cache and without it.
//!
//! The pane asks the desktop theme for an icon per *visible* row, and again
//! whenever one scrolls into view — so the question is not what a directory
//! of fifty thousand costs to load, but what a screenful costs to draw, over
//! and over, while somebody holds a cursor key down.
//!
//! Run: `cargo run -p fc-app --release --example bench_icons`
//!
//! The input is stated as part of the claim (skill 74): forty names, the
//! screenful a pane shows, over the extensions a source tree actually holds
//! rather than forty of the same one — a cache measured on one repeated name
//! would flatter itself by exactly the factor being measured.

use std::collections::HashMap;
use std::time::Instant;

use gtk::gio;

/// A screenful, which is what the cache is asked for at a time.
const ROWS: usize = 40;

/// How many screenfuls are timed. Enough that the per-row figure is not one
/// scheduler hiccup.
const ROUNDS: usize = 500;

/// The extensions a source tree holds, so the cache has real variety to
/// answer for rather than one type repeated.
const EXTENSIONS: [&str; 10] = [
    "rs", "toml", "md", "png", "sh", "py", "json", "txt", "zip", "",
];

fn names() -> Vec<String> {
    (0..ROWS)
        .map(|index| match EXTENSIONS[index % EXTENSIONS.len()] {
            "" => format!("file{index}"),
            extension => format!("file{index}.{extension}"),
        })
        .collect()
}

fn main() {
    let names = names();

    let started = Instant::now();
    for _ in 0..ROUNDS {
        for name in &names {
            let content_type = gio::content_type_guess(Some(name.as_str()), None).0;
            std::hint::black_box(gio::content_type_get_icon(&content_type));
        }
    }
    let cold = started.elapsed();

    let mut cache: HashMap<String, gio::Icon> = HashMap::new();
    let started = Instant::now();
    for _ in 0..ROUNDS {
        for name in &names {
            let content_type = gio::content_type_guess(Some(name.as_str()), None).0;
            let icon = cache
                .entry(content_type.to_string())
                .or_insert_with(|| gio::content_type_get_icon(&content_type));
            std::hint::black_box(icon);
        }
    }
    let warm = started.elapsed();

    let per_row = |total: std::time::Duration| total.as_secs_f64() * 1e6 / (ROWS * ROUNDS) as f64;
    println!(
        "{ROWS} rows over {} types, {ROUNDS} rounds",
        EXTENSIONS.len()
    );
    println!("  theme asked every time   {:>8.3} µs/row", per_row(cold));
    println!("  cached by content type   {:>8.3} µs/row", per_row(warm));
    println!(
        "  a screenful costs        {:>8.3} µs cold, {:.3} µs warm",
        per_row(cold) * ROWS as f64,
        per_row(warm) * ROWS as f64
    );
}
