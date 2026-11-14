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
- Fremdes: `upstream-anfrage` (ein git-bug-Issue an einen Upstream), `wrapper-einbindung` (fremde Crate per Git-Pin
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

## Events / Ereignisse

**DE.** Workflows und CI-Läufe wecken einander über Ereignisse nach CloudEvents (Pflichtfelder `id`, `source`, `specversion`,
`type`). Hülle: `type` = `net.freesynergy.<gegenstand>.<ereignis>`; `source` = Repo-URL des Senders; `subject` = Tag oder Name;
`time` RFC 3339; `data` klein (Namen, Zahlen, Tags), nie Geheimnisse, nie Befehle. Ereignisse sind Weckrufe, die Entscheidung steht
im Repo-Zustand, nicht im Ereignis. **Grundregel: keine Freigabe-Umgehung. Automatisch laufen nur Folgen bereits freigegebener
Schritte** (Gate `none` oder schon erfüllt); kein Ereignis löst Tag, Push, Veröffentlichung, ADR-Push oder Löschen aus. Der
Zeitplan von `bump.yml` (alle 15 Minuten) bleibt als Sicherheitsnetz; es gibt keinen Token zwischen Repos (Pull, Q2b in DESIGN.md).

**EN.** Workflows and CI runs wake each other by events in the CloudEvents form (required `id`, `source`, `specversion`, `type`).
Envelope: `type` = `net.freesynergy.<subject>.<event>`; `source` = repo URL of the sender; `subject` = tag or name; `time`
RFC 3339; `data` small (names, numbers, tags), never secrets, never commands. Events are wake-up calls; the decision is in the
repo state, not in the event. **Basic rule: no bypass of an approval. Only follow-ups of steps that are already approved run
automatically** (gate `none` or already met); no event triggers a tag, push, publication, ADR push or deletion. The 15-minute
schedule of `bump.yml` stays as the safety net; there is no token between repos (pull, Q2b in DESIGN.md).

| type | sender | receiver | data | approval rule |
|---|---|---|---|---|
| `net.freesynergy.release.published` | last job of `release.yml` in the program repo, after `publish` was green (a draft sends nothing); `release-flow` step `announce` | `catalog-bump` (`schedule.on`), `release-flow` step `release` (`listen`) | `repo`, `tag`, `prerelease` (yes/no) | automatic: follow-up of the approved push (the opus-ja and picture OK lay before the tag) |
| `net.freesynergy.catalog.signed` | `sign.yml` of fs-catalog after the push | `release-flow` step `sign` (`listen`) | `commit` | automatic |
| `net.freesynergy.catalog.tested` | check like a user (a task in fs-catalog: in the same `sign.yml` run, because a push with the run token starts no other workflow) | `release-flow` step `sign` | `ok` | automatic; `ok` false is red |
| `net.freesynergy.catalog.changed` | candidate: `sign.yml` after a green check | candidate: rebuild of fs-packages (pull by schedule, later) | `commit` | automatic only if nothing new becomes public; the first publication of a package repo is a gate |
| `net.freesynergy.text.published` | candidate: text/docs repo on a push to main | candidate: docs build | `repo`, `commit` | build into the preview automatic; going public only with an approved source |
| `net.freesynergy.run.failed` | any workflow when red (`_notify-red.yml`, issue `rot: <workflow> <repo>`, label `rot`; closes at the next green) | the coordinator, by one query: `gh search issues --owner FreeSynergy --label rot --state open` | `workflow`, `repo`, `run_url`, `step` | always allowed (a notice only); there is no status board |
| `net.freesynergy.approval` | the Opus approval file | `release-flow` step `review` (`listen`) | head | stays a gate |
| `net.freesynergy.ci.green` | CI of the pushed branch | `release-flow` step `ci` (`listen`) | head | not a gate, only the wait |

**Mapping to Serverless Workflow 1.0 / what is proven.** Send = `emit.event.with` (`source`, `type`, `subject`, `data`; `id` is made by the
runtime) - in the spec (dsl-reference, `emit`), also in v1.0.0. Start by event = top-level `schedule.on` (`one`/`any` with `with.type`) -
in the spec. Wait in the flow = `listen.to` (`one`/`any`/`all`) - in the spec. NOT proven: `schedule.read` in 1.0.x (not set, the default
`data` is used), and whether `every` and `on` may stand together (so the 15-minute net is written in the comment of `catalog-bump.yaml`,
not as a second trigger). The spec defines no transport: how an event really arrives is the implementation. Today: `release.published`
reaches fs-catalog as the schedule of `bump.yml` (pull), `run.failed` as an issue; `listen` steps only describe the wait (the flow stops,
the continuation is a new run). The check (`scripts/check_workflows.py`) requires that every type is `net.freesynergy.*` and is in
this table, and that `coordination-round` starts with `rot-pruefen`.

**Rule: the coordinator acts only on red or at approval gates.** Silence on green. Follow-ups after an approval (bump, sign, catalog
check, package repos, docs build) are never started by hand; they run by event/CI. Red = `bugfix` or `agent-run` for exactly that run.

**Gates that never run automatically / Gates, die nie automatisch laufen:** publication (tag, Pre-release, release), the tag push
itself, the Opus approval (`opus-ja`) for exactly one head, fast-forward of main (`human`), ADR push, deleting (repos, tags, releases,
files in foreign repos), the Immutable-Releases switch, making a new package repo public for the first time, creating or rotating
secrets and tokens, any deviation from the rules. DE: Veröffentlichung, Tag-Push, Opus-JA für genau einen Kopf, ff main, ADR-Push,
Löschen, Immutable-Schalter, erstes Veröffentlichen eines Paketrepos, Geheimnisse anlegen/drehen, jede Regelabweichung.

The `@fs` catalog (`call: <command>:<version>@fs`) is the fixed list of commands of the draft; it is not defined as a
`use.catalogs` entry yet, so these files are valid YAML in the profile's shape but not runnable by a stock SW runtime.
