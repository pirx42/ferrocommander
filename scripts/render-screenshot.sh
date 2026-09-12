#!/usr/bin/env bash
# Renders the screenshot the README shows: the real binary, on a private X
# server, driven with real key presses -- the same way the end-to-end suite
# drives it (docs/ui-shell.md). A screenshot posed by hand stops being true the
# moment the UI moves and nobody notices; this one can be remade in a minute.
#
#   scripts/render-screenshot.sh [fixture-root] [output.png]
#
# Wants xvfb, xdotool and ImageMagick -- the first two are what the UI tests
# already need, so a machine that can run the gate can run this.
set -euo pipefail

REPO=$(cd "$(dirname "$0")/.." && pwd)
ROOT=${1:-/home/pirx}
OUT=${2:-$REPO/assets/screenshot-panes.png}

# The fixture root is the home directory the picture claims to be taken in, and
# its path is *visible* in the image -- which is why it defaults to a name
# rather than to a temporary directory nobody would want to read. That makes it
# a directory somebody may already own, so the script writes into it only when
# it made it itself: a marker from an earlier run, or nothing there at all.
MARKER=$ROOT/.fc-screenshot-fixture
if [ -e "$ROOT" ] && [ ! -e "$MARKER" ]; then
  echo "refusing to write into $ROOT: it exists and no earlier run made it." >&2
  echo "pass a different fixture root, or remove that directory first." >&2
  exit 1
fi

DISPLAY_NUM=${FC_SHOT_DISPLAY:-:97}
WINDOW_WIDTH=1400
WINDOW_HEIGHT=470
# The screen is the window: with no window manager running there is nothing to
# place the window anywhere but the corner, and nothing to draw around it.
SCREEN=${WINDOW_WIDTH}x${WINDOW_HEIGHT}x24
# Rows are counted from the top of the listing, `..` being row 0. The key
# presses further down step through these, so the fixture and the numbers move
# together -- change one and the marks land on the wrong files.
ROW_CRATES=2
ROW_CARGO_TOML=7
ROW_README=11
ROW_CURSOR_RESTS=3
# How long each key press is given to land, and how long the finished pane is
# left alone before the shutter. The end-to-end harness polls for an effect
# rather than sleeping, which a picture cannot do: there is nothing to poll for
# when what you are waiting on is "the pane looks right". These two are what
# was enough here -- a pane still counting a folder photographs as a pane with
# no size in it.
SETTLE=0.35
CAPTURE_SETTLE=1.5
# A channel spread wider than this is a coloured pixel rather than a grey one,
# which is how the drive bar's lower edge is found without knowing the theme's
# accent colour. See `drive_bar_height`.
COLOUR_SPREAD=40
# Far enough down to pass the drive bar under any font size, and no further.
DRIVE_BAR_SEARCH_DEPTH=61

P=$ROOT/projects/ferrocommander
D=$ROOT/Downloads

echo "== fixture in $ROOT"
mkdir -p "$ROOT"
: > "$MARKER"
mkdir -p "$P"/{assets,crates/fc-core/src,crates/fc-core/tests,crates/fc-core/benches} \
         "$P"/{crates/fc-app/src,crates/fc-app/tests,crates/fc-shellmenu/src} \
         "$P"/{docs/skills,docs/plans,packaging,scripts} \
         "$D"/{archives,images,iso}

# Sizes come from `truncate`, so the 5 GB iso costs no disk and the folder count
# stays instant; mtimes are set one by one, because a listing where every row
# carries the same minute looks like exactly what it is.
f() { truncate -s "$2" "$1"; touch -d "$3" "$1"; }
d() { touch -d "$2" "$1"; }

while read -r name size stamp; do
  [ -z "$name" ] && continue
  f "$P/crates/fc-core/src/$name.rs" "$size" "$stamp"
