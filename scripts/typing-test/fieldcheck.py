#!/usr/bin/env python3
"""Picture helpers for the typing test, colours taken from the picture itself (no theme is assumed).

  fieldcheck.py window A.png          x0,y0,x1,y1 of everything that is not the black screen (exit 1: nothing there)
  fieldcheck.py rect A.png B.png      the text area of the focused field, found by its blinking caret (the narrow bar
                                      that differs between two pictures); TT_NEAR=x,y picks the bar nearest to it
  fieldcheck.py same A.png B.png      exit 0 if the field (TT_RECT) looks the same in both, a caret apart
  fieldcheck.py arrived A.png BLANK   exit 0 if the field shows a typed glyph instead of what BLANK showed
  fieldcheck.py caretx A.png B.png    x of the caret (column in the picture) from two pictures"""
import os
import sys
import warnings
from collections import Counter

from PIL import Image

warnings.filterwarnings("ignore", category=DeprecationWarning)


def rect_from_env():
    return tuple(int(v) for v in os.environ.get("TT_RECT", "0,0,1,1").split(","))


def load(path):
    return Image.open(path).convert("RGB")


def background(crop):
    return Counter(crop.getdata()).most_common(1)[0][0]


def ink(img, rect, thr=150):
    """Pixels far from the field's background colour, leaving out the bright blue of the caret."""
    crop = img.crop(rect)
    bg = background(crop)
    w, h = crop.size
    px = crop.load()
    out = {}
    for x in range(w):
        for y in range(h):
            r, g, b = px[x, y]
            d = abs(r - bg[0]) + abs(g - bg[1]) + abs(b - bg[2])
            if d > thr and not (b - r > 55 and b > 180):  # the caret is blue-ish and bright
                out[(x, y)] = min(1.0, d / 600)
    return out


def bars(mask, minh=10, maxh=40, maxw=3):
    """Solid vertical bars of at most maxw columns in a set of (x, y): what a caret looks like."""
    cols = {}
    for x, y in mask:
        cols.setdefault(x, []).append(y)
    solid = {}
    for x, ys in cols.items():
        lo, hi = min(ys), max(ys)
        if minh <= len(ys) <= maxh and hi - lo + 1 == len(ys):
            solid[x] = (lo, hi)
    out, run = [], []
    for x in sorted(solid):
        if run and x != run[-1] + 1:
            out.append(run)
            run = []
        run.append(x)
    if run:
        out.append(run)
    return [(r[0], solid[r[0]][0], solid[r[0]][1]) for r in out if len(r) <= maxw]


def diff(a, b, rect=None, thr=60):
    rect = rect or (0, 0, a.size[0], a.size[1])
    ca, cb = a.crop(rect), b.crop(rect)
    w, h = ca.size
    pa, pb = ca.load(), cb.load()
    return {(x, y) for x in range(w) for y in range(h) if sum(abs(pa[x, y][i] - pb[x, y][i]) for i in range(3)) > thr}


def caret_bar(a, b, near=None):
    found = bars(diff(a, b))
    if not found:
        return None
    if near:
        return min(found, key=lambda t: abs(t[0] - near[0]) + abs(t[1] - near[1]))
    return found[0]


def main():
    mode = sys.argv[1]
    if mode == "window":
        img = load(sys.argv[2])
        w, h = img.size
        px = img.load()
        xs = [x for x in range(0, w, 2) for y in range(0, h, 4) if sum(px[x, y]) > 30]
        ys = [y for x in range(0, w, 4) for y in range(0, h, 2) if sum(px[x, y]) > 30]
        if not xs or not ys or max(xs) - min(xs) < 100 or max(ys) - min(ys) < 100:
            return 1
        print(f"{min(xs)},{min(ys)},{max(xs)},{max(ys)}")
        return 0
    if mode == "rect":
        near = tuple(int(v) for v in os.environ["TT_NEAR"].split(",")) if os.environ.get("TT_NEAR") else None
        bar = caret_bar(load(sys.argv[2]), load(sys.argv[3]), near)
        if not bar:
            return 1
        cx, y0, y1 = bar
        width = int(os.environ.get("TT_TEXT_WIDTH", "170"))
        print(f"{cx - 4},{y0 - 4},{cx + width},{y1 + 4}")
        return 0
    if mode == "caretx":
        bar = caret_bar(load(sys.argv[2]), load(sys.argv[3]))
        if not bar:
            return 1
        print(bar[0])
        return 0
    rect = rect_from_env()
    if mode == "same":
        a, b = load(sys.argv[2]), load(sys.argv[3])
        rest = diff(a, b, rect) - {p for bar in bars(diff(a, b, rect), minh=8) for p in [(bar[0], y) for y in range(bar[1], bar[2] + 1)]}
        return 0 if len(rest) <= 3 else 1
    if mode == "arrived":
        now, blank = len(ink(load(sys.argv[2]), rect)), len(ink(load(sys.argv[3]), rect))
        return 0 if (now < 0.6 * blank if blank >= 15 else now >= 3) else 1
    return 2


if __name__ == "__main__":
    sys.exit(main())
