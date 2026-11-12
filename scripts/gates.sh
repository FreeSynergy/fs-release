#!/usr/bin/env bash
# The one measure, as one command with a real exit code (ADR 0043, coding rules). This file is the ONLY definition
# of the checks and their flags: .github/workflows/check.yml runs it through actions/gates, so CI and local cannot
# drift (tests/packaging/gates.sh enforces that). The toolchain is the one in RUST_VERSION (pinned; see DESIGN.md).
#   scripts/gates.sh <repo root> && git push ... && git tag ...
# Run it on a fresh clone. Do not rely on `set -e` in a tool call: the shell wrapper runs commands inside an
# `&&` list, where bash ignores it. Inside this script it works.
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$1" && pwd)"
rules="$here/check_rust_rules.py"
RUSTUP_TOOLCHAIN="$(tr -d '[:space:]' < "$here/../RUST_VERSION")"
export RUSTUP_TOOLCHAIN
cd "$root"
if [ -z "${CI:-}" ]; then
  export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-4}"
  export TMPDIR="${TMPDIR_GATES:-/srv/FreeSynergy/.scratch}"
  if [ -n "$(git status --porcelain --untracked-files=no)" ]; then
    echo "gates FAILED: uncommitted changes in $root" >&2
    exit 1
  fi
fi
echo "gates: rustc $(rustc --version)"
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings -W clippy::pedantic
cargo test --workspace --all-features
python3 -I "$rules" "$root"
# yaml-check: every .github/workflows/*.yml parses (exit 2 = no PyYAML: nothing was checked, said loudly, not green-washed).
rc=0
python3 -I "$here/check_yaml.py" "$root" || rc=$?
if [ "$rc" -eq 2 ]; then
  echo "gates WARNING: yaml-check SKIPPED, the workflow files were not checked (install python3-pyyaml)" >&2
elif [ "$rc" -ne 0 ]; then
  echo "gates FAILED: yaml-check" >&2
  exit 1
fi
# workflows/ (visible workflows, schema check) only in the repo that has the folder.
if [ -d "$root/workflows" ]; then
  rc=0
  python3 -I "$here/check_workflows.py" "$root/workflows" || rc=$?
  if [ "$rc" -eq 2 ]; then
    echo "gates WARNING: workflow schema check SKIPPED (no PyYAML)" >&2
  elif [ "$rc" -ne 0 ]; then
    echo "gates FAILED: workflows/ schema check" >&2
    exit 1
  fi
fi
echo "gates ok: $root at $(git rev-parse --short HEAD 2>/dev/null || echo '?')"
