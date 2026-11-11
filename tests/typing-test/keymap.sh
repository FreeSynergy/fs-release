#!/usr/bin/env bash
# The injector's key table against xkbcomp (see keymap.py). Builds the injector unless TT_INJECT is given.
#   tests/typing-test/keymap.sh
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [ -z "${TT_INJECT:-}" ]; then
  cache="${XDG_CACHE_HOME:-$HOME/.cache}/fs-typing-inject"
  CARGO_TARGET_DIR="$cache" cargo build --release --locked --quiet --manifest-path "$here/../../scripts/typing-test/inject/Cargo.toml"
  TT_INJECT="$cache/release/fs-typing-inject"
fi
export TT_INJECT
exec python3 -I "$here/keymap.py"
