#!/usr/bin/env bash
# The repeat logic of the typing test (scripts/typing-test-runs.sh) with a fake single run: all green -> green, one red
# run among three -> red, a run that could not run -> exit 2, one run per directory, summary says "x of N red".
#   tests/typing-test/runs.sh
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
runs="$here/../../scripts/typing-test-runs.sh"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
fail=0
check() { if [ "$2" = "$3" ]; then echo "ok   $1"; else echo "FAIL $1: got '$2', wanted '$3'"; fail=1; fi; }

# The fake run: TT_OUT is its own directory; the exit codes come from FAKE_CODES (space separated, one per run).
cat > "$work/fake.sh" <<'FAKE'
#!/usr/bin/env bash
n=$(( $(cat "$FAKE_STATE" 2>/dev/null || echo 0) + 1 )); echo "$n" > "$FAKE_STATE"
read -r -a codes <<<"$FAKE_CODES"; rc="${codes[$((n - 1))]}"
mkdir -p "$TT_OUT"
if [ "$rc" = 1 ]; then
  echo '{"cases":[{"name":"char \"a\" arrives","passed":false,"message":"no glyph"},{"name":"x","passed":true}]}' > "$TT_OUT/result.json"
  echo "typing-test fake: FAIL"
elif [ "$rc" = 0 ]; then
  echo '{"cases":[{"name":"x","passed":true}]}' > "$TT_OUT/result.json"
  echo "typing-test fake: PASS"
fi
exit "$rc"
FAKE
chmod +x "$work/fake.sh"

run() { # codes runs -> exit code of the repeat script
  rm -f "$work/state"; rm -rf "$work/out"
  FAKE_STATE="$work/state" FAKE_CODES="$1" TT_RUNS="$2" TT_RUN_CMD="$work/fake.sh" TT_OUT="$work/out" "$runs" app > "$work/stdout" 2>&1 && echo 0 || echo $?
}

check "three green runs: green" "$(run "0 0 0" 3)" 0
check "every run in its own directory" "$(find "$work/out" -maxdepth 1 -type d -name 'run-[123]' | wc -l)" 3
check "one red run among three: red" "$(run "0 1 0" 3)" 1
check "summary: 1 of 3 red" "$(grep -c '1 of 3 runs red' "$work/out/summary.txt")" 1
check "summary names the red check" "$(grep -c 'red: char "a" arrives' "$work/out/summary.txt")" 1
check "the red run is run 2 in summary.json" "$(python3 -I -c "import json,sys;print([r['run'] for r in json.load(open('$work/out/summary.json'))['results'] if r['exit']==1])")" "[2]"
check "the last run red: red" "$(run "0 0 1" 3)" 1
check "a run that could not run: exit 2" "$(run "0 2 0" 3)" 2
check "red beats could-not-run" "$(run "2 1 0" 3)" 1
check "one run only" "$(run "0" 1)" 0
check "default is three runs" "$(rm -f "$work/state"; rm -rf "$work/out"; FAKE_STATE="$work/state" FAKE_CODES="0 0 0 0" TT_RUN_CMD="$work/fake.sh" TT_OUT="$work/out" "$runs" app >/dev/null 2>&1; cat "$work/state")" 3
check "runs 0 is refused" "$(run "0" 0)" 2
exit "$fail"
