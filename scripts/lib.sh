#!/usr/bin/env bash
# Shared helpers for the fs-release packaging scripts. Source it; do not execute it.
# Expects $PLAN (the JSON produced by actions/plan) in the environment.
set -euo pipefail

warn() { printf '::warning::%s\n' "$*"; }
die() { printf '::error::%s\n' "$*"; exit 1; }
group() { printf '::group::%s\n' "$*"; }
endgroup() { printf '::endgroup::\n'; }

# plan_get KEY -> value of a top-level plan field ("" when null/absent).
plan_get() { jq -r --arg k "$1" '.[$k] // "" | if type == "array" then join(",") else tostring end' <<<"$PLAN"; }

# has_format "a,b,c" b -> success when b is in the comma separated list.
has_format() {
  local list f
  IFS=, read -ra list <<<"$1"
  for f in "${list[@]}"; do [[ "$f" == "$2" ]] && return 0; done
  return 1
}

# cargo_args -> the package / feature selection shared by build and packaging tools.
cargo_pkg_args() { local p; p=$(plan_get cargo_package); [[ -n "$p" ]] && printf -- '-p\n%s\n' "$p"; return 0; }

# --- Package planning (pattern: Value object - a "group" is one cargo package plus the binaries it provides) ---
# One group = one cargo package = one deb and one rpm. Contract:
#   package_groups        stdout: one line per cargo package that provides requested binaries,
#                         "<package>TAB<bin,bin,...>", in order of first appearance in the `binaries` plan field.
#                         A binary is owned by the plan's cargo_package when that package provides it, otherwise by the
#                         first workspace package that does. Needs $PLAN and the workspace as cwd.
#                         Fails (non-zero, ::error::) when a requested binary has no owning package.
#   pkg_select PKG        stdout: "-p" and PKG on two lines when the package must be named (cargo_package is set or the plan
#                         has more than one group); nothing for a single-package repo without cargo_package (as before).
package_groups() { die "package_groups: not implemented"; }
pkg_select() { die "pkg_select: not implemented"; }

# cargo_build TRIPLE -> release build of every requested binary.
cargo_build() {
  local triple=$1 b f
  local -a args=(build --release --locked --target "$triple")
  pkg=(); while IFS= read -r _l; do pkg+=("$_l"); done < <(cargo_pkg_args)   # bash 3.2 (macOS) has no mapfile
  args+=(${pkg[@]+"${pkg[@]}"})
  for b in $(plan_get binaries | tr ',' ' '); do args+=(--bin "$b"); done
  f=$(plan_get features)
  [[ -n "$f" ]] && args+=(--features "$f")
  cargo "${args[@]}"
}

# target_dir -> cargo's target directory (workspace aware).
target_dir() { cargo metadata --format-version 1 --no-deps | jq -r .target_directory; }

# bin_path TRIPLE BIN [EXT] -> built binary path.
bin_path() { printf '%s/%s/release/%s%s\n' "$(target_dir)" "$1" "$2" "${3:-}"; }

# copy_docs DIR -> licences and README next to the binaries.
copy_docs() {
  local f
  for f in LICENSE* COPYING* README*; do [[ -f "$f" ]] && cp "$f" "$1/"; done
  return 0
}

# in_workdir -> cd into the caller-selected working directory.
in_workdir() { cd "${GITHUB_WORKSPACE:-.}/$(plan_get workdir)"; }

dist_dir() { mkdir -p "${GITHUB_WORKSPACE:-$PWD}/dist"; printf '%s/dist\n' "${GITHUB_WORKSPACE:-$PWD}"; }
