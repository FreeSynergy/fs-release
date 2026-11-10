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
# Release builds use the same pinned toolchain as the gates: no 'stable', no other 'toolchain:' value (one allowed: the msrv
# input in ci.yml), no own rustup/cargo +x in a run step. Comments are ignored; the helper is checked with fixtures below.
TS="$ROOT/tests/packaging/toolchain_scan.py"
if bad=$(python3 -I "$TS" "$ROOT"); then pass "gates: no workflow/action names stable or an unpinned toolchain, release builds pinned"; else fail "gates: unpinned toolchain: $bad"; fi
TG="$T/gates"; mkdir -p "$TG/.github/workflows" "$TG/actions"
scan() { python3 -I "$TS" "$TG" >/dev/null 2>&1; }
printf 'jobs:\n  a:\n    steps:\n      - uses: x/setup-rust@1 # stable is not meant here\n        with:\n          toolchain: pinned # the pinned one\n      # stable\n' > "$TG/.github/workflows/c.yml"
scan && pass "gates: 'stable' in a comment and a trailing comment after 'pinned' are fine" || fail "gates: comments must not count"
printf 'jobs:\n  a:\n    steps:\n      - run: rustup default stable\n' > "$TG/.github/workflows/c.yml"
scan && fail "gates: run: rustup default stable must fail" || pass "gates: run: rustup default stable fails"
printf 'jobs:\n  a:\n    steps:\n      - run: cargo +nightly fmt\n' > "$TG/.github/workflows/c.yml"
scan && fail "gates: run: cargo +nightly must fail" || pass "gates: run: cargo +nightly fails"
printf 'jobs:\n  a:\n    steps:\n      - uses: x/setup-rust@1\n        with:\n          toolchain: stable\n' > "$TG/.github/workflows/c.yml"
scan && fail "gates: toolchain: stable must fail" || pass "gates: toolchain: stable fails"
rm -f "$TG/.github/workflows/c.yml"
printf 'jobs:\n  a:\n    steps:\n      - uses: x/setup-rust@1\n        with:\n          a: 1\n          b: 2\n          c: 3\n          toolchain: pinned\n' > "$TG/.github/workflows/_release-x.yml"
scan && pass "gates: toolchain: pinned far below uses: is found" || fail "gates: toolchain more than 2 lines below uses: must pass"
printf 'jobs:\n  a:\n    steps:\n      - uses: x/setup-rust@1\n      - uses: y/other@1\n        with:\n          toolchain: pinned\n' > "$TG/.github/workflows/_release-x.yml"
scan && fail "gates: a pinned toolchain in the NEXT step must not count" || pass "gates: setup-rust without its own toolchain: pinned fails"
rm -rf "$TG"
grep -q 'default: pinned' "$ROOT/actions/setup-rust/action.yml" && pass "gates: setup-rust defaults to pinned" || fail "gates: setup-rust default is not pinned"
