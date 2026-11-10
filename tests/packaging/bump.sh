#!/usr/bin/env bash
# Pattern: none (test). scripts/bump-components.sh: wrong tag refused; the tag lands on every fs-components* git
# dependency; --dry-run pushes nothing; a normal run pushes the branch only when the gates pass.
TB="$T/bump"; mkdir -p "$TB/base"
if "$ROOT/scripts/bump-components.sh" 1.0 >/dev/null 2>&1; then fail "bump: a tag without v must be refused"; else pass "bump: a tag without v is refused"; fi
git init -q --bare "$TB/origin.git"
git init -q -b main "$TB/base/fs-prog"
(
  cd "$TB/base/fs-prog" || exit 1
  git config user.email t@example.org; git config user.name T
  mkdir -p crates/a
  printf '[workspace]\nmembers = ["crates/*"]\n' > Cargo.toml
  printf '[package]\nname = "a"\nversion = "0.0.0"\n[dependencies]\nfs-components-app = { git = "https://github.com/FreeSynergy/fs-components", rev = "abc1234" }\nfs-components-fields = { git = "https://github.com/FreeSynergy/fs-components", tag = "v0.0.1" }\nserde = "1"\n' > crates/a/Cargo.toml
  printf '[[package]]\nname = "fs-components-app"\nversion = "0.0.1"\nsource = "git+https://github.com/FreeSynergy/fs-components?rev=abc1234#abc1234"\n' > Cargo.lock
  git add -A; git commit -qm init; git remote add origin "$TB/origin.git"; git push -q origin main
  git fetch -q origin
)
printf '#!/usr/bin/env bash\nexit 0\n' > "$TB/ok.sh"; printf '#!/usr/bin/env bash\nexit 1\n' > "$TB/bad.sh"
printf '#!/usr/bin/env bash\nif [ "$1" = cargo ]; then exit 0; fi\nexec "$@"\n' > "$TB/limited.sh"
# a stand-in `cargo` and `gates.sh` are placed by the script's own paths: gates.sh sits next to the script, so run a copy
mkdir -p "$TB/scripts"; cp "$ROOT/scripts/bump-components.sh" "$TB/scripts/"; chmod +x "$TB/ok.sh" "$TB/bad.sh" "$TB/limited.sh" "$TB/scripts/bump-components.sh"
cp "$TB/ok.sh" "$TB/scripts/gates.sh"
run() { FS_BASE="$TB/base" FS_LIMITED="$TB/limited.sh" "$TB/scripts/bump-components.sh" "$@" >&2; }
run --dry-run v0.4.0 fs-prog && pass "bump: dry run succeeds" || fail "bump: dry run must succeed"
git -C "$TB/origin.git" rev-parse -q --verify refs/heads/bump-components-v0.4.0 >/dev/null && fail "bump: dry run must not push" || pass "bump: dry run pushes nothing"
check_has "bump: rev becomes tag" "$(grep fs-components-app "$TB/base/.wt-bump-fs-prog/crates/a/Cargo.toml" | grep -o 'tag = "[^"]*"')" 'tag = "v0.4.0"'
check_has "bump: tag moves" "$(grep fs-components-fields "$TB/base/.wt-bump-fs-prog/crates/a/Cargo.toml" | grep -o 'tag = "[^"]*"')" 'tag = "v0.4.0"'
run v0.4.0 fs-prog && pass "bump: green run succeeds" || fail "bump: green run must succeed"
git -C "$TB/origin.git" rev-parse -q --verify refs/heads/bump-components-v0.4.0 >/dev/null && pass "bump: green run pushes the branch" || fail "bump: green run must push"
git -C "$TB/origin.git" tag | grep -q . && fail "bump: never a tag" || pass "bump: never a tag"
cp "$TB/bad.sh" "$TB/scripts/gates.sh"; rm -rf "$TB/base/.wt-bump-fs-prog"; git -C "$TB/base/fs-prog" worktree prune
run v0.5.0 fs-prog && fail "bump: red gates must fail" || pass "bump: red gates fail"
git -C "$TB/origin.git" rev-parse -q --verify refs/heads/bump-components-v0.5.0 >/dev/null && fail "bump: red gates must not push" || pass "bump: red gates push nothing"
