#!/usr/bin/env bash
# Maintain the approved fs-blinc tags (scripts/blinc-approved.txt, read by the lock gate scripts/check_blinc_lock.py).
#   scripts/approve-blinc.sh list
#   scripts/approve-blinc.sh add <fs-blinc-N[.M]> [--expires YYYY-MM-DD]   approve one more tag (transition: two tags at once)
#   scripts/approve-blinc.sh remove <tag>                                  take a tag out again
#   scripts/approve-blinc.sh replace <tag>   (also: approve-blinc.sh <tag>)  exactly this one tag is approved afterwards
# Only after the owner (Opus) approved the tag. The full sha is the commit the tag peels to (annotated tag ^{}), read with
# `git ls-remote` from $FS_BLINC_URL (default https://github.com/FreeSynergy/Blinc); a missing tag is an error. The file
# is rewritten atomically (temp file + mv); comment lines stay. An entry with `--expires` gets `# expires YYYY-MM-DD`;
# the gate is red for everyone once that day has passed. Programs then move with scripts/bump-components.sh and `cargo update`.
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
file="$here/blinc-approved.txt"
usage() { echo "usage: approve-blinc.sh list | add <tag> [--expires YYYY-MM-DD] | remove <tag> | replace <tag>" >&2; exit 2; }
check_tag() { [[ "$1" =~ ^fs-blinc-[0-9]+(\.[0-9]+)?$ ]] || { echo "not an fs-blinc tag: $1" >&2; exit 2; }; }
sha_of() {
  local url="${FS_BLINC_URL:-https://github.com/FreeSynergy/Blinc}" sha
  sha="$(git ls-remote --tags "$url" "refs/tags/$1^{}" | awk '{print $1}')"
  [ -n "$sha" ] || sha="$(git ls-remote --tags "$url" "refs/tags/$1" | awk '{print $1}')"   # lightweight tag
  [[ "$sha" =~ ^[0-9a-f]{40}$ ]] || { echo "tag $1 not found in $url" >&2; exit 1; }
  printf '%s' "$sha"
}
# entries_without TAG: the file without the entry of TAG (comments and other entries stay)
entries_without() { awk -v t="$1" '$1 != t' "$file"; }
write() { cat > "$file.new"; mv "$file.new" "$file"; }
cmd="${1:-}"; [ -n "$cmd" ] || usage
case "$cmd" in
  list) grep -v '^[[:space:]]*\(#\|$\)' "$file" || true ;;
  add)
    tag="${2:-}"; [ -n "$tag" ] || usage; check_tag "$tag"; shift 2
    exp=""
    if [ "${1:-}" = "--expires" ]; then
      exp="${2:-}"
      [[ "$exp" =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}$ ]] && date -d "$exp" >/dev/null 2>&1 || { echo "bad date for --expires: $exp (YYYY-MM-DD)" >&2; exit 2; }
      shift 2
    fi
    [ $# -eq 0 ] || usage
    sha="$(sha_of "$tag")"
    { entries_without "$tag"; echo "$tag $sha${exp:+  # expires $exp}"; } | write
    echo "approved: $tag $sha${exp:+ (expires $exp)}" ;;
  remove)
    tag="${2:-}"; [ -n "$tag" ] || usage; check_tag "$tag"
    grep -q "^${tag}[[:space:]]" "$file" || { echo "tag $tag is not approved" >&2; exit 1; }
    entries_without "$tag" | write
    echo "removed: $tag" ;;
  replace|fs-blinc-*)
    if [ "$cmd" = replace ]; then tag="${2:-}"; else tag="$cmd"; fi
    [ -n "$tag" ] || usage; check_tag "$tag"
    sha="$(sha_of "$tag")"
    { grep '^#' "$file" || true; echo "$tag $sha"; } | write
    echo "approved (only): $tag $sha" ;;
  *) usage ;;
esac
