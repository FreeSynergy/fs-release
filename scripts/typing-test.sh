#!/usr/bin/env bash
# Typing test: start an app under a virtual KWin (Wayland) with a given keyboard layout, type the layout-sensitive
# characters into a focused text field, then check that the text arrives, is visible, and how fast it shows.
# One script for the laptop and for the GitHub runner (actions/typing-test calls it).
#
#   scripts/typing-test.sh <binary> [app args...]
#
# Environment (one of TT_CLICK_REL, TT_CLICK, TT_SCENARIO is required):
#   TT_CLICK_REL "x,y"      where to click to focus the field, from the top left corner of the app window (decoration
#                           included): does not depend on where KWin places the window
#   TT_CLICK     "x,y"      the same on the screen; TT_SCENARIO: bash fragment defining focus() (see session.sh)
#   TT_FIELD     "x,y,w,h"  the field's rectangle on screen (default: found inside the field around the click: blinking caret, else a blue bar)
#   TT_TEXT_WIDTH           pixels of text area right of the caret (default 110; the characters of a layout go in parts of eight)
#   TT_LAYOUT    xkb layout, de or us (default de; the characters typed: scripts/typing-test/chars-<layout>.txt)
#   TT_INJECT_LAYOUT  layout the injector types for (default TT_LAYOUT; set another to make the test red on purpose)
#   TT_SIZE  WxH (default 1280x800)
#   TT_MAX_MS    limit for the median key-to-picture time (default 8). Without a GPU the compositor sets the pace,
#                so there the time is reported but not enforced (TT_ENFORCE_MS=1 enforces it).
#   TT_OUT       result directory (default ./typing-test-out): result.json, junit.xml, shots/, logs
#   TT_SETTLE    seconds to wait at most for the app window (default 30)
#   TT_REF_FONT  font file the typed glyphs are compared with (default: DejaVu Sans)
#   TT_PHASE     "latency": only type a text and count stalls (typing-test/stau.py), no pictures
#   TT_BACKEND   virtual (default) or nested (a window in the running compositor, no input goes to the host)
#   TT_SOCKET    Wayland socket name (default tt-<pid>, so tests can run side by side)
#   TT_INJECT    prebuilt fs-typing-inject (default: built with cargo into the user cache; edition 2024, needs crates)
# The test runs with its own runtime directory, a private D-Bus, no DISPLAY and the toolkits set to Wayland only.
# Exit code: 0 all checks pass, 1 a check failed, 2 the test itself could not run.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TT="$HERE/typing-test"
# shellcheck source=virtual-session.sh
. "$HERE/virtual-session.sh"
vs_capture_real   # the real session's values, before this script replaces the environment

