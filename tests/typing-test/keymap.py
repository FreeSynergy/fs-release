#!/usr/bin/env python3
"""The injector's key table against xkbcomp (no KWin, no app): for every character of the test, on de and us, the
injector (`inject table <layout>`) must send the evdev key code, Shift, AltGr and dead-key flag that the compiled xkb
layout says produce it. A character the layout cannot type (the German letters and the euro sign on us) must have no
entry. Prints one line per character and layout.
  TT_INJECT=<fs-typing-inject> python3 -I tests/typing-test/keymap.py"""
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "..", "scripts", "typing-test"))
import keymap  # noqa: E402


def injector_table(binary, layout):
    out = subprocess.run([binary, "table", layout], capture_output=True, text=True, check=True).stdout
    table = {}
    for line in out.splitlines():
        ch, rest = line[0], line[2:].split()
        table[ch] = tuple(int(x) for x in rest)
    return table


def main():
    binary = os.environ.get("TT_INJECT")
    if not binary or not os.path.exists(binary):
        sys.exit("tests/typing-test/keymap.py: set TT_INJECT to the built fs-typing-inject")
    bad = 0
    for layout in ("de", "us"):
        oracle = keymap.strokes(layout)
        ours = injector_table(binary, layout)
        for ch in keymap.WANTED:
            want = oracle.get(ch)
            want = None if want is None else (want[0], int(want[1]), int(want[2]), int(want[3]))
            got = ours.get(ch)
            ok = want == got
            bad += not ok
            print(f"{'ok  ' if ok else 'FAIL'} {layout} {ch!r}: xkb {want} injector {got}")
    print("keymap:", "all strokes right" if not bad else f"{bad} wrong")
    sys.exit(1 if bad else 0)


main()
