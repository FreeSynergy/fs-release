#!/usr/bin/env python3
"""Idle cost of the app: CPU percent of one core and wl_surface commits per second (app-B.log has the Wayland trace)."""
import json
import re
import sys

out = sys.argv[1]
c0, c1, tck, secs = (float(v) for v in open(f"{out}/idle.txt").read().split())
commits = [
    float(m[1]) * 3600 + float(m[2]) * 60 + float(m[3]) + float(m[4]) / 1e6
    for m in (re.match(r"\[(\d+):(\d+):(\d+)\.(\d+)\].*\s->\s+wl_surface[#@]\d+\.commit", l) for l in open(f"{out}/app-B.log", errors="ignore"))
    if m
]
# Commits are counted over the last `secs` seconds of the trace (the idle stretch).
tail = [t for t in commits if commits and t >= commits[-1] - secs] if commits else []
res = {"cpu_percent": round(100 * (c1 - c0) / tck / secs, 1), "commits_per_s": round(len(tail) / secs, 1)}
json.dump(res, open(f"{out}/idle.json", "w"))
print(json.dumps(res))
