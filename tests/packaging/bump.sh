#!/usr/bin/env bash
# Pattern: none (test). scripts/bump-components.sh + set_components_tag.py: wrong/missing tag refused; the tag lands on
# EVERY dependency on the fs-components git repo (any name, package =, table form, workspace/target, deep paths);
# idempotent; nothing found = failure; a fresh worktree is required; --dry-run pushes nothing; a green run pushes
# HEAD to the branch; red gates push nothing; one run with the real cargo (`cargo update -p`) on a file:// repo.
TB="$T/bump"; mkdir -p "$TB/base" "$TB/comp"
git config --global --get user.email >/dev/null 2>&1 || export GIT_AUTHOR_NAME=T GIT_AUTHOR_EMAIL=t@example.org GIT_COMMITTER_NAME=T GIT_COMMITTER_EMAIL=t@example.org
export CARGO_HOME="$TB/cargo-home"
# the fs-components stand-in: a real git repo named fs-components with tags v0.1.0, v0.2.0 and v0.4.0
CU="file://$TB/comp/fs-components"
git init -q -b main "$TB/comp/fs-components"
(
  cd "$TB/comp/fs-components" || exit 1
  mkdir -p src; echo '' > src/lib.rs
  printf '[package]\nname = "fs-components"\nversion = "0.1.0"\nedition = "2021"\n' > Cargo.toml
  git add -A; git commit -qm one; git tag v0.1.0
  echo '// 2' >> src/lib.rs; git commit -qam two; git tag v0.2.0; git tag v0.4.0
)
export FS_COMPONENTS_URL="$CU"
BS="$ROOT/scripts/bump-components.sh"
if "$BS" 1.0 >/dev/null 2>&1; then fail "bump: a tag without v must be refused"; else pass "bump: a tag without v is refused"; fi
if "$BS" v9.9.9 >/dev/null 2>&1; then fail "bump: a tag missing in fs-components must be refused"; else pass "bump: a tag missing in fs-components is refused"; fi

