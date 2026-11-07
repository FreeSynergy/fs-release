#!/usr/bin/env bash
# Pattern: none (test). Real builds with the real cargo-generate-rpm / cargo-deb when installed (skipped otherwise).
if ! command -v cargo-generate-rpm >/dev/null; then echo "skip real rpm build: cargo-generate-rpm not installed"; return 0; fi
R="$T/real"; host=$(rustc -vV | sed -n 's/^host: //p')
make_pkg "$R/app" tasks-app 0.1.1 fs-tasks-app
make_pkg "$R/server" tasks-server 0.1.1 fs-tasks-server
printf '[workspace]\nmembers = ["app", "server"]\nresolver = "2"\n' > "$R/Cargo.toml"
# The app declares its dependency on the server in its own manifest (this is what the docs tell app repos to do).
printf '\n[package.metadata.generate-rpm]\nassets = [{ source = "target/%s/release/fs-tasks-app", dest = "/usr/bin/fs-tasks-app", mode = "755" }]\n[package.metadata.generate-rpm.requires]\ntasks-server = "*"\n' "$host" >> "$R/app/Cargo.toml"
(cd "$R" && cargo generate-lockfile --offline >/dev/null 2>&1)
plan=$(make_plan "$R" BINARIES=fs-tasks-app,fs-tasks-server KIND=app NAME=fs-tasks CARGO_PACKAGE=tasks-app)
formats=tar.gz,rpm; command -v cargo-deb >/dev/null && formats=tar.gz,deb,rpm
plan=$(jq -c --arg f "$formats" '.linux_formats = $f' <<<"$plan")
mkdir -p "$R/stage"
(cd "$R" && PLAN="$plan" ARCH=x86_64 TRIPLE="$host" RUNNER_TEMP="$R/stage" GITHUB_WORKSPACE="$R" \
  bash "$ROOT/actions/package-linux/package.sh" >/dev/null)
rpms=$(cd "$R/dist" && ls ./*.rpm | sed 's#^\./##' | sort | tr '\n' ' ')
check_has "real: one rpm per package" "$rpms" "tasks-app-0.1.1-1.x86_64.rpm tasks-server-0.1.1-1.x86_64.rpm "
if command -v rpm >/dev/null; then
  check_has "real: app rpm requires the server rpm" "$(rpm -qpR "$R/dist/tasks-app-0.1.1-1.x86_64.rpm")" "tasks-server"
  check_has "real: server rpm ships /usr/bin/fs-tasks-server" "$(rpm -qpl "$R/dist/tasks-server-0.1.1-1.x86_64.rpm")" "/usr/bin/fs-tasks-server"
else echo "skip rpm content checks: rpm not installed"; fi
if command -v cargo-deb >/dev/null; then
  debs=$(cd "$R/dist" && ls ./*.deb | sed 's#^\./##' | wc -l)
  check_has "real: one deb per package" "$debs" "2"
else echo "skip real deb build: cargo-deb not installed"; fi
