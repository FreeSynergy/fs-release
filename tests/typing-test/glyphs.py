#!/usr/bin/env python3
"""analyze.py on synthetic result directories (no app, no KWin): a first character the app does not draw is reported
as lost and does not spoil the judgement of the others (the font size is fitted over all glyphs that arrived); glyphs
of a wrong layout stay red; a field with a placeholder and rounded corners works.
  python3 -I tests/typing-test/glyphs.py"""
import json
import os
import subprocess
import sys
import tempfile

from PIL import Image, ImageDraw, ImageFont

HERE = os.path.dirname(os.path.abspath(__file__))
ANALYZE = os.path.join(HERE, "..", "..", "scripts", "typing-test", "analyze.py")
sys.path.insert(0, os.path.dirname(ANALYZE))
import fieldcheck as fc  # noqa: E402
with open(os.path.join(HERE, "..", "..", "scripts", "typing-test", "chars-de.txt"), encoding="utf-8") as f:
    CHARS = f.read().splitlines()  # what the test types on the German layout, "a" first
PARTS = ["".join(CHARS[i:i + 8]) for i in range(0, len(CHARS), 8)]
WRONG = {c: CHARS[(i + 7) % len(CHARS)] for i, c in enumerate(CHARS)}  # what a wrong layout could have typed
FIELD = (588, 313, 1076, 349)
FONTS = ["/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", "/usr/share/fonts/dejavu-sans-fonts/DejaVuSans.ttf"]


def find_font():
    out = subprocess.run(["fc-match", "-f", "%{file}", "DejaVu Sans"], capture_output=True, text=True).stdout.strip() \
        if subprocess.run(["which", "fc-match"], capture_output=True).returncode == 0 else ""
    return next((f for f in [os.environ.get("TT_REF_FONT", ""), out, *FONTS] if f and os.path.exists(f)), None)


def picture(font, text="", placeholder="", caret=False):
    im = Image.new("RGB", (1280, 800), (244, 245, 247))
    d = ImageDraw.Draw(im)
    d.rounded_rectangle(FIELD, radius=12, fill=(255, 255, 255), outline=(120, 135, 160), width=1)
    x = FIELD[0] + 12
    if placeholder:
        d.text((x, 322), placeholder, fill=(90, 105, 130), font=font)
    if text:
        d.text((x, 322), text, fill=(30, 30, 30), font=font)
        x += int(d.textlength(text, font=font)) + 1
    if caret:
        d.rectangle((x, 321, x + 1, 337), fill=(50, 120, 230))
    d.rectangle((388, 232, 452, 264), fill=(50, 120, 230))  # a blue button above the field
    return im


def make(out, font, typed, lost_first):
    os.makedirs(f"{out}/shots")
    picture(font, placeholder="Name", caret=True).save(f"{out}/shots/blank.png")
    picture(font, placeholder="Name").save(f"{out}/shots/blank0.png")
    for i, ch in enumerate(CHARS):
        if i == 0 and lost_first:
            picture(font, placeholder="Name", caret=True).save(f"{out}/shots/g00.png")
        else:
            picture(font, text=typed[ch], caret=True).save(f"{out}/shots/g{i:02d}.png")
    for n, text in enumerate(PARTS, 1):
        t = "".join(typed[c] for c in text)
        picture(font, text=t, caret=True).save(f"{out}/shots/full{n}.png")
        picture(font, text=t).save(f"{out}/shots/full{n}.b.png")
    rect = fc.field_around(fc.load(f"{out}/shots/blank.png"), (640, 331))  # as session.sh finds it
    with open(f"{out}/rect.txt", "w") as f:
        f.write(",".join(map(str, rect)) + "\n")
    open(f"{out}/app-B.log", "w").close()


def run(font_file, typed, lost_first):
    font = ImageFont.truetype(font_file, 13)
    with tempfile.TemporaryDirectory() as out:
        make(out, font, typed, lost_first)
        subprocess.run([sys.executable, "-I", ANALYZE, out], env={**os.environ, "TT_REF_FONT": font_file, "TT_SOFTWARE": "1", "TT_LAYOUT": "de"},
                       capture_output=True, text=True)
        return {c["name"]: c["passed"] for c in json.load(open(f"{out}/result.json"))["cases"]}


font_file = find_font()
if not font_file:
    print("skip: no DejaVu Sans font")
    sys.exit(0)
fails = []


def check(name, ok):
    print(("ok   " if ok else "FAIL ") + name)
    if not ok:
        fails.append(name)


right = {c: c for c in CHARS}
cases = run(font_file, right, False)
check("all right layout: every character passes", all(v for k, v in cases.items() if k.startswith(("char", "part", "screen"))))
cases = run(font_file, right, True)
check("first character not drawn: reported as not arrived", cases["char 'a' arrives"] is False)
check("first character not drawn: the others are still judged right (size not spoiled)",
      all(cases[f"char {c!r} is right"] for c in CHARS[1:]))
cases = run(font_file, WRONG, False)
check("wrong layout: characters turn red", sum(not v for k, v in cases.items() if k.endswith("is right")) >= len(CHARS) - 6)
# Near twins: the app draws the twin instead of the typed character (dead key, AltGr missing): that character turns red,
# the others stay green.
TWINS = {"Ä": "A", "ä": "a", "ö": "o", "ü": "u", "Ö": "O", "Ü": "U", "[": "(", "{": "(", "]": ")", "}": ")", ")": "]",
         "+": "=", "/": "?", ";": ":", "'": '"', "~": "`", "@": "2", ".": ",", ",": ".", "-": "_", "_": "-", "^": "~"}
for typed_char, twin in TWINS.items():
    cases = run(font_file, {**right, typed_char: twin}, False)
    check(f"{typed_char} drawn as {twin}: red", cases[f"char {typed_char!r} is right"] is False)
    check(f"{typed_char} drawn as {twin}: the others stay green",
          all(cases[f"char {c!r} is right"] for c in CHARS if c != typed_char))
sys.exit(1 if fails else 0)
