# workflows/

**DE.** Hier stehen unsere Abläufe als sichtbare Workflows: Serverless-Workflow-1.0-YAML (`dsl: 1.0.x`) im FS-Profil
(Entwurf `ENTWURF-workflow-standard.md`, Fassung 3). Jeder Schritt trägt `freesynergy.net:actor` (`human` = Person, `ai` =
Application, `machine` = Service; dazu `freesynergy.net:actor-type`) und `freesynergy.net:gate` (Freigabe-Haltepunkt, z. B.
`opus-ja`, `human`, oder `none`) und `freesynergy.net:kind` (`objective` = wiederholen, bis das Ziel erreicht ist, z. B.
Tests grün; `subjective` = Zustand ändern, egal wie das Ergebnis ist, z. B. Bilder zeigen, Urteil einholen). Die Dateien
sind 1:1 in fs-workflows importierbar: Serverless Workflow 1.0 plus FS-Profil, keine Sonderformen. Alle tragen
`document.version` = Crate-Version = Tag (0.0.6). Agenten pushen, taggen und mergen nie. Heute sind sie Beschreibung und
werden nur geprüft (`scripts/check_workflows.py`, Test `crates/fs-release/tests/workflows.rs`; Schrittnamen von `lift` und
`flow` müssen mit dem Code übereinstimmen). In einem zweiten Schritt liest `fs-release flow` diese Dateien und führt den
Ablauf daraus aus. Keine Geheimnisse in diese Dateien.

**Regel: alle Arbeitsschritte laufen über einen Workflow.** Gibt es für einen Schritt keinen, ist der erste Schritt, den
Workflow zu schreiben oder zu erweitern (`workflow-pflege`). Ein Agentenauftrag verweist auf den Flow so:
`Workflow <name>, Schritt <schritt>, Parameter <schlüssel=wert, ...>`, z. B. `Workflow bugfix, Schritt reproduce, Parameter
finding="..."`. Der Agent arbeitet nur die genannten Schritte ab und meldet im Bericht den Flow und die Schritte. Der Auftrag
bleibt kompakt: `Workflow <name>, Schritt <id>, Parameter repo, branch, head, delta, Auftragsinhalt`; Gates, Bash-Regeln,
Berichtsform und Prüfliste stehen im `summary` der Schritte und in `rules.md` (`[GATES]`, `[BASH]`, `[COMMIT]`, `[BERICHT]`,
`[REVIEW]`), nicht im Auftrag (`docs/prompt-template.md`, Messung `scripts/measure-prompts.py`). Ausgaben früherer Schritte
gehen über `export.as` und werden mit `$context.<name>` gelesen; ein Rücksprung ist ein `switch` mit `then`. **`rules.md`**
liegt unter `/srv/FreeSynergy/.scratch/api-runner/rules.md`; diese Datei gilt, im Repo gibt es keine Kopie. Die Regeln
`[EFFIZIENZ]` (ein Lauf = ein Gate-Zyklus, Gates einmal je Kopf, Reviews nur Delta, Mechanik als Skript, Abbruch-Schutz) stehen
als Schritt-Regeln in `agent-run` und `review`. Vor jeder Entscheidung sucht `scripts/adr-search` die vorhandenen ADRs
(`entscheidung`, `user-decision`).

**Die Flows.**
- Release und Bau: `release-flow` (lift, gates, Review, Tag, Push, CI, ff main, Release, bump, sign), `lift`, `catalog-bump`,
  `cleanup-targets`, `agent-run`, `bugfix`, `review`.
- Frontend und Bilder: `frontend-round`, `frontend-umbau` (Bestand auf die Schichten nach ADR 0074), `dossier`, `icon-round`,
  `picture-acceptance`.
- Fremdes: `upstream-anfrage` (ein git-bug-Issue an einen Upstream), `upstream-update` (neuer Upstream-Release in Spiegel und Katalog, Sicherheitsfixes zuerst), `wrapper-einbindung` (fremde Crate per Git-Pin
  unverändert, eigene Wrapper-Crate, Lizenz-Gate in der CI).
- Koordinator: `coordination-round`, `cost-reconcile`, `report-to-strategy`, `slot-check`, `wip-continue`, `download-readonly`,
  `publish-doc`, `workflow-pflege`.
- Strategie: `entscheidung` (erst ADR-Suche, dann Auswertung, Nutzerfrage nur bei Abweichung oder Lücke), `adr-write`,
  `user-decision`, `handoff-update`, `restart-handoff`.

**EN.** Our processes as visible workflows: Serverless Workflow 1.0 YAML (`dsl: 1.0.x`) in the FS profile (draft
`ENTWURF-workflow-standard.md`, version 3). Every step carries `freesynergy.net:actor` (`human` = Person, `ai` =
Application, `machine` = Service; plus `freesynergy.net:actor-type`) and `freesynergy.net:gate` (approval stop, e.g.
`opus-ja`, `human`, or `none`) and `freesynergy.net:kind` (`objective` = repeat until the goal is reached, e.g. tests
green; `subjective` = change the state whatever the result, e.g. show images, ask for a verdict). The files import 1:1 into
fs-workflows: Serverless Workflow 1.0 plus the FS profile, no special forms. All carry `document.version` = crate version
= tag (0.0.6). Agents never push, tag or merge. For now they are a description and are only checked
(`scripts/check_workflows.py`, test `crates/fs-release/tests/workflows.rs`; the step names of `lift` and `flow` must match
the code). In a second step `fs-release flow` will read these files and run the process from them. No secrets in these
files.

**Rule: every work step runs through a workflow.** If a step has none, the first step is to write or extend it
(`workflow-pflege`). An agent order points to a flow like this: `Workflow <name>, step <step>, parameters <key=value, ...>`,
e.g. `Workflow bugfix, step reproduce, parameters finding="..."`. The agent works only the named steps and names flow and
steps in its report. The list of flows is above (same names). The order stays compact: `Workflow <name>, step <id>, parameters
repo, branch, head, delta, content`; gates, bash rules, report form and check list are in the `summary` of the steps and in
`rules.md` (`[GATES]`, `[BASH]`, `[COMMIT]`, `[BERICHT]`, `[REVIEW]`), not in the order (`docs/prompt-template.md`, measured by
`scripts/measure-prompts.py`). Outputs of earlier steps go through `export.as` and are read as `$context.<name>`; a jump back
is a `switch` with `then`. **`rules.md`** is the file `/srv/FreeSynergy/.scratch/api-runner/rules.md`; that one applies, there
is no copy in the repo. The `[EFFIZIENZ]` rules (one run = one gate cycle, gates once per head, reviews read only the delta,
mechanics as a script, abort protection) are step rules in `agent-run` and `review`. Before any decision `scripts/adr-search`
looks up the existing ADRs (`entscheidung`, `user-decision`).

The `@fs` catalog (`call: <command>:<version>@fs`) is the fixed list of commands of the draft; it is not defined as a
`use.catalogs` entry yet, so these files are valid YAML in the profile's shape but not runnable by a stock SW runtime.
