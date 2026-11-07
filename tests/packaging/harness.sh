#!/usr/bin/env bash
# Test helpers for the packaging tests (source it). Pattern: none (test support only).
# Runs the real plan.py and package-linux/package.sh against throw-away Cargo workspaces. cargo is a recording stub
# for build/deb/generate-rpm and the real cargo for `metadata`, so no compiler is needed.
set -euo pipefail
unset -f grep 2>/dev/null || true   # interactive shells may export a wrapper function

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
T=${TEST_TMP:-${RUNNER_TEMP:-/srv/FreeSynergy/.scratch/t4-relrpm}}/run.$$
REAL_CARGO=$(command -v cargo)
FAILS=0
mkdir -p "$T/bin"
trap 'rm -rf "$T"' EXIT

cat > "$T/bin/cargo" <<STUB
#!/usr/bin/env bash
if [[ "\$1" == metadata ]]; then exec "$REAL_CARGO" "\$@"; fi
echo "cargo \$*" >> "\$STUB_LOG"
if [[ "\$1" == build ]]; then
  t=""; bins=()
  while [[ \$# -gt 0 ]]; do
    case "\$1" in --target) t=\$2; shift ;; --bin) bins+=("\$2"); shift ;; esac
    shift
  done
  for b in "\${bins[@]}"; do mkdir -p "target/\$t/release"; : > "target/\$t/release/\$b"; done
fi
STUB
printf '#!/usr/bin/env bash\nexit 0\n' > "$T/bin/strip"
chmod +x "$T/bin/cargo" "$T/bin/strip"

pass() { printf 'ok   %s\n' "$1"; }
fail() { printf 'FAIL %s\n' "$1"; FAILS=$((FAILS + 1)); }

# check NAME ACTUAL EXPECTED_FILE -> compare text with a golden file (UPDATE_GOLDEN=1 rewrites it).
check_golden() {
  if [[ -n "${UPDATE_GOLDEN:-}" ]]; then printf '%s\n' "$2" > "$3"; fi
  if diff <(printf '%s\n' "$2") "$3" >&2; then pass "$1"; else fail "$1"; fi
}
# check_has NAME TEXT NEEDLE -> TEXT contains the exact line NEEDLE.
check_has() { if grep -qxF -- "$3" <<<"$2"; then pass "$1"; else fail "$1 (missing line: $3)"; fi; }
check_lacks() { if grep -qF -- "$3" <<<"$2"; then fail "$1 (unexpected: $3)"; else pass "$1"; fi; }

# make_pkg DIR NAME VERSION BIN... -> a cargo package with the given binaries.
make_pkg() {
  local d=$1 n=$2 v=$3; shift 3
  mkdir -p "$d/src/bin"
  { printf '[package]\nname = "%s"\nversion = "%s"\nedition = "2021"\nauthors = ["T <t@example.org>"]\n' "$n" "$v"
    printf 'description = "d"\nlicense = "MIT"\nrepository = "https://example.org/r"\n'
  } > "$d/Cargo.toml"
  echo 'fn main() {}' > "$d/src/main.rs"
  for b in "$@"; do printf '\n[[bin]]\nname = "%s"\npath = "src/bin/%s.rs"\n' "$b" "$b" >> "$d/Cargo.toml"; echo 'fn main() {}' > "$d/src/bin/$b.rs"; done
}

# make_plan DIR ENV... -> run plan.py in DIR with NAME=value pairs; prints the plan JSON (sorted keys).
make_plan() {
  local d=$1; shift
  : > "$T/out"
  local rc=0
  (cd "$d" && env TAG=v0.1.1 LINUX_TARGETS=x86_64,aarch64 WINDOWS_TARGETS=x86_64 MACOS=true "$@" \
    GITHUB_OUTPUT="$T/out" python3 -I "$ROOT/actions/plan/plan.py" >/dev/null) || rc=$?
  [[ $rc -eq 0 ]] || return "$rc"
  sed -n 's/^plan=//p' "$T/out" | jq -S .
}

# run_linux DIR PLAN FORMATS -> run package-linux/package.sh with stubs; prints normalised log, dist listing, rpm tail.
run_linux() { _run_linux "$@" | sed "s#$1#@R@#g"; }
_run_linux() {
  local d=$1 plan=$2 formats=$3
  plan=$(jq -c --arg f "$formats" '.linux_formats = $f' <<<"$plan")
  rm -rf "$d/dist" "$d/target" "$T/stage"; mkdir -p "$T/stage"; : > "$T/log"
  (cd "$d" && PATH="$T/bin:$PATH" PLAN="$plan" ARCH=x86_64 TRIPLE=x86_64-unknown-linux-gnu RUNNER_TEMP="$T" \
    GITHUB_WORKSPACE="$d" STUB_LOG="$T/log" bash "$ROOT/actions/package-linux/package.sh" >/dev/null)
  while IFS= read -r l; do echo "$l"; done < "$T/log"
  echo "--- dist"; (cd "$d/dist" && ls)
  local m
  while IFS= read -r m; do
    if [[ -n $(awk '/package.metadata.generate-rpm/{print 1}' "$m") ]]; then
      echo "--- rpm metadata in ${m#"$d"/}"; awk '/package.metadata.generate-rpm/{f=1} f{print}' "$m"
    fi
  done < <(cd "$d" && find . -name Cargo.toml | sort | sed "s#^\./#$d/#")
}
