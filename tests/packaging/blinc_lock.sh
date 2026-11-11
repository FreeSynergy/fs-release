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
mkdir -p "$TB/nolock"
if python3 -I "$ROOT/scripts/check_blinc_lock.py" --approved "$TB/approved.txt" "$TB/nolock" >"$TB/out" 2>&1; then pass "blinc-lock: a repo directory without Cargo.lock is green (nothing to check)"; else fail "blinc-lock: no Cargo.lock must pass, got: $(cat "$TB/out")"; fi
check_lacks "blinc-lock: no Cargo.lock gives no traceback" "$(cat "$TB/out")" "Traceback"
if python3 -I "$ROOT/scripts/check_blinc_lock.py" --approved "$TB/approved.txt" "$TB/nolock/Cargo.lock" >/dev/null 2>&1; then fail "blinc-lock: a named missing lock file must still fail"; else pass "blinc-lock: a named missing lock file still fails"; fi
lock "blinc_app=git+https://github.com/FreeSynergy/blinc-app-fix#$SX" | LK || true; check_in "blinc-lock: the error names package, source and approved tag" "$(cat "$TB/out")" "package \`blinc_app\`"
check_in "blinc-lock: the error names fs-components and the approve script for maintainers" "$(cat "$TB/out")" "only through fs-components"
check_in "blinc-lock: the error counts the packages" "$(cat "$TB/out")" "1 Blinc package(s)"
check_in "blinc-lock: the error names approve-blinc.sh" "$(cat "$TB/out")" "approve-blinc.sh"
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
# bypass: a replacement hidden in .cargo/config, only when the lock has Blinc (always on, no .fs-rules.toml)
lock "blinc_app=$B?tag=$tag#$sha" > "$TB/repo/Cargo.lock"; mkdir -p "$TB/repo/.cargo"
printf 'paths = ["../blinc-fork"]\n' > "$TB/repo/.cargo/config.toml"; RRL && fail "rules: paths override with Blinc in the lock must fail" || pass "rules: paths override with Blinc in the lock fails"
printf '[source.crates-io]\nreplace-with = "vendored"\n[source.vendored]\ndirectory = "vendor"\n' > "$TB/repo/.cargo/config.toml"; RRL && fail "rules: [source] replace-with with Blinc in the lock must fail" || pass "rules: [source] replace-with with Blinc in the lock fails"
printf '[net]\nretry = 3\n' > "$TB/repo/.cargo/config.toml"; RRL && pass "rules: a harmless .cargo/config passes" || fail "rules: harmless .cargo/config must pass"
lock > "$TB/repo/Cargo.lock"; printf 'paths = ["../x"]\n' > "$TB/repo/.cargo/config.toml"; RRL && pass "rules: paths in a repo without Blinc in the lock is not this rule's business" || fail "rules: no Blinc in lock: paths must not fail here"
rm -rf "$TB/repo/.cargo"
rm -f "$TB/repo/Cargo.lock"; RRL && pass "rules: no Cargo.lock passes" || fail "rules: no Cargo.lock must pass"
# approve-blinc.sh against a local Blinc repo
git init -q "$TB/bl"; git -C "$TB/bl" -c user.name=t -c user.email=t@example.org commit -q --allow-empty -m x; git -C "$TB/bl" tag fs-blinc-9
mkdir -p "$TB/s"; cp "$ROOT/scripts/approve-blinc.sh" "$TB/s/"; cp "$ROOT/scripts/blinc-approved.txt" "$TB/s/"
FS_BLINC_URL="$TB/bl" bash "$TB/s/approve-blinc.sh" fs-blinc-9 >/dev/null; want="fs-blinc-9 $(git -C "$TB/bl" rev-parse HEAD)"
check_has "approve-blinc.sh writes tag and full sha" "$(cat "$TB/s/blinc-approved.txt")" "$want"
check_lacks "approve-blinc.sh replaces the old tag" "$(cat "$TB/s/blinc-approved.txt")" "fs-blinc-3"
printf 'fs-blinc-3.1 %s\nbad line\n' "$S3" > "$TB/approved2.txt"
if python3 -I "$ROOT/scripts/check_blinc_lock.py" --approved "$TB/approved2.txt" - </dev/null >/dev/null 2>&1; then fail "blinc-lock: malformed approved file must fail"; else pass "blinc-lock: malformed approved file fails"; fi
printf 'fs-blinc-3.1 %s\n' "$S3" > "$TB/approved2.txt"
lock "blinc_app=$B?tag=fs-blinc-3.1#$S3" | python3 -I "$ROOT/scripts/check_blinc_lock.py" --approved "$TB/approved2.txt" - >/dev/null && pass "blinc-lock: tag fs-blinc-3.1 is a valid shape (same as approve-blinc.sh)" || fail "blinc-lock: fs-blinc-3.1 must load"
printf '' > "$TB/s/blinc-approved.txt"; FS_BLINC_URL="$TB/bl" bash "$TB/s/approve-blinc.sh" fs-blinc-9 >/dev/null && pass "approve-blinc.sh: a file without comment lines does not abort" || fail "approve-blinc.sh: file without # lines must work"
if FS_BLINC_URL="$TB/bl" bash "$TB/s/approve-blinc.sh" fs-blinc-8 >/dev/null 2>&1; then fail "approve-blinc.sh: missing tag must fail"; else pass "approve-blinc.sh: missing tag fails"; fi
# Transition: two approved tags at once (fs-blinc-3 and fs-blinc-3.1); a lock still holds exactly ONE of them
S31=3333333333333333333333333333333333333333
printf '# transition\nfs-blinc-3 %s  # expires 2026-12-31 until all programs are lifted\nfs-blinc-3.1 %s\n' "$S3" "$S31" > "$TB/approved.txt"
export BLINC_TODAY=2026-11-12
lk_ok  "two approved tags: a lock with only fs-blinc-3 is green" "blinc_app=$B?tag=fs-blinc-3#$S3" "blinc_layout=$B?tag=fs-blinc-3#$S3"
lk_ok  "two approved tags: a lock with only fs-blinc-3.1 is green" "blinc_app=$B?tag=fs-blinc-3.1#$S31" "blinc_layout=$B?tag=fs-blinc-3.1#$S31"
lk_bad "two approved tags: a lock mixing 3 and 3.1 is red" "blinc_app=$B?tag=fs-blinc-3#$S3" "blinc_layout=$B?tag=fs-blinc-3.1#$S31"
lk_bad "two approved tags: an unapproved tag is red" "blinc_app=$B?tag=fs-blinc-2#$S2"
lk_bad "two approved tags: fs-blinc-3.1 at the sha of fs-blinc-3 is red" "blinc_app=$B?tag=fs-blinc-3.1#$S3"
lock "blinc_app=$B?tag=fs-blinc-3#$S3" | LK && pass "blinc-lock: expiry date not reached stays green" || fail "blinc-lock: not yet expired must pass"
export BLINC_TODAY=2026-12-31; lock "blinc_app=$B?tag=fs-blinc-3#$S3" | LK && pass "blinc-lock: the expiry day itself is still green" || fail "blinc-lock: expiry day must pass"
export BLINC_TODAY=2027-01-01
if lock "blinc_app=$B?tag=fs-blinc-3#$S3" | LK; then fail "blinc-lock: an expired entry must be red"; else pass "blinc-lock: an expired entry is red"; fi
check_in "blinc-lock: the expired error names tag and date" "$(cat "$TB/out")" "approval of fs-blinc-3 expired on 2026-12-31"
lock "blinc_app=$B?tag=fs-blinc-3.1#$S31" | LK && fail "blinc-lock: a lock is red while ANY approval is expired" || pass "blinc-lock: any expired entry makes the gate red (forces the cleanup)"
lock | LK && fail "blinc-lock: a lock without Blinc is also red once an entry expired" || pass "blinc-lock: expiry is red for every repo, also without Blinc"
unset BLINC_TODAY
printf 'fs-blinc-3 %s\nfs-blinc-3 %s\n' "$S3" "$S31" > "$TB/approved2.txt"
if python3 -I "$ROOT/scripts/check_blinc_lock.py" --approved "$TB/approved2.txt" - </dev/null >/dev/null 2>&1; then fail "blinc-lock: a tag listed twice must fail"; else pass "blinc-lock: a tag listed twice fails"; fi
printf 'fs-blinc-3 %s # expires 2026-13-45\n' "$S3" > "$TB/approved2.txt"
if python3 -I "$ROOT/scripts/check_blinc_lock.py" --approved "$TB/approved2.txt" - </dev/null >/dev/null 2>&1; then fail "blinc-lock: a bad expiry date must fail"; else pass "blinc-lock: a bad expiry date fails"; fi
# approve-blinc.sh add / remove / list / replace
git -C "$TB/bl" -c user.name=t -c user.email=t@example.org tag -a -m annotated fs-blinc-9.1
A() { FS_BLINC_URL="$TB/bl" bash "$TB/s/approve-blinc.sh" "$@"; }
SHA9=$(git -C "$TB/bl" rev-parse HEAD); printf '# header\nfs-blinc-3 %s\n' "$S3" > "$TB/s/blinc-approved.txt"
A add fs-blinc-9.1 --expires 2026-12-24 >/dev/null; F=$(cat "$TB/s/blinc-approved.txt")
check_has "approve-blinc.sh add keeps the old entry" "$F" "fs-blinc-3 $S3"
check_has "approve-blinc.sh add writes the commit an annotated tag peels to, with expiry" "$F" "fs-blinc-9.1 $SHA9  # expires 2026-12-24"
check_has "approve-blinc.sh add keeps the comment line" "$F" "# header"
check_in "approve-blinc.sh list shows both" "$(A list)" "fs-blinc-9.1 $SHA9"
A add fs-blinc-9.1 >/dev/null; check_lacks "approve-blinc.sh add again replaces the entry (no expiry now)" "$(cat "$TB/s/blinc-approved.txt")" "expires"
A remove fs-blinc-3 >/dev/null; check_lacks "approve-blinc.sh remove takes the entry out" "$(cat "$TB/s/blinc-approved.txt")" "fs-blinc-3 "
check_in "approve-blinc.sh remove leaves the other entry" "$(cat "$TB/s/blinc-approved.txt")" "fs-blinc-9.1 $SHA9"
if A remove fs-blinc-3 >/dev/null 2>&1; then fail "approve-blinc.sh: removing an unapproved tag must fail"; else pass "approve-blinc.sh: removing an unapproved tag fails"; fi
if A add fs-blinc-x >/dev/null 2>&1; then fail "approve-blinc.sh: a bad tag must fail"; else pass "approve-blinc.sh: a bad tag fails"; fi
if A add fs-blinc-9 --expires 2026-02-31 >/dev/null 2>&1; then fail "approve-blinc.sh: an impossible date must fail"; else pass "approve-blinc.sh: an impossible date fails"; fi
if A add fs-blinc-8 >/dev/null 2>&1; then fail "approve-blinc.sh: add of a missing tag must fail"; else pass "approve-blinc.sh: add of a missing tag fails"; fi
A replace fs-blinc-9 >/dev/null; check_lacks "approve-blinc.sh replace leaves only that tag" "$(cat "$TB/s/blinc-approved.txt")" "fs-blinc-9.1"
python3 -I "$ROOT/scripts/check_blinc_lock.py" --approved "$TB/s/blinc-approved.txt" - </dev/null >/dev/null && pass "approve-blinc.sh output is read by the loader" || fail "approve-blinc.sh output must load"
