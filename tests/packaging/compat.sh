#!/usr/bin/env bash
# Pattern: none (test). Backward compatibility: a single-package repo (fs-capnpc) must plan and package exactly as
# before the per-package change. The golden files were produced by the code on main before that change.
repo="$T/capnpc"
mkdir -p "$repo/src"
cp "$ROOT/tests/fixtures/fs-capnpc/Cargo.toml" "$repo/"
echo 'fn main() {}' > "$repo/src/main.rs"; : > "$repo/src/lib.rs"; : > "$repo/LICENSE-MIT"; : > "$repo/README.md"
# Inputs exactly as in the caller file tests/fixtures/fs-capnpc/release.yml.
plan=$(make_plan "$repo" BINARIES=capnp-compiler KIND=cli APP_ID=net.freesynergy.CapnpCompiler \
  DESCRIPTION="Cap'n Proto schema compiler in Rust")
check_golden "capnpc: plan JSON unchanged" "$plan" "$ROOT/tests/golden/capnpc-plan.json"
check_golden "capnpc: build/deb/rpm commands, dist names unchanged" \
  "$(run_linux "$repo" "$plan" tar.gz,deb,rpm)" "$ROOT/tests/golden/capnpc-linux.txt"
# The same repo with cargo-package set (a documented, still valid input).
plan=$(make_plan "$repo" BINARIES=capnp-compiler KIND=cli CARGO_PACKAGE=capnp-compiler)
check_golden "capnpc + cargo-package: commands unchanged" \
  "$(run_linux "$repo" "$plan" tar.gz,deb,rpm)" "$ROOT/tests/golden/capnpc-pkg-linux.txt"
