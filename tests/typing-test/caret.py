#!/usr/bin/env python3
"""Synthetic pictures for the caret and field detection of scripts/typing-test/fieldcheck.py (no app, no KWin).
  python3 -I tests/typing-test/caret.py
Every picture is 600x300 with a light page; a field with a rounded-less border, optional focus ring and glyph blocks."""
import os
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
FC = os.path.join(HERE, "..", "..", "scripts", "typing-test", "fieldcheck.py")
sys.path.insert(0, os.path.dirname(FC))
import fieldcheck as fc  # noqa: E402
from PIL import Image, ImageDraw  # noqa: E402

PAGE, WHITE, BLUE, DARK, RING = (245, 245, 245), (255, 255, 255), (50, 120, 230), (30, 30, 30), (90, 160, 240)
FIELD = (40, 100, 440, 140)  # outer box of the field (ring included)
CLICK = (200, 120)


def picture(text_chars=8, caret_at=None, ring=True, add_right=True, button_above=False, button_below_same_column=False):
    im = Image.new("RGB", (600, 300), PAGE)
    d = ImageDraw.Draw(im)
    d.rectangle(FIELD, fill=WHITE, outline=RING if ring else (180, 180, 180), width=2 if ring else 1)
    for i in range(text_chars):  # glyph blocks: 6 px wide, 12 px high, 10 px apart
        x = FIELD[0] + 12 + i * 10
        d.rectangle((x, 112, x + 5, 124), fill=DARK)
    if add_right:  # a blue button with light text, right beside the field
        d.rectangle((460, 100, 520, 140), fill=BLUE)
        d.rectangle((475, 115, 500, 125), fill=(255, 255, 255))
    if button_above:  # like the "Save" button above the field of fs-files
        d.rectangle((60, 40, 120, 80), fill=BLUE)
    if button_below_same_column:  # a blue bar in the pixel column of the field's left/right focus ring
        d.rectangle((FIELD[0], 150, FIELD[0] + 1, 200), fill=BLUE)
        d.rectangle((FIELD[2] - 1, 150, FIELD[2], 200), fill=BLUE)
    if caret_at is not None:
        d.rectangle((caret_at, 108, caret_at + 1, 128), fill=BLUE)
    return im


def text_end(n):
    return FIELD[0] + 12 + (n - 1) * 10 + 5


fails = []


def check(name, ok, msg=""):
    print(("ok   " if ok else "FAIL ") + name + (": " + msg if msg and not ok else ""))
    if not ok:
        fails.append(name)


def caret_gap(frames, rect, n=8):
    ink = fc.ink(frames[0], rect)
    end = rect[0] + max(x for x, _ in ink)
    bar = fc.find_caret(frames, rect, after=end)
    return None if bar is None else bar[0] - end - 1


# 1. field around the click: the field itself, not the button column beside it or the ring
for kw in ({}, {"ring": False}, {"button_above": True}, {"button_below_same_column": True}):
    rect = fc.field_around(picture(**kw), CLICK)
    want = (FIELD[0] + fc.INSET_X, FIELD[1] + fc.INSET_Y, FIELD[2] - fc.INSET_X, FIELD[3] - fc.INSET_Y)
    check(f"field found {kw}", rect is not None and all(abs(a - b) <= 3 for a, b in zip(rect, want)), f"{rect} vs {want}")
rect = fc.field_around(picture(), CLICK)

# 2. caret blinking: on in one picture, off in the next (and the other way round): found right behind the text
cx = text_end(8) + 3
for name, frames in {
    "caret on, off": [picture(caret_at=cx), picture()],
    "caret off, on": [picture(), picture(caret_at=cx)],
    "off, off, on (union of several pictures)": [picture(), picture(), picture(caret_at=cx)],
}.items():
    gap = caret_gap(frames, rect)
    check(f"caret {name}", gap is not None and 0 <= gap <= 4, f"gap {gap}")

# 3. caret that does not blink (identical pictures, only blue): the Add button beside and the Save button above are ignored
for kw in ({"add_right": True}, {"add_right": False, "button_above": True}, {"button_below_same_column": True}):
    gap = caret_gap([picture(caret_at=cx, **kw), picture(caret_at=cx, **kw)], fc.field_around(picture(**kw), CLICK))
    check(f"static caret, buttons around {kw}", gap is not None and 0 <= gap <= 4, f"gap {gap}")

# 4. no caret anywhere in the field: nothing is found (the button's blue column is not a caret)
check("no caret: none found", fc.find_caret([picture(), picture(), picture()], rect) is None)

# 5. negative: the caret far before the end of the text stays red (gap far below -1)
early = FIELD[0] + 20
gap = caret_gap([picture(caret_at=early), picture()], rect)
check("caret far before the text end is not accepted", gap is not None and gap < -1, f"gap {gap}")

# 6. the command line used by session.sh: caretx with TT_RECT only looks inside the field
with tempfile.TemporaryDirectory() as tmp:
    paths = []
    for i, im in enumerate([picture(caret_at=cx, add_right=True), picture(caret_at=cx, add_right=True)]):
        p = os.path.join(tmp, f"{i}.png")
        im.save(p)
        paths.append(p)
    env = {**os.environ, "TT_RECT": ",".join(map(str, rect)), "TT_NEAR": f"{CLICK[0]},{CLICK[1]}"}
    r = subprocess.run([sys.executable, "-I", FC, "caretx", *paths], env=env, capture_output=True, text=True)
    check("caretx CLI inside the field", r.returncode == 0 and r.stdout.strip() == str(cx), r.stdout + r.stderr)
    r = subprocess.run([sys.executable, "-I", FC, "field", paths[0]], env=env, capture_output=True, text=True)
    check("field CLI", r.returncode == 0 and tuple(map(int, r.stdout.split(","))) == rect, r.stdout + r.stderr)
    # a field smaller than the real one (explicit TT_FIELD) must not make the caret outside it count
    env["TT_RECT"] = "60,104,100,136"
    r = subprocess.run([sys.executable, "-I", FC, "caretx", *paths], env=env, capture_output=True, text=True)
    check("caret outside an explicit field is not taken", r.returncode == 1, r.stdout)

sys.exit(1 if fails else 0)
