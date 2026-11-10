#!/usr/bin/env python3
"""Check ADR 0043 coding rules in one or more Rust repos.

Checks (exit 1 on any violation):
  * file length: > 300 lines = error, > 200 = warning (generated files `*_capnp.rs`, `generated/` exempt)
  * every `src/**/*.rs` starts with a `//!` module doc
  * every module doc names its pattern (`Pattern:` line) — except mod.rs/lib.rs/main.rs which may only re-export
  * `lib.rs`/`mod.rs` contain no `fn` bodies (only declarations/re-exports)
  * Blinc lives in one place, ONLY in a repo whose root has `.fs-rules.toml` with `blinc = "components-only"` (default
    off until the repo is migrated by hand): no `blinc*` dependency, `[patch]`/`[replace]` (also in
    `.cargo/config.toml`) naming blinc, and one single fs-components tag/rev. fs-components itself (a real
    `crates/fs-components` package of that name) is exempt. Programs take Blinc through `fs_components::blinc`.

    python3 scripts/check_rust_rules.py /srv/FreeSynergy/fs-store [/srv/FreeSynergy/fs-hub …]
"""
import re, sys, tomllib
from pathlib import Path

HARD, SOFT = 300, 200

DEP_TABLES = ("dependencies", "dev-dependencies", "build-dependencies")
SWITCH_FILE = ".fs-rules.toml"
SKIP_DIRS = {"target", ".git", "node_modules"}
COMPONENTS_GIT = re.compile(r"/fs-components(\.git)?/?$", re.I)

def _walk(repo: Path, pattern: str):
    """rglob without target/, .git/, node_modules/ and nested worktrees (.wt-*)."""
    for f in sorted(repo.rglob(pattern)):
        parts = f.relative_to(repo).parts[:-1]
        if any(p in SKIP_DIRS or p.startswith(".wt-") for p in parts): continue
        yield f

def _dep_tables(doc):
    """Yield every dependency table of a parsed Cargo.toml (also `target.*` and workspace ones)."""
    for k, v in doc.items():
        if k in DEP_TABLES and isinstance(v, dict): yield v
    for t in doc.get("target", {}).values(): yield from _dep_tables(t)
    if isinstance(doc.get("workspace"), dict): yield from _dep_tables(doc["workspace"])

def blinc_mode(repo: Path):
    """(mode, errors) from `.fs-rules.toml`: blinc = "off" (default, repo not migrated) | "components-only"."""
    f = repo / SWITCH_FILE
    if not f.is_file(): return "off", []
    try: mode = tomllib.loads(f.read_text(encoding="utf-8")).get("blinc", "off")
    except tomllib.TOMLDecodeError as e: return "off", [f"{SWITCH_FILE}: not valid TOML ({e})"]
    if mode not in ("off", "components-only"):
        return "off", [f'{SWITCH_FILE}: blinc = {mode!r}: use "off" or "components-only"']
    return mode, []

def is_components_repo(repo: Path) -> bool:
    """The one repo that may carry Blinc: it has crates/fs-components whose package is named fs-components
    (and, when it names a repository, that is fs-components). An empty Cargo.toml at that path is not enough."""
    f = repo / "crates" / "fs-components" / "Cargo.toml"
    if not f.is_file(): return False
    try: pkg = tomllib.loads(f.read_text(encoding="utf-8")).get("package", {})
    except tomllib.TOMLDecodeError: return False
    if pkg.get("name") != "fs-components": return False
    return "repository" not in pkg or bool(COMPONENTS_GIT.search(str(pkg["repository"])))

def _blinc_text(*parts) -> bool:
    return "blinc" in " ".join(str(p) for p in parts).lower()

def _patch_errors(rel, doc):
    for source, entries in ((doc.get("patch") or {}).items()):
        for name, spec in (entries or {}).items():
            if _blinc_text(source, name, spec):
                yield f"{rel}: [patch.{source}] `{name}`: Blinc is pinned only in fs-components, a patch never reaches dependants"
    for name, spec in (doc.get("replace") or {}).items():
        if _blinc_text(name, spec):
            yield f"{rel}: [replace] `{name}`: Blinc is pinned only in fs-components"

def check_blinc(repo: Path):
    """Only when `.fs-rules.toml` says blinc = "components-only" (the repo is migrated); fs-components is exempt.
    A program then has no blinc dependency, no blinc [patch]/[replace] (Cargo.toml or .cargo/config.toml) and
    one single fs-components tag/rev in all its Cargo.toml."""
    mode, errors = blinc_mode(repo)
    if mode != "components-only" or is_components_repo(repo): return errors
    pins = {}
    for f in _walk(repo, "Cargo.toml"):
        rel = f.relative_to(repo).as_posix()
        try: doc = tomllib.loads(f.read_text(encoding="utf-8"))
        except tomllib.TOMLDecodeError as e:
            errors.append(f"{rel}: not valid TOML ({e})"); continue
        for table in _dep_tables(doc):
            for name, spec in table.items():
                real = spec.get("package", name) if isinstance(spec, dict) else name
                git = spec.get("git", "") if isinstance(spec, dict) else ""
                if _blinc_text(name, real, git):
                    errors.append(f"{rel}: dependency `{name}`: programs take Blinc through fs_components::blinc")
                if COMPONENTS_GIT.search(str(git)):
                    pins.setdefault(next((f"{k} {spec[k]}" for k in ("tag", "rev", "branch") if k in spec), "unpinned"), []).append(rel)
        errors.extend(_patch_errors(rel, doc))
    for name in ("config.toml", "config"):
        for f in _walk(repo, f".cargo/{name}"):
            rel = f.relative_to(repo).as_posix()
            try: errors.extend(_patch_errors(rel, tomllib.loads(f.read_text(encoding="utf-8"))))
            except tomllib.TOMLDecodeError as e: errors.append(f"{rel}: not valid TOML ({e})")
    if len(pins) > 1:
        errors.append("two fs-components versions in one program (two Blinc): " +
                      "; ".join(f"{k} in {', '.join(sorted(set(v)))}" for k, v in sorted(pins.items())))
    return errors

def check(repo: Path):
    errors, warns = check_blinc(repo), []
    for f in _walk(repo, "*.rs"):
        p = f.as_posix()
        if "/generated/" in p or f.name.endswith("_capnp.rs"):
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
