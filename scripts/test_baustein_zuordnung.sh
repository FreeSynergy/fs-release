#!/usr/bin/env bash
# Self test of scripts/baustein-zuordnung: green case, and counter-checks (each defect must really turn it red).
#   bash scripts/test_baustein_zuordnung.sh
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
tool=(python3 -I "$here/baustein-zuordnung")
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
fails=0
printf 'table\nlist\ntree\ntoolbar\n# comment\n' >"$tmp/bestand.txt"
head='# Fixture\n\n## Baustein-Zuordnung\n\n| Bereich | Was ist das? | Baustein-Id | Module | Neu | Begründung |\n|---|---|---|---|---|---|\n'
expect() { # expect <rc> <needle> <name> <rows>
  local out rc
  printf "$head$4" >"$tmp/d.md"
  out="$("${tool[@]}" --dossier "$tmp/d.md" --bestand "$tmp/bestand.txt" 2>&1)"; rc=$?
  if [ "$rc" -ne "$1" ] || ! grep -qF -- "$2" <<<"$out"; then
    echo "FAIL $3: rc=$rc (want $1), needle '$2'"; printf '    %s\n' "$out"; fails=$((fails+1))
  else echo "ok   $3"; fi
}
expect 0 "new blocks 0 (0%)" green "| File table | table | table | no row heads, no formulas | nein | |\n| Folders | tree | tree | - | nein | |\n"
expect 0 "new blocks 1 (50%)" "new block with reason, share reported" "| Folders | tree | tree | - | nein | |\n| Gantt | timeline | gantt | - | ja | no existing block draws bars on a time axis, no module can add that |\n"
expect 1 "ohne-begruendung" "counter: new block without reason" "| Gantt | timeline | gantt | - | ja | |\n"
expect 1 "schon-da" "counter: new block that exists" "| Sheet | table | table | formulas | ja | we want our own table for the sheet |\n"
expect 1 "unbekannt" "counter: unknown id marked as existing" "| Gantt | timeline | gantt | - | nein | |\n"
expect 1 "ohne-baustein" "counter: area without block id" "| Footer | bar | | - | nein | |\n"
expect 1 "neu-unklar" "counter: Neu column unclear" "| Folders | tree | tree | - | vielleicht | |\n"
printf '# no table here\n' >"$tmp/d.md"
out="$("${tool[@]}" --dossier "$tmp/d.md" --bestand "$tmp/bestand.txt" 2>&1)"; rc=$?
if [ "$rc" -eq 2 ]; then echo "ok   no table = rc 2"; else echo "FAIL no table: rc=$rc"; fails=$((fails+1)); fi
"${tool[@]}" --dossier "$tmp/none.md" --bestand "$tmp/bestand.txt" >/dev/null 2>&1; rc=$?
if [ "$rc" -eq 2 ]; then echo "ok   missing file = rc 2"; else echo "FAIL missing file: rc=$rc"; fails=$((fails+1)); fi
[ "$fails" -eq 0 ] && echo "all green" || { echo "$fails failed"; exit 1; }
