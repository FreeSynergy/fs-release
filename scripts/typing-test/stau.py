#!/usr/bin/env python3
"""Counts input stalls in the app's Wayland trace (app-B.log): keys typed 120 ms apart that reach the app bundled
(several within 40 ms) or whose next wl_surface.commit follows more than 500 ms later.
Writes stau.json, prints one line, exits 0 (no stall) or 1 (stall)."""
import json
import re
import statistics
import sys
from bisect import bisect_right

out = sys.argv[1]
MODIFIERS = {42, 54, 29, 97, 56, 100}


def ts(line):
    m = re.match(r"\[(\d+):(\d+):(\d+)\.(\d+)\]", line)
    if m:
        return int(m[1]) * 3600 + int(m[2]) * 60 + int(m[3]) + int(m[4]) / 1e6
    m = re.match(r"\[\s*(\d+)\.(\d+)\]", line)
    return (int(m[1]) + int(m[2]) / 1000) / 1000 if m else None


key_re = re.compile(r"^\[[\s\d:.]+\]\s+(?:\{[^}]*\}\s+)?wl_keyboard[#@]\d+\.key\(\d+, \d+, (\d+), 1\)")
commit_re = re.compile(r"\s->\s+wl_surface[#@]\d+\.commit\(\)")
keys, commits = [], []
for line in open(f"{out}/app-B.log", errors="ignore"):
    if "wl_keyboard" not in line and "commit" not in line:
        continue
    t = ts(line)
    if t is None:
        continue
    m = key_re.search(line)
    if m:
        if int(m[1]) not in MODIFIERS:
            keys.append(t)
    elif commit_re.search(line):
        commits.append(t)
gaps = []
for t in keys:
    i = bisect_right(commits, t)
    gaps.append((commits[i] - t) * 1000 if i < len(commits) else 5000.0)
bundled = sum(1 for a, b in zip(keys, keys[1:]) if (b - a) < 0.040)
stalled = sum(1 for g in gaps if g > 500)
res = {"keys": len(keys), "stalled_keys": stalled, "bundled_keys": bundled, "stau": bool(stalled or bundled),
       "median_ms": round(statistics.median(gaps), 1) if gaps else None, "max_ms": round(max(gaps), 1) if gaps else None}
json.dump(res, open(f"{out}/stau.json", "w"))
print(json.dumps(res))
sys.exit(1 if res["stau"] or len(keys) < 10 else 0)
