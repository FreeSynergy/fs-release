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
# Also: symlinks are resolved (readlink -f) for the runtime dir and the bus socket; '%' and ';' in the bus address, abstract
# addresses are refused; vs_run on dbus-send/gdbus/busctl/qdbus refuses --address, --system, --bus, -a, -H, -M and unix:/run/user args
# (--session uses the guarded environment). VS_ALLOW_X=1 only for a kwin started with --xwayland, with its own private DISPLAY.
# `vs_guard no-bus` skips the bus check (before dbus-run-session has created the private bus).
# The real session is "/run/user/<uid>" plus the values of the caller's environment before the virtual session starts.

vs_capture_real() {
  VS_REAL_XDG_RUNTIME_DIR="${VS_REAL_XDG_RUNTIME_DIR-${XDG_RUNTIME_DIR:-}}"
  VS_REAL_DBUS="${VS_REAL_DBUS-${DBUS_SESSION_BUS_ADDRESS:-}}"
  VS_REAL_WAYLAND_DISPLAY="${VS_REAL_WAYLAND_DISPLAY-${WAYLAND_DISPLAY:-}}"
  VS_REAL_DISPLAY="${VS_REAL_DISPLAY-${DISPLAY:-}}"
  export VS_REAL_XDG_RUNTIME_DIR VS_REAL_DBUS VS_REAL_WAYLAND_DISPLAY VS_REAL_DISPLAY
}

_vs_real() { readlink -f -- "$1" 2>/dev/null || printf '%s' "$1"; }   # symlinks resolved: a link to /run/user/UID is the real thing
_vs_under() { # _vs_under PATH DIR: PATH is DIR or below it, no ".." parts
  case "$1" in *..*) return 1 ;; esac
  [ -n "$2" ] && case "$1" in "${2%/}"|"${2%/}"/*) return 0 ;; esac
  return 1
}
_vs_roots() { printf '%s\n' /tmp "${TMPDIR:-}" /srv/FreeSynergy/.scratch; printf '%s' "${VS_ROOTS:-}" | tr ':' '\n'; }
_vs_in_roots() { local r; while IFS= read -r r; do [ -n "$r" ] || continue; _vs_under "$1" "$r" && return 0; _vs_under "$1" "$(_vs_real "$r")" && return 0; done < <(_vs_roots); return 1; }
_vs_is_real_path() { # PATH (resolved) is below /run/user or the real runtime dir
  _vs_under "$1" /run/user || _vs_under "$1" "/run/user/$(id -u)" || { [ -n "${VS_REAL_XDG_RUNTIME_DIR:-}" ] && _vs_under "$1" "$(_vs_real "$VS_REAL_XDG_RUNTIME_DIR")"; }
}

# shellcheck disable=SC2120  # the optional argument "no-bus" is passed by callers
vs_guard() {
  local rt="${XDG_RUNTIME_DIR:-}" bus="${DBUS_SESSION_BUS_ADDRESS:-}" real p why=""
  real="/run/user/$(id -u)"
  if [ -z "$rt" ]; then why="XDG_RUNTIME_DIR is empty: refusing to guess a runtime dir"
  elif [ "${rt#/}" = "$rt" ]; then why="XDG_RUNTIME_DIR is not absolute: $rt"
  elif _vs_is_real_path "$rt" || _vs_is_real_path "$(_vs_real "$rt")" || [ "$rt" = "${VS_REAL_XDG_RUNTIME_DIR:-}" ]; then why="XDG_RUNTIME_DIR is (or links to) the REAL session's: $rt"
  elif ! _vs_in_roots "$rt" || ! _vs_in_roots "$(_vs_real "$rt")"; then why="XDG_RUNTIME_DIR $rt is not below /tmp, \$TMPDIR or /srv/FreeSynergy/.scratch (VS_ROOTS)"
  elif [ ! -d "$rt" ]; then why="XDG_RUNTIME_DIR $rt does not exist"
  elif [ "${1:-}" != no-bus ]; then
    if [ -z "$bus" ]; then why="DBUS_SESSION_BUS_ADDRESS is empty: refusing to fall back to the default (real) bus"
    elif [[ "$bus" == *%* || "$bus" == *\;* ]]; then why="DBUS_SESSION_BUS_ADDRESS has '%' (D-Bus decodes it: %2e%2e is ..) or ';' (several addresses): $bus"
    elif [[ "$bus" == unix:abstract=* ]]; then why="an abstract D-Bus address is no file path, nothing says it is private: $bus"
    elif ! [[ "$bus" =~ ^unix:path=[^,]+(,guid=[0-9a-f]+)?$ ]]; then why="DBUS_SESSION_BUS_ADDRESS is not a plain private unix:path= bus: $bus"
    elif [ "$bus" = "${VS_REAL_DBUS:-}" ] || [[ "$bus" == *"/run/user/"* ]] || [[ "$bus" == *"$real"* ]]; then why="DBUS_SESSION_BUS_ADDRESS is the REAL session bus: $bus"
    else
      p="${bus#unix:path=}"; p="${p%%,*}"; p="$(_vs_real "$p")"
      if _vs_is_real_path "$p"; then why="bus socket $p resolves into the REAL session's runtime dir"
      elif ! { _vs_under "$p" "$(_vs_real "$rt")" || _vs_in_roots "$p"; }; then why="bus socket $p is not inside the private runtime dir or /tmp"
      fi
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
vs_run() {
  vs_guard || return $?
  local a
  case "$(basename -- "$1")" in dbus-send|gdbus|busctl|qdbus*|dbus-monitor)
    for a in "${@:2}"; do
      case "$a" in
        --address|--address=*|--system|--bus|--bus=*|-a|-H|--host|-M|--machine|unix:*|*/run/user/*)
          echo "virtual-session: REFUSED: argument '$a' selects another bus; the bus is the private one from the environment" >&2; return 97 ;;
      esac
    done ;;
  esac
  command "$@"
}
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
