#!/usr/bin/env bash
# Pattern: none (test). check.yml must not carry its own copy of the gate flags: it runs scripts/gates.sh (one definition) and the pinned toolchain.
W="$ROOT/.github/workflows/check.yml"; G="$ROOT/scripts/gates.sh"
grep -q 'clippy --workspace --all-targets --all-features -- -D warnings -W clippy::pedantic' "$G" && pass "gates: gates.sh defines clippy -D warnings + pedantic" || fail "gates: gates.sh lacks the clippy flags"
grep -Eq 'cargo (fmt|clippy|test)' "$W" && fail "gates: check.yml has its own cargo command (must only call actions/gates)" || pass "gates: check.yml has no own cargo commands"
grep -q 'actions/gates@' "$W" && pass "gates: check.yml calls actions/gates" || fail "gates: check.yml does not call actions/gates"
grep -q 'gates.sh' "$ROOT/actions/gates/action.yml" && pass "gates: actions/gates runs gates.sh" || fail "gates: actions/gates does not run gates.sh"
grep -q 'toolchain: pinned' "$W" && pass "gates: check.yml uses the pinned toolchain" || fail "gates: check.yml toolchain not pinned"
grep -q 'RUST_VERSION' "$G" && pass "gates: gates.sh reads RUST_VERSION" || fail "gates: gates.sh ignores RUST_VERSION"
grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+$' "$ROOT/RUST_VERSION" && pass "gates: RUST_VERSION is an exact x.y.z" || fail "gates: RUST_VERSION must be exact x.y.z"
