#!/usr/bin/env bash
# Set the ONE approved fs-blinc tag (scripts/blinc-approved.txt, read by the lock gate scripts/check_blinc_lock.py).
#   scripts/approve-blinc.sh <fs-blinc-N>      only after the owner (Opus) approved the tag
# The full sha is the commit the tag peels to, read from $FS_BLINC_URL (default https://github.com/FreeSynergy/Blinc)
# with `git ls-remote`; a missing tag is an error. The file is replaced, so exactly one tag is approved afterwards.
# Programs then move with scripts/bump-components.sh (fs-components tag) and `cargo update` until their Cargo.lock matches.
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
tag="${1:?usage: approve-blinc.sh <fs-blinc-N>}"
[[ "$tag" =~ ^fs-blinc-[0-9]+$ ]] || { echo "not an fs-blinc tag: $tag" >&2; exit 2; }
url="${FS_BLINC_URL:-https://github.com/FreeSynergy/Blinc}"
sha="$(git ls-remote --tags "$url" "refs/tags/$tag^{}" | awk '{print $1}')"
[ -n "$sha" ] || sha="$(git ls-remote --tags "$url" "refs/tags/$tag" | awk '{print $1}')"   # lightweight tag
[[ "$sha" =~ ^[0-9a-f]{40}$ ]] || { echo "tag $tag not found in $url" >&2; exit 1; }
{ grep '^#' "$here/blinc-approved.txt"; echo "$tag $sha"; } > "$here/blinc-approved.txt.new"
mv "$here/blinc-approved.txt.new" "$here/blinc-approved.txt"
echo "approved: $tag $sha"
