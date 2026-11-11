#!/usr/bin/env python3
"""Central Blinc lock: Cargo.lock may carry Blinc ONLY from the approved fs-blinc tag (exit 1 otherwise).

    python3 -I scripts/check_blinc_lock.py [--approved FILE] <Cargo.lock | repo root | ->     (`-` reads the lock from stdin)

Pattern: none (checker; called by check_rust_rules.py, so every repo's `check` gate runs it).
A package counts as Blinc when its name starts with `blinc` or its source mentions blinc (any case, any fork such as
blinc-app-fix / blinc-layout-fix). Each one must have exactly
`source = "git+https://github.com/FreeSynergy/Blinc?tag=<tag>#<sha>"` with `<tag> <sha>` a line of scripts/blinc-approved.txt
(full sha, so a moved tag is caught). crates.io, path, rev/branch sources and forks are errors, and so is a lock that
mixes two Blinc sources. A lock without Blinc packages passes. Missing Cargo.lock = nothing to check.
"""
import re, sys, tomllib
from pathlib import Path

APPROVED = Path(__file__).resolve().parent / "blinc-approved.txt"
URL = "https://github.com/FreeSynergy/Blinc"
FIX = "scripts/approve-blinc.sh <tag> (after the owner approved it), then `cargo update -p blinc_app -p blinc_layout …` in the program"

def load_approved(f: Path):
    """{tag: sha} from the approved file; ValueError on a malformed line."""
    out = {}
    for n, line in enumerate(f.read_text(encoding="utf-8").splitlines(), 1):
        line = line.split("#", 1)[0].strip()
        if not line: continue
        m = re.fullmatch(r"(fs-blinc-[\w.-]+)\s+([0-9a-f]{40})", line)
        if not m: raise ValueError(f"{f}:{n}: expected `<tag> <40-hex sha>`, got {line!r}")
        out[m.group(1)] = m.group(2)
    return out

def is_blinc(pkg: dict) -> bool:
    return pkg.get("name", "").lower().startswith("blinc") or "blinc" in str(pkg.get("source", "")).lower()

def lock_errors(text: str, approved: dict, label: str = "Cargo.lock"):
    try: doc = tomllib.loads(text)
    except tomllib.TOMLDecodeError as e: return [f"{label}: not valid TOML ({e})"]
    want = ", ".join(f"{t} ({s[:12]})" for t, s in sorted(approved.items())) or "none"
    errors, used = [], set()
    for pkg in doc.get("package", []):
        if not is_blinc(pkg): continue
        name, src = pkg.get("name", "?"), pkg.get("source")
        m = re.fullmatch(re.escape("git+" + URL) + r"\?tag=([^#&]+)#([0-9a-f]{40})", src or "")
        if m and approved.get(m.group(1)) == m.group(2): used.add(src); continue
        why = ("no source (a path dependency)" if src is None else
               f"tag {m.group(1)} is at {m.group(2)[:12]}, approved is {approved.get(m.group(1), 'not approved')}" if m else
               f"source {src}")
        errors.append(f"{label}: package `{name}` {pkg.get('version', '')}: {why}; Blinc is allowed only as "
                      f"git+{URL}?tag=<approved>#<sha>, approved: {want}. Fix: {FIX}")
    if len(used) > 1:
        errors.append(f"{label}: two Blinc sources in one lock: {', '.join(sorted(used))}")
    return errors

def check_repo_lock(repo: Path, approved_file: Path = APPROVED):
    """Errors for `<repo>/Cargo.lock` (none when it does not exist)."""
    lock = repo / "Cargo.lock"
    if not lock.is_file(): return []
    try: approved = load_approved(approved_file)
    except (OSError, ValueError) as e: return [f"blinc-approved: {e}"]
    return lock_errors(lock.read_text(encoding="utf-8"), approved)

def main(argv):
    args, approved_file = argv[1:], APPROVED
    if args[:1] == ["--approved"] and len(args) >= 2: approved_file, args = Path(args[1]), args[2:]
    if len(args) != 1: print(__doc__, file=sys.stderr); return 2
    try: approved = load_approved(approved_file)
    except (OSError, ValueError) as e: print(f"ERROR blinc-approved: {e}", file=sys.stderr); return 1
    a = args[0]
    if a == "-": text, label = sys.stdin.read(), "Cargo.lock"
    else:
        p = Path(a); p = p / "Cargo.lock" if p.is_dir() else p
        text, label = p.read_text(encoding="utf-8"), str(p)
    errors = lock_errors(text, approved, label)
    for e in errors: print("ERROR", e)
    if not errors: print(f"blinc lock ok ({label})")
    return 1 if errors else 0

if __name__ == "__main__":
    sys.exit(main(sys.argv))
