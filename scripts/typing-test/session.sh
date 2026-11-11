#!/usr/bin/env bash
# Runs inside the virtual KWin (started by ../typing-test.sh): drives the app and takes the pictures.
# Phase A waits for the window, finds the field by its caret, types one character at a time and takes a picture of
# each, then two parts of a text; phase B types a text with the app's Wayland trace switched on, for the
# key-to-picture time. A scenario file may use c, shot, WX0/WY0 (the window's corner) and set TT_NEAR="x,y"
# (a point in the field, for finding the caret).
set -uo pipefail
D="$TT_OUT"
export WAYLAND_DISPLAY="${TT_SOCKET:-tt-probe}"
WX0=0; WY0=0
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
  until python3 -I "$TT_DIR/fieldcheck.py" arrived "$D/shots/$1.png" "$D/shots/blank.png" || [ "$n" -ge 5 ]; do
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

start_app() { "$TT_BIN" "${APP_ARGS[@]}" > "$D/app-$1.log" 2>&1 & APP=$!; sleep "${2:-${TT_SETTLE:-7}}"; }
stop_app() { kill "$APP" 2>/dev/null; wait "$APP" 2>/dev/null; true; }
# The click point is absolute (TT_CLICK) or relative to the window's corner (TT_CLICK_REL), which does not depend on
# where this KWin places the window.
focus() {
  local x y
  if [ -n "${TT_CLICK_REL:-}" ]; then
    IFS=, read -r x y <<<"$TT_CLICK_REL"; x=$((WX0 + x)); y=$((WY0 + y))
  else
    IFS=, read -r x y <<<"$TT_CLICK"
  fi
  TT_NEAR="$x,$y"; export TT_NEAR
  c "move $x $y" 0.4; c "click $x $y" 1
}
# Wait until the app's window is on the screen and has stopped moving; sets WX0, WY0.
wait_window() {
  local prev="" cur name="${1:-blank0}"
  for _ in $(seq 1 $((${TT_SETTLE:-30} * 2))); do
    shot "$name"
    if cur="$(python3 -I "$TT_DIR/fieldcheck.py" window "$D/shots/$name.png")"; then
      [ "$cur" = "$prev" ] && { IFS=, read -r WX0 WY0 _ <<<"$cur"; sleep 1; return 0; }
      prev="$cur"
    fi
    sleep 0.5
  done
  return 1
}
if [ -n "${TT_SCENARIO:-}" ]; then
  # shellcheck disable=SC1090
  . "$TT_SCENARIO"
fi

# Seconds the injector needs for a text (about 0.16 s per character) plus time for the last picture.
wait_for() { awk -v n="${#1}" 'BEGIN { print n * 0.2 + 1.2 }'; }

# Characters whose keys differ between layouts; "a" is the control. Shift: Ä. AltGr: the rest.
CHARS=(a Ä ä ö ü ß @ € '{' '}' '[' ']' "\\" '|' '~' µ)
PARTS=('aÄäöüß@€' '{}[]\|~µ')  # eight characters each
LAT="${TT_LAT:-Müller Wilhelm @ {x} [y] lilli ÄÖÜ äöüß 123}"

if [ "${TT_PHASE:-all}" != latency ]; then
  TT_SETTLE_KEEP="${TT_SETTLE:-30}"
  start_app A 0
  if ! wait_window; then echo "no window after ${TT_SETTLE_KEEP} s" > "$D/.session-failed"; stop_app; kill "$INJ" 2>/dev/null; exit 0; fi
  focus
  sleep 0.6
  # Where the field is: given (TT_FIELD), else found by its caret (it blinks, so two pictures differ by a narrow bar).
  if [ -z "${TT_FIELD:-}" ]; then
    for _ in 1 2 3 4 5 6 7 8; do
      shot ca; sleep "0.$((RANDOM % 7 + 1))"; shot cb
      if r="$(python3 -I "$TT_DIR/fieldcheck.py" rect "$D/shots/ca.png" "$D/shots/cb.png" 2>> "$D/fieldcheck.log")"; then TT_RECT="$r"; export TT_RECT; break; fi
      echo "no caret in pair near ${TT_NEAR:-?}" >> "$D/fieldcheck.log"
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
  # Two parts of eight characters, so the text stays inside the field; the caret is found in a second picture.
  n=0
  for text in "${PARTS[@]}"; do
    n=$((n + 1))
    c "text $text" "$(wait_for "$text")"
    nudge
    shot_stable "full$n"
    for _ in 1 2 3 4 5 6; do
      sleep "0.$((RANDOM % 7 + 1))"; shot "full$n.b"
      python3 -I "$TT_DIR/fieldcheck.py" caretx "$D/shots/full$n.png" "$D/shots/full$n.b.png" > /dev/null && break
      sleep 0.3
    done
    for _ in 1 2 3 4 5 6 7 8; do c "key 14" 0.12; done
    nudge
  done
  stop_app
fi

WAYLAND_DEBUG=1 start_app B 0
if ! wait_window blank0b; then echo "no window after ${TT_SETTLE:-30} s" > "$D/.session-failed"; stop_app; kill "$INJ" 2>/dev/null; exit 0; fi
focus
# TT_PHASE=idle: no typing, only the app idling for TT_IDLE seconds; its CPU time goes to idle.txt.
if [ "${TT_PHASE:-all}" = idle ]; then
  cpu() { awk '{print $14 + $15}' "/proc/$APP/stat"; }
  c0="$(cpu)"; sleep "${TT_IDLE:-20}"; c1="$(cpu)"
  echo "$c0 $c1 $(getconf CLK_TCK) ${TT_IDLE:-20}" > "$D/idle.txt"
  stop_app; c quit 0.3; kill "$INJ" 2>/dev/null; echo ok > "$D/.session-done"; exit 0
fi
# TT_SAMPLE=1: what the app's main thread waits in, every 50 ms, to see what it does while keys pile up.
if [ -n "${TT_SAMPLE:-}" ]; then
  # TT_SAMPLE_COMM names the process when the app is started through a wrapper script.
  ( SP="$APP"; [ -n "${TT_SAMPLE_COMM:-}" ] && SP="$(pgrep -n -x "$TT_SAMPLE_COMM")"
    while kill -0 "$APP" 2>/dev/null; do
      echo "$(date +%s.%N) $(cat "/proc/$SP/wchan" 2>/dev/null) $(awk '{print $3}' "/proc/$SP/stat" 2>/dev/null)"
      sleep 0.05
    done ) > "$D/wchan.log" 2>&1 &
  SAMPLER=$!
fi
c "text $LAT" "$(wait_for "$LAT")"
[ -n "${SAMPLER:-}" ] && kill "$SAMPLER" 2>/dev/null
stop_app

c quit 0.3
kill "$INJ" 2>/dev/null
echo ok > "$D/.session-done"
