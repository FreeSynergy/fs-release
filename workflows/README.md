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
- Frontend und Bilder: `frontend-round` (mit dem objektiven Schritt `menue-vergleich`, s. u.), `frontend-umbau` (Bestand auf die Schichten nach ADR 0074), `dossier`, `icon-round`,
  `picture-acceptance`.
- Neues Programm: `programm-neu` (17 Schritte in P0 bis P6, s. u.).
- Fremdes: `upstream-anfrage` (ein git-bug-Issue an einen Upstream), `wrapper-einbindung` (fremde Crate per Git-Pin
  unverändert, eigene Wrapper-Crate, Lizenz-Gate in der CI).
- Koordinator: `coordination-round`, `cost-reconcile`, `report-to-strategy`, `slot-check`, `wip-continue`, `download-readonly`,
  `publish-doc`, `workflow-pflege`.
- Strategie: `entscheidung` (erst ADR-Suche, dann Auswertung, Nutzerfrage nur bei Abweichung oder Lücke), `adr-write`,
  `user-decision`, `handoff-update`, `restart-handoff`.

**Menü-Vergleich (R2, Nutzerentscheidung 14.11.).** Menüs, Dialoge und Abläufe eines Frontends müssen denen des Vorbilds
gleich oder ähnlich sein (Erweiterungen später), nicht nur die Bilder. Der Schritt `menue-vergleich` in `frontend-round` (nach
`gates`, vor `acceptance`; rot springt zu `build`) lässt `scripts/menue-vergleich` (Python, ohne Netz, ohne KI) den Menübaum
und die Dialogliste des Programms (Dump `fs-menu-dump/1`) mit `## Menü-Inventar` / Dialogtabelle des Dossiers vergleichen.
Abweichungen (`fehlt`, `zusaetzlich`, `anders-benannt`, `kuerzel`) sind nur mit Begründung in der Ausnahmeliste des Frontends
erlaubt; ohne Begründung ist das Ergebnis rot. Gilt für alle Frontends der Welle 1. Format: `docs/menue-vergleich.md`,
Selbsttest `scripts/test_menue_vergleich.sh`.

**Baustein-Zuordnung (Nutzerentscheidung 14.11., für alle Frontends).** Verbindliche Reihenfolge: (1) Baustein-Analyse - jeder
Bereich des Vorbilds wird einer Baustein-Familie und einer Baustein-Id zugeordnet („was ist das?“: Tabelle, Baum, Liste, Karte,
Leiste, Dialog …), vorhandene Bausteine zuerst, ein neuer nur mit Begründung, warum kein vorhandener (mit Modul) passt; (2) Aussehen -
Stil-Analyse, Tokens und Stilregeln je Baustein (ADR 0074); (3) Zusammensetzen - die Anordnung im Frontend. Beispiel: die Datei-Tabelle
in fs-files ist dieselbe Tabelle wie später in fs-data, nur mit ausgeblendeten Köpfen und ohne Rechen-Modul (Formeln = Modul, Muster
von ADR 0084). Der objektive Prüfschritt `baustein-zuordnung` (in `dossier`, vor dem Veröffentlichen; rot springt zu `research`) lässt
`scripts/baustein-zuordnung` die Tabelle `## Baustein-Zuordnung` in `bausteine.md` prüfen und zählt die roten Runden selbst (Zustandsdatei im Dossier-Ordner, rc 3 ab der dritten) und meldet den Anteil je Quelle
(Familie, Blinc-Widget, neu; Reihenfolge Familie, Blinc, neu mit Begründung). Format: `docs/baustein-zuordnung.md`, Selbsttest `scripts/test_baustein_zuordnung.sh`.

**Neues Programm (`programm-neu`, Nutzerwunsch 14.11.).** Ein Ablauf von der Prüffrage bis zur Abnahme, 17 Schritte in sieben Phasen:
P0 (1 „wessen Daten?“, ADR 0076, Ausgang Programm / Facette / Modul / Suite; 2 Hub und Katalog, ADR 0068), P1 (3 Standards und ihre
Testsammlungen, ADR 0014; 4 Kern-Crate, Wrapper statt Fork; 5 Formate festlegen (Zeiten UTC plus Zonen-Id, Personen als Verweis, ADR 0085 Entwurf) und Umzugsweg), P2 (6 Datenmodell
auf Standards, Speichern ist nicht Exportieren, ADR 0046; 7 Themen-Schnittstelle, ADR 0045/0066; 8 Manifest, ADR 0068), P3 (9 Engine
mit Vertragstests; 9a Importer/Exporter als Grenz-Adapter nach ADR 0085 (Entwurf); 10 Admin-Frontend aus der Typbeschreibung), P4 (11 Vorbild-Dossier; 12 `frontend-round` mit Baustein-Zuordnung,
Stil, Zusammensetzen, `menue-vergleich`; 13 Sprache MF2 de+en und Barrierefreiheit), P5 (14 Hub-Anmeldung und `fs-server`-Adapter
nach ADR 0082; 15 Sicherheit, ADR 0054/0056/0067; 16 Pakete, Katalog ADR 0077, Doku; danach `release-flow`, danach der Serverlauf), P6 (17 Abnahme durch den
Nutzer, Befunde laufen durch `bugfix`). Jeder Agentenschritt (`objective`) hat danach einen objektiven Skript-Prüfschritt; rot
springt per `switch`/`then` zurück, höchstens zweimal: die dritte rote Runde (`runde` im Ergebnis des Prüfskripts, bei Grün innerhalb eines Kreises nicht zurückgesetzt) endet in `eskalation` beim Menschen. `frontend-round` läuft hier mit `release: false` (ein einziges Release, erst nach Sprache, Sicherheit und Paketen). Jeder Schritt nennt seine ADRs in `freesynergy.net:adr` und seine Phase in
`freesynergy.net:phase`. Ausgang Facette, Modul oder Suite verlässt den Ablauf an einem Stub (`facette-abzweig`, `modul-abzweig`,
`suite-abzweig`); **`facette-anlegen` und ein Modul-Ablauf gibt es noch nicht** und sind als TODO geführt, nicht erfunden. Auf echten
Servern arbeitet kein Agent: Schritt 14 läuft nur lokal gegen Testinstanzen, den echten Lauf macht der Mensch (`server-lauf`). Selbsttest:
`crates/fs-release/tests/workflows_programm_neu.rs` (Gegenproben: Schritt ohne `kind`, ohne ADR-Verweis, ohne Prüfschritt, Sprung ins Leere; Strukturregeln: kein toter Schritt, Rücksprung nur auf Agent oder Mensch, Eskalation nach drei roten Runden, Reihenfolge Sicherheit, Pakete, Release, Serverlauf, Eingaben der Unterabläufe vollständig).

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

