#!/usr/bin/env bash
# Create (or refresh) a DRAFT release. Nothing here ever publishes it.
set -euo pipefail
: "${TAG:?}" "${REPO:?}" "${DIST:?}" "${GH_TOKEN:?}"
export GH_REPO=$REPO

footer=$(mktemp)
# shellcheck disable=SC2016  # envsubst wants the literal ${REPO}
REPO=$REPO envsubst '${REPO}' < "$(dirname "${BASH_SOURCE[0]}")/footer.md" > "$footer"

if gh release view "$TAG" >/dev/null 2>&1; then
  echo "Release $TAG exists - replacing its assets"
  gh release upload "$TAG" "$DIST"/* --clobber
else
  args=(--draft --verify-tag --title "${NAME:-$REPO} $TAG" --generate-notes --notes-file "$footer")
  [[ "${PRERELEASE:-false}" == "true" ]] && args+=(--prerelease)
  gh release create "$TAG" "${args[@]}" "$DIST"/*
fi
