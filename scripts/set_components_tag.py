#!/usr/bin/env python3
"""Point every dependency on the fs-components git repo at `tag = "<tag>"` (rev/branch are removed).

    scripts/set_components_tag.py <tag> <repo root>      exit 0: changed or already on the tag; 3: no such dependency; 1: could not

Pattern: none (helper of bump-components.sh).
The files are edited line by line (comments and layout stay), the parser is the judge: after the edit every
dependency whose `git` is the fs-components repo, under any name or `package =`, in `dependencies`,
`dev-`/`build-dependencies`, `target.*` and `workspace.dependencies`, must have exactly `tag = <tag>` and no rev/branch.
Handled forms: inline tables, `[dependencies.name]` tables (also `[target.'cfg(..)'.dependencies.name]`).
Anything else fails the check and exits 1, nothing is written then.
"""
import re, sys, tomllib
from pathlib import Path

REPO_URL = re.compile(r"/fs-components(\.git)?/?$", re.I)
SKIP = {"target", ".git", "node_modules"}
VERSION_KEYS = ("rev", "branch", "tag")

def cargo_tomls(root: Path):
    """Every Cargo.toml below root, but not inside target/, .git/ or nested worktrees (.wt-*)."""
    for f in sorted(root.rglob("Cargo.toml")):
        parts = f.relative_to(root).parts[:-1]
        if any(p in SKIP or p.startswith(".wt-") for p in parts): continue
        yield f

def is_components(spec) -> bool:
    return isinstance(spec, dict) and isinstance(spec.get("git"), str) and bool(REPO_URL.search(spec["git"]))

def dep_tables(doc):
    for k, v in doc.items():
        if k in ("dependencies", "dev-dependencies", "build-dependencies") and isinstance(v, dict): yield v
    for t in (doc.get("target") or {}).values():
        if isinstance(t, dict): yield from dep_tables(t)
    if isinstance(doc.get("workspace"), dict): yield from dep_tables(doc["workspace"])

def components_deps(doc):
    return [(n, s) for t in dep_tables(doc) for n, s in t.items() if is_components(s)]

def _inline(line: str, tag: str) -> str:
    """`name = { git = "…/fs-components", rev = "x" }` -> tag = "<tag>" right after git."""
    if not re.search(r"git\s*=\s*[\"'][^\"']*/fs-components(\.git)?/?[\"']", line, re.I): return line
    line = re.sub(r",?\s*\b(rev|branch|tag)\s*=\s*(\"[^\"]*\"|'[^']*')", "", line)
    return re.sub(r"(\bgit\s*=\s*(\"[^\"]*\"|'[^']*'))", lambda m: f'{m.group(1)}, tag = "{tag}"', line, count=1)

def edit(text: str, tag: str) -> str:
    out, section = [], []                       # section: lines of the current [table] (header excluded)
    def flush():
        body = "".join(section)
        if re.search(r"^\s*git\s*=\s*[\"'][^\"']*/fs-components(\.git)?/?[\"']", body, re.M | re.I):
            kept = [l for l in section if not re.match(r"\s*(rev|branch|tag)\s*=", l)]
            i = next(i for i, l in enumerate(kept) if re.match(r"\s*git\s*=", l))
            kept.insert(i + 1, f'tag = "{tag}"\n')
            section[:] = kept
        out.extend(section); section.clear()
    for line in text.splitlines(keepends=True):
        if re.match(r"\s*\[", line):
            flush(); out.append(line)
        else:
            section.append(_inline(line, tag) if "=" in line and "{" in line else line)
    flush()
    return "".join(out)

def main(argv):
    if len(argv) != 3: print(__doc__, file=sys.stderr); return 2
    tag, root = argv[1], Path(argv[2])
    found, writes = 0, {}
    for f in cargo_tomls(root):
        text = f.read_text(encoding="utf-8")
        try: doc = tomllib.loads(text)
        except tomllib.TOMLDecodeError as e: print(f"{f}: not valid TOML ({e})", file=sys.stderr); return 1
        if not components_deps(doc): continue
        found += len(components_deps(doc))
        new = edit(text, tag)
        try: ndoc = tomllib.loads(new)
        except tomllib.TOMLDecodeError as e: print(f"{f}: edit broke the file ({e})", file=sys.stderr); return 1
        deps = components_deps(ndoc)
        bad = [n for n, s in deps if s.get("tag") != tag or "rev" in s or "branch" in s]
        if len(deps) != len(components_deps(doc)): bad.append("(a dependency lost its git)")
        if bad:
            print(f"{f}: cannot set the tag on: {', '.join(bad)} (form not handled; edit by hand)", file=sys.stderr); return 1
        if new != text: writes[f] = new
    if not found:
        print("no dependency on the fs-components git repo found", file=sys.stderr); return 3
    for f, new in writes.items(): f.write_text(new, encoding="utf-8")
    print(f"changed {len(writes)} file(s), {found} dependencies" if writes else f"already on {tag}")
    return 0

if __name__ == "__main__":
    sys.exit(main(sys.argv))
