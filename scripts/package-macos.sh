#!/usr/bin/env bash
# Builds the macOS package (Apple Silicon), and says where it put it.
#
# **A script rather than steps in a workflow file**, for the reason the other
# two packaging scripts give: a GitHub workflow can only be tested by pushing
# it, so the logic lives here, where it can be run, read and fixed on a Mac.
# Until the day a Mac is in hand, the runner *is* that Mac — this script is
# the first thing that ever runs ferrocommander on macOS at all
# (`docs/plans/2026-08-31-macos-package.md`).
#
# Run it on Apple Silicon with Homebrew's GTK4 installed:
#
#     brew install gtk4
#     scripts/package-macos.sh
set -euo pipefail
cd "$(dirname "$0")/.."

. scripts/version.sh

# What ships, named once each because the staging and the checking below both
# refer to them (skill 16).
readonly APP=FerroCommander.app
# The real binary and the launcher around it. The launcher exists for the
# same reason the Windows `.cmd` does — the binary alone is a trap — but for
# a different trap: GLib finds its compiled GSettings schemas through the
# data directories of the prefix it was *built* for, which is the build
# machine's Homebrew and not this bundle. Without the launcher pointing
# GSETTINGS_SCHEMA_DIR into the bundle, GTK aborts at startup on every Mac
# except the one that built it — the silent-only-works-here failure again.
# `exec` in the launcher, so the smoke test's timeout signals the app itself
# rather than a shell wrapped around it.
readonly BIN=ferrocommander-bin
readonly LAUNCHER=Contents/MacOS/ferrocommander
readonly PLIST=Contents/Info.plist
readonly NOTES=README.txt
readonly SCHEMAS=Contents/Resources/glib-2.0/schemas/gschemas.compiled

# Native, not cross: the arm64 runner's host is the target. The guard stays
# anyway — it costs three lines, and CI run #17 is what a wrong host looks
# like without one: a long build ending in a wall of link errors, or worse,
# an x86_64 artifact shipped with an arm64 name.
readonly HOST=aarch64-apple-darwin

# How long the app is given to fall over before the smoke test calls it
# alive. Windows earned this number from a crash six seconds in; macOS has
# no measured crash yet, and the span that caught the one is the span that
# would catch the other.
readonly SMOKE_SECONDS=20

# Where Homebrew keeps the dylibs and the schema sources. Asked, not
# hardcoded: /opt/homebrew on Apple Silicon, but the answer is brew's to
# give, not this script's to guess.
brew_prefix=$(brew --prefix)

host=$(rustc -vV | sed -n 's/^host: //p')
if [ "$host" != "$HOST" ]; then
    echo "FAIL: rustc builds for $host, but this package is $HOST." >&2
    exit 1
fi

echo "=== building ferrocommander ${version}"
cargo build --release --locked

echo "=== staging"
# The folder inside the archive carries the version, so unpacking says which
# build this is; the archive itself does not, for the rolling-URL reason
# under "packaging" below. The `.app` name stays versionless — it is what
# lands in /Applications, and nobody wants a new app for every build.
staged="target/macos/ferrocommander-${version}"
app="$staged/$APP"
rm -rf "$staged"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Frameworks" \
         "$app/Contents/Resources"

# Not stripped, for the Windows zip's reason one platform over: this is the
# first time the program runs on this OS at all, and the symbols are what
# any report about the first crash would be worth something with.
cp "target/release/ferrocommander" "$app/Contents/MacOS/$BIN"

# The dylibs the binary actually links, walked with `otool -L` rather than
# listed — the Windows script's argument, unchanged: a hand-written list of
# a GTK program's runtime dependencies goes stale silently, because the
# build machine has the library and the bundle that lacks it works there
# and nowhere else.
#
# A dependency outside the Homebrew prefix is a system library — /usr/lib,
# the OS frameworks — which every Mac has and none may ship. An @rpath or
# @loader_path reference is resolved against the *referencing dylib's own
# directory*, then Homebrew's lib directory: run #21 met the real case,
# libwebp naming its sibling libsharpyuv through the rpath Homebrew points
# at the package's own lib dir. That is why the walk queues the dylibs'
# ORIGINAL paths rather than the staged copies — a copy in Frameworks no
# longer knows the directory its siblings live in. One that resolves
# nowhere is still a FAIL, not a skip.
# `[[:space:]]` and not `\t`: BSD sed reads `\t` as a literal t, which would
# make this match nothing and the walk ship a binary with no dylibs at all —
# caught by the standalone check, but better not written. The header line
# (`file:`) carries no ` (` and falls through the pattern on its own.
imports() {
    otool -L "$1" | sed -n 's/^[[:space:]]*\([^ ]*\) (.*$/\1/p'
}

