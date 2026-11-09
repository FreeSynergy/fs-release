#!/usr/bin/env bash
# Pattern: Strategy - one function per package format; each installs the package the way a person would, starts every
# binary once ($SMOKE_ARGS, default --version, must exit 0 within 30 s) and removes it again (the removal is checked too).
# Contract: env PLAN (plan JSON), FORMAT (deb|zip|msi|exe), DIST (artefact directory of one platform + arch),
# IMAGE (deb only: container image), SMOKE_ARGS. Fails with ::error:: on the first problem.
# deb runs `docker run IMAGE` with this script inside (INSIDE=1): Debian 13 and Ubuntu 24.04 are real images, no emulation of apt.
# CONTAINER_TOOL (default docker) and CONTAINER_ARGS let the same step run locally with podman.
set -euo pipefail
# shellcheck source=../../scripts/lib.sh
source "$(dirname "${BASH_SOURCE[0]}")/../../scripts/lib.sh"

smoke_args=${SMOKE_ARGS---version}
read -ra bins <<<"$(plan_get binaries | tr ',' ' ')"

# start_one PATH -> the binary starts and exits 0 (stdin closed, so a program waiting for input ends instead of hanging).
start_one() {
  local out rc=0
  # shellcheck disable=SC2086  # the arguments are a word list on purpose
  out=$(timeout 30 "$1" $smoke_args </dev/null 2>&1) || rc=$?
  [[ $rc -eq 0 ]] || die "$1 $smoke_args exited with $rc: $out"
  printf 'started %s -> %s\n' "$1" "$(head -n1 <<<"$out")"
}

# --- deb: inside the container -------------------------------------------------------------------------------------
deb_inside() {
  local debs=("$DIST"/*.deb) pkgs=() d b
  [[ -e "${debs[0]}" ]] || die "no .deb in $DIST"
  export DEBIAN_FRONTEND=noninteractive
  apt-get update -qq >/dev/null
  apt-get install -y --no-install-recommends "${debs[@]}" >/dev/null   # apt resolves the dependencies, like a person would
  for d in "${debs[@]}"; do pkgs+=("$(dpkg-deb -f "$d" Package)"); done
  for b in "${bins[@]}"; do
    command -v "$b" >/dev/null || die "$b is not on PATH after installing ${pkgs[*]}"
    start_one "$(command -v "$b")"
  done
  apt-get remove -y "${pkgs[@]}" >/dev/null
  for b in "${bins[@]}"; do ! command -v "$b" >/dev/null || die "$b is still there after removing ${pkgs[*]}"; done
  echo "deb ok on $(. /etc/os-release && echo "$PRETTY_NAME"): ${pkgs[*]}"
}

# deb: on the runner -> start the container with the artefacts mounted read-only.
deb_package() {
  [[ -n "$IMAGE" ]] || die "format deb needs an image"
  local here; here=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
  # shellcheck disable=SC2086  # CONTAINER_ARGS is a word list on purpose
  ${CONTAINER_TOOL:-docker} run --rm ${CONTAINER_ARGS:-} -e INSIDE=1 -e FORMAT=deb -e PLAN="$PLAN" -e SMOKE_ARGS="$smoke_args" -e DIST=/dist \
    -v "$DIST:/dist:ro" -v "$here:/fs-release:ro" "$IMAGE" bash -c 'apt-get update -qq >/dev/null && apt-get install -y -qq --no-install-recommends jq >/dev/null && exec bash /fs-release/actions/smoke/smoke.sh'
}

# --- Windows (git-bash on the runner) ------------------------------------------------------------------------------
# find_installed BIN -> path of BIN.exe below Program Files (msi and NSIS install there), empty when absent.
find_installed() { find "/c/Program Files" "/c/Program Files (x86)" -maxdepth 5 -iname "$1.exe" 2>/dev/null | head -n1; }

# ps_run EXE ARG... -> run a program via PowerShell and wait; returns its exit code (no path mangling of /qn style flags).
ps_run() {
  local exe=$1; shift
  local list; list=$(printf "'%s'," "$@"); list=${list%,}
  powershell -NoProfile -Command "\$p = Start-Process -FilePath '$exe' -ArgumentList $list -Wait -PassThru; exit \$p.ExitCode" || { local rc=$?; echo "$exe exited with $rc"; return "$rc"; }
}

zip_package() {
  local z=("$DIST"/*.zip) tmp b exe
  [[ -e "${z[0]}" ]] || die "no .zip in $DIST"
  tmp=$(mktemp -d); 7z x -y "-o$tmp" "${z[0]}" >/dev/null
  for b in "${bins[@]}"; do
    exe=$(find "$tmp" -iname "$b.exe" | head -n1); [[ -n "$exe" ]] || die "$b.exe is not in ${z[0]}"
    start_one "$exe"
  done
  rm -rf "$tmp"; echo "zip ok"
}

msi_package() {
  local m=("$DIST"/*.msi) f b exe
  [[ -e "${m[0]}" ]] || die "no .msi in $DIST"
  for f in "${m[@]}"; do
    ps_run msiexec /i "$(cygpath -w "$f")" /qn /norestart "/l*v" "$(cygpath -w "$RUNNER_TEMP")\\msi-install.log" || die "msiexec /i $f failed (see msi-install.log)"
  done
  for b in "${bins[@]}"; do
    exe=$(find_installed "$b"); [[ -n "$exe" ]] || die "$b.exe not found below Program Files after installing the msi"
    start_one "$exe"
  done
  for f in "${m[@]}"; do ps_run msiexec /x "$(cygpath -w "$f")" /qn /norestart || die "msiexec /x $f failed"; done
  for b in "${bins[@]}"; do [[ -z "$(find_installed "$b")" ]] || die "$b.exe is still there after uninstalling the msi"; done
  echo "msi ok"
}

exe_package() {
  local s=("$DIST"/*-setup.exe) b exe un
  [[ -e "${s[0]}" ]] || die "no -setup.exe in $DIST"
  ps_run "$(cygpath -w "${s[0]}")" /S || die "setup.exe /S failed"
  for b in "${bins[@]}"; do
    exe=$(find_installed "$b"); [[ -n "$exe" ]] || die "$b.exe not found below Program Files after the setup.exe"
    start_one "$exe"
  done
  # NSIS copies the uninstaller to a temp file and returns at once, so wait (up to 60 s) until the files are gone.
  un=$(find_installed uninstall); [[ -n "$un" ]] || die "uninstall.exe not found"
  ps_run "$(cygpath -w "$un")" /S || die "uninstall.exe /S failed"
  for b in "${bins[@]}"; do
    for _ in $(seq 60); do [[ -z "$(find_installed "$b")" ]] && break; sleep 1; done
    [[ -z "$(find_installed "$b")" ]] || die "$b.exe is still there 60 s after uninstalling"
  done
  echo "exe ok"
}

if [[ "${INSIDE:-}" == 1 ]]; then deb_inside; exit 0; fi
case "$FORMAT" in
  deb|zip|msi|exe) "${FORMAT}_package" ;;
  *) die "unknown format '$FORMAT' (deb, zip, msi, exe)" ;;
esac
