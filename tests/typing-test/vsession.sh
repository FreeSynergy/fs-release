#!/usr/bin/env bash
# The virtual-session guard (scripts/virtual-session.sh): refuses the real bus / real Wayland / empty environment,
# lets a private session through, and its wrappers run the guard before the tool.
#   tests/typing-test/vsession.sh
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
vs="$here/../../scripts/virtual-session.sh"
work="$(mktemp -d /tmp/vs-test.XXXXXX)"
trap 'rm -rf "$work"' EXIT
fail=0
uid="$(id -u)"
rt="$work/rt"; mkdir -p "$rt" "$work/bin"
# GUARD [VAR=value ...]: run `virtual-session.sh guard` in an otherwise EMPTY environment.
GUARD() { env -i PATH="$PATH" "$@" bash "$vs" guard; }
want() { # want NAME EXPECTED_RC OUTPUT_NEEDLE -- cmd...
  local name="$1" rc="$2" needle="$3"; shift 4
  local out got; out="$("$@" 2>&1)"; got=$?
  if [ "$got" = "$rc" ] && { [ -z "$needle" ] || grep -qF -- "$needle" <<<"$out"; }; then echo "ok   $name"
  else echo "FAIL $name: exit $got (wanted $rc), output: $out"; fail=1; fi
}
PRIV="XDG_RUNTIME_DIR=$rt DBUS_SESSION_BUS_ADDRESS=unix:path=/tmp/dbus-abc,guid=1"
# shellcheck disable=SC2086
want "(a) the real bus address is refused" 97 "REAL session bus" -- GUARD XDG_RUNTIME_DIR="$rt" DBUS_SESSION_BUS_ADDRESS="unix:path=/run/user/$uid/bus"
want "(a2) the real runtime dir is refused" 97 "REAL session" -- GUARD XDG_RUNTIME_DIR="/run/user/$uid" DBUS_SESSION_BUS_ADDRESS="unix:path=/tmp/dbus-abc"
want "(a3) an empty bus (default fallback) is refused" 97 "empty" -- GUARD XDG_RUNTIME_DIR="$rt"
want "(a4) the real bus captured before the start is refused" 97 "REAL session bus" -- GUARD XDG_RUNTIME_DIR="$rt" DBUS_SESSION_BUS_ADDRESS="unix:abstract=/tmp/dbus-real" VS_REAL_DBUS="unix:abstract=/tmp/dbus-real"
# shellcheck disable=SC2086
want "(b) the private bus passes" 0 "" -- GUARD $PRIV
want "(b2) a private bus inside the runtime dir passes" 0 "" -- GUARD XDG_RUNTIME_DIR="$rt" DBUS_SESSION_BUS_ADDRESS="unix:path=$rt/bus"
# shellcheck disable=SC2086
want "(b3) the virtual session's own Wayland socket passes" 0 "" -- GUARD $PRIV WAYLAND_DISPLAY=tt-1
# shellcheck disable=SC2086
want "(c) the real session's WAYLAND_DISPLAY is refused" 97 "WAYLAND_DISPLAY" -- GUARD $PRIV WAYLAND_DISPLAY=wayland-0 VS_REAL_WAYLAND_DISPLAY=wayland-0
# shellcheck disable=SC2086
want "(c2) an absolute socket outside the runtime dir is refused" 97 "outside" -- GUARD $PRIV WAYLAND_DISPLAY="/run/user/$uid/wayland-0"
# shellcheck disable=SC2086
want "(c3) WAYLAND_SOCKET is refused" 97 "WAYLAND_SOCKET" -- GUARD $PRIV WAYLAND_SOCKET=3
# shellcheck disable=SC2086
want "(c4) DISPLAY is refused" 97 "DISPLAY" -- GUARD $PRIV DISPLAY=:0
want "(d) an empty environment is refused" 97 "empty" -- GUARD
want "(d2) a relative runtime dir is refused" 97 "absolute" -- GUARD XDG_RUNTIME_DIR=rt DBUS_SESSION_BUS_ADDRESS="unix:path=/tmp/dbus-abc"
want "(d3) a runtime dir outside /tmp and the scratch dir is refused" 97 "not below" -- GUARD XDG_RUNTIME_DIR=/var/tmp DBUS_SESSION_BUS_ADDRESS="unix:path=/tmp/dbus-abc"
# The wrappers: guard first, the tool only when it passes.
for t in dbus-send gdbus qdbus6 xdotool; do printf '#!/bin/sh\necho "RAN %s $*" >> "%s/ran"\n' "$t" "$work" > "$work/bin/$t"; chmod +x "$work/bin/$t"; done
WRAP() { env -i PATH="$work/bin:$PATH" "$@" bash -c '. "$1"; vs_dbus a; vs_gdbus b; vs_qdbus c; vs_run xdotool d' x "$vs"; }
: > "$work/ran"
want "wrappers refuse on the real bus" 97 "REFUSED" -- WRAP XDG_RUNTIME_DIR="$rt" DBUS_SESSION_BUS_ADDRESS="unix:path=/run/user/$uid/bus"
want "wrappers do not start a tool when refused" 0 "" -- test ! -s "$work/ran"
# shellcheck disable=SC2086
want "wrappers run every tool on the private bus" 0 "" -- WRAP $PRIV
for t in "dbus-send a" "gdbus b" "qdbus6 c" "xdotool d"; do
  want "wrapper ran: $t" 0 "" -- grep -q "RAN $t" "$work/ran"
done
cat "$work/ran" >&2; exit "$fail"
