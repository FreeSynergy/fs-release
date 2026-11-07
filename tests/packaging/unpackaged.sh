#!/usr/bin/env bash
# Pattern: none (test). A binary under crates/ that is not in `binaries` fails the plan unless it is listed in ignore-binaries.
TU="$T/unpkg"
make_pkg "$TU/crates/cli" ucli 0.1.1
make_pkg "$TU/crates/helper" uhelper 0.1.1
make_pkg "$TU/tools/other" uother 0.1.1   # not under crates/: not checked
printf '[workspace]\nmembers = ["crates/cli", "crates/helper", "tools/other"]\nresolver = "2"\n' > "$TU/Cargo.toml"
if make_plan "$TU" BINARIES=ucli >"$T/o" 2>&1; then fail "unpackaged: loose crates/ binary must fail"; else pass "unpackaged: loose crates/ binary fails"; fi
msg=$(cd "$TU" && env TAG=v0.1.1 BINARIES=ucli GITHUB_OUTPUT="$T/out" python3 -I "$ROOT/actions/plan/plan.py" 2>&1 || true)
check_has "unpackaged: message names the binary" "$(grep -o 'uhelper (uhelper)' <<<"$msg")" "uhelper (uhelper)"
check_lacks "unpackaged: binary outside crates/ is not reported" "$msg" "uother"
check_has "unpackaged: listed in binaries -> ok" "$(make_plan "$TU" BINARIES=ucli,uhelper | jq -r '.binaries | join(",")')" "ucli,uhelper"
check_has "unpackaged: ignore-binaries -> ok" "$(make_plan "$TU" BINARIES=ucli IGNORE_BINARIES=uhelper | jq -r '.binaries | join(",")')" "ucli"
check_has "unpackaged: ignore-binaries * -> ok" "$(make_plan "$TU" BINARIES=ucli 'IGNORE_BINARIES=*' | jq -r '.binaries | join(",")')" "ucli"
