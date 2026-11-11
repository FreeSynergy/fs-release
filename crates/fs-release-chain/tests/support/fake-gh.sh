#!/usr/bin/env bash
# Fake `gh` for the tests: answers from files in $FAKE_DIR and logs every call. Never talks to GitHub.
d="$FAKE_DIR"
echo "gh $*" >> "$d/calls.log"
repo=""
args=("$@")
for ((i = 0; i < ${#args[@]}; i++)); do [ "${args[$i]}" = "--repo" ] && repo="${args[$((i + 1))]}"; done
key="${repo//\//_}"
case "$1 $2" in
  "run list")
    f="$d/runs-$key.json"
    if [ -d "$f.seq" ] && [ -n "$(ls "$f.seq")" ]; then
      n="$(ls "$f.seq" | sort | head -1)"
      text="$(cat "$f.seq/$n")"
      [ "$(ls "$f.seq" | wc -l)" -gt 1 ] && rm "$f.seq/$n"
    else
      text="$(cat "$f" 2>/dev/null || echo '[]')"
    fi
    main="$(git --git-dir="$FAKE_CAT_ORIGIN" rev-parse main 2>/dev/null)"
    printf '%s\n' "${text//@MAIN@/$main}" ;;
  "run view") cat "$d/log-$3.txt" 2>/dev/null || echo "no log" ;;
  "run rerun")
    f="$d/runs-$key.json"
    [ -f "$f.rerun" ] && cp "$f.rerun" "$f" && rm -rf "$f.seq" ;;
  "release view")
    f="$d/release-$key-$3.json"
    if [ -f "$f" ]; then cat "$f"; else echo "release not found" >&2; exit 1; fi ;;
  "release edit")
    f="$d/release-$key-$3.json"
    if [[ " $* " == *" --prerelease "* ]]; then echo '{"isDraft":false,"isPrerelease":true}' > "$f"; else echo '{"isDraft":false,"isPrerelease":false}' > "$f"; fi ;;
  "workflow run") ;;
  *) echo "fake gh: not expected: $*" >&2; exit 2 ;;
esac
