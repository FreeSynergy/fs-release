#!/usr/bin/env bash
# Runs inside the virtual KWin (started by ../typing-test.sh): drives the app and takes the pictures.
# Phase A types one character at a time and takes a picture of each; phase B types a text with the app's
# Wayland trace switched on, for the key-to-picture time.
set -uo pipefail
D="$TT_OUT"
export WAYLAND_DISPLAY=tt-probe
IFS=$'\x1f' read -r -d '' -a APP_ARGS <<<"${TT_APP_ARGS:-}" || true
[ "${#APP_ARGS[@]}" -gt 0 ] && [ -z "${APP_ARGS[-1]}" ] && unset 'APP_ARGS[-1]'

fifo="$D/.inject-in"; rm -f "$fifo"; mkfifo "$fifo"
"$TT_INJECT" repl < "$fifo" >> "$D/inject.log" 2>&1 &
INJ=$!
exec 3>"$fifo"
c() { printf '%s\n' "$1" >&3; sleep "${2:-1}"; }
shot() { spectacle -b -n -o "$D/shots/$1.png" >> "$D/spectacle.log" 2>&1; }

start_app() { "$TT_BIN" "${APP_ARGS[@]}" > "$D/app-$1.log" 2>&1 & APP=$!; sleep "${TT_SETTLE:-7}"; }
stop_app() { kill "$APP" 2>/dev/null; wait "$APP" 2>/dev/null; true; }
focus() {
  IFS=, read -r x y <<<"$TT_CLICK"
  c "move $x $y" 0.4; c "click $x $y" 1
}
if [ -n "${TT_SCENARIO:-}" ]; then
  # shellcheck disable=SC1090
  . "$TT_SCENARIO"
fi

# Seconds the injector needs for a text (about 0.16 s per character) plus time for the last picture.
wait_for() { awk -v n="${#1}" 'BEGIN { print n * 0.2 + 1.2 }'; }

# Characters whose keys differ between layouts; "a" is the control. Shift: Ä. AltGr: the rest.
CHARS=(a Ä ä ö ü ß @ € '{' '}' '[' ']' "\\" '|' '~' µ)
FULL='aÄäöüß@€{}[]\|~µ'
LAT='Müller Wilhelm @ {x} [y] lilli ÄÖÜ äöüß 123'

start_app A
shot blank0
focus
sleep 0.6
shot blank
for i in "${!CHARS[@]}"; do
  c "text ${CHARS[$i]}" 0.5
  shot "g$(printf %02d "$i")"
  c "key 14" 0.4
done
c "text $FULL" "$(wait_for "$FULL")"
shot full
stop_app

WAYLAND_DEBUG=1 start_app B
focus
c "text $LAT" "$(wait_for "$LAT")"
stop_app

c quit 0.3
kill "$INJ" 2>/dev/null
echo ok > "$D/.session-done"
