#!/usr/bin/env bash
# The one measure, as one command with a real exit code (ADR 0043, coding rules). It must check exactly what
# .github/workflows/check.yml checks; change both together.
#   scripts/gates.sh <repo root> && git push ... && git tag ...
# Run it on a fresh clone. Do not rely on `set -e` in a tool call: the shell wrapper runs commands inside an
# `&&` list, where bash ignores it. Inside this script it works.
set -euo pipefail
root="$(cd "$1" && pwd)"
rules="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/check_rust_rules.py"
cd "$root"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-4}"
export TMPDIR="${TMPDIR_GATES:-/srv/FreeSynergy/.scratch}"
if [ -n "$(git status --porcelain --untracked-files=no)" ]; then
  echo "gates FAILED: uncommitted changes in $root" >&2
  exit 1
fi
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings -W clippy::pedantic
cargo test --workspace --all-features
python3 -I "$rules" "$root"
echo "gates ok: $root at $(git rev-parse --short HEAD)"
