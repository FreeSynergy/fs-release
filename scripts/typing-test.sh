#!/usr/bin/env bash
# Typing test: start an app under a virtual KWin (Wayland) with a given keyboard layout, type the layout-sensitive
# characters into a focused text field, then check that the text arrives, is visible, and how fast it shows.
# One script for the laptop and for the GitHub runner (actions/typing-test calls it).
#
#   scripts/typing-test.sh <binary> [app args...]
#
# Environment (one of TT_CLICK / TT_SCENARIO is required, with TT_SCENARIO also TT_FIELD):
#   TT_CLICK     "x,y"      where to click to focus the text field
#   TT_SCENARIO  file       bash fragment that defines focus(), using c "<inject command>" [secs] and shot NAME
#   TT_FIELD     "x,y,w,h"  the field's rectangle in the screenshot (default: a band of 60 px at the click point)
#   TT_LAYOUT    xkb layout (default de)      TT_SIZE  WxH (default 1280x800)
#   TT_MAX_MS    limit for the median key-to-picture time (default 8)
#   TT_OUT       result directory (default ./typing-test-out): result.json, junit.xml, shots/, logs
#   TT_SETTLE    seconds to wait for the first picture (default 7)
#   TT_INJECT    prebuilt fs-typing-inject (default: built with cargo into the user cache)
# Exit code: 0 all checks pass, 1 a check failed, 2 the test itself could not run.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TT="$HERE/typing-test"

[ $# -ge 1 ] || { echo "usage: typing-test.sh <binary> [app args...]" >&2; exit 2; }
BIN="$(readlink -f "$1")"; shift
[ -x "$BIN" ] || { echo "typing-test: $BIN is not executable" >&2; exit 2; }
[ -n "${TT_CLICK:-}${TT_SCENARIO:-}" ] || { echo "typing-test: set TT_CLICK=x,y or TT_SCENARIO" >&2; exit 2; }
[ -n "${TT_CLICK:-}${TT_FIELD:-}" ] || { echo "typing-test: with TT_SCENARIO also set TT_FIELD=x,y,w,h" >&2; exit 2; }
if [ -n "${TT_SCENARIO:-}" ]; then
  TT_SCENARIO="$(readlink -f "$TT_SCENARIO")"
  export TT_SCENARIO
fi
for t in kwin_wayland dbus-run-session spectacle python3; do
  command -v "$t" >/dev/null || { echo "typing-test: $t is missing" >&2; exit 2; }
done

OUT="$(mkdir -p "${TT_OUT:-typing-test-out}" && cd "${TT_OUT:-typing-test-out}" && pwd)"
rm -rf "$OUT/shots" "$OUT"/*.log "$OUT/result.json" "$OUT/junit.xml"
mkdir -p "$OUT/shots"

if [ -z "${TT_INJECT:-}" ]; then
  cache="${XDG_CACHE_HOME:-$HOME/.cache}/fs-typing-inject"
  CARGO_TARGET_DIR="$cache" cargo build --release --locked --quiet --manifest-path "$HERE/typing-test/inject/Cargo.toml"
  TT_INJECT="$cache/release/fs-typing-inject"
fi
TT_INJECT="$(readlink -f "$TT_INJECT")"

# KWin only lets a program use fake input when its desktop file says so; the file points at the inject binary.
mkdir -p "$OUT/.xdg/applications"
cat > "$OUT/.xdg/applications/fs-typing-inject.desktop" <<DESK
[Desktop Entry]
Type=Application
Name=fs-typing-inject
Exec=$TT_INJECT
X-KDE-Wayland-Interfaces=org_kde_kwin_fake_input
NoDisplay=true
DESK

SIZE="${TT_SIZE:-1280x800}"
export TT_OUT="$OUT" TT_BIN="$BIN" TT_INJECT TT_LAYOUT="${TT_LAYOUT:-${LAYOUT:-de}}" TT_SIZE="$SIZE"
TT_APP_ARGS=""
for a in "$@"; do TT_APP_ARGS+="$a"$'\x1f'; done
export TT_APP_ARGS
export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}"
export XDG_DATA_DIRS="$OUT/.xdg:${XDG_DATA_DIRS:-/usr/local/share:/usr/share}"
export XKB_DEFAULT_LAYOUT="$TT_LAYOUT"
export KWIN_SCREENSHOT_NO_PERMISSION_CHECKS=1
# Without a GPU (GitHub runner) KWin composes in software.
[ -e /dev/dri/renderD128 ] || export KWIN_COMPOSE="${KWIN_COMPOSE:-Q}"

rc=0
timeout "${TT_TIMEOUT:-300}" dbus-run-session -- kwin_wayland --virtual --width "${SIZE%x*}" --height "${SIZE#*x}" \
  --socket tt-probe --no-lockscreen --exit-with-session "$TT/session.sh" \
  > "$OUT/kwin.log" 2>&1 || rc=$?
[ -s "$OUT/.session-done" ] || { echo "typing-test: session did not finish (kwin exit $rc), see $OUT/kwin.log" >&2; tail -20 "$OUT/kwin.log" >&2; exit 2; }

exec python3 -I "$TT/analyze.py" "$OUT"
