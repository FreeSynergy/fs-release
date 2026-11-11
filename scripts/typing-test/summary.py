#!/usr/bin/env python3
"""Summary of the runs of typing-test-runs.sh: summary.json and summary.txt in the result directory.

  summary.py OUT N CODE1 CODE2 ...   (CODEi: exit code of run i: 0 green, 1 red, 2 did not run)"""
import json
import os
import sys

out, n, codes = sys.argv[1], int(sys.argv[2]), [int(c) for c in sys.argv[3:]]
runs = []
for i, code in enumerate(codes, 1):
    red = []
    try:
        with open(f"{out}/run-{i}/result.json") as f:
            red = [c["name"] + (": " + c["message"] if c.get("message") else "") for c in json.load(f)["cases"] if not c["passed"]]
    except (OSError, ValueError, KeyError):
        pass
    runs.append({"run": i, "exit": code, "state": {0: "green", 1: "red"}.get(code, "did not run"), "red_checks": red})
n_red = sum(1 for r in runs if r["exit"] == 1)
n_broken = sum(1 for r in runs if r["exit"] >= 2)
summary = {"runs": n, "red": n_red, "did_not_run": n_broken, "passed": n_red == 0 and n_broken == 0, "results": runs}
with open(f"{out}/summary.json", "w") as f:
    json.dump(summary, f, ensure_ascii=False, indent=2)
lines = [f"typing-test: {n_red} of {n} runs red" + (f", {n_broken} did not run" if n_broken else "") + (" -> PASS" if summary["passed"] else " -> FAIL")]
for r in runs:
    lines.append(f"  run {r['run']}: {r['state']}")
    lines += [f"    red: {c}" for c in r["red_checks"]]
with open(f"{out}/summary.txt", "w") as f:
    f.write("\n".join(lines) + "\n")
