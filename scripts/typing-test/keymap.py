#!/usr/bin/env python3
"""The keyboard layouts of the typing test as xkbcomp compiles them: which strokes (evdev key code, Shift, AltGr, a
dead key followed by Space) produce a character. This is the oracle the injector's table is checked against
(tests/typing-test/keymap.sh); it does not depend on the injector.

  python3 -I keymap.py <layout>          prints "char evdev shift altgr dead" for every wanted character
Needs `xkbcomp` and the xkb data (package x11-xkb-utils / xkb-data)."""
import os
import re
import subprocess
import sys
import tempfile

# The characters the test types (besides letters and digits, which every layout has on the letter keys, a to z, 0 to 9).
SPECIAL = list(".,-_+@/:;'\"()[]{}~^\\|") + ["€", "ä", "ö", "ü", "ß", "Ä", "Ö", "Ü"]
EXTRA = list("!#$%&*<=>?`") + ["µ"]  # more of what other texts of the test type
WANTED = SPECIAL + EXTRA + list("abcdefghijklmnopqrstuvwxyz0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ")

KEYSYMS = {
    "period": ".", "comma": ",", "minus": "-", "underscore": "_", "plus": "+", "at": "@", "slash": "/", "colon": ":",
    "semicolon": ";", "apostrophe": "'", "quotedbl": '"', "parenleft": "(", "parenright": ")", "bracketleft": "[",
    "bracketright": "]", "braceleft": "{", "braceright": "}", "asciitilde": "~", "asciicircum": "^", "backslash": "\\",
    "bar": "|", "EuroSign": "€", "adiaeresis": "ä", "odiaeresis": "ö", "udiaeresis": "ü", "ssharp": "ß",
    "Adiaeresis": "Ä", "exclam": "!", "numbersign": "#", "dollar": "$", "percent": "%", "ampersand": "&",
    "asterisk": "*", "less": "<", "equal": "=", "greater": ">", "question": "?", "grave": "`", "mu": "µ", "Odiaeresis": "Ö", "Udiaeresis": "Ü", "space": " ",
    "dead_circumflex": "^",  # a dead key: typed with a Space after it
}
for d in "0123456789":
    KEYSYMS[d] = d  # xkb names the digit keysyms "0" to "9"
for c in "abcdefghijklmnopqrstuvwxyz":
    KEYSYMS[c] = c
    KEYSYMS[c.upper()] = c.upper()
# Every key's levels: 1 plain, 2 Shift, 3 AltGr, 4 AltGr+Shift (xkb types TWO_LEVEL, ALPHABETIC, FOUR_LEVEL...).
LEVEL = [(False, False), (True, False), (False, True), (True, True)]


def compile_layout(layout):
    src = ('xkb_keymap {\n xkb_keycodes { include "evdev+aliases(qwerty)" };\n xkb_types { include "complete" };\n'
           ' xkb_compat { include "complete" };\n xkb_symbols { include "pc+%s+inet(evdev)" };\n};\n' % layout)
    with tempfile.TemporaryDirectory() as d:
        s, o = os.path.join(d, "k.src"), os.path.join(d, "k.xkb")
        with open(s, "w") as f:
            f.write(src)
        subprocess.run(["xkbcomp", "-xkb", s, o], capture_output=True, check=True)
        with open(o) as f:
            return f.read()


def parse(text):
    """{xkb key name: keycode} and {key name: [keysym of level 1..4]} from a compiled keymap."""
    codes = {m.group(1): int(m.group(2)) for m in re.finditer(r"<(\w+)>\s*=\s*(\d+);", text.split("xkb_types")[0])}
    syms = {}
    for m in re.finditer(r"key <(\w+)>\s*\{(.*?)\};", text.split("xkb_symbols")[1], re.S):
        body = m.group(2)
        g = re.search(r"symbols\[Group1\]\s*=\s*\[([^\]]*)\]", body) or re.search(r"\[([^\]]*)\]", body)
        if g:
            syms[m.group(1)] = [s.strip() for s in g.group(1).split(",")]
    return codes, syms


def codes_syms_items(codes, syms):
    return [(k, v) for k, v in syms.items() if k in codes and 9 <= codes[k] <= 135]  # the main block, not keypad/media


def strokes(layout):
    """{char: (evdev code, shift, altgr, dead)} for every wanted character the layout can type. Of several ways the
    first by key code wins."""
    codes, syms = parse(compile_layout(layout))
    best = {}
    for key, levels in codes_syms_items(codes, syms):
        for i, name in enumerate(levels[:4]):
            ch = KEYSYMS.get(name)
            if ch is None or ch not in WANTED + [" "]:
                continue
            shift, altgr = LEVEL[i]
            dead = name.startswith("dead_")
            rank = (dead, shift + altgr, codes[key])  # a plain stroke beats a dead key beats a chord; then the lower key
            if ch not in best or rank < best[ch][0]:
                best[ch] = (rank, (codes[key] - 8, shift, altgr, dead))
    out = {ch: v[1] for ch, v in best.items()}
    return out


if __name__ == "__main__":
    table = strokes(sys.argv[1])
    for ch in WANTED:
        if ch in table:
            k, sh, alt, dead = table[ch]
            print(ch, k, int(sh), int(alt), int(dead))
        else:
            print(ch, "none")
