# workflows/

**DE.** Hier stehen unsere Abläufe als sichtbare Workflows: Serverless-Workflow-1.0-YAML (`dsl: 1.0.x`) im FS-Profil
(Entwurf `ENTWURF-workflow-standard.md`, Fassung 3). Jeder Schritt trägt `freesynergy.net:actor` (`human` = Person, `ai` =
Application, `machine` = Service; dazu `freesynergy.net:actor-type`) und `freesynergy.net:gate` (Freigabe-Haltepunkt, z. B.
`opus-ja`, `human`, oder `none`) und `freesynergy.net:kind` (`objective` = wiederholen, bis das Ziel erreicht ist, z. B.
Tests grün; `subjective` = Zustand ändern, egal wie das Ergebnis ist, z. B. Bilder zeigen, Urteil einholen). Dateien:
`release-flow.yaml`, `lift.yaml`, `catalog-bump.yaml`, `cleanup-targets.yaml`, `agent-run.yaml`, `frontend-round.yaml`,
`bugfix.yaml`, `review.yaml`, `dossier.yaml`, `icon-round.yaml`. Alle tragen `document.version` = Crate-Version = Tag
(0.0.2). Agenten pushen, taggen und mergen nie. Heute sind sie Beschreibung und werden nur geprüft (`scripts/check_workflows.py`, Test
`crates/fs-release/tests/workflows.rs`; Schrittnamen von `lift` und `flow` müssen mit dem Code übereinstimmen). In einem
zweiten Schritt liest `fs-release flow` diese Dateien und führt den Ablauf daraus aus. Keine Geheimnisse in diese Dateien.

**EN.** Our processes as visible workflows: Serverless Workflow 1.0 YAML (`dsl: 1.0.x`) in the FS profile (draft
`ENTWURF-workflow-standard.md`, version 3). Every step carries `freesynergy.net:actor` (`human` = Person, `ai` =
Application, `machine` = Service; plus `freesynergy.net:actor-type`) and `freesynergy.net:gate` (approval stop, e.g.
`opus-ja`, `human`, or `none`) and `freesynergy.net:kind` (`objective` = repeat until the goal is reached, e.g. tests
green; `subjective` = change the state whatever the result, e.g. show images, ask for a verdict). Files:
`release-flow.yaml`, `lift.yaml`, `catalog-bump.yaml`, `cleanup-targets.yaml`, `agent-run.yaml`, `frontend-round.yaml`,
`bugfix.yaml`, `review.yaml`, `dossier.yaml`, `icon-round.yaml`. All carry `document.version` = crate version = tag
(0.0.2). Agents never push, tag or merge. For now they are a description and are only checked (`scripts/check_workflows.py`, test
`crates/fs-release/tests/workflows.rs`; the step names of `lift` and `flow` must match the code). In a second step
`fs-release flow` will read these files and run the process from them. No secrets in these files.

The `@fs` catalog (`call: <command>:<version>@fs`) is the fixed list of commands of the draft; it is not defined as a
`use.catalogs` entry yet, so these files are valid YAML in the profile's shape but not runnable by a stock SW runtime.
