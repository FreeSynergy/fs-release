#!/usr/bin/env python3
"""Check ADR 0043 coding rules in one or more Rust repos.

Checks (exit 1 on any violation):
  * file length: > 300 lines = error, > 200 = warning (generated files `*_capnp.rs`, `generated/` exempt)
  * every `src/**/*.rs` starts with a `//!` module doc
  * every module doc names its pattern (`Pattern:` line) — except mod.rs/lib.rs/main.rs which may only re-export
  * `lib.rs`/`mod.rs` contain no `fn` bodies (only declarations/re-exports)
  * Blinc lives in one place, ONLY in a repo whose root has `.fs-rules.toml` with `blinc = "components-only"` (default
    off until the repo is migrated by hand): no `blinc*` dependency, NO `[patch]`/`[replace]` on blinc or fs-components
    (also in `.cargo/config(.toml)`: `paths`, `[source]` replacement naming them), one single fs-components tag/rev,
    and the git url of fs-components must be exactly github.com/FreeSynergy/fs-components (no forks).
    fs-components itself is exempt: `crates/fs-components/Cargo.toml` with name "fs-components" AND the identity
    github.com/FreeSynergy/fs-components (package `repository`, or the git `origin` of the repo root).
    LIMIT: these are file checks. Whoever deletes `.fs-rules.toml` (or fakes the identity of the exempt repo in a
    repo whose origin is elsewhere AND whose Cargo.toml says so) turns the rule off; only review catches that.
    Programs take Blinc through `fs_components::blinc`.

    python3 scripts/check_rust_rules.py /srv/FreeSynergy/fs-store [/srv/FreeSynergy/fs-hub …]
"""
import re, subprocess, sys, tomllib
from pathlib import Path

HARD, SOFT = 300, 200

DEP_TABLES = ("dependencies", "dev-dependencies", "build-dependencies")
SWITCH_FILE = ".fs-rules.toml"
SKIP_DIRS = {"target", ".git", "node_modules"}
COMPONENTS_ANY = re.compile(r"/fs-components(\.git)?/?$", re.I)       # any repo named fs-components
COMPONENTS_GIT = re.compile(r"^(https://|ssh://git@|git@)github\.com[/:]FreeSynergy/fs-components(\.git)?/?$", re.I)  # the FreeSynergy one

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

def _origin_url(repo: Path) -> str:
    """git origin of `repo` itself (not of a parent repository); "" when it is no repo root or has no origin."""
    try:
        top = subprocess.run(["git", "-C", str(repo), "rev-parse", "--show-toplevel"], capture_output=True, text=True, timeout=20)
        if top.returncode or Path(top.stdout.strip()).resolve() != repo.resolve(): return ""
        r = subprocess.run(["git", "-C", str(repo), "remote", "get-url", "origin"], capture_output=True, text=True, timeout=20)
        return r.stdout.strip() if r.returncode == 0 else ""
    except (OSError, subprocess.SubprocessError): return ""

def is_components_repo(repo: Path) -> bool:
    """The one repo that may carry Blinc: crates/fs-components is a package named fs-components AND the repo is
    FreeSynergy/fs-components (package `repository` or git origin). A self-made Cargo.toml alone is not enough."""
    f = repo / "crates" / "fs-components" / "Cargo.toml"
    if not f.is_file(): return False
    try: pkg = tomllib.loads(f.read_text(encoding="utf-8")).get("package", {})
    except tomllib.TOMLDecodeError: return False
    if pkg.get("name") != "fs-components": return False
    repository = pkg.get("repository")
    return (isinstance(repository, str) and bool(COMPONENTS_GIT.search(repository))) or bool(COMPONENTS_GIT.search(_origin_url(repo)))

def _forbidden_text(*parts) -> bool:
    """Blinc or fs-components named anywhere in a patch/replace/source entry."""
    t = " ".join(str(p) for p in parts).lower()
    return "blinc" in t or "fs-components" in t

def _patch_errors(rel, doc):
    """In a migrated program EVERY [patch]/[replace] on blinc or fs-components is forbidden (path, git, branch alike)."""
    for source, entries in ((doc.get("patch") or {}).items()):
        for name, spec in (entries or {}).items():
            if _forbidden_text(source, name, spec):
                yield f"{rel}: [patch.{source}] `{name}`: Blinc/fs-components are pinned only by the fs-components tag, a patch is forbidden"
    for name, spec in (doc.get("replace") or {}).items():
        if _forbidden_text(name, spec):
            yield f"{rel}: [replace] `{name}`: Blinc/fs-components are pinned only by the fs-components tag"

def _cargo_config_errors(rel, doc):
    yield from _patch_errors(rel, doc)
    if doc.get("paths"):
        yield f"{rel}: `paths` override: forbidden in a migrated program (it can replace fs-components/Blinc unseen)"
    for name, spec in (doc.get("source") or {}).items():
        if _forbidden_text(name, spec):
            yield f"{rel}: [source.{name}] names Blinc/fs-components: forbidden"

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
                if "blinc" in f"{name} {real} {git}".lower():
                    errors.append(f"{rel}: dependency `{name}`: programs take Blinc through fs_components::blinc")
                if COMPONENTS_ANY.search(str(git)) and not COMPONENTS_GIT.search(str(git)):
                    errors.append(f"{rel}: dependency `{name}`: git url {git} is not github.com/FreeSynergy/fs-components")
                if COMPONENTS_GIT.search(str(git)):
                    pins.setdefault(next((f"{k} {spec[k]}" for k in ("tag", "rev", "branch") if k in spec), "unpinned"), []).append(rel)
        errors.extend(_patch_errors(rel, doc))
    for name in ("config.toml", "config"):
        for f in _walk(repo, f".cargo/{name}"):
            rel = f.relative_to(repo).as_posix()
            try: errors.extend(_cargo_config_errors(rel, tomllib.loads(f.read_text(encoding="utf-8"))))
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
