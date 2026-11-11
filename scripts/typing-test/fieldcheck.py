#!/usr/bin/env python3
"""Picture helpers for the typing test, colours taken from the picture itself (no theme is assumed).

  fieldcheck.py window A.png          x0,y0,x1,y1 of everything that is not the black screen (exit 1: nothing there)
  fieldcheck.py field A.png           the inside of the input field around the click point TT_NEAR=x,y (exit 1: none found)
  fieldcheck.py same A.png B.png      exit 0 if the field (TT_RECT) looks the same in both, a caret apart
  fieldcheck.py arrived A.png BLANK   exit 0 if the field shows a typed glyph instead of what BLANK showed
  fieldcheck.py caretx A.png B.png... x of the caret inside the field TT_RECT, from two or more pictures"""
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


INSET = 4  # the focus ring and the border are left out of the field's inside


def _far(px, x, y, bg, thr):
    r, g, b = px[x, y]
    return abs(r - bg[0]) + abs(g - bg[1]) + abs(b - bg[2]) > thr


def field_around(img, point, thr=12, minline=40):
    """The input field that contains `point` (the click): the nearest enclosing rectangle whose edge differs from the
    field's own background. Top and bottom are rows with a long unbroken line across the click's column (glyphs and a
    caret never make one), left and right are columns that are 'far' over the middle of that height (a caret is too short).
    Buttons and other things beside or above the field are never looked at. Returns (x0, y0, x1, y1) of the inside
    (ring and border cut off) or None."""
    w, h = img.size
    px = img.load()
    cx, cy = point
    if not (0 <= cx < w and 0 <= cy < h):
        return None
    box = img.crop((max(0, cx - 80), max(0, cy - 6), min(w, cx + 80), min(h, cy + 7)))
    bg = background(box)

    def hline(y):
        lo = hi = cx
        while lo > 0 and _far(px, lo - 1, y, bg, thr):
            lo -= 1
        while hi < w - 1 and _far(px, hi + 1, y, bg, thr):
            hi += 1
        return _far(px, cx, y, bg, thr) and hi - lo + 1 >= minline

    top = next((y for y in range(cy, -1, -1) if hline(y)), None)
    bottom = next((y for y in range(cy, h) if hline(y)), None)
    if top is None or bottom is None or bottom - top < 12:
        return None
    height = bottom - top + 1

    # Corners are rounded, so only the middle 60 % of the height must be an edge; a caret is shorter than that.
    band = range(top + int(height * 0.2), bottom - int(height * 0.2) + 1)

    def vline(x):
        return sum(_far(px, x, y, bg, thr) for y in band) >= 0.9 * len(band)

    left = next((x for x in range(cx, -1, -1) if vline(x)), None)
    right = next((x for x in range(cx, w) if vline(x)), None)
    if left is None or right is None or right - left < 20:
        return None
    return left + INSET, top + INSET, right - INSET, bottom - INSET


def _blue_mask(img, rect):
    px = img.load()
    x0, y0, x1, y1 = rect
    return {(x - x0, y - y0) for x in range(x0, x1 + 1) for y in range(y0, y1 + 1)
            if px[x, y][2] - px[x, y][0] > 100 and px[x, y][0] < 130}


def find_caret(frames, rect, after=None):
    """The caret in the field `rect` (inside only, so a button or the focus ring in the same pixel column cannot be
    taken for it): a narrow bar that differs between some of the pictures (it blinks; the union over all pairs), else
    a narrow saturated-blue bar present in some picture. With several candidates the one nearest to x `after` (the end
    of the text). Returns (x, y0, y1) in picture coordinates or None."""
    x0, y0, x1, y1 = rect
    maxh = y1 - y0 + 1
    mask = set()
    for i in range(len(frames)):
        for j in range(i + 1, len(frames)):
            mask |= diff(frames[i], frames[j], (x0, y0, x1 + 1, y1 + 1))
    found = bars(mask, maxh=maxh)
    if not found:
        blue = set()
        for f in frames:
            blue |= _blue_mask(f, rect)
        found = bars(blue, maxh=maxh)
    if not found:
        return None
    pick = min(found, key=lambda t: abs(t[0] - ((after if after is not None else 0) - x0)))
    return pick[0] + x0, pick[1] + y0, pick[2] + y0


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
    if mode == "field":
        near = tuple(int(v) for v in os.environ["TT_NEAR"].split(","))
        rect = field_around(load(sys.argv[2]), near)
        if not rect:
            return 1
        width = os.environ.get("TT_TEXT_WIDTH")
        if width:
            rect = (rect[0], rect[1], min(rect[2], rect[0] + int(width)), rect[3])
        print(",".join(map(str, rect)))
        return 0
    if mode == "caretx":
        frames = [load(p) for p in sys.argv[2:]]
        bar = find_caret(frames, rect_from_env())
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
