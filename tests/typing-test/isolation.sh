#!/usr/bin/env bash
# The typing test must not touch the user's own session: the app it starts sees no DISPLAY, only the test's own Wayland
# socket and its own runtime directory, and nothing of the test stays behind. The "app" here only records what it sees.
#   tests/typing-test/isolation.sh
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

host_runtime="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}"
cat > "$work/spy.sh" <<SPY
#!/usr/bin/env bash
env > "$work/seen.env"
sleep 30
SPY
chmod +x "$work/spy.sh"

# No window ever appears, so the test ends with "no window" (exit 2) after the short wait: that is expected here.
rc=0
DISPLAY="${DISPLAY:-:0}" TT_OUT="$work/out" TT_CLICK_REL=10,10 TT_SETTLE=3 TT_SOCKET=tt-iso-$$ \
  "$here/../../scripts/typing-test.sh" "$work/spy.sh" > "$work/run.log" 2>&1 || rc=$?
[ "$rc" = 2 ] || { echo "expected exit 2 (no window), got $rc"; cat "$work/run.log"; exit 1; }
[ -s "$work/seen.env" ] || { echo "the app did not start"; cat "$work/run.log"; exit 1; }

fail=0
check() { "$@" || { echo "FAILED: $*"; fail=1; }; }
not() { ! "$@"; }
check not grep -q '^DISPLAY=' "$work/seen.env"
check grep -q "^WAYLAND_DISPLAY=tt-iso-$$\$" "$work/seen.env"
check grep -q '^WINIT_UNIX_BACKEND=wayland$' "$work/seen.env"
check grep -q '^GDK_BACKEND=wayland$' "$work/seen.env"
check grep -q '^QT_QPA_PLATFORM=wayland$' "$work/seen.env"
check not grep -q "^XDG_RUNTIME_DIR=$host_runtime\$" "$work/seen.env"
check not grep -q "^WAYLAND_DISPLAY=${WAYLAND_DISPLAY:-wayland-0}\$" "$work/seen.env"
check not pgrep -f "socket tt-iso-$$" > /dev/null
check test -z "$(find "${TMPDIR:-/tmp}" -maxdepth 1 -name 'tt-run.*' -newer "$work/spy.sh" 2> /dev/null)"
[ "$fail" = 0 ] && echo "typing-test isolation: ok"
exit "$fail"
