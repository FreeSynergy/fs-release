#!/usr/bin/env bash
# Self test of scripts/menue-vergleich with small fixtures: equal, missing, extra (with / without reason), renamed, shortcut,
# empty and broken inputs, plus counter-checks (the script must really go red / red must really need a reason).
#   bash scripts/test_menue_vergleich.sh
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
tool=(python3 -I "$here/menue-vergleich")
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
fails=0

expect() { # expect <rc> <text-that-must-appear or -> <name> -- args...
  local want="$1" needle="$2" name="$3"
  shift 4
  local out rc
  out="$("${tool[@]}" "$@" 2>&1)"
  rc=$?
  if [ "$rc" -ne "$want" ] || { [ "$needle" != "-" ] && ! grep -qF -- "$needle" <<<"$out"; }; then
    echo "FAIL $name: rc=$rc (want $want), needle '$needle'"
    printf "    %s\n" "${out//$'\n'/$'\n    '}"
    fails=$((fails + 1))
  else
    echo "ok   $name"
  fi
}

cat >"$tmp/menus.md" <<'EOT'
# Fixture - Menus

Prose is ignored.

## Menü-Inventar

- File
  - Open [Ctrl+O]
  - Save as… [Ctrl+Shift+S] {de: Speichern unter}
  - Quit [Ctrl+Q] {id: fs.quit}
- Edit
  - Copy [Strg+C]
- Help
  - About

## Other heading

- not part of the inventory
EOT
cat >"$tmp/dialogs.md" <<'EOT'
# Fixture - Dialogs

| Dialog | Purpose | Mark |
|--------|---------|------|
| Delete confirmation | ask | ok |
| Settings (General, Advanced) | options | ok |
EOT
cat >"$tmp/dump.json" <<'EOT'
{"schema": "fs-menu-dump/1", "frontend": "fixture",
 "menus": [
  {"id": "file", "label": "&File", "children": [
    {"id": "file.open", "label": "Open", "shortcut": "Strg+O"},
    {"separator": true, "label": ""},
    {"id": "file.saveas", "label": "SAVE AS...", "shortcut": "Shift+Ctrl+S"},
    {"id": "fs.quit", "label": "&Quit", "shortcut": "Ctrl+Q"}]},
  {"id": "edit", "label": "Edit", "children": [{"id": "edit.copy", "label": "Copy", "shortcut": "Ctrl+C"}]},
  {"id": "help", "label": "Help", "children": [{"id": "help.about", "label": "About", "enabled": false}]}],
 "dialogs": [{"id": "delete", "label": "Delete confirmation"}, {"id": "settings", "label": "Settings"}]}
EOT
D=(--dossier "$tmp/menus.md" "$tmp/dialogs.md")

# 1 equal (normalization: & accelerator, case, ellipsis, Strg = Ctrl, modifier order, separator ignored, translation, id)
expect 0 "green" "equal" -- "${D[@]}" --dump "$tmp/dump.json"

# 2 missing entry in the program: red, and the same run is green with a reasoned exception (counter-check)
python3 -I - "$tmp/dump.json" "$tmp/miss.json" <<'EOT'
import json, sys
d = json.load(open(sys.argv[1]))
d["menus"][1]["children"] = []
json.dump(d, open(sys.argv[2], "w"))
EOT
expect 1 "fehlt" "missing entry is red" -- "${D[@]}" --dump "$tmp/miss.json"
expect 1 "OHNE GRUND" "missing entry names the deviation" -- "${D[@]}" --dump "$tmp/miss.json"
cat >"$tmp/ok.toml" <<'EOT'
[[ausnahme]]
pfad = "Edit > Copy"
art = "fehlt"
grund = "The program has Copy only in the context menu, decided in ADR 9999."
EOT
expect 0 "Grund: The program has Copy" "missing entry with a reason is green" -- "${D[@]}" --dump "$tmp/miss.json" --ausnahmen "$tmp/ok.toml"
printf '[[ausnahme]]\npfad = "Edit > Copy"\ngrund = ""\n' >"$tmp/empty-reason.toml"
expect 1 "without a reason" "empty reason stays red" -- "${D[@]}" --dump "$tmp/miss.json" --ausnahmen "$tmp/empty-reason.toml"
printf '[[ausnahme]]\npfad = "Edit > Paste"\ngrund = "x"\n' >"$tmp/wrong-path.toml"
expect 1 "fehlt" "exception on another path does not help" -- "${D[@]}" --dump "$tmp/miss.json" --ausnahmen "$tmp/wrong-path.toml"
printf '[[ausnahme]]\npfad = "Edit"\nart = "zusaetzlich"\ngrund = "x"\n' >"$tmp/wrong-kind.toml"
expect 1 "fehlt" "exception of another kind does not help" -- "${D[@]}" --dump "$tmp/miss.json" --ausnahmen "$tmp/wrong-kind.toml"
printf '[[ausnahme]]\npfad = "edit"\ngrund = "whole menu is later"\n' >"$tmp/parent.toml"
expect 0 "-" "exception on the parent covers the children" -- "${D[@]}" --dump "$tmp/miss.json" --ausnahmen "$tmp/parent.toml"
printf '{"ausnahme": [{"pfad": "Edit > Copy", "grund": "json form"}]}' >"$tmp/ok.json"
expect 0 "json form" "exception list as JSON" -- "${D[@]}" --dump "$tmp/miss.json" --ausnahmen "$tmp/ok.json"

