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
package_groups() {
  local out
  out=$(cargo metadata --format-version 1 --no-deps | jq -r --arg bins "$(plan_get binaries)" --arg want "$(plan_get cargo_package)" '
    .packages as $p
    | reduce ($bins | split(",") | map(select(length > 0)))[] as $b ([];
        ([$p[] | select(any(.targets[]; .name == $b and (.kind | index("bin"))))] | map(.name)) as $o
        | (if ($o | index($want)) != null then $want else $o[0] end) as $owner
        | if $owner == null then error("no cargo package provides binary " + $b)
          elif any(.[]; .pkg == $owner) then map(if .pkg == $owner then .bins += [$b] else . end)
          else . + [{pkg: $owner, bins: [$b]}] end)
    | .[] | "\(.pkg)\t\(.bins | join(","))"') || die "cannot plan packages for binaries: $(plan_get binaries)"
  printf '%s\n' "$out"
}

pkg_select() {
  if [[ -n "$(plan_get cargo_package)" || $(package_groups | wc -l) -gt 1 ]]; then printf -- '-p\n%s\n' "$1"; fi
  return 0
}

# group_list -> fills the global array `groups` with the lines of package_groups (bash 3.2 safe, no mapfile).
group_list() { groups=(); while IFS= read -r _l; do groups+=("$_l"); done < <(package_groups); }

# cargo_build TRIPLE -> release build, one cargo invocation per package group (a workspace may keep binaries in several packages).
cargo_build() {
  local triple=$1 g p bl b f sel
  local -a args groups
  group_list
  f=$(plan_get features)
  for g in "${groups[@]}"; do
    p=${g%%$'\t'*} bl=${g#*$'\t'}
    args=(build --release --locked --target "$triple")
    sel=(); while IFS= read -r _l; do sel+=("$_l"); done < <(pkg_select "$p")
    args+=(${sel[@]+"${sel[@]}"})
    for b in ${bl//,/ }; do args+=(--bin "$b"); done
    [[ -n "$f" ]] && args+=(--features "$f")
    cargo "${args[@]}"
  done
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

# --- Artefact file names (rule: see DESIGN.md "File names") ---
# Every released file name uses only [A-Za-z0-9._-]. GitHub rewrites other characters on upload (rpm '~' becomes '.'), so
# SHA256SUMS.txt would name files that do not exist under that name. We rename first, then hash.
# dist_normalise DIR  renames every file in DIR whose name has a character outside the set, each such character becomes '-'
#                     (rpm "1.0~rc.1" -> "1.0-rc.1"). Fails when two files would collide. Package contents are not touched.
# dist_check DIR      fails (::error::) when a file in DIR still has such a name.
dist_normalise() {
  local f n new
  for f in "$1"/*; do
    [[ -f "$f" ]] || continue
    n=${f##*/}; new=$(printf '%s' "$n" | LC_ALL=C sed 's/[^A-Za-z0-9._-]/-/g')
    [[ "$n" == "$new" ]] && continue
    [[ ! -e "$1/$new" ]] || die "cannot rename $n to $new: target exists"
    mv "$f" "$1/$new"
  done
}
dist_check() {
  local f bad=0
  for f in "$1"/*; do
    [[ -f "$f" ]] || continue
    if printf '%s' "${f##*/}" | LC_ALL=C grep -q '[^A-Za-z0-9._-]'; then printf '::error::unsafe file name: %s\n' "${f##*/}"; bad=1; fi
  done
  return "$bad"
}
