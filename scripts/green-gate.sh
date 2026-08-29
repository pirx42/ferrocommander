#!/usr/bin/env bash
# The sequence that must pass before every commit (skill 25).
#
# A script rather than a chain typed at a prompt, because a chain gets
# truncated: `cargo clippy ... | tail -1` hides a failure behind its last line,
# and a `&&` chain that stops early still lets the next `;` command run and
# look green. That happened, and cost a commit that did not build clean.
#
# Runs every step, reports each, and fails if any failed — so a red step is
# visible even when a later one passes.
set -uo pipefail
cd "$(dirname "$0")/.."

failed=()
step() {
    local name="$1"
    shift
    printf '\n=== %s ===\n' "$name"
    if "$@"; then
        printf '  OK   %s\n' "$name"
    else
        printf '  FAIL %s\n' "$name"
        failed+=("$name")
    fi
}

step "fmt"            cargo fmt --all -- --check
step "clippy"         cargo clippy --workspace --all-targets -- -D warnings
step "clippy-windows" cargo clippy -p tc-core --all-targets \
                          --target x86_64-pc-windows-gnu -- -D warnings
step "tests"          cargo test --workspace
step "release"        cargo build --release
step "links"          python3 scripts/check-links.py

printf '\n===============================\n'
if [ ${#failed[@]} -eq 0 ]; then
    echo "GREEN — all steps passed"
    exit 0
fi
printf 'RED — %d step(s) failed: %s\n' "${#failed[@]}" "${failed[*]}"
exit 1