**Menu comparison (R2, user decision 14 Nov).** Menus, dialogs and flows of a Frontend must be equal or similar to the model's
(extensions later), not only the pictures. The step `menue-vergleich` in `frontend-round` (after `gates`, before `acceptance`;
red jumps to `build`) lets `scripts/menue-vergleich` (Python, no network, no AI) compare the program's menu tree and dialog
list (dump `fs-menu-dump/1`) with the `## Menü-Inventar` list / dialogs table of the dossier. Deviations (`fehlt`,
`zusaetzlich`, `anders-benannt`, `kuerzel`) are allowed only with a reason in the Frontend's exception list; without one the
result is red. Applies to every Frontend of wave 1. Format: `docs/menue-vergleich.md`, self test
`scripts/test_menue_vergleich.sh`.

**Block assignment (user decision 14 Nov, all Frontends).** Fixed order: (1) block analysis - every area of the model is assigned to a
block family and block id ("what is this?": table, tree, list, card, bar, dialog ...), existing blocks first, a new one only with the
reason why no existing block (with a module) fits; (2) look - style analysis, tokens and style rules per block (ADR 0074); (3)
composition - the arrangement in the Frontend. Example: the file table of fs-files is the same table as later in fs-data, only with
hidden headers and no calculation module (formulas = a module, ADR 0084 pattern). The objective check `baustein-zuordnung` (in
`dossier`, before publish; red jumps to `research`) lets `scripts/baustein-zuordnung` check the `## Baustein-Zuordnung` table of
`bausteine.md` and reports the share of newly built blocks. Format: `docs/baustein-zuordnung.md`, self test
`scripts/test_baustein_zuordnung.sh`.

**New program (`programm-neu`, user request 14 Nov).** One flow from the test question to acceptance, 17 steps in seven phases:
P0 (1 "whose data?", ADR 0076, outcome Programm / Facette / Modul / Suite; 2 hub and catalog, ADR 0068), P1 (3 standards and their test
collections, ADR 0014; 4 core crate, wrapper not fork; 5 fix the formats (times UTC plus zone id, persons as references, ADR 0085 draft) and the way of moving house), P2 (6 data
model on the standards, saving is not exporting, ADR 0046; 7 topic interface, ADR 0045/0066; 8 manifest, ADR 0068), P3 (9 engine with
contract tests; 9a importers/exporters as border adapters per ADR 0085 (draft); 10 admin Frontend from the type description), P4 (11 model dossier; 12 `frontend-round` with block assignment, look,
composition, `menue-vergleich`; 13 language MF2 de+en and accessibility), P5 (14 hub registration and the `fs-server` adapter per ADR
0082; 15 security, ADR 0054/0056/0067; 16 packages, catalog ADR 0077, docs; then `release-flow`, then the server run), P6 (17 acceptance by the user, findings run
through `bugfix`). Every agent step (`objective`) is followed by an objective script check; red jumps back by `switch`/`then`, at most twice: the third red round (`runde` in the result of the check script, not reset by green inside a loop) ends in `eskalation` with the person. `frontend-round` runs here with `release: false` (one single release, only after language, security and packages). Every step names its
ADRs in `freesynergy.net:adr` and its phase in `freesynergy.net:phase`. The outcomes Facette, Modul and Suite leave the flow at a stub
(`facette-abzweig`, `modul-abzweig`, `suite-abzweig`); **`facette-anlegen` and a module flow do not exist yet** and are kept as a TODO,
not invented. No agent works on a real server: step 14 runs only locally against test instances, the real run is the person's
(`server-lauf`). Self test: `crates/fs-release/tests/workflows_programm_neu.rs` (counter-tests: a step without `kind`, without an ADR
reference, without a check step, a jump to nothing; structure rules: no dead step, jumps back land only on an agent or a person, escalation after three red rounds, order security, packages, release, server run, complete inputs of the sub-flows).
