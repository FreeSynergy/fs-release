#!/usr/bin/env python3
"""Reads what session.sh left in the result directory and decides: did the text arrive, is it visible, how fast.

Writes result.json and junit.xml, prints a table, exits 0 (all pass) or 1 (a check failed).
Settings come from the TT_* environment (see ../typing-test.sh)."""
import json
import os
import re
import statistics
import sys
from bisect import bisect_right
from xml.sax.saxutils import quoteattr

from PIL import Image

out = sys.argv[1]
CHARS = ["a", "Ä", "ä", "ö", "ü", "ß", "@", "€", "{", "}", "[", "]", "\\", "|", "~", "µ"]
MAX_MS = float(os.environ.get("TT_MAX_MS", "8"))
MIN_KEYS = 10
CARET_GAP_PX = 4
MAX_GLYPH_INK = 120


def field_rect():
    return tuple(int(v) for v in open(f"{out}/rect.txt").read().strip().split(","))


def shot(name):
    return Image.open(f"{out}/shots/{name}.png").convert("RGB")


def is_ink(p):  # dark body text; the lighter placeholder and the blue caret are not ink
    r, g, b = p
    return r < 130 and g < 140 and b - r < 60


def is_caret(p):
    r, g, b = p
    return b - r > 80 and r < 200


def scan(img, rect):
    """Ink pixels (relative to the first ink pixel's box), and the caret columns, inside the rectangle."""
    crop = img.crop(rect)
    w, h = crop.size
    px = crop.load()
    ink = {(x, y) for x in range(w) for y in range(h) if is_ink(px[x, y])}
    caret = sorted({x for x in range(w) for y in range(h) if is_caret(px[x, y])})
    return ink, caret


def faint(img, rect):
    """Every pixel that is not the field's background and not the caret: thin parts like the dots of an umlaut too."""
    crop = img.crop(rect)
    w, h = crop.size
    px = crop.load()
    return {(x, y) for x in range(w) for y in range(h) if sum(px[x, y]) < 700 and not is_caret(px[x, y])}


def signature(ink):
    if not ink:
        return None
    x0 = min(x for x, _ in ink)
    y0 = min(y for _, y in ink)
    return tuple(sorted((x - x0, y - y0) for x, y in ink))


def latency(path):
    def ts(line):
        # libwayland prints [HH:MM:SS.uuuuuu] (new) or [<milliseconds>.<microseconds>] (old).
        m = re.match(r"\[(\d+):(\d+):(\d+)\.(\d+)\]", line)
        if m:
            return int(m[1]) * 3600 + int(m[2]) * 60 + int(m[3]) + int(m[4]) / 1e6
        m = re.match(r"\[\s*(\d+)\.(\d+)\]", line)
        return (int(m[1]) + int(m[2]) / 1000) / 1000 if m else None

    keys, commits = [], []
    key_re = re.compile(r"^\[[\s\d:.]+\]\s+(?:\{[^}]*\}\s+)?wl_keyboard[#@]\d+\.key\(\d+, \d+, (\d+), 1\)")
    commit_re = re.compile(r"\s->\s+wl_surface[#@]\d+\.commit\(\)")
    modifiers = {42, 54, 29, 97, 56, 100}
    for line in open(path, errors="ignore"):
        if "wl_keyboard" not in line and "commit" not in line:
            continue
        t = ts(line)
        if t is None:
            continue
        m = key_re.search(line)
        if m:
            if int(m[1]) not in modifiers:
                keys.append(t)
        elif commit_re.search(line):
            commits.append(t)
    ms = []
    for t in keys:
        i = bisect_right(commits, t)
        if i < len(commits):
            ms.append((commits[i] - t) * 1000)
    ms.sort()
    return ms


if not os.path.exists(f"{out}/shots/full.png") or not os.path.exists(f"{out}/shots/g00.png"):
    sys.exit(f"typing-test: no pictures in {out}/shots (screenshot tool failed, see spectacle.log); the test did not run")

cases = []  # (name, passed, message)


def case(name, ok, msg=""):
    cases.append((name, bool(ok), msg))


rect = field_rect()
result = {"layout": os.environ.get("TT_LAYOUT", "de"), "app": os.path.basename(os.environ.get("TT_BIN", "app")), "chars": {}}

# 1. every character arrives as its own glyph
sigs = {}
for i, ch in enumerate(CHARS):
    ink, _ = scan(shot(f"g{i:02d}"), rect)
    sigs[ch] = signature(faint(shot(f"g{i:02d}"), rect))
    result["chars"][ch] = {"ink_pixels": len(ink)}
    # A single glyph has few ink pixels; many mean the placeholder text is still showing (nothing arrived).
    case(f"char {ch!r} arrives", 3 <= len(ink) <= MAX_GLYPH_INK, f"{len(ink)} ink pixels in the field")
