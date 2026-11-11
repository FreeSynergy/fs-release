#!/usr/bin/env bash
# analyze.py of the typing test on synthetic result directories (no app, no KWin).
#   tests/typing-test/glyphs.sh
set -euo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec python3 -I "$here/glyphs.py"
