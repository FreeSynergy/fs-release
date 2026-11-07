#!/usr/bin/env bash
# Pattern: none (test). The plan's `features` reach EVERY package build (cargo build per package, cargo deb), filtered to what each package declares.
TF="$T/feat"; X=x86_64-unknown-linux-gnu
make_pkg "$TF/app" fapp 0.1.1 fapp-bin
make_pkg "$TF/server" fserver 0.1.1 fserver-bin
printf '[workspace]\nmembers = ["app", "server"]\nresolver = "2"\n' > "$TF/Cargo.toml"
printf '\n[features]\nfast = []\nonly-app = []\n' >> "$TF/app/Cargo.toml"
printf '\n[features]\nfast = []\nonly-server = []\n' >> "$TF/server/Cargo.toml"
: > "$TF/LICENSE-MIT"
plan=$(make_plan "$TF" BINARIES=fapp-bin,fserver-bin KIND=app NAME=fx FEATURES=fast,only-server)
out=$(run_linux "$TF" "$plan" tar.gz,deb,rpm)
check_has "features: first package build" "$out" "cargo build --release --locked --target $X -p fapp --bin fapp-bin --features fast"
check_has "features: second package build" "$out" "cargo build --release --locked --target $X -p fserver --bin fserver-bin --features fast,only-server"
check_has "features: deb of first package" "$out" "cargo deb --no-build --target $X -p fapp --features fast --output @R@/dist/"
check_has "features: deb of second package" "$out" "cargo deb --no-build --target $X -p fserver --features fast,only-server --output @R@/dist/"
# a feature no package declares fails before any build
plan=$(make_plan "$TF" BINARIES=fapp-bin,fserver-bin KIND=app NAME=fx FEATURES=nope)
rm -rf "$TF/dist" "$TF/target"; mkdir -p "$T/stage"; : > "$T/log"; rc=0
(cd "$TF" && PATH="$T/bin:$PATH" PLAN="$(jq -c '.linux_formats = "tar.gz"' <<<"$plan")" ARCH=x86_64 TRIPLE=$X RUNNER_TEMP="$T" \
  GITHUB_WORKSPACE="$TF" STUB_LOG="$T/log" bash "$ROOT/actions/package-linux/package.sh") >/dev/null 2>&1 || rc=$?
[[ $rc -ne 0 && ! -s "$T/log" ]] && pass "features: undeclared feature fails before any build" || fail "features: undeclared feature must fail before any build (rc=$rc)"
# explicit dep/feature syntax is passed through
plan=$(make_plan "$TF" BINARIES=fapp-bin KIND=app NAME=fx CARGO_PACKAGE=fapp FEATURES=serde/std)
check_has "features: dep/feature passes through" "$(run_linux "$TF" "$plan" tar.gz)" "cargo build --release --locked --target $X -p fapp --bin fapp-bin --features serde/std"
