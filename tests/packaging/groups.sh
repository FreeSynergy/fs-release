#!/usr/bin/env bash
# Pattern: none (test). One RPM/DEB per cargo package that provides a binary (workspace with an app and a server).
T3="$T/tasks"; X=x86_64-unknown-linux-gnu
make_pkg "$T3/app" tasks-app 0.1.1 fs-tasks-app
make_pkg "$T3/server" tasks-server 0.1.1 fs-tasks-server
printf '[workspace]\nmembers = ["app", "server"]\nresolver = "2"\n' > "$T3/Cargo.toml"
# The app repo declares its own rpm metadata, including the dependency on the server rpm.
printf '\n[package.metadata.generate-rpm]\nassets = [{ source = "target/release/fs-tasks-app", dest = "/usr/bin/fs-tasks-app", mode = "755" }]\n[package.metadata.generate-rpm.requires]\ntasks-server = "*"\n' >> "$T3/app/Cargo.toml"
: > "$T3/LICENSE-MIT"
app_sum=$(sha256sum "$T3/app/Cargo.toml")

# --- package_groups / pkg_select (unit)
# shellcheck source=../../scripts/lib.sh
source "$ROOT/scripts/lib.sh"
groups_of() { (cd "$1" && PLAN=$2 package_groups); }
TAB=$'\t'
check_has "groups: two packages -> two groups (app)" \
  "$(groups_of "$T3" '{"binaries":["fs-tasks-app","fs-tasks-server"],"cargo_package":""}')" "tasks-app${TAB}fs-tasks-app"
check_has "groups: two packages -> two groups (server)" \
  "$(groups_of "$T3" '{"binaries":["fs-tasks-app","fs-tasks-server"],"cargo_package":"tasks-app"}')" "tasks-server${TAB}fs-tasks-server"
check_golden "groups: order follows the binaries input" \
  "$(groups_of "$T3" '{"binaries":["fs-tasks-server","fs-tasks-app"],"cargo_package":""}')" <(printf 'tasks-server\tfs-tasks-server\ntasks-app\tfs-tasks-app\n')
T4="$T/multi"; make_pkg "$T4" solo 0.1.1 one two
check_has "groups: one package with two binaries -> one group" \
  "$(groups_of "$T4" '{"binaries":["one","two"],"cargo_package":""}')" "solo${TAB}one,two"
check_golden "pkg_select: single package without cargo-package names nothing" \
  "$(cd "$T4" && PLAN='{"binaries":["one","two"],"cargo_package":""}' pkg_select solo)" <(printf '\n')
check_golden "pkg_select: cargo-package set -> -p" \
  "$(cd "$T4" && PLAN='{"binaries":["one"],"cargo_package":"solo"}' pkg_select solo)" <(printf -- '-p\nsolo\n')
check_golden "pkg_select: more than one group -> -p" \
  "$(cd "$T3" && PLAN='{"binaries":["fs-tasks-app","fs-tasks-server"],"cargo_package":""}' pkg_select tasks-app)" <(printf -- '-p\ntasks-app\n')
if (cd "$T3" && PLAN='{"binaries":["nope"],"cargo_package":""}' package_groups) >/dev/null 2>&1; then
  fail "groups: unknown binary must fail"; else pass "groups: unknown binary fails"; fi

# --- the whole Linux flow, with and without cargo-package (both must give the same result)
for cp in "" "CARGO_PACKAGE=tasks-app"; do
  plan=$(make_plan "$T3" BINARIES=fs-tasks-app,fs-tasks-server KIND=app NAME=fs-tasks ${cp:+"$cp"})
  out=$(run_linux "$T3" "$plan" tar.gz,deb,rpm)
  l="[${cp:-no cargo-package}]"
  for p in "app:tasks-app:fs-tasks-app" "server:tasks-server:fs-tasks-server"; do
    IFS=: read -r dir pk bn <<<"$p"
    check_has "$l build per package: $pk" "$out" "cargo build --release --locked --target $X -p $pk --bin $bn"
    check_has "$l deb per package: $pk" "$out" "cargo deb --no-build --target $X -p $pk --output @R@/dist/"
    check_has "$l rpm per package: $pk" "$out" "cargo generate-rpm --target $X -p $dir -s version = \"0.1.1\" --output @R@/dist/"
  done
  check_lacks "$l no build mixes both binaries" "$(grep '^cargo build' <<<"$out")" "--bin fs-tasks-app --bin fs-tasks-server"
  check_has "$l one tar.gz with the shared name" "$out" "fs-tasks-0.1.1-linux-x86_64.tar.gz"
  [[ "$(sha256sum "$T3/app/Cargo.toml")" == "$app_sum" ]] && pass "$l app rpm metadata (own requires) untouched" || fail "$l app manifest changed"
  check_lacks "$l server default assets list only the server" \
    "$(grep -A1 'rpm metadata in server' <<<"$out" | tail -n1)" "fs-tasks-app"
  tar -tzf "$T3/dist/fs-tasks-0.1.1-linux-x86_64.tar.gz" | sort | tr '\n' ' ' > "$T/tarlist"
  check_has "$l tar.gz holds both binaries" "$(cat "$T/tarlist")" "fs-tasks-0.1.1-linux-x86_64/ fs-tasks-0.1.1-linux-x86_64/fs-tasks-app fs-tasks-0.1.1-linux-x86_64/fs-tasks-server fs-tasks-0.1.1-linux-x86_64/LICENSE-MIT "
  # restore the server manifest (the flow appends to it)
  make_pkg "$T3/server" tasks-server 0.1.1 fs-tasks-server
done

# --- plan.py: binaries outside cargo-package are accepted when another package provides them; unknown ones fail early
if make_plan "$T3" BINARIES=fs-tasks-app,nope CARGO_PACKAGE=tasks-app >/dev/null 2>&1; then
  fail "plan: unknown binary must fail"; else pass "plan: unknown binary fails"; fi
check_has "plan: cargo-package + binary of the other package" \
  "$(make_plan "$T3" BINARIES=fs-tasks-app,fs-tasks-server CARGO_PACKAGE=tasks-app NAME=fs-tasks | jq -r '.binaries | join(",")')" "fs-tasks-app,fs-tasks-server"
