# baustein-zuordnung

Step `baustein-zuordnung` of `workflows/dossier.yaml` (shown in `frontend-round`). Order for every Frontend: (1) block analysis,
(2) look (style analysis, tokens and rules per block, ADR 0074), (3) composition. This step is (1), checked deterministically,
without network and AI:

```
python3 -I scripts/baustein-zuordnung --dossier bausteine.md --bestand bestand.txt --blinc blinc.txt [--json report.json]
```

rc 0 = green, rc 1 = deviation (red round 1 or 2), rc 2 = bad input (no table, empty inventory, missing file; never green, the round counter stays untouched), rc 3 = deviation in the third red round or later (escalate to the person). Self test:
`bash scripts/test_baustein_zuordnung.sh`.

## Source order (rule of the user)

For every area of the model, in this order: (1) a block of the **fs-components family** (the crates `fs-components-*`); (2) a **Blinc widget**
through an adapter (wrapper with pin, no fork; the stock of the Blinc widgets is listed in `/srv/FreeSynergy/.scratch/blinc-bausteine.md`);
(3) only then a **new block**, with a reason that says why neither a family block nor a Blinc widget fits. The list
`/srv/FreeSynergy/.scratch/bestand-gemalte-nachbildungen.md` stays the source for the wave 1 rounds and is not changed by this rule.

## Dossier template: `bausteine.md`

```markdown
## Baustein-Zuordnung

| Bereich | Was ist das? | Baustein-Id | Module | Quelle | Begründung |
|---|---|---|---|---|---|
| Dateiliste | Tabelle | table | Zeilenköpfe aus, Spaltenköpfe an | familie | |
| Seitenleiste | Baum | tree | - | familie | |
| Dialog | Dialog | dialog | - | blinc | |
| Gantt-Leiste | Zeitachse mit Balken | gantt | - | neu | Kein Baustein der Familie zeichnet Balken auf einer Zeitachse, auch nicht mit Modul; kein Blinc-Widget tut es (auch nicht cn oder canvas_kit). |
```

- Every area of the model (every region of the states and every dialog of the dossier) has a row; "what is this?" is the plain kind
  (table, tree, list, card, bar, dialog ...).
- `Baustein-Id`: the fixed name of the block (ADR 0074). `Module`: what is switched on or added on the same block (hidden row/column
  headers, sorting, formulas ...; ADR 0084 pattern: one block, several modules, no second copy).
- `Quelle` = `familie` (the id must be in the family inventory), `blinc` (the id must be in the Blinc inventory and **not** in the family
  inventory: the family comes first) or `neu` (the id must be in neither inventory, and `Begründung` must say why no family block and why
  no Blinc widget fits; the script checks that the reason names both, "Familie"/"family" and "Blinc").
- Inventories (`--bestand` family, `--blinc` Blinc widgets): one block id per line, or a JSON list; the coordinator writes them from the
  `fs-components` crates and from the Blinc stock.
- Round counter: the state file `.baustein-zuordnung-runde` in the dossier folder counts the red rounds (+1 on red, 0 on green); the
  output line `runde=N` and rc 3 from N >= 3 are what `dossier.yaml` uses to stop and send the person (`eingabe-fehler`).

## Report

Deviations: `ohne-baustein`, `quelle-unklar`, `unbekannt` (id not in the inventory of its source), `familie-zuerst` (marked Blinc, the family
has it), `schon-da` (marked new, exists), `ohne-begruendung`, `begruendung-unvollstaendig`. The last lines give areas, the number and share
per source (`familie`, `blinc`, `neu`; `--json`: `sources`, `share_new`) and `runde=N`; the shares are reported in the round's report, a high
share of `neu` is a reason to look again, not red by itself.
