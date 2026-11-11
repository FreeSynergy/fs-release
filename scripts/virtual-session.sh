#!/usr/bin/env bash
# Guard for virtual sessions: every test, spike and the typing test sources this ONE file, so nothing a script does
# with D-Bus, KWin, input or config can reach the user's REAL session (incident: a KWin script ran against the real bus).
#   source scripts/virtual-session.sh      then: vs_capture_real (BEFORE the virtual session replaces the environment),
#                                          vs_guard, vs_run <tool> args | vs_dbus | vs_qdbus | vs_gdbus
#   scripts/virtual-session.sh guard | run <tool> [args...]     the same as commands (exit 97 = refused; the values captured
#                                          by an earlier vs_capture_real are inherited as VS_REAL_*)
# Pattern: Guard (Facade for the wrappers).
# vs_guard refuses (message on stderr, return/exit 97) unless ALL hold; it never guesses, an empty environment is refused:
#   - XDG_RUNTIME_DIR is set, absolute, an existing directory, NOT /run/user/*, not the real one captured by
#     vs_capture_real (VS_REAL_*), and below /tmp, $TMPDIR or /srv/FreeSynergy/.scratch (VS_ROOTS: more roots, ":"-separated)
#   - DBUS_SESSION_BUS_ADDRESS is set and private: unix:path= inside that runtime dir or under a root (dbus-run-session
#     uses /tmp/dbus-*), or unix:abstract=/tmp/...; never /run/user/*, never the real address
#   - WAYLAND_DISPLAY (if set) is not the real one, and an absolute path lies inside the runtime dir; WAYLAND_SOCKET and
#     DISPLAY are unset (DISPLAY allowed only with VS_ALLOW_X=1 and when it is not the real one)
# `vs_guard no-bus` skips the bus check (before dbus-run-session has created the private bus).
# The real session is "/run/user/<uid>" plus the values of the caller's environment before the virtual session starts.

vs_capture_real() {
  VS_REAL_XDG_RUNTIME_DIR="${VS_REAL_XDG_RUNTIME_DIR-${XDG_RUNTIME_DIR:-}}"
  VS_REAL_DBUS="${VS_REAL_DBUS-${DBUS_SESSION_BUS_ADDRESS:-}}"
  VS_REAL_WAYLAND_DISPLAY="${VS_REAL_WAYLAND_DISPLAY-${WAYLAND_DISPLAY:-}}"
  VS_REAL_DISPLAY="${VS_REAL_DISPLAY-${DISPLAY:-}}"
  export VS_REAL_XDG_RUNTIME_DIR VS_REAL_DBUS VS_REAL_WAYLAND_DISPLAY VS_REAL_DISPLAY
}

