#!/usr/bin/env python3
"""Check ADR 0043 coding rules in one or more Rust repos.

Checks (exit 1 on any violation):
  * file length: > 300 lines = error, > 200 = warning (generated files `*_capnp.rs`, `generated/` exempt)
  * every `src/**/*.rs` starts with a `//!` module doc
  * every module doc names its pattern (`Pattern:` line) — except mod.rs/lib.rs/main.rs which may only re-export
  * `lib.rs`/`mod.rs` contain no `fn` bodies (only declarations/re-exports)

    python3 scripts/check_rust_rules.py /srv/FreeSynergy/fs-store [/srv/FreeSynergy/fs-hub …]
"""
import re, sys
from pathlib import Path

HARD, SOFT = 300, 200

def check(repo: Path):
    errors, warns = [], []
    for f in sorted(repo.rglob("*.rs")):
        p = f.as_posix()
        if "/target/" in p or "/generated/" in p or f.name.endswith("_capnp.rs"):
            continue
        lines = f.read_text(encoding="utf-8").splitlines()
        rel = f.relative_to(repo).as_posix()
        if len(lines) > HARD: errors.append(f"{rel}: {len(lines)} lines (> {HARD})")
        elif len(lines) > SOFT: warns.append(f"{rel}: {len(lines)} lines (> {SOFT})")
        if "/src/" in "/" + rel:
            if not lines or not lines[0].startswith("//!"):
                errors.append(f"{rel}: missing //! module doc on line 1")
            elif f.name not in ("lib.rs", "mod.rs", "main.rs"):
                head = "\n".join(l for l in lines[:30] if l.startswith("//!"))
                if "Pattern:" not in head:
                    errors.append(f"{rel}: module doc has no 'Pattern:' line")
            if f.name in ("lib.rs", "mod.rs") and re.search(r"^\s*(pub\s+)?(async\s+)?fn\s+\w+.*\{", "\n".join(lines), re.M):
                errors.append(f"{rel}: logic in {f.name} (only modules/re-exports allowed)")
    return errors, warns

def main():
    repos = [Path(a) for a in sys.argv[1:]] or [Path.cwd()]
    total = 0
    for r in repos:
        e, w = check(r)
        print(f"== {r}: {len(e)} errors, {len(w)} warnings")
        for x in e: print("  ERROR", x)
        for x in w: print("  warn ", x)
        total += len(e)
    return 1 if total else 0

if __name__ == "__main__":
    sys.exit(main())
