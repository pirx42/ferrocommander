#!/usr/bin/env bash
# Builds the Ubuntu package, and says where it put it.
#
# **A script rather than steps in a workflow file**, and that is the whole
# point of it. A GitHub workflow can only be tested by pushing it — there is
# no local run, no probe, no green gate — so every line of real logic lives
# here, where it can be run, read and fixed on a laptop. The workflow's job is
# to install the build dependencies and call this.
#
# The same reasoning as `green-gate.sh`, one level out: a sequence nobody can
# run is a sequence nobody can check.
set -euo pipefail
cd "$(dirname "$0")/.."

# The version the window title already carries, so `dpkg -l` and a screenshot
# agree about which build somebody is running. `build.rs` computes the same
# number the same way — see `crates/tc-app/build.rs`.
#
# Zero when git cannot answer (a source tarball, an image with no git), which
# is a version that sorts below every real build rather than a failure.
crate_version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
build_number=$(git rev-list --count HEAD 2>/dev/null || echo 0)
version="${crate_version}-${build_number}"

echo "=== building ferrocommander ${version}"
# `cargo deb` would build this itself, but doing it here means the release
# binary is the one the green gate just built and checked, rather than a
# second one produced by a different invocation.
cargo build --release --locked

echo "=== packaging"
# `--no-build` for the reason above; `--no-strip` left off, so the shipped
# binary is stripped and the package is a tenth of the size.
cargo deb --package tc-app --no-build --deb-version "$version" --output target/debian

deb=$(ls -1 target/debian/*.deb | head -1)

echo "=== checking what was built"
# Three things worth failing over, each of which has been wrong in somebody's
# package: nothing declared, nothing installed, and the desktop entry silently
# malformed so the launcher shows nothing.
depends=$(dpkg-deb --field "$deb" Depends)
if [ -z "$depends" ]; then
    echo "FAIL: the package declares no dependencies at all" >&2
    exit 1
fi
case "$depends" in
    *libgtk-4*) ;;
    *)
        echo "FAIL: no GTK 4 runtime in Depends: $depends" >&2
        exit 1
        ;;
esac

for path in usr/bin/ferrocommander \
            usr/share/applications/st.rose.Ferrocommander.desktop \
            usr/share/icons/hicolor/scalable/apps/st.rose.Ferrocommander.svg; do
    if ! dpkg-deb -c "$deb" | grep -q " ./$path\$"; then
        echo "FAIL: $path is not in the package" >&2
        exit 1
    fi
done

# Ubuntu ships this; a machine without it says so and carries on, because a
# missing validator is not a broken package.
if command -v desktop-file-validate >/dev/null; then
    desktop-file-validate packaging/st.rose.Ferrocommander.desktop
else
    echo "  (desktop-file-validate not installed; entry not validated)"
fi

echo
echo "built    $deb"
echo "version  $version"
echo "depends  $depends"
