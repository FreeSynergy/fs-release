#!/usr/bin/env bash
# Pattern: none (test). File names in dist and in SHA256SUMS.txt agree: no '~' (rpm pre-release) or other unsafe characters.
# shellcheck source=../../scripts/lib.sh
source "$ROOT/scripts/lib.sh"
TN="$T/names"; mkdir -p "$TN"

# unit: normalise + check
D="$TN/unit"; mkdir -p "$D"
: > "$D/a-0.6.1~rc.2-1.x86_64.rpm"; : > "$D/a_0.6.1~rc.2-1_amd64.deb"; : > "$D/ok-0.6.1-rc.2.tar.gz"; : > "$D/b-1.0+build.zip"
if dist_check "$D" >/dev/null 2>&1; then fail "names: unsafe names must be detected"; else pass "names: unsafe names detected"; fi
dist_normalise "$D"
check_golden "names: ~ and + become -" "$(cd "$D" && LC_ALL=C ls)" <(printf 'a-0.6.1-rc.2-1.x86_64.rpm\na_0.6.1-rc.2-1_amd64.deb\nb-1.0-build.zip\nok-0.6.1-rc.2.tar.gz\n')
dist_check "$D" >/dev/null && pass "names: check passes after normalise" || fail "names: check after normalise"
: > "$D/c-1~x.rpm"; : > "$D/c-1-x.rpm"
if (dist_normalise "$D") >/dev/null 2>&1; then fail "names: collision must fail"; else pass "names: collision fails"; fi

# flow: package-linux with a 0.6.1-rc.2 style version; stubs emit rpm/deb files named like the real tools do (with '~').
R="$TN/rc"; make_pkg "$R" rcapp 0.6.1-rc.2 rcapp
plan=$(make_plan "$R" TAG=v0.6.1-rc.2 BINARIES=rcapp KIND=cli)
out=$(STUB_EMIT='0.6.1~rc.2' run_linux "$R" "$plan" tar.gz,deb,rpm)
check_has "names: rpm plan version uses ~" "$out" "cargo generate-rpm --target x86_64-unknown-linux-gnu -s version = \"0.6.1~rc.2\" --output @R@/dist/"
check_has "names: rpm in dist has no ~" "$out" "stub-0.6.1-rc.2-1.x86_64.rpm"
check_has "names: deb in dist has no ~" "$out" "stub_0.6.1-rc.2-1_amd64.deb"
check_has "names: tar.gz keeps the SemVer version" "$out" "rcapp-0.6.1-rc.2-linux-x86_64.tar.gz"
# SHA256SUMS.txt via the real checksums step (same shell code as actions/checksums/action.yml)
sums=$(cd "$R/dist" && source "$ROOT/scripts/lib.sh" && dist_normalise . && dist_check . && find . -maxdepth 1 -type f -printf '%P\n' | LC_ALL=C sort | xargs -d '\n' sha256sum)
( cd "$R/dist" && printf '%s\n' "$sums" | sha256sum -c - >/dev/null ) && pass "names: every name in the checksum list exists in dist" || fail "names: checksum list does not match dist"
check_lacks "names: checksum list has no ~" "$sums" "~"
