#!/usr/bin/env bash
# build-slot.sh - run a heavy command (cargo build/test) in one of N shared build slots.
#
#   scripts/build-slot.sh <command> [args...]
#   scripts/build-slot.sh cargo test -p fs-foo
#
# Takes a flock on the first free slot file BUILD_DIR/build.lock.1 .. build.lock.N
# (non-blocking flock -n loop, short sleep, timeout 7200 s), then runs the command
# via limited.sh with a memory limit (default 12G). The slot is held until the command ends.
# If the command is `cargo`, -j8 is added unless a -j/--jobs argument is present
# (inserted before a literal `--`, so test-binary arguments stay untouched).
#
# Environment:
#   BUILD_SLOTS    number of slots (default 2)
#   BUILD_DIR      lock directory (default /srv/FreeSynergy/.scratch)
#   BUILD_TIMEOUT  seconds to wait for a slot (default 7200)
#   BUILD_MEM      memory limit for limited.sh (default 12G)
#   BUILD_JOBS     cargo jobs (default 8)
#   LIMITED_SH     path to limited.sh (default: next to this script, else the strategy worktree)
#
# The old single lock build.lock is not used here and stays untouched.
set -euo pipefail

[ "$#" -ge 1 ] || { echo "usage: $0 <command> [args...]" >&2; exit 2; }

slots="${BUILD_SLOTS:-2}"
dir="${BUILD_DIR:-/srv/FreeSynergy/.scratch}"
timeout_s="${BUILD_TIMEOUT:-7200}"
mem="${BUILD_MEM:-12G}"
jobs="${BUILD_JOBS:-8}"
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
limited="${LIMITED_SH:-}"
if [ -z "$limited" ]; then
  if [ -x "$here/limited.sh" ]; then limited="$here/limited.sh"
  else limited=/srv/FreeSynergy/.wt-doc-strategy/scripts/limited.sh; fi
fi

args=("$@")
if [ "${args[0]}" = "cargo" ]; then
  has_j=0
  for a in "${args[@]}"; do
    [ "$a" = "--" ] && break
    case "$a" in -j|-j[0-9]*|--jobs|--jobs=*) has_j=1 ;; esac
  done
  if [ "$has_j" -eq 0 ]; then
    new=()
    done_ins=0
    for a in "${args[@]}"; do
      if [ "$a" = "--" ] && [ "$done_ins" -eq 0 ]; then new+=("-j$jobs"); done_ins=1; fi
      new+=("$a")
    done
    [ "$done_ins" -eq 1 ] || new+=("-j$jobs")
    args=("${new[@]}")
  fi
fi

deadline=$((SECONDS + timeout_s))
while :; do
  for ((i = 1; i <= slots; i++)); do
    exec 9>>"$dir/build.lock.$i"
    if flock -n 9; then
      echo "build-slot: slot $i of $slots acquired" >&2
      rc=0
      "$limited" -m "$mem" "${args[@]}" || rc=$?
      exit "$rc"
    fi
    exec 9>&-
  done
  if [ "$SECONDS" -ge "$deadline" ]; then
    echo "build-slot: timeout after ${timeout_s}s waiting for a slot" >&2
    exit 124
  fi
  sleep 2
done
