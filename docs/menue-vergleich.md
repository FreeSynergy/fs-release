# menue-vergleich

Step `menue-vergleich` of `workflows/frontend-round.yaml` (rule R2: menus, dialogs and flows of a Frontend are equal or
similar to the model's, extensions later). `scripts/menue-vergleich` compares, deterministically, without network and AI:

```
python3 -I scripts/menue-vergleich --dossier menus.md dialogs.md --dump dump.json [--ausnahmen ausnahmen.toml] [--json report.json]
```

rc 0 = every deviation has a reason (or there is none), rc 1 = deviation without a reason, rc 2 = bad input (missing file,
no inventory in the dossier, wrong dump schema, bad exception list; never green). Self test: `bash scripts/test_menue_vergleich.sh`.

## 1. Dossier (docs, `programs/.../frontends/<model>/latest/`)

The existing files are prose, so the script reads two machine-readable parts that the dossier research (`dossier.yaml`,
step `research`) writes:

- `menus.md`: a heading `## Menü-Inventar` (or `## Menu inventory`), below it a nested bullet list (menu, sub menu, entry; the
  indentation is the depth), until the next heading. Per line: `Label [Shortcut] {de: Name, id: stable.id}`; `[...]` and
  `{...}` are optional. Several shortcuts: `[Ctrl+O, F3]`. `{de: ..}` / `{en: ..}` are the other-language names of the label
  (translation table), `{id: ..}` ties the line to the program's id, so a renamed entry shows as `anders-benannt`.
- `dialogs.md`: the table (first column = dialog name, a trailing `(...)` is dropped, `{de: ..}` allowed) or a
  `## Dialog-Inventar` list.

```markdown
## Menü-Inventar
- File
  - Save as… [Ctrl+Shift+S] {de: Speichern unter, id: file.saveas}
  - Quit [Ctrl+Q]
```

## 2. Dump of the program (schema `fs-menu-dump/1`)

```json
{"schema": "fs-menu-dump/1", "frontend": "wiki",
 "menus": [{"id": "file", "label": "&File", "children": [
     {"id": "file.save", "label": "Save", "shortcut": "Ctrl+S", "enabled": true, "checked": false},
     {"separator": true, "label": ""}]}],
 "dialogs": [{"id": "delete", "label": "Delete confirmation"}]}
```

`label` is required, `id`, `shortcut`, `enabled`, `checked`, `children` are optional; separators are ignored. `enabled` and
`checked` are carried in the dump but not compared yet.

How a program writes the dump: a test of the program builds its window menus (`fs-components-app` `Menu` / `MenuEntry`:
`id`, `label`, `disabled`, `checked`, `children`) and writes them as this JSON into a file in a temp directory; the
dialog list comes from the program's dialog registry or a list in the test. State of the code (read, not changed):
`MenuEntry` has no shortcut field and no dump function exists yet, so today a dump has to add `shortcut` itself or the
dossier's shortcuts show as `kuerzel` deviations. The dump writer is a task for `fs-components-app`, not for this script.

## 3. Exception list (per Frontend)

TOML (or JSON `{"ausnahme": [...]}`): one `[[ausnahme]]` per deviation.

```toml
[[ausnahme]]
pfad = "Help > Tips"            # path like in the report; an entry also covers everything below it
art = "zusaetzlich"             # optional: fehlt | zusaetzlich | anders-benannt | kuerzel
grund = "Extension, see roadmap."   # required; empty = the deviation stays red
```

Dialogs have the path `Dialog > <name>`. Exceptions that match nothing are reported as warnings.

## 4. Comparison rules

Case, `&` accelerators, a trailing `…`/`...` and surplus spaces are ignored; shortcuts are normalized (`Strg` = `Ctrl`,
`Umschalt` = `Shift`, `Entf` = `Delete`, modifier order). An entry is matched by `id` first, then by label or a translation.
Report: text on stdout, `--json` for the machine form (`result`, `deviations[]` with `art`, `pfad`, `detail`, `grund`).
