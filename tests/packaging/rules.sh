#!/usr/bin/env bash
# Pattern: none (test). scripts/check_rust_rules.py (used by check.yml via actions/rust-rules): passes a clean tree, fails a rule break.
TR="$T/rules"; mkdir -p "$TR/src"
printf '//! Entry.\n//! Pattern: none\nfn main() {}\n' > "$TR/src/main.rs"
python3 -I "$ROOT/scripts/check_rust_rules.py" "$TR" >/dev/null && pass "rules: clean tree passes" || fail "rules: clean tree must pass"
echo 'fn main() {}' > "$TR/src/bad.rs"
python3 -I "$ROOT/scripts/check_rust_rules.py" "$TR" >/dev/null 2>&1 && fail "rules: missing module doc must fail" || pass "rules: missing module doc fails"
