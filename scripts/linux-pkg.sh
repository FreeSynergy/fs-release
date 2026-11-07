#!/usr/bin/env bash
# Pattern: Strategy - one function per native Linux package format, each building ONE cargo package.
# Source it after scripts/lib.sh; package.sh (Template method) calls each strategy once per group from package_groups.
# Contract (all need $PLAN, the workspace as cwd; DIST is an existing output directory):
#   deb_package PKG TRIPLE DIST        cargo deb for PKG (binaries must already be built for TRIPLE); writes into DIST.
#   rpm_package PKG BINS TRIPLE DIST   cargo generate-rpm for PKG (-p <package dir>, see below); BINS is the comma list of PKG's binaries, used for the
#                                      default assets when the package has no [package.metadata.generate-rpm]; writes into DIST.
set -euo pipefail

deb_package() {
  local sel=() f; while IFS= read -r _l; do sel+=("$_l"); done < <(pkg_select "$1")
  f=$(pkg_features "$1"); [[ -n "$f" ]] && sel+=(--features "$f")
  cargo deb --no-build --target "$2" ${sel[@]+"${sel[@]}"} --output "$3/"
}

rpm_package() {
  local pkg=$1 bins=$2 triple=$3 dist=$4 manifest b assets="" sel=() v
  # cargo-generate-rpm can only overwrite an existing [package.metadata.generate-rpm] table, not create one.
  # If the package has none, append it to the manifest of this CI checkout only (never committed).
  # A package that declares its own table (assets, requires, ...) is left alone.
  # cargo-generate-rpm -p takes the package DIRECTORY relative to the workspace root ("." for the root package), not its name.
  local meta dir
  meta=$(cargo metadata --format-version 1 --no-deps \
    | jq -r --arg p "$pkg" '.workspace_root as $w | .packages[] | select(.name == $p) | .manifest_path + "\t" + $w' | head -n1)
  [[ -n "$meta" ]] || die "no cargo package named $pkg"
  manifest=${meta%%$'\t'*}
  dir=$(dirname "${manifest#"${meta#*$'\t'}"/}"); [[ "$manifest" == "${meta#*$'\t'}/Cargo.toml" ]] && dir=.
  if ! grep -q 'package.metadata.generate-rpm' "$manifest"; then
    for b in ${bins//,/ }; do
      assets+="{ source = \"$(bin_path "$triple" "$b")\", dest = \"/usr/bin/$b\", mode = \"755\" }, "
    done
    printf '\n[package.metadata.generate-rpm]\nassets = [%s]\n' "${assets%, }" >> "$manifest"
  fi
  # RPM forbids '-' in versions; a SemVer pre-release becomes '~' (sorts before the release, e.g. 1.0~rc.1 < 1.0).
  v=$(plan_get version); v=${v/-/\~}
  [[ -n "$(pkg_select "$pkg")" ]] && sel=(-p "$dir")
  cargo generate-rpm --target "$triple" ${sel[@]+"${sel[@]}"} -s "version = \"$v\"" --output "$dist/"
}
