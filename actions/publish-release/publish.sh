#!/usr/bin/env bash
# Publish the draft release TAG of REPO as a Pre-release (0.0.N is always a pre-release, ADR 0064).
# Guard: SHA256SUMS.txt exists, every file in it is a release asset, every asset has its detached .asc.
# Idempotent: an already published release is left alone. DRY_RUN=true checks and reports, changes nothing.
set -euo pipefail
: "${TAG:?}" "${REPO:?}" "${GH_TOKEN:?}"
export GH_REPO=$REPO
dry=${DRY_RUN:-false}

die() { printf '::error::%s\n' "$*"; exit 1; }

meta=$(gh release view "$TAG" --json isDraft,isPrerelease,assets) || die "$REPO $TAG: no release found (did the draft step run?)"
if [[ "$(jq -r .isDraft <<<"$meta")" != "true" ]]; then
  echo "$REPO $TAG is already published - nothing to do"
  exit 0
fi

names=$(jq -r '.assets[].name' <<<"$meta")
grep -qxF SHA256SUMS.txt <<<"$names" || die "$REPO $TAG: SHA256SUMS.txt is missing - not publishing"

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
gh release download "$TAG" --pattern SHA256SUMS.txt --dir "$tmp"

problems=0
fail() { printf '::error::%s\n' "$*"; problems=$((problems + 1)); }

# every file of the list is an asset, with a signature
listed=0
while read -r _ file; do
  [[ -n "${file:-}" ]] || continue
  file=${file#\*}
  listed=$((listed + 1))
  grep -qxF -- "$file" <<<"$names" || fail "$file is in SHA256SUMS.txt but not attached to the release"
  grep -qxF -- "$file.asc" <<<"$names" || fail "$file has no detached signature $file.asc"
done <"$tmp/SHA256SUMS.txt"
[[ $listed -gt 0 ]] || fail "SHA256SUMS.txt is empty"

# every other asset is in the list
while IFS= read -r a; do
  [[ "$a" == *.asc || "$a" == SHA256SUMS.txt ]] && continue
  grep -qE "^[0-9a-f]{64} [ *]?${a//./\\.}\$" "$tmp/SHA256SUMS.txt" || fail "$a is attached but missing from SHA256SUMS.txt"
done <<<"$names"
grep -qxF SHA256SUMS.txt.asc <<<"$names" || fail "SHA256SUMS.txt has no detached signature"

[[ $problems -eq 0 ]] || die "$REPO $TAG stays a draft: $problems problem(s) above"

if [[ "$dry" == "true" ]]; then
  echo "dry-run: $REPO $TAG passes all checks ($listed files) - would run: gh release edit $TAG --draft=false --prerelease"
  exit 0
fi
gh release edit "$TAG" --draft=false --prerelease --latest=false
echo "$REPO $TAG published as Pre-release"
