#!/usr/bin/env bash
# Move every program repo to a new fs-components tag, test it, and push a branch (no tag, no merge).
#   scripts/bump-components.sh [--dry-run] <tag> [repo ...]     e.g. scripts/bump-components.sh v0.4.0
# Repos default to every /srv/FreeSynergy/fs-* with a Cargo.toml that names fs-components, except fs-components itself.
# Per repo: worktree from origin/main on branch bump-components-<tag>, set the tag on every fs-components* git
# dependency (tag = or rev = become tag =), `cargo update -p fs-components*` (every package of that git source), then gates.sh (fmt, clippy, tests, rules),
# and only when green push the branch. --dry-run does everything up to and including the gates but pushes nothing
# and prints what it would push. Builds go through limited.sh. A failing repo does not stop the others; the exit
# code is 1 when any repo failed.
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
base="${FS_BASE:-/srv/FreeSynergy}"
limited="${FS_LIMITED:-$base/.wt-doc-strategy/scripts/limited.sh}"
dry=0
if [ "${1:-}" = "--dry-run" ]; then dry=1; shift; fi
tag="${1:?usage: bump-components.sh [--dry-run] <tag> [repo ...]}"; shift
[[ "$tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+([-.][A-Za-z0-9.]+)?$ ]] || { echo "not a version tag: $tag" >&2; exit 2; }
branch="bump-components-$tag"
repos=("$@")
if [ "${#repos[@]}" -eq 0 ]; then
  for d in "$base"/fs-*/; do
    d="${d%/}"
    [ "$(basename "$d")" = fs-components ] && continue
    grep -qs 'fs-components' "$d/Cargo.toml" "$d"/crates/*/Cargo.toml && repos+=("$d")
  done
fi
# One repo, in its own shell: `set -e` is ignored inside a subshell that is the left side of `||`, so a failing step
# would not stop it. Run as `bash -c` so any failing step ends this repo (and only this repo).
bump_one() {
  set -euo pipefail
  repo="$1"; wt="$2"
  git -C "$repo" fetch -q origin
  [ -e "$wt" ] || git -C "$repo" worktree add -q "$wt" -B "$branch" origin/main
  cd "$wt"
  mapfile -t tomls < <(grep -ls 'fs-components' Cargo.toml crates/*/Cargo.toml || true)
  # a dependency line `fs-components... = { git = ".../fs-components", tag|rev = "..." }` gets tag = "<tag>"
  sed -i -E '/^[[:space:]]*fs-components[A-Za-z0-9_-]*[[:space:]]*=.*github\.com\/FreeSynergy\/fs-components/ s/(tag|rev)[[:space:]]*=[[:space:]]*"[^"]*"/tag = "'"$tag"'"/' "${tomls[@]}"
  if git diff --quiet && [ "$(git rev-parse HEAD)" = "$(git rev-parse origin/main)" ]; then echo "   already on $tag"; return 0; fi
  if ! git diff --quiet; then
    # every fs-components* package that comes from the fs-components git source moves to the new tag
    pkgs=()
    while read -r p; do pkgs+=(-p "$p"); done < <(awk '/^name = /{n=$3} /^source = .*FreeSynergy\/fs-components/{gsub("\"","",n); print n}' Cargo.lock | sort -u)
    [ "${#pkgs[@]}" -gt 0 ] || { echo "   no fs-components package in Cargo.lock" >&2; return 1; }
    "$limited" cargo update "${pkgs[@]}"
    git add -A
    git commit -q -m "fs-components $tag

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
  fi
  "$limited" "$here/gates.sh" "$wt"
  if [ "$dry" = 1 ]; then echo "   dry run: would push $branch of $name"; else git push -q origin "$branch"; echo "   pushed $branch"; fi
}
failed=()
for repo in "${repos[@]}"; do
  [ -d "$repo" ] || repo="$base/$repo"
  name="$(basename "$repo")"
  echo "== $name"
  wt="$base/.wt-bump-$name"
  export tag branch dry limited here name
  if ! bash -c "$(declare -f bump_one); bump_one \"\$1\" \"\$2\"" _ "$repo" "$wt"; then
    echo "   FAILED: $name" >&2; failed+=("$name")
  fi
done
[ "${#failed[@]}" -eq 0 ] || { echo "failed: ${failed[*]}" >&2; exit 1; }