done <<'FILES'
vfs 24118 2026-09-02 11:24
listing 18922 2026-08-30 09:41
ops 31204 2026-09-08 17:52
archive 22870 2026-09-04 14:03
search 15662 2026-08-31 20:18
rename 12440 2026-09-01 10:07
viewer 17318 2026-09-03 16:35
config 20064 2026-09-09 08:12
command 9882 2026-09-10 19:44
watch 8210 2026-08-29 13:50
sort 6714 2026-08-30 09:41
mark 5926 2026-08-30 22:16
path 11470 2026-09-02 11:24
progress 7358 2026-09-06 15:29
error 4602 2026-08-28 18:05
lib 3214 2026-09-10 19:44
FILES

while read -r name size stamp; do
  [ -z "$name" ] && continue
  f "$P/crates/fc-core/tests/$name.rs" "$size" "$stamp"
done <<'FILES'
ops_copy 41208 2026-09-08 17:52
ops_delete 27664 2026-09-07 12:31
archive_roundtrip 33910 2026-09-04 14:03
listing_sort 18204 2026-08-30 09:41
search_content 21778 2026-08-31 20:18
command 14330 2026-09-10 19:44
FILES

while read -r name size stamp; do
  [ -z "$name" ] && continue
  f "$P/crates/fc-app/src/$name.rs" "$size" "$stamp"
done <<'FILES'
main 48722 2026-09-11 21:06
pane 61340 2026-09-11 21:06
menu 23884 2026-09-09 18:47
dialogs 34116 2026-09-06 15:29
keymap 19550 2026-09-03 16:35
constants 21902 2026-09-11 21:06
actions 44018 2026-09-10 19:44
viewer 16744 2026-09-03 16:35
progress 12208 2026-09-06 15:29
command_line 10986 2026-09-10 19:44
clipboard 9174 2026-09-02 11:24
watcher 7460 2026-08-29 13:50
FILES

while read -r name size stamp; do
  [ -z "$name" ] && continue
  f "$P/docs/$name.md" "$size" "$stamp"
done <<'FILES'
vfs 14208 2026-09-02 11:24
listing 11640 2026-08-30 09:41
ops 18332 2026-09-08 17:52
archives 12776 2026-09-04 14:03
search 8904 2026-08-31 20:18
multi-rename 7218 2026-09-01 10:07
viewer 9460 2026-09-03 16:35
ui-shell 16022 2026-09-11 21:06
keymap 25314 2026-09-10 19:44
config 19886 2026-09-09 08:12
packaging 15470 2026-09-11 07:33
windows 13208 2026-09-10 19:44
command-line 8842 2026-09-10 19:44
clipboard 6350 2026-09-02 11:24
watching 5904 2026-08-29 13:50
performance 17766 2026-09-08 17:52
reliability 14092 2026-09-07 12:31
future-improvements 9538 2026-09-11 21:06
FILES

while read -r path size stamp; do
  [ -z "$path" ] && continue
  f "$ROOT/$path" "$size" "$stamp"
