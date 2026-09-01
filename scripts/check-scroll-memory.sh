#!/usr/bin/env bash
# Who is allowed to move a pane's viewport?
#
# The end-to-end suite cannot answer that: a scroll offset is not a window
# title, a file on disk or a key press, so `xdotool` cannot see it. This
# drives the real app on a private X server and leaves screenshots to look
# at — the only check there is for either of the two rules below.
#
# Both share one set-up, and it is the case the cursor cannot explain: the
# cursor stays on the *first* row while the view is wheeled far down, so
# anything that scrolls back to the cursor shows the top.
#
#   returned.png   the per-directory scroll memory. `dir-21` means a pane
#                  came back where it was left; `dir-01` means it did not.
#   after-space.png   marking does not scroll. `dir-21` with the status line
#                  reading `1 of 60 selected` means `Space` marked the
#                  off-screen cursor row and left the view alone; `dir-01`
#                  means the list jumped to the cursor, which it did until
#                  2026-09-01 for every marking key, not just this one.
#   after-watch.png   a re-read does not scroll. A file appears in the
#                  directory from outside, the watcher notices, and the rows
#                  are brought up to date — `dir-21` still at the top means
#                  the view stayed; anything else means the rebuild took the
#                  list's scroll anchor with it, which is what made a pane
#                  jump after a copy and after somebody else's change.
#
#   scripts/check-scroll-memory.sh && xdg-open "${TMPDIR:-/tmp}"/ferrocommander-scroll-check/returned.png
#
# **What it does not check is the flicker**: coming back used to paint one
# frame at the top before the remembered offset landed, and one frame is
# shorter than a screenshot. That was verified by logging every painted
# frame's offset from inside the app — 39 px, then 828 — which is
# instrumentation rather than a check, and is written up in docs/ui-shell.md.
#
# Needs a debug build, Xvfb, xdotool and ImageMagick's `import`.
set -euo pipefail

DISPLAY_NUM="${DISPLAY_NUM:-:110}"
OUT="${OUT:-${TMPDIR:-/tmp}/ferrocommander-scroll-check}"
BIN="${BIN:-$(dirname "$0")/../target/debug/ferrocommander}"

rm -rf "$OUT"; mkdir -p "$OUT"
HOME_DIR="$OUT/home"
# Directories sort first, so a *directory* at the bottom needs sixty of them:
# `End` lands on the last one and entering it is what the memory is about.
for i in $(seq -w 1 60); do mkdir -p "$HOME_DIR/src/dir-$i"; done
echo inner > "$HOME_DIR/src/dir-60/inner.txt"

Xvfb "$DISPLAY_NUM" -screen 0 1400x900x24 >"$OUT/xvfb.log" 2>&1 &
XVFB_PID=$!
trap 'kill -9 $XVFB_PID $APP_PID 2>/dev/null || true' EXIT
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

key() { DISPLAY="$DISPLAY_NUM" xdotool key "$1"; sleep "${2:-0.2}"; }
shoot() { DISPLAY="$DISPLAY_NUM" import -window root "$OUT/$1.png"; }

key Down; key Return 0.6           # into src
key Down 0.3                       # onto dir-01, the first real row
# Cursor on dir-01, at the top. Then scroll the *view* far down with the
# wheel, leaving the cursor where it is: this is the case the cursor cannot
# explain, because coming back to a cursor at the top would show the top.
DISPLAY="$DISPLAY_NUM" xdotool mousemove 300 400
for _ in $(seq 1 12); do DISPLAY="$DISPLAY_NUM" xdotool click 5; done
sleep 0.5
shoot left-scrolled
key Return 0.8                     # into dir-01, which the cursor is still on
shoot inside
key BackSpace 0.9                  # back out
shoot returned

# And the second rule, from the same place: the view is far from the cursor
# again (the memory just put it there), so a `Space` that scrolls is a `Space`
# that moved the viewport for a keystroke that moved nothing.
key space 0.8
shoot after-space

# And the third: somebody else changes the directory. The watcher re-reads it
# under the user, which must not be something the user can see happening.
echo appeared > "$HOME_DIR/src/zz-appeared.txt"
sleep 2
shoot after-watch

echo "wrote $OUT/{left-scrolled,inside,returned,after-space,after-watch}.png"