# 3 extra entry in the program (extension): red without, green with a reason
python3 -I - "$tmp/dump.json" "$tmp/extra.json" <<'EOT'
import json, sys
d = json.load(open(sys.argv[1]))
d["menus"][2]["children"].append({"id": "help.tips", "label": "Tips"})
d["dialogs"].append({"id": "wizard", "label": "Wizard"})
json.dump(d, open(sys.argv[2], "w"))
EOT
expect 1 "zusaetzlich" "extra entry without a reason is red" -- "${D[@]}" --dump "$tmp/extra.json"
cat >"$tmp/extra.toml" <<'EOT'
[[ausnahme]]
pfad = "Help > Tips"
grund = "Extension, planned in the roadmap."

[[ausnahme]]
pfad = "Dialog > Wizard"
grund = "Extension: first-start wizard."
EOT
expect 0 "Extension, planned" "extra entry with a reason is green" -- "${D[@]}" --dump "$tmp/extra.json" --ausnahmen "$tmp/extra.toml"
expect 0 "matches nothing" "unused exception is only a warning" -- "${D[@]}" --dump "$tmp/dump.json" --ausnahmen "$tmp/extra.toml"

# 4 renamed (same id, other label) and shortcut differs
python3 -I - "$tmp/dump.json" "$tmp/renamed.json" <<'EOT'
import json, sys
d = json.load(open(sys.argv[1]))
d["menus"][0]["children"][2]["label"] = "Save a copy"
d["menus"][0]["children"][2]["id"] = "file.saveas"
d["menus"][0]["children"][0]["shortcut"] = "Ctrl+P"
d["menus"][1]["children"][0].pop("shortcut")
json.dump(d, open(sys.argv[2], "w"))
EOT
python3 -I - "$tmp/menus.md" <<'EOT'
import sys
p = sys.argv[1]
s = open(p).read().replace("{de: Speichern unter}", "{de: Speichern unter, id: file.saveas}")
open(p, "w").write(s)
EOT
expect 1 "anders-benannt  File > Save as…" "renamed" -- "${D[@]}" --dump "$tmp/renamed.json"
expect 1 "kuerzel         File > Open - dossier ctrl+o, program ctrl+p" "shortcut differs" -- "${D[@]}" --dump "$tmp/renamed.json"
expect 1 "kuerzel         Edit > Copy - dossier ctrl+c, program none" "shortcut missing in the program" -- "${D[@]}" --dump "$tmp/renamed.json"
expect 0 "-" "equal again with id in the dossier" -- "${D[@]}" --dump "$tmp/dump.json"

# 5 JSON report
expect 1 "-" "json report written" -- "${D[@]}" --dump "$tmp/renamed.json" --json "$tmp/out.json"
python3 -I - "$tmp/out.json" <<'EOT' || { echo "FAIL json content"; fails=$((fails + 1)); }
import json, sys
r = json.load(open(sys.argv[1]))
assert r["result"] == "red" and r["unexplained"] == 3 and {d["art"] for d in r["deviations"]} == {"anders-benannt", "kuerzel"}, r
EOT

# 6 empty and broken inputs: rc 2, never green
printf '# nothing\n' >"$tmp/none.md"
expect 2 "no \`Menü-Inventar\`" "dossier without inventory" -- --dossier "$tmp/none.md" --dump "$tmp/dump.json"
printf '## Menü-Inventar\n\n' >"$tmp/emptyinv.md"
expect 2 "nothing to compare" "empty inventory" -- --dossier "$tmp/emptyinv.md" --dump "$tmp/dump.json"
expect 2 "dossier" "dossier file missing" -- --dossier "$tmp/nope.md" --dump "$tmp/dump.json"
expect 2 "dump" "dump file missing" -- "${D[@]}" --dump "$tmp/nope.json"
printf '{' >"$tmp/bad.json"
expect 2 "not JSON" "dump is not JSON" -- "${D[@]}" --dump "$tmp/bad.json"
printf '{"schema": "other", "menus": [], "dialogs": []}' >"$tmp/schema.json"
expect 2 "schema" "dump with a wrong schema" -- "${D[@]}" --dump "$tmp/schema.json"
printf '{"schema": "fs-menu-dump/1", "menus": [{"id": "x"}], "dialogs": []}' >"$tmp/nolabel.json"
expect 2 "label" "dump entry without label" -- "${D[@]}" --dump "$tmp/nolabel.json"
printf '{"schema": "fs-menu-dump/1", "menus": [], "dialogs": []}' >"$tmp/emptydump.json"
expect 1 "fehlt" "empty dump: everything is missing, red" -- "${D[@]}" --dump "$tmp/emptydump.json"
expect 2 "exceptions" "exception list broken" -- "${D[@]}" --dump "$tmp/dump.json" --ausnahmen "$tmp/bad.json"
printf '[[ausnahme]]\ngrund = "x"\n' >"$tmp/nopath.toml"
expect 2 "pfad" "exception without a path" -- "${D[@]}" --dump "$tmp/dump.json" --ausnahmen "$tmp/nopath.toml"
printf '[[ausnahme]]\npfad = "a"\nart = "foo"\ngrund = "x"\n' >"$tmp/badkind.toml"
expect 2 "art" "exception with an unknown kind" -- "${D[@]}" --dump "$tmp/dump.json" --ausnahmen "$tmp/badkind.toml"
expect 2 "-" "no arguments" --

if [ "$fails" -ne 0 ]; then
  echo "menue-vergleich self test FAILED: $fails"
  exit 1
fi
echo "menue-vergleich self test green"
