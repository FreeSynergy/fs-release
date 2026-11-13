#!/usr/bin/env bash
# The file checks of scripts/gates.sh (called by it, and by tests): yaml-check of .github/workflows and the schema check of
# workflows/. Exit codes of the python scripts: 0 ok, 2 no PyYAML (nothing was checked: said loudly, not green), else red.
#   bash scripts/gates-files.sh <repo root>
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$1" && pwd)"
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