done <<'FILES'
projects/ferrocommander/crates/fc-core/benches/listing.rs 6120 2026-09-05 09:58
projects/ferrocommander/crates/fc-app/tests/ui.rs 214733 2026-09-11 21:06
projects/ferrocommander/crates/fc-shellmenu/src/lib.rs 4118 2026-09-09 18:47
projects/ferrocommander/crates/fc-shellmenu/src/shell.rs 21604 2026-09-09 18:47
projects/ferrocommander/crates/CLAUDE.md 6842 2026-09-09 18:47
projects/ferrocommander/docs/skills/01-clarify-with-options.md 2480 2026-08-28 18:05
projects/ferrocommander/docs/skills/10-plan-lifecycle.md 3104 2026-08-28 18:05
projects/ferrocommander/docs/skills/25-green-suite-before-commit.md 2866 2026-08-28 18:05
projects/ferrocommander/docs/skills/44-no-redundancy.md 1920 2026-08-28 18:05
projects/ferrocommander/docs/skills/65-verify-or-ask-never-assume.md 2344 2026-08-28 18:05
projects/ferrocommander/docs/plans/2026-09-12-readme-screenshot.md 4820 2026-09-12 07:15
projects/ferrocommander/packaging/st.rose.Ferrocommander.desktop 412 2026-09-11 07:33
projects/ferrocommander/packaging/st.rose.Ferrocommander.ico 92062 2026-09-11 07:33
projects/ferrocommander/packaging/st.rose.Ferrocommander.icns 268511 2026-09-11 07:33
projects/ferrocommander/packaging/st.rose.Ferrocommander.svg 3140 2026-09-11 07:33
projects/ferrocommander/scripts/green-gate.sh 4820 2026-09-11 07:33
projects/ferrocommander/scripts/render-icons.py 7106 2026-09-11 07:33
projects/ferrocommander/scripts/render-screenshot.sh 6402 2026-09-12 07:15
projects/ferrocommander/scripts/check-links.py 2914 2026-08-28 18:05
projects/ferrocommander/scripts/check-naming.py 1866 2026-08-28 18:05
projects/ferrocommander/assets/screenshot-panes.png 184320 2026-09-12 07:15
projects/ferrocommander/Cargo.lock 38914 2026-09-09 18:47
projects/ferrocommander/Cargo.toml 412 2026-09-09 18:47
projects/ferrocommander/CHANGELOG.md 3908 2026-09-11 21:06
projects/ferrocommander/CLAUDE.md 11204 2026-09-11 21:06
projects/ferrocommander/LICENSE 1067 2026-08-28 18:05
projects/ferrocommander/README.md 6248 2026-09-12 07:15
projects/ferrocommander/rustfmt.toml 96 2026-08-28 18:05
Downloads/ferrocommander-linux-x86_64.deb 4980736 2026-09-11 22:14
Downloads/ferrocommander-macos-arm64.zip 41943040 2026-09-11 22:15
Downloads/ferrocommander-windows-x86_64.zip 66060288 2026-09-11 22:16
Downloads/gtk4-4.20.2.tar.xz 14680064 2026-08-24 10:02
Downloads/rust-book.pdf 8605696 2026-07-19 21:38
Downloads/notes.md 1432 2026-09-12 07:02
Downloads/archives/fc-core-src.tar.gz 184320 2026-09-05 09:58
Downloads/archives/docs-backup.zip 512000 2026-09-05 09:58
Downloads/images/panes.png 248320 2026-09-12 07:15
Downloads/images/icon-sheet.png 96256 2026-09-11 07:33
Downloads/iso/ubuntu-24.04.3-desktop-amd64.iso 6183256064 2026-08-14 16:20
FILES

chmod +x "$P/scripts/green-gate.sh" "$P/scripts/render-screenshot.sh"

# Directory mtimes last: writing a file into one puts the clock back on it.
d "$P/assets" "2026-09-12 07:15"
d "$P/crates" "2026-09-11 21:06"
d "$P/docs" "2026-09-12 07:15"
d "$P/packaging" "2026-09-11 07:33"
d "$P/scripts" "2026-09-11 07:33"
d "$D/archives" "2026-09-05 09:58"
d "$D/images" "2026-09-12 07:15"
d "$D/iso" "2026-08-14 16:20"

mkdir -p "$ROOT/.config/ferrocommander" "$ROOT/.local/share"
cat > "$ROOT/.config/ferrocommander/config.toml" <<CONFIG
active_pane = 0

[window]
width = $WINDOW_WIDTH
height = $WINDOW_HEIGHT

[[panes]]
directory = "$P"

[[panes]]
directory = "$D"
CONFIG

echo "== building"
cargo build --quiet -p fc-app --manifest-path "$REPO/Cargo.toml"
BINARY=$REPO/target/debug/ferrocommander

