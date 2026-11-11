#!/usr/bin/env python3
"""Central Blinc lock: Cargo.lock may carry Blinc ONLY from an approved fs-blinc tag (exit 1 otherwise).

    python3 -I scripts/check_blinc_lock.py [--approved FILE] <Cargo.lock | repo root | ->     (`-` reads the lock from stdin)

Pattern: none (checker; called by check_rust_rules.py, so every repo's `check` gate runs it).
A package counts as Blinc when its name starts with `blinc` or its source mentions blinc (any case, any fork such as
blinc-app-fix / blinc-layout-fix). Each one must have exactly
`source = "git+https://github.com/FreeSynergy/Blinc?tag=<tag>#<sha>"` with `<tag> <sha>` a line of scripts/blinc-approved.txt
(full sha, so a moved tag is caught). crates.io, path, rev/branch sources and forks are errors, and so is a lock that
mixes two Blinc sources, even when both are approved (during a transition two tags are approved, a lock still holds ONE).
A lock without Blinc packages passes. Missing Cargo.lock = nothing to check.
Approved file: one entry per line `<tag> <40-hex sha>`, optionally followed by a comment; a comment containing
`expires YYYY-MM-DD` makes the entry expire: from the day AFTER that date it is an error (red for every repo) and no longer
approves anything, so a transition cannot stay forever. Whole-line comments are ignored. `BLINC_TODAY=YYYY-MM-DD` replaces
today's date (tests only).
"""
import datetime, os, re, sys, tomllib
from pathlib import Path

APPROVED = Path(__file__).resolve().parent / "blinc-approved.txt"
URL = "https://github.com/FreeSynergy/Blinc"
FIX = ("programs take Blinc only through fs-components (tag as approved in scripts/blinc-approved.txt; bump with "
       "scripts/bump-components.sh, then `cargo update`). scripts/approve-blinc.sh <tag> is for maintainers approving a NEW tag")
TAG_RE = r"fs-blinc-[0-9]+(?:\.[0-9]+)?"          # the same shape approve-blinc.sh accepts

def today() -> datetime.date:
    return datetime.date.fromisoformat(os.environ["BLINC_TODAY"]) if os.environ.get("BLINC_TODAY") else datetime.date.today()

def load_approved(f: Path):
    """{tag: (sha, expires date or None)} from the approved file; ValueError on a malformed line or a duplicate tag."""
    out = {}
    for n, line in enumerate(f.read_text(encoding="utf-8").splitlines(), 1):
        entry, _, comment = line.partition("#")
        entry = entry.strip()
        if not entry: continue
        m = re.fullmatch(rf"({TAG_RE})\s+([0-9a-f]{{40}})", entry)
        if not m: raise ValueError(f"{f}:{n}: expected `<tag> <40-hex sha> [# expires YYYY-MM-DD ...]`, got {entry!r}")
        e = re.search(r"\bexpires\s+(\d{4}-\d{2}-\d{2})\b", comment)
        try: exp = datetime.date.fromisoformat(e.group(1)) if e else None
        except ValueError: raise ValueError(f"{f}:{n}: bad expiry date {e.group(1)!r}") from None
        if m.group(1) in out: raise ValueError(f"{f}:{n}: tag {m.group(1)} listed twice")
        out[m.group(1)] = (m.group(2), exp)
    return out

def is_blinc(pkg: dict) -> bool:
    return pkg.get("name", "").lower().startswith("blinc") or "blinc" in str(pkg.get("source", "")).lower()

def lock_has_blinc(text: str) -> bool:
    try: return any(is_blinc(p) for p in tomllib.loads(text).get("package", []))
    except tomllib.TOMLDecodeError: return False

def lock_errors(text: str, approved: dict, label: str = "Cargo.lock"):
    try: doc = tomllib.loads(text)
    except tomllib.TOMLDecodeError as e: return [f"{label}: not valid TOML ({e})"]
    errors, used, bad = [], set(), 0
    now = today()
    for t, (_, exp) in sorted(approved.items()):
        if exp is not None and exp < now:
            errors.append(f"{label}: approval of {t} expired on {exp}: finish the transition and run scripts/approve-blinc.sh remove {t} (or renew with add --expires)")
    approved = {t: s for t, (s, exp) in approved.items() if exp is None or exp >= now}      # expired entries approve nothing
    want = ", ".join(f"{t} ({s[:12]})" for t, s in sorted(approved.items())) or "none"
    for pkg in doc.get("package", []):
        if not is_blinc(pkg): continue
        name, src = pkg.get("name", "?"), pkg.get("source")
        m = re.fullmatch(re.escape("git+" + URL) + r"\?tag=([^#&]+)#([0-9a-f]{40})", src or "")
        if m and approved.get(m.group(1)) == m.group(2): used.add(src); continue
        why = ("no source (a path dependency)" if src is None else
               f"tag {m.group(1)} is at {m.group(2)[:12]}, approved is {approved.get(m.group(1), 'not approved')}" if m else
               f"source {src}")
        errors.append(f"{label}: package `{name}` {pkg.get('version', '')}: {why}")
        bad += 1
    if bad:
        errors.append(f"{label}: {bad} Blinc package(s) not from the approved source; allowed only as git+{URL}?tag=<approved>#<sha>, "
                      f"approved: {want}. Fix: {FIX}")
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
        p = Path(a)
        if p.is_dir():
            p = p / "Cargo.lock"
            if not p.is_file(): print(f"blinc lock ok (no Cargo.lock in {a}: nothing to check)"); return 0
        text, label = p.read_text(encoding="utf-8"), str(p)
    errors = lock_errors(text, approved, label)
    for e in errors: print("ERROR", e)
    if not errors: print(f"blinc lock ok ({label})")
    return 1 if errors else 0

if __name__ == "__main__":
    sys.exit(main(sys.argv))
