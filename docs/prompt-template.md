# Compact agent orders (prompt template)

**DE.** Ein Agentenauftrag nennt nur Workflow, Schritt und Parameter. Alles Wiederkehrende steht in der Workflow-Datei
(`workflows/<name>.yaml`, je Schritt `summary`) und in `rules.md` (die Blöcke `[GATES]`, `[BASH]`, `[COMMIT]`, `[TESTS]`,
`[WORTWAHL]`, `[BERICHT]`, `[REVIEW]`); der Agent liest beide selbst. Nicht in den Auftrag gehören: die Gate-Befehle, die
Bash-Regeln, die Berichtsform, die JA/NEIN-Bindung an den Kopf, die Prüfliste je Art (Review), die Wortwahl. In den Auftrag
gehört nur, was sich von Lauf zu Lauf ändert: die Parameter `repo`, `branch`, `head`, `delta` und der Auftragsinhalt (was
konkret zu bauen, zu beheben, zu prüfen ist, mit den Nutzerbefunden im Wortlaut). Messen: `python3 -I scripts/measure-prompts.py`
(mittlere Auftragsgröße und Kosten je Art: build, fix, rev, dossier).

**EN.** An agent order names only the workflow, the step and the parameters. Everything recurring is in the workflow file
(`workflows/<name>.yaml`, a `summary` per step) and in `rules.md` (blocks `[GATES]`, `[BASH]`, `[COMMIT]`, `[TESTS]`,
`[WORTWAHL]`, `[BERICHT]`, `[REVIEW]`); the agent reads both itself. Not in the order: the gate commands, the bash rules, the
report form, the JA/NEIN binding to the head, the check list per kind (review), the wording rules. In the order goes only
what changes from run to run: the parameters `repo`, `branch`, `head`, `delta` and the content (what exactly to build, fix or
check, with the user's findings verbatim). Measure with `python3 -I scripts/measure-prompts.py` (mean order size and cost per
kind: build, fix, rev, dossier).

## Template (each at most 12 lines)

Build:

```
Workflow agent-run, Schritt build. Parameter: repo=fs-store, branch=unreviewed/store-017, head=7b5cb6e, delta=-.
Auftrag: <was gebaut wird, 3-8 Zeilen, Entscheidungen des Nutzers im Wortlaut>.
Grenzen: <nur wenn von rules.md abweichend, sonst weglassen>.
```

Fix (after a review NEIN or a user finding):

```
Workflow bugfix, Schritt reproduce, dann fix. Parameter: repo=fs-store, branch=unreviewed/store-016-fix, head=7b5cb6e,
delta=8b3dcf3..7b5cb6e, finding="Neu laden funktioniert nicht zuverlaessig".
Befunde des Reviews: <Datei:Zeile, je eine Zeile>.
```

Review:

```
Workflow review, Schritt read. Parameter: repo=fs-store, branch=unreviewed/store-016, head=7b5cb6e,
delta=8b3dcf3..7b5cb6e, scope=B4, tests=reload_notices,window_draws.
Nutzerbefunde und Pruefpunkte: (a) <...> (b) <...> ... (nur die Sachfragen dieses Laufs).
Bericht: BERICHT.md.
```

Dossier:

```
Workflow dossier, Schritt research. Parameter: frontend=fs-wiki, repo=fs-documentation, branch=unreviewed/dossier-wiki.
Quellen und Vorbilder: <Namen/URLs, die erlaubt sind>. Schwerpunkt: <z. B. Kontextmenüs, Kürzel>.
```

## Before / after: `rev-store-016`

Before (`prompts/rev-store-016.txt`, about 3 KB): role line, branch and delta, "schlank (<= 40 Tool-Aufrufe)", read
rules.md, the six user findings (a) to (f), seven numbered check points, the three gate commands with the "Bash einzeln, keine
&& ; |" rule, "nichts aendern ausser BERICHT.md", the report form (JA/NEIN for push and tag bound to the head, <= 16 lines
German, findings file:line).

After (about 1 KB, the same content, nothing lost):

```
Workflow review, Schritt read. Parameter: repo=fs-store, branch=unreviewed/store-016, head=7b5cb6e,
delta=8b3dcf3..7b5cb6e, scope=store-016 (B4), tests=reload_notices,window_draws.
Nutzerbefunde: (a) Neu laden unzuverlaessig - Ursache + Test; (b) Reload-Knopf view-refresh, Tooltip "Katalog neu laden", keine
Beschriftung; (c) Vorschau IST die Bestaetigung, Passwortabfrage angekuendigt; (d) Meldungen blenden aus, Schliessen, Verlauf;
(e) Start = Ueberblick, Programme = Liste, Zaehlung exakt; (f) Icons erst B5.
Pruefpunkte: Ursachen von (a) plausibel gefixt (Listing-Fehler leert offers; Panic laesst running haengen), Gegenproben;
Groesse/Kontrast von cell_button (16 px?); keine zweite Stufe in der Zustandsmaschine; Zeiten 6 s/20 s, Verlauf begrenzt;
window_draws unter Windows ohne GPU (cfg?); Version 0.0.16, CHANGELOG/README/DESIGN EN+DE.
```

What moved out of the order: the gates ([GATES]), the Bash rule ([BASH]), "only BERICHT.md" and "lean" ([REVIEW]), the report
form and the binding to the head ([BERICHT]), the word rule ([WORTWAHL]); the check list by kind is in `review.yaml`, step
`read`. The size is measured, not guessed: `scripts/measure-prompts.py` shows the mean size per kind before and after the
switch (the table of the day is the "before" figure; run it again after some orders in the new form).
