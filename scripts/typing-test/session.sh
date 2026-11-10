#!/usr/bin/env bash
# Runs inside the virtual KWin (started by ../typing-test.sh): drives the app and takes the pictures.
# Phase A types one character at a time and takes a picture of each; phase B types a text with the app's
# Wayland trace switched on, for the key-to-picture time.
set -uo pipefail
D="$TT_OUT"
export WAYLAND_DISPLAY=tt-probe
mapfile -d $'\x1f' -t APP_ARGS < <(printf '%s' "${TT_APP_ARGS:-}")

fifo="$D/.inject-in"; rm -f "$fifo"; mkfifo "$fifo"
"$TT_INJECT" repl < "$fifo" >> "$D/inject.log" 2>&1 &
INJ=$!
exec 3>"$fifo"
c() { printf '%s\n' "$1" >&3; sleep "${2:-1}"; }
shot() { QT_QPA_PLATFORM=wayland spectacle -b -n -o "$D/shots/$1.png" >> "$D/spectacle.log" 2>&1; }
# KWin can hold the last injected key until the next input event arrives; a pointer nudge over the field releases it.
nudge() { c "move $((NX + 1)) $NY" 0.1; c "move $NX $NY" 0.2; }
# Like shot_stable, and waits (nudging) until a typed glyph shows; how many times it had to wait goes to late.log.
shot_arrived() {
  shot_stable "$1"
  local n=0
  until python3 -I "$TT_DIR/fieldcheck.py" arrived "$D/shots/$1.png" || [ "$n" -ge 5 ]; do
    n=$((n + 1)); sleep 0.5; nudge; shot_stable "$1"
  done
  echo "$1 $n" >> "$D/late.log"
}
# A picture of the field once it has stopped changing: input and drawing may lag behind the injected key.
shot_stable() {
  shot "$1"
  for _ in 1 2 3 4 5 6; do
    shot "$1.next"
    if python3 -I "$TT_DIR/fieldcheck.py" same "$D/shots/$1.png" "$D/shots/$1.next.png"; then rm -f "$D/shots/$1.next.png"; return; fi
    mv "$D/shots/$1.next.png" "$D/shots/$1.png"
  done
}

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
# Where the field is: given (TT_FIELD), else found by its caret in the first pictures after the focus click.
if [ -z "${TT_FIELD:-}" ]; then
  for _ in 1 2 3 4 5 6 7 8; do
    shot blank
    if r="$(python3 -I "$TT_DIR/fieldcheck.py" rect "$D/shots/blank.png")"; then TT_RECT="$r"; export TT_RECT; break; fi
    sleep 0.3
  done
fi
echo "$TT_RECT" > "$D/rect.txt"
IFS=, read -r rx0 ry0 rx1 ry1 <<<"$TT_RECT"
NX=$(((rx0 + rx1) / 2)); NY=$(((ry0 + ry1) / 2))
shot_stable blank
for i in "${!CHARS[@]}"; do
  c "text ${CHARS[$i]}" 0.3
  nudge
  shot_arrived "g$(printf %02d "$i")"
  c "key 14" 0.3
  nudge
done
c "text $FULL" "$(wait_for "$FULL")"
nudge
shot_stable full
stop_app

WAYLAND_DEBUG=1 start_app B
focus
c "text $LAT" "$(wait_for "$LAT")"
stop_app

c quit 0.3
kill "$INJ" 2>/dev/null
echo ok > "$D/.session-done"