# Where a reference actually points: absolute ones answer for themselves,
# @-relative ones are looked for beside their referencer and then in brew's
# lib. Empty answer = nowhere, and the caller fails loudly.
resolve() {
    local dep=$1 refdir=$2 name candidate
    case "$dep" in
        @rpath/* | @loader_path/*)
            name="${dep#@*/}"
            for candidate in "$refdir/$name" "$brew_prefix/lib/$name"; do
                if [ -f "$candidate" ]; then
                    echo "$candidate"
                    return
                fi
            done
            ;;
        *)
            echo "$dep"
            ;;
    esac
}

queue=("$app/Contents/MacOS/$BIN")
next=0
while [ "$next" -lt "${#queue[@]}" ]; do
    file="${queue[$next]}"
    next=$((next + 1))
    while read -r dep; do
        resolved=$(resolve "$dep" "$(dirname "$file")")
        case "$resolved" in
            "$brew_prefix"/*)
                base=$(basename "$resolved")
                if [ ! -f "$app/Contents/Frameworks/$base" ]; then
                    cp "$resolved" "$app/Contents/Frameworks/$base"
                    chmod u+w "$app/Contents/Frameworks/$base"
                    queue+=("$resolved")
                fi
                ;;
            /*) ;; # a system library: every Mac has it, none may ship it
            "")
                echo "FAIL: $file wants '$dep', which resolves nowhere" >&2
                exit 1
                ;;
        esac
    done < <(imports "$file")
done

# Copying is half the job. Every copied dylib still *names* its dependencies
# by their absolute Homebrew paths, and the binary names the dylibs the same
# way — so without the rewrite, dyld loads the build machine's libraries and
# the bundle only works where it was built, silently. `@executable_path`
# resolves against the running binary in Contents/MacOS, so ../Frameworks is
# the bundle's own copy wherever the .app lands.
rewrite() {
    local file=$1
    while read -r dep; do
        case "$dep" in
            "$brew_prefix"/* | @rpath/* | @loader_path/*)
                install_name_tool -change "$dep" \
                    "@executable_path/../Frameworks/$(basename "$dep")" "$file"
                ;;
        esac
    done < <(imports "$file")
}
rewrite "$app/Contents/MacOS/$BIN"
for dylib in "$app/Contents/Frameworks/"*.dylib; do
    install_name_tool -id \
        "@executable_path/../Frameworks/$(basename "$dylib")" "$dylib"
    rewrite "$dylib"
done

# arm64 macOS refuses to run unsigned Mach-O at all, and install_name_tool
# has just invalidated the ad-hoc signatures the linkers left — so every
# rewritten file is re-signed or the smoke test dies with `Killed: 9` and
# says nothing about why. Ad-hoc (`-s -`): free, no account, enough to run.
# What it does not buy is Gatekeeper's blessing on a downloaded zip; that
# is the quarantine sentence in $NOTES, not a problem this script can sign
# away. Only the Mach-O files are signed — the launcher is a script, which
# cannot carry a signature and does not need one.
for dylib in "$app/Contents/Frameworks/"*.dylib; do
    codesign --force -s - "$dylib"
done
codesign --force -s - "$app/Contents/MacOS/$BIN"

# GTK reads its own settings through GSettings and aborts at startup when
# the schemas are missing. Compiled here rather than copied, for the .deb
# script's reason: one code path that is always right, instead of a copy
# that depends on a package post-install hook having run.
mkdir -p "$app/$(dirname "$SCHEMAS")"
glib-compile-schemas "$brew_prefix/share/glib-2.0/schemas" \
    --targetdir "$app/$(dirname "$SCHEMAS")"

cat > "$app/$LAUNCHER" <<LAUNCHER_EOF
#!/bin/sh
# FerroCommander ${version} — see the comment on BIN in package-macos.sh
# for why this launcher exists at all.
here=\$(cd "\$(dirname "\$0")" && pwd)
export GSETTINGS_SCHEMA_DIR="\$here/../Resources/glib-2.0/schemas"
exec "\$here/${BIN}" "\$@"
LAUNCHER_EOF
chmod +x "$app/$LAUNCHER"

# The minimum Finder needs to treat the folder as an app: what to run, what
# to call it, and who it is. Versioned so "Get Info" answers the question
# the window title also answers, the .deb's dpkg-l-and-screenshot agreement
# one platform over.
cat > "$app/$PLIST" <<PLIST_EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>      <string>ferrocommander</string>
    <key>CFBundleIdentifier</key>      <string>st.rose.Ferrocommander</string>
    <key>CFBundleName</key>            <string>FerroCommander</string>
    <key>CFBundlePackageType</key>     <string>APPL</string>
    <key>CFBundleShortVersionString</key> <string>${version}</string>
    <key>NSHighResolutionCapable</key> <true/>
</dict>
</plist>
PLIST_EOF

cat > "$staged/$NOTES" <<NOTES_EOF
FerroCommander ${version} - macOS (Apple Silicon)

Drag ${APP} wherever you like and start it.

The first start needs a right-click (Control-click) and "Open": the app is
not notarized with Apple, so a plain double-click on a downloaded copy is
refused by Gatekeeper with a message that does not mention this way around
it. Once opened that way, it opens normally ever after.

Everything the app needs is inside the bundle - nothing is installed, and
deleting it removes it.

https://github.com/pirx42/ferrocommander
NOTES_EOF

echo "=== checking what was built"
# The same three things the other two packages check, in the forms macOS
# offers them: nothing declared, nothing installed, and the part that
# breaks silently rather than loudly.

# Nothing declared. Walked again, flat, rather than reusing the traversal
# above — a check that reuses the traversal it is checking cannot catch
# that traversal being wrong. Every reference must now be a system library
# or resolve inside Frameworks; one still naming the Homebrew prefix is
# the rewrite having missed it, which the smoke test below cannot catch —
# on this machine that absolute path still works. This line is the only
# thing standing between a missed rewrite and a shipped bundle that only
# runs here.
missing=()
for file in "$app/Contents/MacOS/$BIN" "$app/Contents/Frameworks/"*.dylib; do
    while read -r dep; do
        case "$dep" in
            "$brew_prefix"/* | @rpath/* | @loader_path/*)
                missing+=("$dep (not rewritten, in $(basename "$file"))")
                ;;
            @executable_path/../Frameworks/*)
                if [ ! -f "$app/Contents/Frameworks/$(basename "$dep")" ]; then
                    missing+=("$dep (needed by $(basename "$file"))")
                fi
                ;;
        esac
    done < <(imports "$file")
done
if [ "${#missing[@]}" -gt 0 ]; then
    echo "FAIL: the bundle does not stand alone:" >&2
    printf '  %s\n' "${missing[@]}" >&2
    exit 1
fi

# Nothing installed.
for path in "Contents/MacOS/$BIN" "$LAUNCHER" "$PLIST" "$SCHEMAS"; do
    if [ ! -f "$app/$path" ]; then
        echo "FAIL: $path is not in the bundle" >&2
        exit 1
    fi
done
if [ ! -f "$staged/$NOTES" ]; then
    echo "FAIL: $NOTES is not beside the bundle" >&2
    exit 1
fi

# It starts, and stays started — through the launcher, so the environment
# it sets is part of what is tested. Stock macOS has no `timeout`; the
# runner's Homebrew coreutils spells it `gtimeout`. Resolved here rather
# than discovered as `command not found` twenty minutes into a run.
#
# Status 124 is the only pass, for the Windows script's reason: it is
# `timeout` reporting it had to stop the app itself, which is the one
# outcome that means the app was still running.
if command -v gtimeout >/dev/null; then
    timeout_cmd=gtimeout
elif command -v timeout >/dev/null; then
    timeout_cmd=timeout
else
    echo "FAIL: neither gtimeout nor timeout exists (brew install coreutils)" >&2
    exit 1
fi
echo "=== starting it for ${SMOKE_SECONDS}s"
log=$(mktemp)
status=0
"$timeout_cmd" --kill-after="$SMOKE_SECONDS" "$SMOKE_SECONDS" \
    "$app/$LAUNCHER" >"$log" 2>&1 || status=$?
if [ "$status" -ne 124 ]; then
    echo "FAIL: it stopped inside ${SMOKE_SECONDS}s (status $status)" >&2
    echo "--- what it printed ---" >&2
    cat "$log" >&2
    exit 1
fi
if [ -s "$log" ]; then
    echo "--- what it printed while running ---"
    cat "$log"
fi
rm -f "$log"

echo "=== packaging"
# No version in the archive's name, for the other packages' reason: the
# release is rolling, so the download URL has to be one somebody can put in
# a README and not revisit. `zip -y` keeps symlinks as symlinks should any
# ever appear in the bundle, instead of silently doubling the payload.
zip_name=ferrocommander-macos-arm64.zip
(cd target/macos && rm -f "$zip_name" && zip -qry "$zip_name" "ferrocommander-${version}")

echo
echo "built    target/macos/$zip_name"
echo "version  $version"
echo "holds    $(find "$staged" -type f | wc -l | tr -d ' ') files, $(du -sh "$staged" | cut -f1)"
