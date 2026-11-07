#!/usr/bin/env bash
# Pattern: none (test). package-windows: one msi per cargo package, names that cannot clash.
X=x86_64-pc-windows-msvc
printf '#!/usr/bin/env bash\nexit 0\n' > "$T/bin/7z"; chmod +x "$T/bin/7z"
# run_win DIR PLAN -> run package-windows/package.sh (msi only) with stubs; prints normalised cargo log and dist listing.
run_win() {
  local d=$1 plan; plan=$(jq -c '.windows_formats = "msi"' <<<"$2")
  rm -rf "$d/dist" "$d/target" "$T/stage"; mkdir -p "$T/stage"; : > "$T/log"
  (cd "$d" && PATH="$T/bin:$PATH" PLAN="$plan" ARCH=x86_64 TRIPLE=$X RUNNER_TEMP="$T" WIX=x GITHUB_WORKSPACE="$d" \
    STUB_LOG="$T/log" STUB_EMIT=1 bash "$ROOT/actions/package-windows/package.sh" >/dev/null)
  while IFS= read -r l; do echo "$l"; done < "$T/log"
  echo "--- dist"; (cd "$d/dist" && LC_ALL=C ls)
}
TM="$T/msi2"
make_pkg "$TM/app" wapp 0.1.1 wapp-bin
make_pkg "$TM/server" wserver 0.1.1 wserver-bin
printf '[workspace]\nmembers = ["app", "server"]\nresolver = "2"\n' > "$TM/Cargo.toml"
plan=$(make_plan "$TM" BINARIES=wapp-bin,wserver-bin KIND=app NAME=wx)
out=$(run_win "$TM" "$plan" | sed "s#$TM#@R@#g")
check_has "msi: wix init for app" "$out" "cargo wix init -p wapp"
check_has "msi: wix init for server" "$out" "cargo wix init -p wserver"
check_has "msi: app msi" "$out" "cargo wix --no-build --nocapture --target $X -p wapp --install-version 0.1.1 --output @R@/dist/wapp-0.1.1-windows-x86_64.msi"
check_has "msi: server msi" "$out" "cargo wix --no-build --nocapture --target $X -p wserver --install-version 0.1.1 --output @R@/dist/wserver-0.1.1-windows-x86_64.msi"
check_has "msi: two distinct files in dist" "$out" "wapp-0.1.1-windows-x86_64.msi"
check_has "msi: two distinct files in dist (server)" "$out" "wserver-0.1.1-windows-x86_64.msi"
# an existing wix/main.wxs of a package is kept (no init for it)
mkdir -p "$TM/app/wix"; : > "$TM/app/wix/main.wxs"
check_lacks "msi: existing main.wxs -> no init" "$(run_win "$TM" "$plan")" "cargo wix init -p wapp"
# single package keeps the old name
TS="$T/msi1"; make_pkg "$TS" wsolo 0.1.1 wsolo
out=$(run_win "$TS" "$(make_plan "$TS" BINARIES=wsolo NAME=wsolo)" | sed "s#$TS#@R@#g")
check_has "msi: single package keeps <name>-<version>-windows-<arch>.msi" "$out" "wsolo-0.1.1-windows-x86_64.msi"
check_has "msi: single package build unchanged (no -p)" "$out" "cargo wix --no-build --nocapture --target $X --install-version 0.1.1 --output @R@/dist/wsolo-0.1.1-windows-x86_64.msi"