# mkrepo NAME -> bare origin + clone in base; files are written by the caller before `commit_repo`
mkrepo() { git init -q --bare "$TB/origin-$1.git"; git init -q -b main "$TB/base/$1"; }
commit_repo() { (cd "$TB/base/$1" && git add -A && git commit -qm init && git remote add origin "$TB/origin-$1.git" && git push -q origin main && git fetch -q origin); }
LOCK='[[package]]\nname = "fs-components-app"\nversion = "0.0.1"\nsource = "git+%s?rev=abc1234#abc1234"\n'
mkrepo fs-prog; P="$TB/base/fs-prog"
mkdir -p "$P/crates/a" "$P/crates/deep/er/b"
printf '[workspace]\nmembers = ["crates/*"]\n[workspace.dependencies]\nws = { git = "%s", branch = "main" }\n' "$CU" > "$P/Cargo.toml"
printf '[package]\nname = "a"\nversion = "0.0.0"\n[dependencies]\nfs-components-app = { git = "%s", rev = "abc1234" }\nfs-admin-frontend = { git = "%s", rev = "abc1234" }\nfs-components-fields = { git = "%s", tag = "v0.0.1" }\nrenamed = { package = "fs-components-words", git = "%s", rev = "abc1234", features = ["x"] }\nserde = "1"\n[dependencies.tbl]\ngit = "%s"\nrev = "abc1234"\n[target."cfg(unix)".dev-dependencies.tgt]\ngit = "%s"\nbranch = "main"\n' "$CU" "$CU" "$CU" "$CU" "$CU" "$CU" > "$P/crates/a/Cargo.toml"
printf '[package]\nname = "b"\nversion = "0.0.0"\n[build-dependencies]\nbd = { git = "%s", rev = "def" }\n' "$CU" > "$P/crates/deep/er/b/Cargo.toml"
# shellcheck disable=SC2059
printf "$LOCK" "$CU" > "$P/Cargo.lock"
commit_repo fs-prog
printf '#!/usr/bin/env bash\nexit 0\n' > "$TB/ok.sh"; printf '#!/usr/bin/env bash\nexit 1\n' > "$TB/bad.sh"
printf '#!/usr/bin/env bash\nif [ "$1" = cargo ]; then exit 0; fi\nexec "$@"\n' > "$TB/limited.sh"
printf '#!/usr/bin/env bash\nif [ "$1" = cargo ]; then shift; exec "%s" "$@"; fi\nexec "$@"\n' "$REAL_CARGO" > "$TB/limited-real.sh"
# gates.sh sits next to the script, so run a copy with the stub gates beside it
mkdir -p "$TB/scripts"; cp "$ROOT"/scripts/bump-components.sh "$ROOT"/scripts/set_components_tag.py "$TB/scripts/"
chmod +x "$TB"/*.sh "$TB/scripts/bump-components.sh"
cp "$TB/ok.sh" "$TB/scripts/gates.sh"
run() { FS_BASE="$TB/base" FS_LIMITED="${LIM:-$TB/limited.sh}" "$TB/scripts/bump-components.sh" "$@" >&2; }
wtg() { grep -rh "$1" "$TB/base/.wt-bump-fs-prog" --include=Cargo.toml; }
run --dry-run v0.4.0 fs-prog && pass "bump: dry run succeeds" || fail "bump: dry run must succeed"
git -C "$TB/origin-fs-prog.git" rev-parse -q --verify refs/heads/bump-components-v0.4.0 >/dev/null && fail "bump: dry run must not push" || pass "bump: dry run pushes nothing"
W="$TB/base/.wt-bump-fs-prog"
left=$(grep -rE '(rev|branch)[[:space:]]*=' "$W" --include=Cargo.toml || true)
[ -z "$left" ] && pass "bump: no rev/branch left in any Cargo.toml" || fail "bump: rev/branch left: $left"
n=$(grep -rc 'tag = "v0.4.0"' "$W" --include=Cargo.toml | awk -F: '{s+=$2} END{print s}')
[ "$n" = 8 ] && pass "bump: all 8 dependencies (any name, package =, tables, workspace, target, build, deep) on the tag" || fail "bump: $n of 8 dependencies on the tag"
check_has "bump: other fields stay" "$(wtg renamed)" "renamed = { package = \"fs-components-words\", git = \"$CU\", tag = \"v0.4.0\", features = [\"x\"] }"
if out=$(run v0.4.0 fs-prog 2>&1); then fail "bump: an existing worktree must not be reused"; else pass "bump: an existing worktree is an error, not reused"; fi
check_in "bump: the existing-worktree message says what to do" "$out" "remove it yourself"
git -C "$TB/base/fs-prog" worktree remove --force "$W"
run v0.4.0 fs-prog && pass "bump: green run succeeds" || fail "bump: green run must succeed"
git -C "$TB/origin-fs-prog.git" rev-parse -q --verify refs/heads/bump-components-v0.4.0 >/dev/null && pass "bump: green run pushes the branch" || fail "bump: green run must push"
[ "$(git -C "$TB/origin-fs-prog.git" rev-parse bump-components-v0.4.0)" = "$(git -C "$W" rev-parse HEAD)" ] && pass "bump: the pushed branch is the worktree HEAD" || fail "bump: pushed ref differs from HEAD"
git -C "$TB/origin-fs-prog.git" tag | grep -q . && fail "bump: never a tag" || pass "bump: never a tag"
cp "$TB/bad.sh" "$TB/scripts/gates.sh"; git -C "$TB/base/fs-prog" worktree remove --force "$W"
run v0.2.0 fs-prog && fail "bump: red gates must fail" || pass "bump: red gates fail"
git -C "$TB/origin-fs-prog.git" rev-parse -q --verify refs/heads/bump-components-v0.2.0 >/dev/null && fail "bump: red gates must not push" || pass "bump: red gates push nothing"
cp "$TB/ok.sh" "$TB/scripts/gates.sh"; git -C "$TB/base/fs-prog" worktree remove --force "$W"
# second run after a green push: the branch is on origin, clear message, nothing deleted, nothing created
if out=$(run v0.4.0 fs-prog 2>&1); then fail "bump: a branch already on origin must be refused"; else pass "bump: a branch already on origin is refused"; fi
check_in "bump: the message names the remote branch" "$out" "already exists on origin"
[ ! -e "$W" ] && git -C "$TB/origin-fs-prog.git" rev-parse -q --verify refs/heads/bump-components-v0.4.0 >/dev/null && pass "bump: the refusal deletes and creates nothing" || fail "bump: the refusal must leave everything as it was"
# branch checked out in another worktree
git -C "$TB/base/fs-prog" worktree add -q "$TB/elsewhere" -b bump-components-v0.1.0 origin/main
if out=$(run v0.1.0 fs-prog 2>&1); then fail "bump: a branch checked out elsewhere must be refused"; else pass "bump: a branch checked out in another worktree is refused"; fi
check_in "bump: the message names the other worktree" "$out" "another worktree"
git -C "$TB/base/fs-prog" worktree remove --force "$TB/elsewhere"

# a repo without Cargo.lock: clear message
mkrepo fs-nolock; printf '[package]\nname = "n"\nversion = "0"\n[dependencies]\nx = { git = "%s", rev = "abc" }\n' "$CU" > "$TB/base/fs-nolock/Cargo.toml"; commit_repo fs-nolock
if out=$(run v0.4.0 fs-nolock 2>&1); then fail "bump: no Cargo.lock must fail"; else pass "bump: a repo without Cargo.lock fails"; fi
check_in "bump: the message names Cargo.lock" "$out" "no Cargo.lock"
git -C "$TB/origin-fs-nolock.git" rev-parse -q --verify refs/heads/bump-components-v0.4.0 >/dev/null && fail "bump: no Cargo.lock must push nothing" || pass "bump: no Cargo.lock pushes nothing"

# [patch]/[replace] on fs-components: error, never set silently
mkrepo fs-patch; printf '[package]\nname = "n"\nversion = "0"\n[dependencies]\nx = { git = "%s", rev = "abc" }\n[patch."%s"]\nx = { git = "%s", branch = "main" }\n' "$CU" "$CU" "$CU" > "$TB/base/fs-patch/Cargo.toml"; commit_repo fs-patch
if out=$(run v0.4.0 fs-patch 2>&1); then fail "bump: a [patch] on fs-components must fail"; else pass "bump: a [patch] on fs-components fails"; fi
check_in "bump: the message names the patch" "$out" "[patch]/[replace]"
for kind in 'patch-path:[patch."%s"]\nfs-components = { path = "../x" }' 'replace-path:[replace]\n"fs-components:0.1.0" = { path = "../x" }'; do
  nm="fs-${kind%%:*}"; mkrepo "$nm"
  # shellcheck disable=SC2059
  { printf '[package]\nname = "n"\nversion = "0"\n[dependencies]\nx = { git = "%s", rev = "abc" }\n' "$CU"; printf "${kind#*:}\n" "$CU"; } > "$TB/base/$nm/Cargo.toml"; commit_repo "$nm"
  if out=$(run v0.4.0 "$nm" 2>&1); then fail "bump: $nm must fail"; else pass "bump: $nm (path patch without own git url) fails"; fi
  check_in "bump: $nm names the override" "$out" "[patch]/[replace]"
done
for cfg in config config.toml; do
  nm="fs-cfg-$cfg"; nm="${nm//./-}"; mkrepo "$nm"; mkdir -p "$TB/base/$nm/.cargo"
  printf '[package]\nname = "n"\nversion = "0"\n[dependencies]\nx = { git = "%s", rev = "abc" }\n' "$CU" > "$TB/base/$nm/Cargo.toml"
  printf '[patch."%s"]\nfs-components = { path = "../x" }\n' "$CU" > "$TB/base/$nm/.cargo/$cfg"; commit_repo "$nm"
  if out=$(run v0.4.0 "$nm" 2>&1); then fail "bump: .cargo/$cfg patch must fail"; else pass "bump: a .cargo/$cfg patch on fs-components fails"; fi
  check_in "bump: .cargo/$cfg message" "$out" "cargo config overrides"
done
# tag before git
mkrepo fs-order; printf '[package]\nname = "n"\nversion = "0"\n[dependencies]\nx = { tag = "v0.1.0", git = "%s", features = ["a"] }\ny = { rev = "abc", git = "%s" }\n' "$CU" "$CU" > "$TB/base/fs-order/Cargo.toml"; printf "$LOCK" "$CU" > "$TB/base/fs-order/Cargo.lock"; commit_repo fs-order
run --dry-run v0.4.0 fs-order && pass "bump: { tag = .., git = .. } and { rev = .., git = .. } are handled" || fail "bump: tag before git must work"
check_has "bump: tag-first form is rewritten" "$(grep -h '^x =' "$TB/base/.wt-bump-fs-order/Cargo.toml")" "x = { git = \"$CU\", tag = \"v0.4.0\", features = [\"a\"] }"

# idempotence: a repo already on the tag is ok and pushes nothing; no fs-components dependency at all is a failure
mkrepo fs-done; mkdir -p "$TB/base/fs-done"
printf '[package]\nname = "d"\nversion = "0"\n[dependencies]\nx = { git = "%s", tag = "v0.4.0" }\n[dependencies.y]\ngit = "%s"\ntag = "v0.4.0"\n' "$CU" "$CU" > "$TB/base/fs-done/Cargo.toml"; commit_repo fs-done
run v0.4.0 fs-done && pass "bump: a repo already on the tag is ok" || fail "bump: already on the tag must be ok"
git -C "$TB/origin-fs-done.git" rev-parse -q --verify refs/heads/bump-components-v0.4.0 >/dev/null && fail "bump: already on the tag pushes nothing" || pass "bump: already on the tag pushes nothing"
mkrepo fs-none; printf '[package]\nname = "n"\nversion = "0"\n[dependencies]\nserde = "1"\n# fs-components is only a word here\n' > "$TB/base/fs-none/Cargo.toml"; commit_repo fs-none
run v0.4.0 fs-none && fail "bump: nothing found must be a failure" || pass "bump: no fs-components dependency is a failure, not 'already on the tag'"
mkrepo fs-odd; printf '[package]\nname = "o"\nversion = "0"\n[dependencies]\nx.git = "%s"\nx.rev = "abc"\n' "$CU" > "$TB/base/fs-odd/Cargo.toml"; commit_repo fs-odd
run v0.4.0 fs-odd && fail "bump: an unhandled form must be a failure" || pass "bump: an unhandled form (dotted keys) fails instead of passing silently"

# the real cargo: `cargo update -p url#name@version` on a file:// fs-components repo, v0.1.0 -> v0.2.0
mkrepo fs-real; mkdir -p "$TB/base/fs-real/src"; echo 'fn main() {}' > "$TB/base/fs-real/src/main.rs"
printf '[package]\nname = "real"\nversion = "0.0.0"\nedition = "2021"\n[dependencies]\nfs-components = { git = "%s", tag = "v0.1.0" }\n' "$CU" > "$TB/base/fs-real/Cargo.toml"
(cd "$TB/base/fs-real" && "$REAL_CARGO" generate-lockfile -q 2>/dev/null); commit_repo fs-real
LIM="$TB/limited-real.sh" run v0.2.0 fs-real && pass "bump: real cargo update run succeeds" || fail "bump: real cargo update run must succeed"
RW="$TB/base/.wt-bump-fs-real"
sha2=$(git -C "$TB/comp/fs-components" rev-parse v0.2.0)
check_has "bump: Cargo.lock moved to the new tag and commit" "$(grep -F 'source =' "$RW/Cargo.lock")" "source = \"git+$CU?tag=v0.2.0#$sha2\""
