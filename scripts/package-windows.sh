#!/usr/bin/env bash
# Builds the Windows package, and says where it put it.
#
# **A script rather than steps in a workflow file**, for the reason
# `package-deb.sh` gives one platform over: a GitHub workflow can only be
# tested by pushing it, so the logic lives here, where it can be run, read and
# fixed on a laptop. Here that argument is stronger rather than weaker — the
# laptop is the *only* place any of this can be exercised at all, because
# nothing about a GTK bundle for Windows can be checked from Linux.
#
# Run it from an MSYS2 MINGW64 shell, which is the environment
# `docs/windows.md` sets up anyway:
#
#     scripts/package-windows.sh
set -euo pipefail
cd "$(dirname "$0")/.."

. scripts/version.sh

# What ships, named once each because the staging and the checking below both
# refer to them (skill 16).
readonly EXE=ferrocommander.exe
readonly LAUNCHER=ferrocommander.cmd
readonly NOTES=README.txt
readonly SCHEMAS=share/glib-2.0/schemas/gschemas.compiled

# The only toolchain that can link against MSYS2's GTK4. The Rust installer
# defaults to the MSVC host, which cannot, and says so at link time rather
# than at install time — so this is checked before a long build rather than
# after one (`docs/windows.md` § The toolchain).
readonly HOST=x86_64-pc-windows-gnu

# The renderer the app has to be told to use, or it exits about six seconds in
# with 0xC0000005 and never presents a window (`docs/ui-shell.md` § The
# renderer on Windows). Named here and used twice — written into the launcher
# that ships, and exported for the smoke test that proves the launcher names a
# value the app survives — so the shipped setting and the tested setting are
# the same string by construction and cannot drift apart.
readonly RENDERER=cairo

# How long the app is given to fall over before the smoke test calls it alive.
# The documented failure is not a refusal to start but a silent death about
# six seconds in, so a check that only asked "did it launch" would pass over
# exactly the bug it is here for.
readonly SMOKE_SECONDS=20

# Where MSYS2 keeps the DLLs and the schema sources. The MINGW64 shell sets
# this; the fallback spells out the same path rather than guessing at another,
# because a shell without it is a UCRT64 or MSYS one and the wrong environment
# either way — which the host check below then says out loud.
mingw="${MINGW_PREFIX:-/mingw64}"

host=$(rustc -vV | sed -n 's/^host: //p')
if [ "$host" != "$HOST" ]; then
    echo "FAIL: rustc builds for $host, but the GTK4 here is $HOST." >&2
    echo "      docs/windows.md — the MSVC host cannot link MSYS2's GTK4." >&2
    exit 1
fi

echo "=== building ferrocommander ${version}"
cargo build --release --locked

echo "=== staging"
# The folder inside the archive carries the version, so unpacking says which
# build this is. The archive *itself* does not, for the rolling-URL reason
# under "packaging" below — and this is the only place the version can be read
# back at all, because a zip has no `dpkg -I`.
staged="target/windows/ferrocommander-${version}"
rm -rf "$staged"
mkdir -p "$staged"
# Not stripped, unlike the .deb's binary, and on purpose. Windows is where
# this program has a crash nobody has explained yet — the one the launcher
# below works around — and the symbols are what a report about it would be
# worth anything with (`docs/future-improvements.md`). The download is a few
# megabytes larger for it, which is the cheaper half of that trade.
cp "target/release/$EXE" "$staged/$EXE"

imports() {
    objdump -p "$1" | sed -n 's/^[[:space:]]*DLL Name: //p'
}

# A DLL MSYS2 does not provide is a Windows system DLL — kernel32, user32,
# msvcrt — which every Windows already has and none of which may be shipped.
provided_by_msys2() {
    [ -f "$mingw/bin/$1" ]
}

# Everything the binary needs from MSYS2, found rather than listed.
#
# The same argument as the .deb's `depends = "$auto"`, and it bites harder
# here: a hand-written list of a GTK program's runtime dependencies goes stale
# the first time one of them moves, and on Windows it goes stale *silently* —
# the build machine has the DLL on its PATH, so a bundle missing it works
# perfectly right up until somebody else unzips it.
#
# `objdump` rather than `ntldd`, because it comes with the toolchain group
# `docs/windows.md` already installs. One fewer package to be missing on a
# fresh machine is one fewer way for this to fail there and nowhere else.
#
# Breadth-first, with a moving head rather than a shrinking array: nothing is
# removed, so there are no sparse indices to reason about.
queue=("$staged/$EXE")
next=0
while [ "$next" -lt "${#queue[@]}" ]; do
    file="${queue[$next]}"
    next=$((next + 1))
    while read -r dll; do
        if provided_by_msys2 "$dll" && [ ! -f "$staged/$dll" ]; then
            cp "$mingw/bin/$dll" "$staged/$dll"
            queue+=("$staged/$dll")
        fi
    done < <(imports "$file")
done

# GTK reads its own settings through GSettings and aborts at startup when the
# schemas are not there, so this is not decoration. Compiled here rather than
# copied out of the MSYS2 prefix: one code path that is always right, instead
# of a copy that depends on a package post-install hook having run.
mkdir -p "$staged/$(dirname "$SCHEMAS")"
glib-compile-schemas "$mingw/share/glib-2.0/schemas" \
    --targetdir "$staged/$(dirname "$SCHEMAS")"

