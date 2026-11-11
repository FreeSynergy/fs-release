#!/usr/bin/env bash
# Caret and field detection of the typing test on synthetic pictures (no app, no KWin).
#   tests/typing-test/caret.sh
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec python3 -I "$here/caret.py"
