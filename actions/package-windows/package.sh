#!/usr/bin/env bash
# Pattern: Template method - build once, one function per format (git-bash on the runner).
# msi and exe are only built for x86_64: WiX 3 and NSIS have no usable arm64 story yet.
set -euo pipefail
# shellcheck source=../../scripts/lib.sh
source "$(dirname "${BASH_SOURCE[0]}")/../../scripts/lib.sh"

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
name=$(plan_get name) version=$(plan_get version) core=$(plan_get version_core)
formats=$(plan_get windows_formats)
dist=$(dist_dir)
in_workdir
base="$name-$version-windows-$ARCH"
read -ra bins <<<"$(plan_get binaries | tr ',' ' ')"
groups=(); group_list   # one entry per cargo package: "<package>TAB<bin,bin>"
sel=()

group "build $TRIPLE"
cargo_build "$TRIPLE"
endgroup

stage="$RUNNER_TEMP/stage/$base"
rm -rf "$stage"; mkdir -p "$stage"
for b in "${bins[@]}"; do cp "$(bin_path "$TRIPLE" "$b" .exe)" "$stage/"; done
copy_docs "$stage"

make_zip() { (cd "$RUNNER_TEMP/stage" && 7z a -tzip -mx=9 "$dist/$base.zip" "$base" >/dev/null); }

# One msi per cargo package group (cargo wix builds one package at a time). A single group keeps the plain name
# "$base.msi"; several groups get "<package>-<version>-windows-<arch>.msi" so the files cannot clash.
make_msi() {
  local g p out dir sel
  command -v candle.exe >/dev/null || [[ -n "${WIX:-}" ]] || choco install wixtoolset -y --no-progress
  for g in "${groups[@]}"; do
    p=${g%%$'\t'*}
    if [[ ${#groups[@]} -eq 1 ]]; then out="$dist/$base.msi"; else out="$dist/$p-$version-windows-$ARCH.msi"; fi
    sel=(); while IFS= read -r _l; do sel+=("$_l"); done < <(pkg_select "$p")
    dir=$(pkg_dir "$p")
    # wix/main.wxs lives in the package directory; an existing one (the app repo's own) is never overwritten.
    [[ -f "$dir/wix/main.wxs" ]] || cargo wix init ${sel[@]+"${sel[@]}"}
    cargo wix --no-build --nocapture --target "$TRIPLE" ${sel[@]+"${sel[@]}"} --install-version "$core" --output "$out"
  done
}

make_exe() {
  export MSYS2_ARG_CONV_EXCL="*"  # keep /DNAME=... literal in git-bash
  local nsis="/c/Program Files (x86)/NSIS/makensis.exe"
  [[ -x "$nsis" ]] || { choco install nsis -y --no-progress; }
  "$nsis" /V2 "/DNAME=$(plan_get app_name)" "/DVERSION=$core" "/DSTAGE=$(cygpath -w "$stage")" "/DOUT=$(cygpath -w "$dist/$base-setup.exe")" \
    "$(cygpath -w "$here/installer.nsi")"
}

for f in zip msi exe; do
  has_format "$formats" "$f" || continue
  if [[ "$f" != zip && "$ARCH" != x86_64 ]]; then warn "$f is only built for x86_64 - skipped on $ARCH"; continue; fi
  group "package $f"
  "make_$f"
  endgroup
done
ls -l "$dist"