# No icon theme, and no gdk-pixbuf loaders. Not an oversight, and not a few
# megabytes saved on a hunch: the app names no icon and loads no image — a
# grep for `icon_name`, `IconTheme`, `Pixbuf` and `Image::` over
# `crates/tc-app/src` finds nothing — and the iconography GTK's own widgets
# use is compiled into libgtk-4-1.dll as a GResource. The smoke test below is
# what keeps that claim honest rather than merely stated.

# Both files below go out with CRLF endings. Not tidiness: `cmd.exe` reads a
# batch file line by line as it runs it, and bare LF endings are a documented
# way to get surprised by that; and whoever unzips this has no reason to
# expect a Unix text file in a Windows download.
crlf() {
    sed 's/$/\r/'
}

# The launcher, because the .exe on its own is a trap: double-clicked, it dies
# silently, and whoever unzipped it has no way to find out why.
crlf > "$staged/$LAUNCHER" <<LAUNCHER_EOF
@echo off
rem FerroCommander ${version}
rem
rem The app does not start under the renderer GTK picks for itself: it exits
rem about six seconds in with 0xC0000005, without ever presenting a window.
rem Set here rather than in the binary, so the code keeps GTK's own default
rem on the platforms where the default works. See docs/windows.md.
set "GSK_RENDERER=${RENDERER}"
"%~dp0${EXE}" %*
LAUNCHER_EOF

crlf > "$staged/$NOTES" <<NOTES_EOF
FerroCommander ${version} - Windows x86_64

Run ${LAUNCHER}, not ${EXE}.

The .cmd sets one environment variable the app needs in order to start at
all; started directly, the .exe exits after about six seconds without ever
showing a window. Everything the app needs is in this folder - nothing is
installed, and deleting the folder removes it.

https://github.com/pirx42/ferrocommander
NOTES_EOF

echo "=== checking what was built"
# The same three things the .deb checks, in the forms Windows offers them:
# nothing declared, nothing installed, and the one part that breaks silently
# rather than loudly.

# Nothing declared. Walked again, flat over the staged directory, rather than
# taken from the queue above: a check that reuses the traversal it is checking
# cannot catch that traversal being wrong.
missing=()
for file in "$staged"/*.exe "$staged"/*.dll; do
    while read -r dll; do
        if provided_by_msys2 "$dll" && [ ! -f "$staged/$dll" ]; then
            missing+=("$dll (needed by $(basename "$file"))")
        fi
    done < <(imports "$file")
done
if [ "${#missing[@]}" -gt 0 ]; then
    echo "FAIL: the bundle is short of what it imports:" >&2
    printf '  %s\n' "${missing[@]}" >&2
    exit 1
fi

# Nothing installed.
for path in "$EXE" "$LAUNCHER" "$NOTES" "$SCHEMAS"; do
    if [ ! -f "$staged/$path" ]; then
        echo "FAIL: $path is not in the bundle" >&2
        exit 1
    fi
done

# It starts, and stays started. The Windows counterpart of
# `desktop-file-validate`: what this catches does not announce itself, it just
# means nobody can run what was published.
echo "=== starting it for ${SMOKE_SECONDS}s"
log=$(mktemp)
status=0
# `timeout` rather than a background job and `kill -0`, which was the first
# way this was written and was wrong: a child that has exited but not been
# reaped still answers signal 0, so a dead app would have passed the check
# meant to catch it dying. Here the verdict is a number and there is nothing
# to get subtly right — 124 is `timeout` reporting it had to stop the app
# itself, which is the only outcome that means the app was still running.
#
# `--kill-after` the same span again, so a process that ignores the polite
# signal cannot leave a CI job hanging until the six-hour ceiling.
GSK_RENDERER="$RENDERER" timeout --kill-after="$SMOKE_SECONDS" "$SMOKE_SECONDS" \
    "$staged/$EXE" >"$log" 2>&1 || status=$?
if [ "$status" -ne 124 ]; then
    echo "FAIL: it stopped inside ${SMOKE_SECONDS}s (status $status)" >&2
    echo "--- what it printed ---" >&2
    cat "$log" >&2
    exit 1
fi
# GTK complains on stderr about things it then survives — an icon it could not
# find, a setting it fell back on. Not fatal, and not swallowed either:
# whoever changes what goes into the bundle should see what the app made of it.
if [ -s "$log" ]; then
    echo "--- what it printed while running ---"
    cat "$log"
fi
rm -f "$log"

echo "=== packaging"
# No version in the archive's name, for the .deb's reason: the release is
# rolling — one tag, replaced every commit — so the download URL has to be one
# somebody can put in a README and not revisit.
zip_name=ferrocommander-windows-x86_64.zip
(cd target/windows && rm -f "$zip_name" && zip -qr "$zip_name" "ferrocommander-${version}")

echo
echo "built    target/windows/$zip_name"
echo "version  $version"
echo "holds    $(find "$staged" -type f | wc -l) files, $(du -sh "$staged" | cut -f1)"
