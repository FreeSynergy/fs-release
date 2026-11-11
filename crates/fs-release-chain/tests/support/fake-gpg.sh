#!/usr/bin/env bash
# Fake `gpg`: a --verify of a file named *.asc passes when it says SIG-OK (or SIG-OTHER-KEY: valid, but another key).
echo "gpg $*" >> "$FAKE_DIR/calls.log"
[[ " $* " == *" --verify "* ]] || exit 0
for a in "$@"; do
  case "$a" in
    *.asc)
      case "$(cat "$a" 2>/dev/null)" in
        SIG-OK) echo "[GNUPG:] VALIDSIG AAAA 2026-11-12 1 0 4 0 1 10 00 16A4A3EE563AEF51FCF0C55DED7B57C4A1904342"; exit 0 ;;
        SIG-OTHER-KEY) echo "[GNUPG:] VALIDSIG BBBB 2026-11-12 1 0 4 0 1 10 00 0000000000000000000000000000000000000000"; exit 0 ;;
        *) echo "gpg: BAD signature" >&2; exit 1 ;;
      esac ;;
  esac
done
