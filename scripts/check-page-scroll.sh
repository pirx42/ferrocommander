#!/usr/bin/env bash
# Does a Page Down look the way it looked when the widget did the paging?
#
# The end-to-end suite cannot answer this. It asserts on the filesystem, so it
# can say which *row* a page landed on and never where that row sits on the
# screen — and a down-and-back-up test cannot see the overlap either, because
# both directions use the same step and cancel out. The arithmetic is pinned
# by a unit test (`a_page_is_a_screenful_less_the_row_that_carries_over`);
# this is the only check there is on the appearance.
#
# What to look for: `page-1.png` must share its top row with `start.png`'s
# bottom row, and `page-2.png` with `page-1.png`'s. That shared row is the
# overlap the widget had, and reproducing it was the requirement.
#
#   scripts/check-page-scroll.sh && xdg-open "${TMPDIR:-/tmp}"/ferrocommander-page-check/page-1.png
#
# Needs a debug build, Xvfb, xdotool and ImageMagick's `import`.
set -euo pipefail

DISPLAY_NUM="${DISPLAY_NUM:-:111}"
OUT="${OUT:-${TMPDIR:-/tmp}/ferrocommander-page-check}"
BIN="${BIN:-$(dirname "$0")/../target/debug/ferrocommander}"

rm -rf "$OUT"; mkdir -p "$OUT"
HOME_DIR="$OUT/home"
mkdir -p "$HOME_DIR/src"
# Numbered so a screenshot says which rows are on screen at a glance, and
# enough of them that three pages do not reach the end.
for i in $(seq -w 1 120); do echo x > "$HOME_DIR/src/row-$i.txt"; done

Xvfb "$DISPLAY_NUM" -screen 0 1400x900x24 >"$OUT/xvfb.log" 2>&1 &
XVFB_PID=$!
trap 'kill -9 $XVFB_PID ${APP_PID:-} 2>/dev/null || true' EXIT
for _ in $(seq 1 100); do
  DISPLAY="$DISPLAY_NUM" xdotool getdisplaygeometry >/dev/null 2>&1 && break
  sleep 0.1
done

DISPLAY="$DISPLAY_NUM" HOME="$HOME_DIR" \
  DBUS_SESSION_BUS_ADDRESS=unix:path=/nope GDK_BACKEND=x11 \
  XDG_CONFIG_HOME="$HOME_DIR/.config" XDG_DATA_HOME="$HOME_DIR/.local/share" \
  "$BIN" >"$OUT/app.log" 2>&1 &
APP_PID=$!

WIN=""
for _ in $(seq 1 300); do
  WIN=$(DISPLAY="$DISPLAY_NUM" xdotool search --name FerroCommander 2>/dev/null | head -1 || true)
  [ -n "$WIN" ] && break
  sleep 0.1
done
[ -n "$WIN" ] || { echo "no window"; cat "$OUT/app.log"; exit 1; }
sleep 1
for _ in $(seq 1 20); do
  DISPLAY="$DISPLAY_NUM" xdotool windowfocus "$WIN" 2>/dev/null && break
  sleep 0.3
done
sleep 0.5

key() { DISPLAY="$DISPLAY_NUM" xdotool key "$1"; sleep "${2:-0.3}"; }
shoot() { DISPLAY="$DISPLAY_NUM" import -window root "$OUT/$1.png"; }

key Down; key Return 0.8           # into src
key Home 0.5
shoot start
key Next 0.6; shoot page-1
key Next 0.6; shoot page-2
key Prior 0.6; shoot back-1        # must look like page-1
key Prior 0.6; shoot back-2        # must look like start

echo "wrote $OUT/{start,page-1,page-2,back-1,back-2}.png"
echo "look for: the top row of page-1 is the bottom row of start, and back-2 == start"
