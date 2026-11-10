#!/usr/bin/env python3
"""Check ADR 0043 coding rules in one or more Rust repos.

Checks (exit 1 on any violation):
  * file length: > 300 lines = error, > 200 = warning (generated files `*_capnp.rs`, `generated/` exempt)
  * every `src/**/*.rs` starts with a `//!` module doc
  * every module doc names its pattern (`Pattern:` line) — except mod.rs/lib.rs/main.rs which may only re-export
  * `lib.rs`/`mod.rs` contain no `fn` bodies (only declarations/re-exports)
  * Blinc lives in one place: no repo except fs-components (the one with a crate `crates/fs-components`) has a
    `blinc*` dependency or a `[patch]` entry for blinc in any Cargo.toml; programs take Blinc through
    `fs_components::blinc` (Cargo applies `[patch]` only at the top, so a patch never reaches dependants).

    python3 scripts/check_rust_rules.py /srv/FreeSynergy/fs-store [/srv/FreeSynergy/fs-hub …]
"""
import re, sys, tomllib
from pathlib import Path

HARD, SOFT = 300, 200

DEP_TABLES = ("dependencies", "dev-dependencies", "build-dependencies")

def _dep_tables(doc):
    """Yield every dependency table of a parsed Cargo.toml (also `target.*` and workspace ones)."""
    for k, v in doc.items():
        if k in DEP_TABLES and isinstance(v, dict): yield v
    for t in doc.get("target", {}).values(): yield from _dep_tables(t)
    if isinstance(doc.get("workspace"), dict): yield from _dep_tables(doc["workspace"])

def check_blinc(repo: Path):
    """A program (every repo but fs-components) has no blinc dependency and no blinc [patch]."""
    if (repo / "crates" / "fs-components" / "Cargo.toml").is_file():
        return []
    errors = []
    for f in sorted(repo.rglob("Cargo.toml")):
        p = f.as_posix()
        if "/target/" in p or "/.git/" in p: continue
        rel = f.relative_to(repo).as_posix()
        try: doc = tomllib.loads(f.read_text(encoding="utf-8"))
        except tomllib.TOMLDecodeError as e:
            errors.append(f"{rel}: not valid TOML ({e})"); continue
        for table in _dep_tables(doc):
            for name, spec in table.items():
                real = spec.get("package", name) if isinstance(spec, dict) else name
                if name.lower().startswith("blinc") or str(real).lower().startswith("blinc"):
                    errors.append(f"{rel}: dependency `{name}`: programs take Blinc through fs_components::blinc")
        for source, entries in (doc.get("patch") or {}).items():
            for name, spec in entries.items():
                text = f"{source} {name} {spec}".lower()
                if "blinc" in text:
                    errors.append(f"{rel}: [patch.{source}] `{name}`: Blinc is pinned only in fs-components, a patch never reaches dependants")
    return errors

def check(repo: Path):
    errors, warns = check_blinc(repo), []
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
