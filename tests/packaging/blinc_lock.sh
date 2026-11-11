#!/usr/bin/env bash
# Pattern: none (test). scripts/check_blinc_lock.py + the hook in check_rust_rules.py: Cargo.lock may carry Blinc only from the approved fs-blinc tag.
TB="$T/blinc"; mkdir -p "$TB/repo/src"
S3=8a0251b2105a070626ad8390cd14d7ef7483c0f5; S2=1111111111111111111111111111111111111111; SX=2222222222222222222222222222222222222222
printf 'fs-blinc-3 %s\n' "$S3" > "$TB/approved.txt"
B="git+https://github.com/FreeSynergy/Blinc"
# lock NAME SOURCE... -> Cargo.lock text with a serde package and one package per NAME=SOURCE pair
lock() { printf '[[package]]\nname = "serde"\nversion = "1.0.0"\nsource = "registry+https://github.com/rust-lang/crates.io-index"\n\n'
  for p in "$@"; do printf '[[package]]\nname = "%s"\nversion = "0.6.0"\nsource = "%s"\n\n' "${p%%=*}" "${p#*=}"; done; }
LK() { python3 -I "$ROOT/scripts/check_blinc_lock.py" --approved "$TB/approved.txt" - >"$TB/out" 2>&1; }
lk_ok()  { lock "${@:2}" | LK && pass "blinc-lock: $1" || fail "blinc-lock: $1 (must pass): $(cat "$TB/out")"; }
lk_bad() { if lock "${@:2}" | LK; then fail "blinc-lock: $1 (must fail)"; else pass "blinc-lock: $1"; fi; }
lk_ok  "only fs-blinc-3 is green" "blinc_app=$B?tag=fs-blinc-3#$S3" "blinc_layout=$B?tag=fs-blinc-3#$S3"
lk_bad "blinc-app-fix is red" "blinc_app=git+https://github.com/FreeSynergy/blinc-app-fix#$SX"
lk_bad "blinc-layout-fix with rev is red" "blinc_layout=git+https://github.com/FreeSynergy/blinc-layout-fix?rev=$SX#$SX"
lk_bad "crates.io blinc_app is red" "blinc_app=registry+https://github.com/rust-lang/crates.io-index"
lk_bad "path blinc package (no source) is red" "blinc_core="
lk_bad "two Blinc tags (fs-blinc-2 + fs-blinc-3) are red" "blinc_app=$B?tag=fs-blinc-2#$S2" "blinc_layout=$B?tag=fs-blinc-3#$S3"
lk_bad "fs-blinc-3 at another sha (tag moved) is red" "blinc_app=$B?tag=fs-blinc-3#$SX"
lk_bad "Blinc by branch is red" "blinc_app=$B?branch=main#$S3"
lk_bad "a fork under a non-blinc name with a blinc source is red" "other=git+https://github.com/someone/blinc#$SX"
lk_ok  "a lock without Blinc is green"
if printf '' | LK; then pass "blinc-lock: an empty lock is green"; else fail "blinc-lock: empty lock must pass"; fi
lock "blinc_app=git+https://github.com/FreeSynergy/blinc-app-fix#$SX" | LK || true; check_in "blinc-lock: the error names package, source and approved tag" "$(cat "$TB/out")" "package \`blinc_app\`"
check_in "blinc-lock: the error names the bump script" "$(cat "$TB/out")" "approve-blinc.sh"
check_in "blinc-lock: the error names the approved tag" "$(cat "$TB/out")" "fs-blinc-3"
# the real approved file is one full-sha line for the current tag
if python3 -I "$ROOT/scripts/check_blinc_lock.py" - </dev/null >/dev/null; then pass "blinc-lock: real blinc-approved.txt parses"; else fail "blinc-lock: real blinc-approved.txt broken"; fi
# the gate: check_rust_rules.py reads <repo>/Cargo.lock without any switch
printf '//! Entry.\n//! Pattern: none\nfn main() {}\n' > "$TB/repo/src/main.rs"
RRL() { python3 -I "$ROOT/scripts/check_rust_rules.py" "$TB/repo" >/dev/null 2>&1; }
APPROVED_REAL=$(grep -v '^#' "$ROOT/scripts/blinc-approved.txt" | head -1)
tag=${APPROVED_REAL% *}; sha=${APPROVED_REAL#* }
lock "blinc_app=$B?tag=$tag#$sha" > "$TB/repo/Cargo.lock"; RRL && pass "rules: lock with the approved Blinc passes the gate" || fail "rules: approved lock must pass"
lock "blinc_app=git+https://github.com/FreeSynergy/blinc-app-fix#$SX" > "$TB/repo/Cargo.lock"; RRL && fail "rules: blinc-app-fix in Cargo.lock must fail the gate" || pass "rules: blinc-app-fix in Cargo.lock fails the gate"
lock "blinc_app=$B?tag=fs-blinc-2#$S2" > "$TB/repo/Cargo.lock"; RRL && fail "rules: unapproved tag must fail the gate" || pass "rules: unapproved tag fails the gate"
lock > "$TB/repo/Cargo.lock"; RRL && pass "rules: lock without Blinc passes the gate" || fail "rules: lock without Blinc must pass"
rm -f "$TB/repo/Cargo.lock"; RRL && pass "rules: no Cargo.lock passes" || fail "rules: no Cargo.lock must pass"
# approve-blinc.sh against a local Blinc repo
git init -q "$TB/bl"; git -C "$TB/bl" -c user.name=t -c user.email=t@example.org commit -q --allow-empty -m x; git -C "$TB/bl" tag fs-blinc-9
mkdir -p "$TB/s"; cp "$ROOT/scripts/approve-blinc.sh" "$TB/s/"; cp "$ROOT/scripts/blinc-approved.txt" "$TB/s/"
FS_BLINC_URL="$TB/bl" bash "$TB/s/approve-blinc.sh" fs-blinc-9 >/dev/null; want="fs-blinc-9 $(git -C "$TB/bl" rev-parse HEAD)"
check_has "approve-blinc.sh writes tag and full sha" "$(cat "$TB/s/blinc-approved.txt")" "$want"
check_lacks "approve-blinc.sh replaces the old tag" "$(cat "$TB/s/blinc-approved.txt")" "fs-blinc-3"
if FS_BLINC_URL="$TB/bl" bash "$TB/s/approve-blinc.sh" fs-blinc-8 >/dev/null 2>&1; then fail "approve-blinc.sh: missing tag must fail"; else pass "approve-blinc.sh: missing tag fails"; fi
