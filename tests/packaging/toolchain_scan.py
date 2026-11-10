#!/usr/bin/env python3
"""Scan workflows/actions for a Rust toolchain other than the pinned one. Pattern: none (test helper of gates.sh).

    toolchain_scan.py <root>        prints one violation per line; exit 1 when any

Comments (whole line or ` # …` at the line end) are ignored. Violations: the word `stable`, a `toolchain:` value other
than `pinned` / `${{ inputs.msrv }}`, `rustup default|toolchain|override|update`, `cargo +x`, `RUSTUP_TOOLCHAIN` in a
step, and a `setup-rust` step in `_release-*.yml` without `toolchain: pinned` anywhere in ITS step (any distance).
"""
import re, sys
from pathlib import Path

def code(line: str) -> str:
    return re.sub(r"(^|\s)#.*$", "", line).rstrip()

def main(root: Path) -> int:
    out = []
    files = [f for d in ("/.github/workflows", "/actions") for f in sorted(Path(str(root) + d).rglob("*.yml"))]
    for f in files:
        if f.parent.name == "setup-rust": continue          # the action that resolves `pinned` itself
        lines = [code(l) for l in f.read_text(encoding="utf-8").splitlines()]
        rel = f.relative_to(root).as_posix()
        for n, l in enumerate(lines, 1):
            m = re.search(r"\btoolchain:\s*(.*)$", l)
            if m and m.group(1).strip() not in ("pinned", "${{ inputs.msrv }}"):
                out.append(f"{rel}:{n}: toolchain: {m.group(1).strip()}")
            if re.search(r"\bstable\b", l) and not m: out.append(f"{rel}:{n}: names stable")
            if re.search(r"\brustup\s+(default|toolchain|override|update|install)\b|\bcargo\s+\+\S+|RUSTUP_TOOLCHAIN", l):
                out.append(f"{rel}:{n}: own toolchain choice: {l.strip()}")
        if f.name.startswith("_release-"):
            for n, l in enumerate(lines):
                if "setup-rust@" not in l: continue
                key = len(l) - len(l.lstrip(" ")) + (2 if l.lstrip().startswith("- ") else 0)
                step = []
                for k in lines[n + 1:]:
                    if k.strip().startswith("- ") and len(k) - len(k.lstrip()) < key: break
                    step.append(k)
                if not any(re.search(r"toolchain:\s*pinned$", k) for k in step):
                    out.append(f"{rel}:{n + 1}: setup-rust without toolchain: pinned")
    print("\n".join(out))
    return 1 if out else 0

if __name__ == "__main__":
    sys.exit(main(Path(sys.argv[1])))
