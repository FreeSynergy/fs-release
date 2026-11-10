#!/usr/bin/env python3
"""fieldcheck.py same A.png B.png | arrived A.png | rect A.png

rect:    print x0,y0,x1,y1 of the focused field's text area, found by its caret (exit 1 if no caret is showing).
same:    exit 0 if the field looks the same in both pictures (the blinking caret is ignored).
arrived: exit 0 if the field shows a typed glyph (a few dark pixels, not the long light placeholder text)."""
import os
import sys

from PIL import Image

rect = tuple(int(v) for v in os.environ.get("TT_RECT", "0,0,1,1").split(","))


def pixels(path):
    px = Image.open(path).convert("RGB").crop(rect)
    w, h = px.size
    data = px.load()
    return {(x, y): data[x, y] for x in range(w) for y in range(h)}


def body(path):
    return {p for p, (r, g, b) in pixels(path).items() if b - r < 80 and r + g + b < 600}


def ink(path):
    return {p for p, (r, g, b) in pixels(path).items() if r < 130 and g < 140 and b - r < 60}


if sys.argv[1] == "rect":
    img = Image.open(sys.argv[2]).convert("RGB")
    w, h = img.size
    data = img.load()
    cols = {}
    for x in range(w):
        ys = [y for y in range(h) if data[x, y][2] - data[x, y][0] > 100 and data[x, y][0] < 130]
        if 10 <= len(ys) <= 40 and max(ys) - min(ys) + 1 == len(ys):  # a solid vertical bar
            cols[x] = (min(ys), max(ys))
    # A caret is a bar of at most 3 columns; wider blue areas are buttons.
    xs = sorted(cols)
    bars, run = [], []
    for x in xs:
        if run and x != run[-1] + 1:
            bars.append(run)
            run = []
        run.append(x)
    if run:
        bars.append(run)
    bars = [b for b in bars if len(b) <= 3]
    if not bars:
        sys.exit(1)
    near = tuple(int(v) for v in os.environ["TT_CLICK"].split(",")) if os.environ.get("TT_CLICK") else (0, 0)
    bar = min(bars, key=lambda b: abs(b[0] - near[0]) + abs(cols[b[0]][0] - near[1]))
    cx = bar[0]
    y0, y1 = cols[cx]
    width = int(os.environ.get("TT_TEXT_WIDTH", "150"))
    print(f"{cx - 10},{y0 - 8},{cx + width},{y1 + 8}")
    sys.exit(0)
if sys.argv[1] == "same":
    sys.exit(0 if body(sys.argv[2]) == body(sys.argv[3]) else 1)
sys.exit(0 if 3 <= len(ink(sys.argv[2])) <= 120 else 1)
