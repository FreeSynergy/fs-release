#!/usr/bin/env bash
# Raise the tag of REPO in CATALOG_REPO/packages.toml and push to main. The push starts fs-catalog's `sign` workflow.
# Only plain release tags vX.Y.Z of a program that is listed in the catalog; anything else is skipped with a notice.
# Idempotent: the same tag again changes nothing; an older tag never lowers the catalog. DRY_RUN=true pushes nothing.
set -euo pipefail
: "${TAG:?}" "${REPO:?}"
catalog=${CATALOG_REPO:-FreeSynergy/fs-catalog}
dry=${DRY_RUN:-false}
here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)

die() { printf '::error::%s\n' "$*"; exit 1; }

if [[ ! "$TAG" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "::notice::$TAG is not a plain release tag (vX.Y.Z) - the catalog is not touched"
  exit 0
fi

token=${CATALOG_TOKEN:-}
if [[ -z "$token" ]]; then
  msg="secret CATALOG_TOKEN is missing: a fine-grained token (or GitHub App token) with Contents: read and write on $catalog, passed to the release workflow as secrets.CATALOG_TOKEN. $REPO $TAG is published as Pre-release but NOT in the catalog; run the catalog step again once the secret exists."
  if [[ "$dry" == "true" ]]; then printf '::warning::dry-run: %s\n' "$msg"; else die "$msg"; fi
fi

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
if [[ -n "$token" ]]; then
  echo "::add-mask::$token"
  # the token travels in a header, never in the URL or the remote config
  auth=$(printf 'x-access-token:%s' "$token" | base64 -w0)
  echo "::add-mask::$auth"
  git_auth=(-c "http.https://github.com/.extraheader=AUTHORIZATION: basic $auth")
  git "${git_auth[@]}" clone --quiet --depth 1 --branch main "https://github.com/$catalog.git" "$work/catalog"
else
  # dry-run without a token: the catalog is public
  git clone --quiet --depth 1 --branch main "https://github.com/$catalog.git" "$work/catalog"
  git_auth=()
fi
cd "$work/catalog"

for attempt in 1 2 3; do
  result=$(python3 -I "$here/bump_toml.py" packages.toml "$REPO" "$TAG")
  read -r state id old _ <<<"$result"
  case "$state" in
    unknown) echo "::notice::$REPO is not a program of $catalog - nothing to bump"; exit 0 ;;
    same) echo "$id is already at $TAG in the catalog - nothing to do"; exit 0 ;;
    older) echo "::warning::$id: catalog has $old, $TAG is older - not lowering it"; exit 0 ;;
    changed) ;;
    *) die "unexpected answer from bump_toml.py: $result" ;;
  esac
  git diff --stat
  if [[ "$dry" == "true" ]]; then
    echo "dry-run: would commit 'catalog: $id ${TAG#v}' ($old -> $TAG) and push to $catalog main"
    exit 0
  fi
  git config user.name "FreeSynergy Release"
  git config user.email "support@freesynergy.net"
  git commit --quiet -am "catalog: $id ${TAG#v}"
  if git "${git_auth[@]}" push --quiet origin HEAD:main; then
    echo "$catalog now offers $id $TAG; its sign workflow regenerates and signs catalog.xml"
    exit 0
  fi
  echo "::warning::push rejected (attempt $attempt) - fetching and trying again"
  git "${git_auth[@]}" fetch --quiet origin main
  git reset --quiet --hard origin/main
done
die "could not push to $catalog after 3 attempts (branch protection without a bypass for the token?)"
