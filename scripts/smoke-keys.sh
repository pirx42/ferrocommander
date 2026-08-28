#!/usr/bin/env bash
# Drives the built app with real keystrokes on a virtual display and checks
# what actually happened on disk.
#
# Why this exists: unit tests cannot reach the layer where every bug in this
# project has so far lived — the composition between GTK and the model. Phase
# 1 shipped three of them with a green suite. Here the X server really
# delivers the key, so a binding that is wired up wrong fails here.
#
#   Usage: scripts/smoke-keys.sh [path-to-binary]
#   Needs: Xvfb, xdotool, ImageMagick   (apt: xvfb xdotool imagemagick)
#
# Three details cost time to find and are the reason this is a file rather
# than a paragraph in a doc:
#
#   * There is no window manager on a bare Xvfb, so `xdotool windowactivate`
#     fails with "_NET_ACTIVE_WINDOW not supported". `windowfocus` works.
#   * `xdotool key --window` sends XSendEvent, which GTK ignores. Plain
#     `xdotool key` uses XTEST and arrives as a real key press.
#   * The app is single-instance. On a real session bus it hands off to the
#     running instance and exits 0, so the run tests nothing; an unreachable
#     DBUS_SESSION_BUS_ADDRESS forces a private instance.
set -uo pipefail

BINARY="$(cd "$(dirname "$0")/.." && pwd)/${1:-target/release/tc-app}"
DISPLAY_NUM=":99"
STARTUP_WAIT=6      # the first frame on software rendering is not instant
STEP_WAIT=1.2       # after each keystroke: dialogs and jobs need a moment

WORK="$(mktemp -d)"
export HOME="$WORK/home"
export DBUS_SESSION_BUS_ADDRESS="unix:path=/nope"
mkdir -p "$HOME/src" "$HOME/dst"
echo "workdir: $WORK"

echo "hello from ferrocommander" > "$HOME/src/notes.txt"
echo "already here"              > "$HOME/dst/notes.txt"   # forces a conflict

cleanup() {
    pkill -f "$BINARY" 2>/dev/null
    pkill -f "Xvfb $DISPLAY_NUM" 2>/dev/null
    sleep 1
    # Re-check rather than trust the kill.
    if pgrep -f "Xvfb $DISPLAY_NUM" >/dev/null; then echo "WARNING: Xvfb survived"; fi
}
trap cleanup EXIT

Xvfb "$DISPLAY_NUM" -screen 0 1400x900x24 >"$WORK/xvfb.log" 2>&1 &
sleep 2
export DISPLAY="$DISPLAY_NUM"
"$BINARY" >"$WORK/app.log" 2>&1 &
sleep "$STARTUP_WAIT"

WINDOW="$(xdotool search --name Ferrocommander | head -1)"
if [ -z "$WINDOW" ]; then
    echo "FAIL: no window appeared"; cat "$WORK/app.log"; exit 1
fi

FAILURES=0
check() { local what="$1"; shift
    if "$@"; then echo "  ok   $what"
    else echo "  FAIL $what"; FAILURES=$((FAILURES + 1)); fi
}
focus_main() { xdotool windowfocus "$WINDOW"; sleep 0.4; }
focus_dialog() { # focus_dialog <window title regex>
    local id; id="$(xdotool search --name "$1" | head -1)"
    [ -n "$id" ] || { echo "  FAIL no dialog matching $1"; FAILURES=$((FAILURES + 1)); return 1; }
    xdotool windowfocus "$id"; sleep 0.4
}
k() { xdotool key "$1"; sleep "$STEP_WAIT"; }
shot() { import -window root "$WORK/$1.png" 2>/dev/null; }

# ------------------------------------------------ put the panes on src and dst
focus_main
k Tab; k Down; k Return          # right pane into dst (row 1 of `..`,dst,src)
k Tab; k Down; k Down; k Return  # left pane into src
shot 01-panes
check "the left pane is showing src" test -d "$HOME/src"

# --------------------------------------------------------- F7: make a directory
k F7
focus_dialog "New directory" && { xdotool type "made-by-f7"; sleep 0.4; k Return; }
check "F7 created the directory in the active pane's directory" test -d "$HOME/src/made-by-f7"

# ------------------------------------------- F5: copy onto an existing name
k Home; k Down; k Down; k Down   # `..`, made-by-f7, notes.txt
k F5
focus_dialog "^Copy$" && k Return
shot 02-conflict
# Skip starts focused, so Enter is the answer that loses nothing.
focus_dialog "already exists" && k Return
check "Skip left the existing target untouched" \
    test "$(cat "$HOME/dst/notes.txt")" = "already here"
check "Skip left the source untouched" \
    test "$(cat "$HOME/src/notes.txt")" = "hello from ferrocommander"

# ------------------------------------------------------- F8: delete to trash
focus_main
k Home; k Down                   # onto made-by-f7
k F8
shot 03-delete
focus_dialog "Confirm delete" && k Return
check "F8 removed the entry from its directory" test ! -e "$HOME/src/made-by-f7"
check "F8 put it in the trash, recoverably" \
    test -e "$HOME/.local/share/Trash/files/made-by-f7"

echo
if [ "$FAILURES" -eq 0 ]; then echo "PASS — screenshots in $WORK"; exit 0; fi
echo "$FAILURES check(s) failed — screenshots and logs in $WORK"; exit 1
