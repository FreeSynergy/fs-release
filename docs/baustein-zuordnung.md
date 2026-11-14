# baustein-zuordnung

Step `baustein-zuordnung` of `workflows/dossier.yaml` (shown in `frontend-round`). Order for every Frontend: (1) block analysis,
(2) look (style analysis, tokens and rules per block, ADR 0074), (3) composition. This step is (1), checked deterministically,
without network and AI:

```
python3 -I scripts/baustein-zuordnung --dossier bausteine.md --bestand bestand.txt [--json report.json]
```

rc 0 = green, rc 1 = deviation, rc 2 = bad input (no table, empty inventory, missing file; never green). Self test:
`bash scripts/test_baustein_zuordnung.sh`.

## Dossier template: `bausteine.md`

```markdown
## Baustein-Zuordnung

| Bereich | Was ist das? | Baustein-Id | Module | Neu | Begründung |
|---|---|---|---|---|---|
| Dateiliste | Tabelle | table | Zeilenköpfe aus, Spaltenköpfe an | nein | |
| Seitenleiste | Baum | tree | - | nein | |
| Gantt-Leiste | Zeitachse mit Balken | gantt | - | ja | Kein vorhandener Baustein zeichnet Balken auf einer Zeitachse, auch nicht mit Modul. |
```

- Every area of the model (every region of the states and every dialog of the dossier) has a row; "what is this?" is the plain kind
  (table, tree, list, card, bar, dialog ...).
- `Baustein-Id`: the fixed name of the block (ADR 0074). Existing blocks first. `Module`: what is switched on or added on the same
  block (hidden row/column headers, sorting, formulas ...; ADR 0084 pattern: one block, several modules, no second copy).
- `Neu` = `nein` (the id must be in the inventory) or `ja` (the id must not be in the inventory, and `Begründung` says why no existing
  block with a module fits).
- Inventory (`--bestand`): one block id per line, or a JSON list; the coordinator writes it from `fs-components` (the family crates).

## Report

Deviations: `ohne-baustein`, `neu-unklar`, `unbekannt` (marked existing, not in the inventory), `schon-da` (marked new, exists),
`ohne-begruendung`. The last line gives areas, number and share of new blocks (`--json`: `share_new`); the share is reported in
the round's report, a high share is a reason to look again, not red by itself.
