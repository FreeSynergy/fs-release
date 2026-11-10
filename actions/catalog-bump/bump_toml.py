#!/usr/bin/env python3
"""bump_toml.py FILE REPO TAG -> raises `tag` of the [[package]] whose repo is REPO.

Exit 0 and prints one of: `changed <id> <old> <new>`, `same <id> <tag>`, `older <id> <old> <new>` (not downgraded),
`unknown` (REPO is no catalog package). Only the tag line is rewritten, comments and layout stay as they are.
"""
import re
import sys
import tomllib


def version(tag):
    return tuple(int(x) for x in re.fullmatch(r"v(\d+)\.(\d+)\.(\d+)", tag).groups())


def main(path, repo, tag):
    text = open(path, encoding="utf-8").read()
    packages = tomllib.loads(text)["package"]
    match = [p for p in packages if p.get("repo") == repo]
    if not match:
        print("unknown")
        return
    p = match[0]
    old = p["tag"]
    if old == tag:
        print(f"same {p['id']} {tag}")
        return
    if version(tag) < version(old):
        print(f"older {p['id']} {old} {tag}")
        return
    # the `tag = ` line inside the block that holds this repo
    blocks = re.split(r"(?m)^(?=\[\[package\]\])", text)
    for i, block in enumerate(blocks):
        if re.search(rf'(?m)^repo\s*=\s*"{re.escape(repo)}"\s*$', block):
            blocks[i] = re.sub(r'(?m)^(tag\s*=\s*)"[^"]*"', lambda m: f'{m.group(1)}"{tag}"', block, count=1)
    open(path, "w", encoding="utf-8").write("".join(blocks))
    print(f"changed {p['id']} {old} {tag}")


if __name__ == "__main__":
    main(*sys.argv[1:4])
