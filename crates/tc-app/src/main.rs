//! GTK4 shell of the file manager.
//!
//! The dual-pane window arrives in phase 1C; until then this binary exists to
//! prove the workspace wiring — that `tc-app` really builds against `tc-core`.

mod constants;

use constants::APP_NAME;

fn main() {
    println!("{}", banner(tc_core::version()));
}

/// One-line startup banner, e.g. `Ferrocommander 0.1.0`.
fn banner(version: &str) -> String {
    format!("{APP_NAME} {version}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn banner_combines_app_name_and_engine_version() {
        assert_eq!(banner("1.2.3"), "Ferrocommander 1.2.3");
    }
}