[ $# -ge 1 ] || { echo "usage: typing-test.sh <binary> [app args...]" >&2; exit 2; }
BIN="$(readlink -f "$1")"; shift
[ -x "$BIN" ] || { echo "typing-test: $BIN is not executable" >&2; exit 2; }
[ -n "${TT_CLICK:-}${TT_CLICK_REL:-}${TT_SCENARIO:-}" ] || { echo "typing-test: set TT_CLICK=x,y, TT_CLICK_REL=x,y or TT_SCENARIO" >&2; exit 2; }
if [ -n "${TT_SCENARIO:-}" ]; then
  TT_SCENARIO="$(readlink -f "$TT_SCENARIO")"
  export TT_SCENARIO
fi
for t in kwin_wayland dbus-run-session spectacle python3; do
  command -v "$t" >/dev/null || { echo "typing-test: $t is missing" >&2; exit 2; }
done

OUT="$(mkdir -p "${TT_OUT:-typing-test-out}" && cd "${TT_OUT:-typing-test-out}" && pwd)"
rm -rf "$OUT/shots" "$OUT"/*.log "$OUT/result.json" "$OUT/junit.xml" "$OUT"/.session-*
mkdir -p "$OUT/shots"

TT_LAYOUT="${TT_LAYOUT:-${LAYOUT:-de}}"
case "$TT_LAYOUT" in de | us) ;; *) echo "typing-test: layout '$TT_LAYOUT' has no character list and no key table; there are: de, us" >&2; exit 2 ;; esac
case "${TT_INJECT_LAYOUT:-$TT_LAYOUT}" in de | us) ;; *) echo "typing-test: inject layout '${TT_INJECT_LAYOUT}' has no key table; there are: de, us" >&2; exit 2 ;; esac
if [ -z "${TT_INJECT:-}" ]; then
  # One build directory per state of the injector's sources, so another branch's binary (one without `inject table`) is never taken
  # and worktrees side by side do not disturb each other.
  key="$(cat "$HERE/typing-test/inject/Cargo.toml" "$HERE/typing-test/inject/Cargo.lock" "$HERE/typing-test/inject/src/"*.rs | sha256sum | cut -c1-16)"
  cache="${XDG_CACHE_HOME:-$HOME/.cache}/fs-typing-inject/$key"
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

# The field's rectangle on screen as x0,y0,x1,y1: TT_FIELD, else 300x40 around the click point (a fallback; the
# session finds the field from the edges around the click, the caret inside it: blinking, else a blue bar).
if [ -n "${TT_FIELD:-}" ]; then
  IFS=, read -r fx fy fw fh <<<"$TT_FIELD"
elif [ -n "${TT_CLICK:-}" ]; then
  IFS=, read -r cx cy <<<"$TT_CLICK"
  fx=$((cx - 150)); fy=$((cy - 20)); fw=300; fh=40
else
  fx=0; fy=0; fw=1; fh=1  # replaced by what the caret shows
fi
export TT_RECT="$fx,$fy,$((fx + fw)),$((fy + fh))"

SIZE="${TT_SIZE:-1280x800}"
TT_APP_ARGS=""
for a in "$@"; do TT_APP_ARGS+="$a"$'\x1f'; done
export TT_APP_ARGS

# Shut the test off from the user's own session: its own runtime directory and Wayland socket, no DISPLAY, and the
# toolkits told to use Wayland only, so nothing opens in the real session and no input reaches it.
HOST_SOCKET=""
if [ "${TT_BACKEND:-virtual}" = nested ]; then
  HOST_SOCKET="${TT_HOST_DISPLAY:-${VS_REAL_WAYLAND_DISPLAY:-wayland-0}}"
  case "$HOST_SOCKET" in /*) ;; *) HOST_SOCKET="${VS_REAL_XDG_RUNTIME_DIR:-/run/user/$(id -u)}/$HOST_SOCKET" ;; esac
fi
RUNTIME="$(mktemp -d "${TMPDIR:-/tmp}/tt-run.XXXXXX")"
unset DISPLAY WAYLAND_DISPLAY WAYLAND_SOCKET XAUTHORITY DBUS_SESSION_BUS_ADDRESS
export XDG_RUNTIME_DIR="$RUNTIME" WINIT_UNIX_BACKEND=wayland GDK_BACKEND=wayland QT_QPA_PLATFORM=wayland SDL_VIDEODRIVER=wayland
export TT_SOCKET="${TT_SOCKET:-tt-$$}" TT_DIR="$TT" TT_OUT="$OUT" TT_BIN="$BIN" TT_INJECT TT_LAYOUT="${TT_LAYOUT:-${LAYOUT:-de}}" TT_SIZE="$SIZE"
export XDG_DATA_DIRS="$OUT/.xdg:${XDG_DATA_DIRS:-/usr/local/share:/usr/share}"
export XKB_DEFAULT_LAYOUT="$TT_LAYOUT"
# A laptop's KWin takes the layout from the user's kxkbrc, not from XKB_DEFAULT_LAYOUT: give it a config of its own.
mkdir -p "$RUNTIME/config"
printf '[Layout]\nLayoutList=%s\nUse=true\nVariantList=\n' "$TT_LAYOUT" > "$RUNTIME/config/kxkbrc"
# The app must not write into the real home either: HOME and the XDG directories live in the run directory.
mkdir -p "$RUNTIME/home" "$RUNTIME/data" "$RUNTIME/cache"
export HOME="$RUNTIME/home" XDG_DATA_HOME="$RUNTIME/data" XDG_CACHE_HOME="$RUNTIME/cache" XDG_CONFIG_HOME="$RUNTIME/config"
vs_guard no-bus || exit 2   # the private bus does not exist yet: session.sh checks it too
export KWIN_SCREENSHOT_NO_PERMISSION_CHECKS=1
# Without a GPU (GitHub runner) KWin composes with OpenGL 2 in software (llvmpipe); its screenshot service needs OpenGL.
if [ ! -e /dev/dri/renderD128 ]; then
  export LIBGL_ALWAYS_SOFTWARE=1 KWIN_COMPOSE="${TT_KWIN_COMPOSE:-O2}" TT_SOFTWARE=1
fi

# TT_BACKEND=nested: KWin as a window inside the running compositor (real frame callbacks) instead of a virtual
# screen. Only this KWin gets the injected input; the host session gets none, but sees a window.
if [ -n "$HOST_SOCKET" ]; then backend=(--wayland-display "$HOST_SOCKET"); else backend=(--virtual); fi
rc=0
# Own process group, so that nothing of the test (KWin, the app, the injector) outlives it.
setsid timeout "${TT_TIMEOUT:-600}" dbus-run-session -- kwin_wayland "${backend[@]}" --width "${SIZE%x*}" --height "${SIZE#*x}" \
  --socket "$TT_SOCKET" --no-lockscreen --exit-with-session "$TT/session.sh" > "$OUT/kwin.log" 2>&1 &
KPID=$!
cleanup() { kill -TERM -- "-$KPID" 2>/dev/null || true; sleep 0.5; kill -KILL -- "-$KPID" 2>/dev/null || true; sleep 0.3; rm -rf "$RUNTIME"; }
trap cleanup EXIT
trap 'exit 130' INT TERM
wait "$KPID" || rc=$?
[ -s "$OUT/.session-failed" ] && { echo "typing-test: $(cat "$OUT/.session-failed")" >&2; exit 2; }
[ -s "$OUT/.session-done" ] || { echo "typing-test: session did not finish (kwin exit $rc), see $OUT/kwin.log" >&2; tail -20 "$OUT/kwin.log" >&2; exit 2; }

if [ "${TT_PHASE:-all}" = latency ]; then python3 -I "$TT/stau.py" "$OUT"; else python3 -I "$TT/analyze.py" "$OUT"; fi
