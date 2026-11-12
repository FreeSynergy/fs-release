#!/usr/bin/env python3
"""Every `.github/workflows/*.yml|*.yaml` of a repo must parse as YAML (a step name like `Build (debug, the test: x)` is not
YAML and GitHub answers "workflow file issue" only after the push). With `actionlint` installed it runs on top of that.

usage: python3 -I check_yaml.py <repo root> [--simulate-missing-yaml]
exit 0 green, 1 red (first line `file:line: reason`), 2 skipped (no PyYAML: nothing was checked, never reported as green).
"""
import glob
import os
import shutil
import subprocess
import sys


def first_line(text):
    return next((ln.strip() for ln in text.splitlines() if ln.strip()), "no output")


def main(argv):
    root = os.path.abspath(argv[1])
    try:
        if "--simulate-missing-yaml" in argv:
            raise ImportError("simulated")
        import yaml
    except ImportError:
        print("skipped: PyYAML (python3 yaml module) is not installed, the workflow files were NOT checked")
        return 2
    wf = os.path.join(root, ".github", "workflows")
    files = sorted(glob.glob(os.path.join(wf, "*.yml")) + glob.glob(os.path.join(wf, "*.yaml")))
    for path in files:
        rel = os.path.relpath(path, root)
        try:
            with open(path, encoding="utf-8") as f:
                yaml.safe_load(f)
        except yaml.YAMLError as e:
            mark = getattr(e, "problem_mark", None)
            line = mark.line + 1 if mark else 0
            why = getattr(e, "problem", None) or first_line(str(e))
            print(f"{rel}:{line}: {why}")
            return 1
    note = f"{len(files)} workflow file(s) parse"
    lint = shutil.which("actionlint")
    if lint and files:
        r = subprocess.run([lint, "-no-color", *files], cwd=root, capture_output=True, text=True, check=False)
        if r.returncode != 0:
            print(first_line(r.stdout + "\n" + r.stderr))
            return 1
        note += ", actionlint ok"
    print(note)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
