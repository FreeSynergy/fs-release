#!/usr/bin/env bash
# Pattern: none (test). actions/smoke: the deb step starts the right container with the artefacts mounted read-only;
# unknown formats and missing images fail. The install itself needs apt and runs in CI (and by hand with podman, see DESIGN.md).
TS="$T/smoke"; mkdir -p "$TS/dist"
printf '#!/usr/bin/env bash\necho "$@" > "%s/call"\n' "$TS" > "$TS/fake-container"; chmod +x "$TS/fake-container"
plan='{"binaries":["a","b"]}'
run_smoke() { (cd "$TS" && env -u INSIDE PLAN="$plan" DIST="$TS/dist" CONTAINER_TOOL="$TS/fake-container" "$@" bash "$ROOT/actions/smoke/smoke.sh" >/dev/null 2>&1); }
run_smoke FORMAT=deb IMAGE=debian:13 && pass "smoke: deb starts the container" || fail "smoke: deb starts the container"
call=$(cat "$TS/call")
check_has "smoke: image and mount" "$(tr ' ' '\n' <<<"$call")" "debian:13"
check_has "smoke: artefacts read-only" "$(tr ' ' '\n' <<<"$call")" "$TS/dist:/dist:ro"
if run_smoke FORMAT=deb IMAGE=; then fail "smoke: deb without image must fail"; else pass "smoke: deb without image fails"; fi
if run_smoke FORMAT=rpm IMAGE=x; then fail "smoke: unknown format must fail"; else pass "smoke: unknown format fails"; fi
if run_smoke FORMAT=zip; then fail "smoke: zip without a zip must fail"; else pass "smoke: zip without a zip fails"; fi
