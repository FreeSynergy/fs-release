#!/usr/bin/env bash
# Pin all internal fs-release references (workflows and composite actions) from @main to a tag.
# Usage: scripts/pin-refs.sh v1.0.0   (run on the release commit, then tag that commit)
# GitHub cannot resolve "same ref as the caller" for nested reusable workflows and composite
# actions across repositories, so the references are explicit and rewritten at release time.
set -euo pipefail
ref=${1:?usage: pin-refs.sh <tag>}
cd "$(dirname "${BASH_SOURCE[0]}")/.."
grep -rlE 'FreeSynergy/fs-release/.+@' .github actions | while read -r f; do
  sed -i -E "s#(FreeSynergy/fs-release/[A-Za-z0-9_./-]+)@[A-Za-z0-9_.-]+#\1@${ref}#g" "$f"
done
git diff --stat
