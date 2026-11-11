#!/usr/bin/env bash
# Move every program repo to a new fs-components tag, test it, and push a branch (no tag, no merge).
#   scripts/bump-components.sh [--dry-run] <tag> [repo ...]     e.g. scripts/bump-components.sh v0.4.0
# WHAT IT DOES NOT DO: it does not convert programs. It removes no blinc* line, no [patch], rewrites no `use`.
# The first move of a repo onto "Blinc only through fs-components" is manual work per repo (then add
# `.fs-rules.toml` with `blinc = "components-only"`, see DESIGN.md); this script only moves the tag afterwards.
# The approved fs-blinc tag (the gate checks every Cargo.lock against it) is set by scripts/approve-blinc.sh, not here.
# Repos default to every /srv/FreeSynergy/fs-* with a Cargo.toml (any depth, not target/ or .wt-*) that names
# fs-components, except fs-components itself. Before anything: the tag must exist in the fs-components repo
# ($FS_COMPONENTS_URL, default https://github.com/FreeSynergy/fs-components; `git ls-remote --tags`).
# Per repo: a FRESH worktree .wt-bump-<repo> from origin/main on branch bump-components-<tag> (an existing worktree, the branch
# checked out elsewhere, or the branch already on origin is an error with a clear message: nothing is removed or
# force-pushed, clean up yourself; a repo without Cargo.lock is an error too, its worktree is kept); scripts/set_components_tag.py sets tag = "<tag>" on EVERY
# dependency on the fs-components git repo (any name, package =, table form, workspace/dev/build/target tables;
# rev/branch removed; parser-verified); `cargo update -p <pkg-spec>` for every package from that git source;
# gates.sh (fmt, clippy, tests, rules); only when green `git push origin HEAD:refs/heads/<branch>`.
# No dependency found = FAILED. Already on the tag = ok, nothing pushed. --dry-run does everything up to and
# including the gates but pushes nothing. Builds go through limited.sh. A failing repo does not stop the others;
# the exit code is 1 when any repo failed.
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
base="${FS_BASE:-/srv/FreeSynergy}"
limited="${FS_LIMITED:-$base/.wt-doc-strategy/scripts/limited.sh}"
dry=0
if [ "${1:-}" = "--dry-run" ]; then dry=1; shift; fi
tag="${1:?usage: bump-components.sh [--dry-run] <tag> [repo ...]}"; shift
[[ "$tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+([-.][A-Za-z0-9.]+)?$ ]] || { echo "not a version tag: $tag" >&2; exit 2; }
comp_url="${FS_COMPONENTS_URL:-https://github.com/FreeSynergy/fs-components}"
[ -n "$(git ls-remote --tags "$comp_url" "refs/tags/$tag" 2>/dev/null)" ] || { echo "tag $tag does not exist in $comp_url" >&2; exit 2; }
branch="bump-components-$tag"
repos=("$@")
has_components() { # any Cargo.toml (not in target/, .git/, .wt-*) that mentions fs-components
  find "$1" \( -name target -o -name .git -o -name 'node_modules' -o -name '.wt-*' \) -prune -o -name Cargo.toml -print0 | xargs -0 -r grep -ls 'fs-components' >/dev/null
}
if [ "${#repos[@]}" -eq 0 ]; then
  for d in "$base"/fs-*/; do
    d="${d%/}"
    [ "$(basename "$d")" = fs-components ] && continue
    has_components "$d" && repos+=("$d")
  done
fi
# One repo, in its own shell: `set -e` is ignored inside a subshell that is the left side of `||`, so a failing step
# would not stop it. Run as `bash -c` so any failing step ends this repo (and only this repo).
bump_one() {
  set -euo pipefail
  repo="$1"; wt="$2"
  git -C "$repo" fetch -q origin
  git -C "$repo" worktree prune
  [ ! -e "$wt" ] || { echo "   worktree $wt exists (left over from an earlier run or a failed gate?): inspect it, remove it yourself with 'git -C $repo worktree remove --force $wt', then run again. Nothing was changed." >&2; return 1; }
  other="$(git -C "$repo" worktree list --porcelain | awk -v b="refs/heads/$branch" '/^worktree /{w=$2} $1=="branch" && $2==b{print w}')"
  [ -z "$other" ] || { echo "   branch $branch is checked out in another worktree ($other): remove that worktree yourself, then run again. Nothing was changed." >&2; return 1; }
  [ -z "$(git -C "$repo" ls-remote --heads origin "refs/heads/$branch" 2>/dev/null)" ] || { echo "   branch $branch already exists on origin (an earlier run pushed it): review or delete that branch (or its pull request) yourself, then run again. Nothing was changed, nothing is deleted or force-pushed." >&2; return 1; }
  git -C "$repo" worktree add -q "$wt" -B "$branch" origin/main
  cd "$wt"
  set +e; python3 -I "$here/set_components_tag.py" "$tag" "$wt"; rc=$?; set -e
  [ "$rc" -eq 0 ] || { echo "   could not set the tag (exit $rc)" >&2; return 1; }
  if git diff --quiet; then echo "   already on $tag, nothing to do"; return 0; fi
  [ -f Cargo.lock ] || { echo "   no Cargo.lock in $name: nothing to lock (library, or the lock file is not committed). The tag was set in the worktree $wt but not committed or pushed; commit a Cargo.lock to the repo first, or bump this repo by hand." >&2; return 1; }
  # every package from the fs-components git source moves to the new tag; the spec carries url#name@version so a
  # same-named package from another source cannot be meant
  mapfile -t pkgs < <(awk '/^name = /{n=$3; gsub("\"","",n)} /^version = /{v=$3; gsub("\"","",v)} /^source = "git\+[^"]*\/fs-components(\.git)?[?#]/{u=$3; gsub("\"","",u); sub(/^git\+/,"",u); sub(/[?#].*/,"",u); print u "#" n "@" v}' Cargo.lock | sort -u)
  [ "${#pkgs[@]}" -gt 0 ] || { echo "   no fs-components package in Cargo.lock" >&2; return 1; }
  args=(); for p in "${pkgs[@]}"; do args+=(-p "$p"); done
  "$limited" cargo update "${args[@]}" || { echo "   cargo update failed (ambiguous or unknown package spec?)" >&2; return 1; }
  git add -A
  git commit -q -m "fs-components $tag

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
  "$limited" "$here/gates.sh" "$wt"
  if [ "$dry" = 1 ]; then echo "   dry run: would push $branch of $name"; else git push -q origin "HEAD:refs/heads/$branch"; echo "   pushed $branch"; fi
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
