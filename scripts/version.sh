# The version a package carries: the crate version and the commit count.
#
# **Sourced rather than written out again**, because it was already written
# twice. `crates/tc-app/build.rs` computes the same commit count a third time
# and cannot avoid it — it runs inside a Rust build script that has to work on
# Windows with no shell, so it cannot call this. Two places is the fewest
# available; a third, in a second packaging script that *could* have shared
# it, would have been one nobody was forced into (skill 44).
#
# `docs/packaging.md` is where it says the two must agree.
#
# Expects the working directory to be the repository root, which every caller
# has already changed to.

crate_version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
# Zero when git cannot answer — a source tarball, an image with no git —
# which is a version that sorts below every real build rather than a failure.
build_number=$(git rev-list --count HEAD 2>/dev/null || echo 0)
version="${crate_version}-${build_number}"
