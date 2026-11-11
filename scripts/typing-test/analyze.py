#!/usr/bin/env python3
"""Reads what session.sh left in the result directory and decides: is every character the right one, is the text
visible, how fast does it show.

Writes result.json and junit.xml, prints a table, exits 0 (all pass) or 1 (a check failed).
Settings come from the TT_* environment (see ../typing-test.sh)."""
import glob
import json
import os
import warnings
import re
import statistics
import subprocess
import sys
from bisect import bisect_right
from xml.sax.saxutils import quoteattr

warnings.filterwarnings("ignore", category=DeprecationWarning)
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import fieldcheck as fc  # noqa: E402
from PIL import Image, ImageDraw, ImageFont  # noqa: E402

out = sys.argv[1]
CHARS = ["a", "Ä", "ä", "ö", "ü", "ß", "@", "€", "{", "}", "[", "]", "\\", "|", "~", "µ"]
PARTS = ["aÄäöüß@€", "{}[]\\|~µ"]
MAX_MS = float(os.environ.get("TT_MAX_MS") or "8")
MIN_KEYS = 10
CARET_GAP_PX = 4
# What a wrong layout could have produced instead: printable ASCII and the German extras.
CANDIDATES = [chr(c) for c in range(33, 127)] + list("äöüÄÖÜß€µ§°´²³")
LOOKALIKES = {"|": set("|lIi!1")}  # a plain bar cannot be told from these by shape, so "|" is only checked that weakly
# Near twins a wrong layout or a missing accent could produce: the typed character must rank above its twin, not only be in the
# best three (a dead key gives a for ä, a missing AltGr a bracket of the wrong kind).
RIVALS = {"Ä": "A", "ä": "a", "ö": "o", "ü": "u", "[": "(", "{": "(", "]": ")", "}": ")"}
ACCENTED = set("ÄäöÜü")
RANK_OK = 3# the typed character must be among the best three matches ("|" looks like "l" in many fonts)


def shot(name):
    return fc.load(f"{out}/shots/{name}.png")


def reference_font():
    for cand in [os.environ.get("TT_REF_FONT", "")] + [
        subprocess.run(["fc-match", "-f", "%{file}", "DejaVu Sans"], capture_output=True, text=True).stdout.strip()
        if subprocess.run(["which", "fc-match"], capture_output=True).returncode == 0 else "",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/dejavu-sans-fonts/DejaVuSans.ttf",
    ]:
        if cand and os.path.exists(cand):
            return cand
    return None


def glyph(img, rect):
    """The ink of one glyph as a grey bitmap (1 = ink) cut to its bounding box, or None."""
    ink = fc.ink(img, rect, thr=40, faint_caret=True)
    if len(ink) < 3:
        return None
    x0, x1 = min(x for x, _ in ink), max(x for x, _ in ink)
    y0, y1 = min(y for _, y in ink), max(y for _, y in ink)
    return [[ink.get((x, y), 0.0) for x in range(x0, x1 + 1)] for y in range(y0, y1 + 1)]