echo "== running on $DISPLAY_NUM"
Xvfb "$DISPLAY_NUM" -screen 0 "$SCREEN" >/dev/null 2>&1 &
XVFB=$!
trap 'kill "${APP:-}" "$XVFB" 2>/dev/null || true' EXIT
sleep 1
# `env -i` so the machine's own session -- its theme, its D-Bus, its data dirs --
# cannot reach in and change what the picture shows.
env -i DISPLAY="$DISPLAY_NUM" HOME="$ROOT" PATH="$PATH" GDK_BACKEND=x11 \
    DBUS_SESSION_BUS_ADDRESS=unix:path=/nonexistent \
    XDG_CONFIG_HOME="$ROOT/.config" XDG_DATA_HOME="$ROOT/.local/share" \
    XDG_DATA_DIRS=/usr/local/share:/usr/share \
    "$BINARY" >/dev/null 2>&1 &
APP=$!

X() { DISPLAY="$DISPLAY_NUM" xdotool "$@"; }
# `--onlyvisible`, because the window exists under its name for a moment before
# it is mapped, and focusing one that is not yet on screen is an X BadMatch.
for _ in $(seq 1 40); do
  WINDOW=$(X search --onlyvisible --name FerroCommander 2>/dev/null | head -1) || true
  [ -n "${WINDOW:-}" ] && break
  sleep 0.5
done
[ -n "${WINDOW:-}" ] || { echo "the app never put up a window" >&2; exit 1; }
sleep 2
for _ in $(seq 1 10); do
  X windowfocus "$WINDOW" 2>/dev/null && break
  sleep 0.5
done
sleep 0.5

press() { for _ in $(seq 1 "$2"); do X key --clearmodifiers "$1"; sleep "$SETTLE"; done; }

echo "== marking"
press Down "$ROW_CRATES"
press space 1
press Down $((ROW_CARGO_TOML - ROW_CRATES))
press space 1
press Down $((ROW_README - ROW_CARGO_TOML))
press space 1
press Up $((ROW_README - ROW_CURSOR_RESTS))
sleep "$CAPTURE_SETTLE"

echo "== capturing to $OUT"
mkdir -p "$(dirname "$OUT")"
RAW=$(mktemp --suffix=.png)
DISPLAY="$DISPLAY_NUM" import -window "$WINDOW" "$RAW"

# The drive bar is cropped away. Not for looks: a drive button is labelled with
# the mount it stands for, so the bar photographs whatever the machine that took
# the picture happens to have mounted -- which on a build container is the build
# container. The bar is found rather than measured, so a theme with a taller one
# does not shift the crop into the listing: the first coloured row down the left
# edge is the top of the path bar, everything above it is the drive bar.
drive_bar_height() {
  convert "$RAW" -crop "1x${DRIVE_BAR_SEARCH_DEPTH}+4+0" +repage -depth 8 txt:- |
    awk -v spread="$COLOUR_SPREAD" '
      $1 ~ /^[0-9]+,[0-9]+:$/ {
        split($1, pos, ",")
        rgb = $2
        gsub(/[()]/, "", rgb)
        split(rgb, c, ",")
        hi = c[1] + 0; lo = c[1] + 0
        for (i = 2; i <= 3; i++) {
          if (c[i] + 0 > hi) hi = c[i] + 0
          if (c[i] + 0 < lo) lo = c[i] + 0
        }
        if (hi - lo > spread) { print pos[2] + 0; exit }
      }'
}
TOP=$(drive_bar_height)
[ -n "$TOP" ] || { echo "could not find the drive bar's edge" >&2; exit 1; }
convert "$RAW" -crop "${WINDOW_WIDTH}x$((WINDOW_HEIGHT - TOP))+0+${TOP}" +repage "$OUT"
rm -f "$RAW"
echo "== done: $(identify -format '%wx%h' "$OUT") at $OUT"
