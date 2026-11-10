#!/usr/bin/env bash
# Pattern: none (test). scripts/check_rust_rules.py (used by check.yml via actions/rust-rules): passes a clean tree, fails a rule break.
TR="$T/rules"; mkdir -p "$TR/src"
printf '//! Entry.\n//! Pattern: none\nfn main() {}\n' > "$TR/src/main.rs"
python3 -I "$ROOT/scripts/check_rust_rules.py" "$TR" >/dev/null && pass "rules: clean tree passes" || fail "rules: clean tree must pass"
echo 'fn main() {}' > "$TR/src/bad.rs"
python3 -I "$ROOT/scripts/check_rust_rules.py" "$TR" >/dev/null 2>&1 && fail "rules: missing module doc must fail" || pass "rules: missing module doc fails"
# Blinc in one place: only a repo with .fs-rules.toml blinc = "components-only" is checked (default off).
RR() { python3 -I "$ROOT/scripts/check_rust_rules.py" "$TR" >/dev/null 2>&1; }
ok()  { if RR; then pass "rules: $1"; else fail "rules: $1 (must pass)"; fi; }
bad() { if RR; then fail "rules: $1 (must fail)"; else pass "rules: $1"; fi; }
rm -f "$TR/src/bad.rs"
CG='git = "https://github.com/FreeSynergy/fs-components"'
printf '[package]\nname = "p"\n[dependencies]\nblinc_app = "0.6"\n[patch.crates-io]\nblinc_app = { git = "https://github.com/FreeSynergy/Blinc" }\n' > "$TR/Cargo.toml"
ok "an unmigrated repo (no .fs-rules.toml) keeps blinc and [patch] without error"
printf 'blinc = "off"\n' > "$TR/.fs-rules.toml"; ok "blinc = off checks nothing"
printf 'blinc = "maybe"\n' > "$TR/.fs-rules.toml"; bad "unknown switch value is an error"
printf 'blinc = \n' > "$TR/.fs-rules.toml"; bad "broken .fs-rules.toml is an error"
printf 'blinc = "components-only"\n' > "$TR/.fs-rules.toml"
printf '[package]\nname = "p"\n[dependencies]\nserde = "1"\n' > "$TR/Cargo.toml"; ok "migrated program without blinc passes"
printf '[package]\nname = "p"\n[dependencies]\nblinc_app = "0.6"\n' > "$TR/Cargo.toml"; bad "blinc dependency fails"
printf '[package]\nname = "p"\n[dependencies]\nx = { package = "blinc_layout", version = "0.6" }\n' > "$TR/Cargo.toml"; bad "renamed blinc dependency fails"
printf '[package]\nname = "p"\n[target."cfg(unix)".dev-dependencies]\nblinc_app = "0.6"\n' > "$TR/Cargo.toml"; bad "blinc in target dev-dependencies fails"
printf '[package]\nname = "p"\n[dependencies]\nx = { git = "https://github.com/FreeSynergy/Blinc" }\n' > "$TR/Cargo.toml"; bad "blinc by git url under another name fails"
printf '[package]\nname = "p"\n[patch.crates-io]\nblinc_app = { git = "https://github.com/FreeSynergy/Blinc", tag = "fs-blinc-1" }\n' > "$TR/Cargo.toml"; bad "blinc patch fails"
printf '[package]\nname = "p"\n[replace]\n"blinc_app:0.6.0" = { git = "https://github.com/FreeSynergy/Blinc" }\n' > "$TR/Cargo.toml"; bad "blinc [replace] fails"
printf '[package]\nname = "p"\n' > "$TR/Cargo.toml"
mkdir -p "$TR/.cargo"; printf '[patch.crates-io]\nblinc_app = { path = "../Blinc" }\n' > "$TR/.cargo/config.toml"; bad "blinc [patch] in .cargo/config.toml fails"
rm -rf "$TR/.cargo"
printf '[package]\nname = "p"\n[dependencies]\na = { %s, tag = "v0.4.0" }\n[dependencies.b]\n%s\ntag = "v0.4.0"\n' "$CG" "$CG" > "$TR/Cargo.toml"; ok "one fs-components tag everywhere passes"
mkdir -p "$TR/crates/x"; printf '[package]\nname = "x"\n[dependencies]\nc = { %s, tag = "v0.3.0" }\n' "$CG" > "$TR/crates/x/Cargo.toml"; bad "two fs-components tags in one program fail"
printf '[package]\nname = "x"\n[dependencies]\nc = { %s, rev = "abc" }\n' "$CG" > "$TR/crates/x/Cargo.toml"; bad "a tag and a rev of fs-components fail"
rm -rf "$TR/crates"
# false alarms: Cargo.toml in nested worktrees and target/ do not count
mkdir -p "$TR/.wt-old" "$TR/target/pkg"; printf '[package]\nname = "o"\n[dependencies]\nblinc_app = "1"\n' | tee "$TR/.wt-old/Cargo.toml" > "$TR/target/pkg/Cargo.toml"
ok "Cargo.toml in .wt-* and target/ are ignored"
rm -rf "$TR/.wt-old" "$TR/target"
# fs-components itself may carry blinc; an empty or foreign Cargo.toml at that path is not enough
printf '[package]\nname = "p"\n[dependencies]\nblinc_app = "0.6"\n' > "$TR/Cargo.toml"
mkdir -p "$TR/crates/fs-components"; : > "$TR/crates/fs-components/Cargo.toml"; bad "an empty crates/fs-components/Cargo.toml is no exemption"
printf '[package]\nname = "other"\n' > "$TR/crates/fs-components/Cargo.toml"; bad "a crate with another name at that path is no exemption"
printf '[package]\nname = "fs-components"\nrepository = "https://github.com/someone/else"\n' > "$TR/crates/fs-components/Cargo.toml"; bad "a foreign repository url is no exemption"
printf '[package]\nname = "fs-components"\nrepository = "https://github.com/FreeSynergy/fs-components"\n' > "$TR/crates/fs-components/Cargo.toml"; ok "the real fs-components crate may carry blinc"
rm -rf "$TR/crates" "$TR/Cargo.toml" "$TR/.fs-rules.toml"
