#!/usr/bin/env bash
# The typing test N times, each run with a fresh session (new KWin, new app window), one result directory per run.
# The whole is green only if ALL runs are green: the point of repeating is to FIND rare faults (a field that is dead in
# one run of eight), not to throw them away. Exit code: 0 all green, 1 at least one run red, 2 no run was red but at
# least one could not run.
#
#   scripts/typing-test-runs.sh <binary> [app args...]
#
# Environment: TT_RUNS (default 3; the mandatory gate takes 3, before a tag take 10), TT_OUT (default ./typing-test-out;
# run i writes to TT_OUT/run-i), the rest as in typing-test.sh. Writes TT_OUT/summary.json and summary.txt and, on a
# runner, the summary into the job summary. TT_RUN_CMD replaces the single-run script (tests only).
set -uo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RUNS="${TT_RUNS:-3}"
RUN_CMD="${TT_RUN_CMD:-$HERE/typing-test.sh}"
case "$RUNS" in '' | *[!0-9]*) echo "typing-test-runs: TT_RUNS must be a number of at least 1, got '$RUNS'" >&2; exit 2 ;; esac
RUNS=$((10#$RUNS))  # 00 and 000 are zero, too (and 08 is not an invalid octal number)
if [ "$RUNS" -lt 1 ]; then echo "typing-test-runs: TT_RUNS must be a number of at least 1, got '$RUNS'" >&2; exit 2; fi

OUT="$(mkdir -p "${TT_OUT:-typing-test-out}" && cd "${TT_OUT:-typing-test-out}" && pwd)"
red=0 broken=0
codes=()
for i in $(seq 1 "$RUNS"); do
  rc=0
  TT_OUT="$OUT/run-$i" "$RUN_CMD" "$@" > "$OUT/run-$i.log" 2>&1 || rc=$?
  codes+=("$rc")
  [ "$rc" -eq 1 ] && red=$((red + 1))
  [ "$rc" -ge 2 ] && broken=$((broken + 1))
  echo "typing-test run $i of $RUNS: exit $rc"
  tail -n 40 "$OUT/run-$i.log" | grep -E "FAIL|PASS|typing-test:" || true
done

# The summary: per run its number, green or red and the red checks (names from result.json, if the run got that far).
python3 -I "$HERE/typing-test/summary.py" "$OUT" "$RUNS" "${codes[@]}" || true
[ -f "$OUT/summary.txt" ] && cat "$OUT/summary.txt"
if [ -n "${GITHUB_STEP_SUMMARY:-}" ] && [ -f "$OUT/summary.txt" ]; then
  { echo "### typing-test: $red of $RUNS runs red"; echo; echo '```'; cat "$OUT/summary.txt"; echo '```'; } >> "$GITHUB_STEP_SUMMARY"
fi

if [ "$red" -gt 0 ]; then exit 1; fi
if [ "$broken" -gt 0 ]; then exit 2; fi
exit 0