def ref_glyph(font_file, size, ch):
    font = ImageFont.truetype(font_file, size)
    im = Image.new("L", (size * 2, size * 2), 0)
    ImageDraw.Draw(im).text((size // 2, size // 2), ch, fill=255, font=font)
    box = im.getbbox()
    if not box:
        return None
    return im.crop(box)


def resized(bitmap, w, h):
    im = Image.new("L", (len(bitmap[0]), len(bitmap)))
    im.frombytes(bytes(int(v * 255) for row in bitmap for v in row))
    return im.resize((w, h), Image.BILINEAR) if im.size != (w, h) else im


def similarity(bitmap, ref):
    """Soft overlap of two glyph bitmaps of about the same size, less a penalty for a different shape box."""
    h, w = len(bitmap), len(bitmap[0])
    rw, rh = ref.size
    r = ref.resize((w, h), Image.BILINEAR)
    pr = list(r.tobytes())
    inter = union = 0.0
    for i, row in enumerate(bitmap):
        for j, v in enumerate(row):
            p = pr[i * w + j] / 255
            inter += min(v, p)
            union += max(v, p)
    shape = abs(w - rw) / max(w, rw) + abs(h - rh) / max(h, rh)
    return (inter / union if union else 0.0) - 0.25 * shape


def font_size_for(font_file, glyphs):
    """Pixel size of the reference font at which the glyphs that arrived look most like the characters meant: the best
    sum of similarities over all of them, so one glyph that did not arrive (it shows what was in the field before)
    cannot spoil the size for the others."""
    sizes = {}
    for size in range(8, 48):
        total = 0.0
        for ch, bitmap in glyphs.items():
            ref = ref_glyph(font_file, size, ch)
            if bitmap is not None and ref is not None:
                total += similarity(bitmap, ref)
        sizes[size] = total
    return max(sizes, key=sizes.get)


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


if not os.path.exists(f"{out}/shots/full1.png") or not os.path.exists(f"{out}/shots/g00.png"):
    print(f"typing-test: no pictures in {out}/shots (screenshot tool failed, see spectacle.log); the test did not run", file=sys.stderr)
    sys.exit(2)

cases = []  # (name, passed, message)


def case(name, ok, msg=""):
    cases.append((name, bool(ok), msg))


rect = tuple(int(v) for v in open(f"{out}/rect.txt").read().strip().split(","))
result = {"layout": os.environ.get("TT_LAYOUT", "de"), "app": os.path.basename(os.environ.get("TT_BIN", "app")), "chars": {}}
blank = shot("blank")

# 1. every character arrives, and it is the right glyph (best matches of a reference rendering of all candidates)
font_file = reference_font()
glyphs = {ch: glyph(shot(f"g{i:02d}"), rect) for i, ch in enumerate(CHARS)}
here = os.path.dirname(os.path.abspath(__file__))


def arrived_in(i):
    return subprocess.run([sys.executable, "-I", os.path.join(here, "fieldcheck.py"), "arrived", f"{out}/shots/g{i:02d}.png",
                           f"{out}/shots/blank.png"], env={**os.environ, "TT_RECT": ",".join(map(str, rect))}).returncode == 0


arrived_all = {ch: arrived_in(i) for i, ch in enumerate(CHARS)}
size = font_size_for(font_file, {ch: g for ch, g in glyphs.items() if arrived_all[ch]}) if font_file and any(arrived_all.values()) else None
single_ink = {}
for i, ch in enumerate(CHARS):
    img = shot(f"g{i:02d}")
    n = len(fc.ink(img, rect))
    single_ink[ch] = n
    arrived = arrived_all[ch]
    result["chars"][ch] = {"ink_pixels": n}
    if not arrived:
        case(f"char {ch!r} arrives", False, f"the field shows no typed glyph ({n} ink pixels)")
        continue
    if size is None or glyphs[ch] is None:
        case(f"char {ch!r} is right", False, "no reference font (set TT_REF_FONT) or no glyph")
        continue
    ranked = sorted(((similarity(glyphs[ch], rg), k) for k in CANDIDATES if (rg := ref_glyph(font_file, size, k))), reverse=True)
    top = [k for _, k in ranked[:RANK_OK]]
    result["chars"][ch]["best_matches"] = "".join(top)
    right = bool(set(top) & LOOKALIKES.get(ch, {ch}))
    rival = RIVALS.get(ch)
    if right and rival:  # the typed character must score above its near twin (Ä over A, [ over ( ...)
        if ch in ACCENTED:  # the marks are thin and blur at small sizes, but they make the glyph taller: height decides
            h = len(glyphs[ch])
            right = abs(h - ref_glyph(font_file, size, ch).size[1]) < abs(h - ref_glyph(font_file, size, rival).size[1])
        else:
            score = {k: s for s, k in ranked}
            right = score.get(ch, -9) > score.get(rival, -9)
    case(f"char {ch!r} is right", right, f"drawn like {''.join(top)!r}" + ("" if right else f", expected {ch!r}" + (f" and not {rival!r}" if rival else "")))

# Glyphs that showed only after waiting (the picture lagged behind the key): reported, not a failure.
late = {}
if os.path.exists(f"{out}/late.log"):
    for line in open(f"{out}/late.log"):
        name, _, n = line.partition(" ")
        if n.strip() not in ("", "0"):
            late[CHARS[int(name[1:])]] = int(n)
result["late_pictures"] = late

# 2. the text typed in two parts is visible (not black, plenty of ink) and the caret sits right behind it
screen = shot("blank0").resize((64, 40))
lum = statistics.fmean(sum(screen.getpixel((x, y))) / 3 for x in range(64) for y in range(40))
result["visible"] = {"screen_mean_luminance": round(lum, 1)}
case("screen is not black", lum > 10, f"mean luminance {lum:.0f}")
for n, text in enumerate(PARTS, 1):
    img = shot(f"full{n}")
    px = len(fc.ink(img, rect))
    wanted = sum(single_ink[c] for c in text)
    result["visible"][f"part{n}"] = {"ink_pixels": px, "sum_of_single_glyphs": wanted}
    case(f"part {n} is visible", px >= 0.6 * wanted and px >= 4 * len(text), f"{px} ink pixels, single glyphs {wanted}")
    case(f"part {n} typed in one go equals one by one", 0.6 * wanted <= px <= 1.4 * wanted, f"{px} vs {wanted}")
    gap = None
    extra = sorted(glob.glob(f"{out}/shots/full{n}.?.png"))
    if extra:
        ink_px = fc.ink(img, rect)
        if ink_px:
            end = rect[0] + max(x for x, _ in ink_px)
            bar = fc.find_caret([img] + [fc.load(p) for p in extra], rect, after=end)
            if bar:
                gap = bar[0] - end - 1
    result["visible"][f"part{n}"]["caret_gap_px"] = gap
    case(f"part {n} caret sits behind the last glyph", gap is not None and -1 <= gap <= CARET_GAP_PX, f"gap {gap} px")

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