_vs_under() { # _vs_under PATH DIR: PATH is DIR or below it, no ".." parts
  case "$1" in *..*) return 1 ;; esac
  [ -n "$2" ] && case "$1" in "${2%/}"|"${2%/}"/*) return 0 ;; esac
  return 1
}
_vs_roots() { printf '%s\n' /tmp "${TMPDIR:-}" /srv/FreeSynergy/.scratch; printf '%s' "${VS_ROOTS:-}" | tr ':' '\n'; }
_vs_in_roots() { local r; while IFS= read -r r; do _vs_under "$1" "$r" && return 0; done < <(_vs_roots); return 1; }

vs_guard() {
  local rt="${XDG_RUNTIME_DIR:-}" bus="${DBUS_SESSION_BUS_ADDRESS:-}" real p why=""
  real="/run/user/$(id -u)"
  if [ -z "$rt" ]; then why="XDG_RUNTIME_DIR is empty: refusing to guess a runtime dir"
  elif [ "${rt#/}" = "$rt" ]; then why="XDG_RUNTIME_DIR is not absolute: $rt"
  elif _vs_under "$rt" /run/user || _vs_under "$rt" "$real" || [ "$rt" = "${VS_REAL_XDG_RUNTIME_DIR:-}" ]; then why="XDG_RUNTIME_DIR is the REAL session's: $rt"
  elif ! _vs_in_roots "$rt"; then why="XDG_RUNTIME_DIR $rt is not below /tmp, \$TMPDIR or /srv/FreeSynergy/.scratch (VS_ROOTS)"
  elif [ ! -d "$rt" ]; then why="XDG_RUNTIME_DIR $rt does not exist"
  elif [ "${1:-}" != no-bus ]; then
    if [ -z "$bus" ]; then why="DBUS_SESSION_BUS_ADDRESS is empty: refusing to fall back to the default (real) bus"
    elif [ "$bus" = "${VS_REAL_DBUS:-}" ] || [[ "$bus" == *"/run/user/"* ]] || [[ "$bus" == *"$real"* ]]; then why="DBUS_SESSION_BUS_ADDRESS is the REAL session bus: $bus"
    else
      case "$bus" in
        unix:path=*) p="${bus#unix:path=}"; p="${p%%,*}"; _vs_under "$p" "$rt" || _vs_in_roots "$p" || why="bus socket $p is not inside the private runtime dir or /tmp" ;;
        unix:abstract=*) p="${bus#unix:abstract=}"; p="${p%%,*}"; _vs_in_roots "$p" || why="abstract bus $p is not below /tmp" ;;
        *) why="DBUS_SESSION_BUS_ADDRESS is not a private unix bus: $bus" ;;
      esac
    fi
  fi
  if [ -z "$why" ]; then
    if [ -n "${WAYLAND_SOCKET:-}" ]; then why="WAYLAND_SOCKET is set (an inherited socket of another session): $WAYLAND_SOCKET"
    elif [ -n "${WAYLAND_DISPLAY:-}" ] && [ "$WAYLAND_DISPLAY" = "${VS_REAL_WAYLAND_DISPLAY:-}" ]; then why="WAYLAND_DISPLAY is the REAL session's: $WAYLAND_DISPLAY"
    elif [ -n "${WAYLAND_DISPLAY:-}" ] && [ "${WAYLAND_DISPLAY#/}" != "$WAYLAND_DISPLAY" ] && ! _vs_under "$WAYLAND_DISPLAY" "$rt"; then why="WAYLAND_DISPLAY $WAYLAND_DISPLAY is outside the private runtime dir"
    elif [ -n "${DISPLAY:-}" ] && { [ "${VS_ALLOW_X:-}" != 1 ] || [ "$DISPLAY" = "${VS_REAL_DISPLAY:-}" ]; }; then why="DISPLAY is set ($DISPLAY): the virtual session has no X (VS_ALLOW_X=1 allows a private one)"
    fi
  fi
  if [ -n "$why" ]; then
    echo "virtual-session: REFUSED: $why. Run only inside a virtual session (scripts/typing-test.sh, see DESIGN.md), never against the real one." >&2
    return 97
  fi
}
# vs_run TOOL ARGS...: the guard, then the tool (qdbus, dbus-send, gdbus, kwriteconfig, xdotool, wtype, kwin scripts ...).
vs_run() { vs_guard || return $?; command "$@"; }
vs_dbus()  { vs_run dbus-send "$@"; }
vs_qdbus() { local q; q="$(command -v qdbus6 || command -v qdbus-qt6 || command -v qdbus || echo qdbus)"; vs_run "$q" "$@"; }
vs_gdbus() { vs_run gdbus "$@"; }

if [ "${BASH_SOURCE[0]}" = "$0" ]; then   # run as a command, not sourced
  case "${1:-}" in
    guard) vs_guard; exit $? ;;
    run) shift; [ $# -ge 1 ] || { echo "usage: virtual-session.sh run <tool> [args...]" >&2; exit 2; }; vs_run "$@"; exit $? ;;
    *) echo "usage: virtual-session.sh guard | run <tool> [args...]  (or source it)" >&2; exit 2 ;;
  esac
fi