same = [(a, b) for i, a in enumerate(CHARS) for b in CHARS[i + 1 :] if sigs[a] is not None and sigs[a] == sigs[b]]
case("all 16 glyphs differ", not same, "same picture (missing glyph or wrong key): " + ", ".join(f"{a!r}={b!r}" for a, b in same[:12]))

# Glyphs that showed only after waiting (the picture lagged behind the key): reported, not a failure.
late = {}
if os.path.exists(f"{out}/late.log"):
    for line in open(f"{out}/late.log"):
        name, _, n = line.partition(" ")
        if n.strip() not in ("", "0"):
            late[CHARS[int(name[1:])]] = int(n)
result["late_pictures"] = late

# 2. the whole text is visible: not black, ink share, caret right behind the last glyph
full = shot("full")
full_ink, caret = scan(full, rect)
rw, rh = rect[2] - rect[0], rect[3] - rect[1]
small = full.resize((64, 40))
screen_lum = statistics.fmean(sum(small.getpixel((x, y))) / 3 for x in range(64) for y in range(40))
share = len(full_ink) / (rw * rh)
single_sum = sum(v["ink_pixels"] for v in result["chars"].values())
result["visible"] = {"screen_mean_luminance": round(screen_lum, 1), "ink_share_percent": round(share * 100, 2),
                     "ink_pixels": len(full_ink), "sum_of_single_glyphs": single_sum}
case("screen is not black", screen_lum > 40, f"mean luminance {screen_lum:.0f}")
case("text is visible", share >= 0.01, f"ink share {share * 100:.2f} %")
case("typed in one go equals one by one", 0.6 * single_sum <= len(full_ink) <= 1.4 * single_sum,
     f"{len(full_ink)} ink pixels vs {single_sum}")
if full_ink and caret:
    gap = min(caret) - max(x for x, _ in full_ink) - 1
    result["visible"]["caret_gap_px"] = gap
    case("caret sits behind the last glyph", -1 <= gap <= CARET_GAP_PX, f"gap {gap} px")
else:
    case("caret sits behind the last glyph", False, "no caret or no text found")

# 3. key to picture
ms = latency(f"{out}/app-B.log")
if len(ms) >= MIN_KEYS:
    lat = {"keys": len(ms), "median_ms": round(statistics.median(ms), 2), "p90_ms": round(ms[int(len(ms) * 0.9)], 2),
           "max_ms": round(ms[-1], 2), "min_ms": round(ms[0], 2), "limit_ms": MAX_MS}
    enforced = not os.environ.get("TT_SOFTWARE") or os.environ.get("TT_ENFORCE_MS")
    lat["enforced"] = bool(enforced)
    note = f"median {lat['median_ms']} ms, p90 {lat['p90_ms']}, max {lat['max_ms']}, limit {MAX_MS}"
    case("key to picture, median", lat["median_ms"] <= MAX_MS or not enforced, note + ("" if enforced else " (software rendering: reported, not enforced)"))
else:
    lat = {"keys": len(ms)}
    case("key to picture, median", False, f"only {len(ms)} keys measured")
result["latency"] = lat

failed = [c for c in cases if not c[1]]
result["passed"] = not failed
result["cases"] = [{"name": n, "passed": ok, "message": m} for n, ok, m in cases]
with open(f"{out}/result.json", "w") as f:
    json.dump(result, f, ensure_ascii=False, indent=2)

name = os.environ.get("TT_NAME") or result["app"]
with open(f"{out}/junit.xml", "w", encoding="utf-8") as f:
    f.write(f'<?xml version="1.0" encoding="UTF-8"?>\n<testsuite name={quoteattr("typing-test " + name)} tests="{len(cases)}" failures="{len(failed)}">\n')
    for n, ok, m in cases:
        f.write(f"  <testcase classname={quoteattr('typing-test.' + name)} name={quoteattr(n)}>")
        if not ok:
            f.write(f"<failure message={quoteattr(m)}/>")
        f.write("</testcase>\n")
    f.write("</testsuite>\n")

lines = [f"typing-test {name} (layout {result['layout']}): {'PASS' if not failed else 'FAIL'}"]
lines += [f"  {'ok  ' if ok else 'FAIL'} {n}: {m}" if m else f"  {'ok  ' if ok else 'FAIL'} {n}" for n, ok, m in cases]
print("\n".join(lines))
summary = os.environ.get("GITHUB_STEP_SUMMARY")
if summary:
    with open(summary, "a", encoding="utf-8") as f:
        f.write(f"### typing-test {name}: {'pass' if not failed else 'FAIL'}\n\n```\n" + "\n".join(lines[1:]) + "\n```\n")
sys.exit(1 if failed else 0)
