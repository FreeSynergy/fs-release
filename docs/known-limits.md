# Known limits

Things the shared tools do not solve, written down so nobody has to rediscover them. One entry per limit: what, why it
stays, where it lives. (`git-bug` is not set up for this repository; this file is the place until it is.)

## typing-test: a light-blue selection is taken for the caret

- **What:** a light-blue selection over a character of 2-3 px width is recognised as the caret (gap 1 behind the text): a
  false negative, the test is green where a missing caret should make it red.
- **Why it stays:** the third stage of the caret search accepts any light-blue bar of 2-3 columns and at least 12 px height
  inside the field (the caret frozen at a faint point of its fade, measured (192,216,247)); a selection of that size looks the
  same. The real apps draw no such selection in the test, and no light-blue fringes at glyph edges (162 real pictures).
- **Context:** branch `tt-faint-caret`, commit `43d8da2`; `scripts/typing-test/fieldcheck.py` (`FAINT_MINH`, `minw`);
  `DESIGN.md` "Field and caret". Known, not solved.
