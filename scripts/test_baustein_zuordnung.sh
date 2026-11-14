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
printf 'dialog\ntabs\n# comment\n' >"$tmp/blinc.txt"
head='# Fixture\n\n## Baustein-Zuordnung\n\n| Bereich | Was ist das? | Baustein-Id | Module | Quelle | Begründung |\n|---|---|---|---|---|---|\n'
n=0
expect() { # expect <rc> <needle> <name> <rows>; every case in a fresh folder, so the round counter starts at 0
  local out rc
  n=$((n+1)); mkdir -p "$tmp/c$n"
  printf "$head$4" >"$tmp/c$n/d.md"
  out="$("${tool[@]}" --dossier "$tmp/c$n/d.md" --bestand "$tmp/bestand.txt" --blinc "$tmp/blinc.txt" 2>&1)"; rc=$?
  if [ "$rc" -ne "$1" ] || ! grep -qF -- "$2" <<<"$out"; then
    echo "FAIL $3: rc=$rc (want $1), needle '$2'"; printf '    %s\n' "$out"; fails=$((fails+1))
  else echo "ok   $3"; fi
}
why='No family block and no Blinc widget draws bars on a time axis, no module can add that'
expect 0 "sources familie 2 (100%), blinc 0 (0%), neu 0 (0%)" green "| File table | table | table | no row heads | familie | |\n| Folders | tree | tree | - | familie | |\n"
expect 0 "blinc 1 (33%)" "Blinc widget accepted, shares per source reported" "| Folders | tree | tree | - | familie | |\n| Dialog | dialog | dialog | - | blinc | |\n| Gantt | timeline | gantt | - | neu | $why |\n"
expect 0 "new blocks 1 (50%)" "new block with a reason that names family and Blinc" "| Folders | tree | tree | - | familie | |\n| Gantt | timeline | gantt | - | neu | $why |\n"
expect 1 "ohne-begruendung" "counter: new block without reason" "| Gantt | timeline | gantt | - | neu | |\n"
expect 1 "begruendung-unvollstaendig" "counter: reason names only the family" "| Gantt | timeline | gantt | - | neu | No family block draws bars on a time axis at all |\n"
expect 1 "begruendung-unvollstaendig" "counter: reason names only Blinc" "| Gantt | timeline | gantt | - | neu | No Blinc widget draws bars on a time axis at all |\n"
expect 1 "schon-da" "counter: new block that exists in the family" "| Sheet | table | table | formulas | neu | $why |\n"
expect 1 "schon-da" "counter: new block that exists as Blinc widget" "| Dlg | dialog | dialog | - | neu | $why |\n"
expect 1 "familie-zuerst" "counter: Blinc widget although the family has the block" "| Sheet | table | table | - | blinc | |\n"
expect 1 "unbekannt" "counter: unknown id marked as family" "| Gantt | timeline | gantt | - | familie | |\n"
expect 1 "unbekannt" "counter: Blinc widget that is not in the Blinc inventory" "| Gantt | timeline | gantt | - | blinc | |\n"
expect 1 "unbekannt" "counter: Blinc id marked as family" "| Dlg | dialog | dialog | - | familie | |\n"
expect 1 "ohne-baustein" "counter: area without block id" "| Footer | bar | | - | familie | |\n"
expect 1 "quelle-unklar" "counter: Quelle column unclear" "| Folders | tree | tree | - | vielleicht | |\n"
expect 1 "quelle-unklar" "counter: old column value ja/nein is no source" "| Folders | tree | tree | - | nein | |\n"
printf '# no table here\n' >"$tmp/d.md"
out="$("${tool[@]}" --dossier "$tmp/d.md" --bestand "$tmp/bestand.txt" --blinc "$tmp/blinc.txt" 2>&1)"; rc=$?
if [ "$rc" -eq 2 ]; then echo "ok   no table = rc 2"; else echo "FAIL no table: rc=$rc"; fails=$((fails+1)); fi
"${tool[@]}" --dossier "$tmp/none.md" --bestand "$tmp/bestand.txt" --blinc "$tmp/blinc.txt" >/dev/null 2>&1; rc=$?
if [ "$rc" -eq 2 ]; then echo "ok   missing file = rc 2"; else echo "FAIL missing file: rc=$rc"; fails=$((fails+1)); fi
rc2() { # rc2 <name> <dossier> <bestand> [blinc]
  "${tool[@]}" --dossier "$2" --bestand "$3" --blinc "${4:-$tmp/blinc.txt}" >/dev/null 2>&1; rc=$?
  if [ "$rc" -eq 2 ]; then echo "ok   $1 = rc 2"; else echo "FAIL $1: rc=$rc (want 2)"; fails=$((fails+1)); fi
}
printf "$head| Folders | tree | tree | - | familie | |\n" >"$tmp/ok.md"
printf '\377\376 not utf-8\n' >"$tmp/bad-utf8.md"
rc2 "dossier not UTF-8" "$tmp/bad-utf8.md" "$tmp/bestand.txt"
printf '\377\376' >"$tmp/bad-utf8.txt"
rc2 "inventory not UTF-8" "$tmp/ok.md" "$tmp/bad-utf8.txt"
rc2 "Blinc inventory not UTF-8" "$tmp/ok.md" "$tmp/bestand.txt" "$tmp/bad-utf8.txt"
printf '# only a comment\n' >"$tmp/empty.txt"; rc2 "Blinc inventory empty" "$tmp/ok.md" "$tmp/bestand.txt" "$tmp/empty.txt"
printf '## Baustein-Zuordnung\n\n| Bereich | Was ist das? | Baustein-Id | Module | Neu | Begründung |\n|---|---|---|---|---|---|\n| a | b | table | - | nein | |\n' >"$tmp/oldhead.md"
rc2 "old header with Neu instead of Quelle" "$tmp/oldhead.md" "$tmp/bestand.txt"
printf '## Baustein-Zuordnung\n\n| Foo | Bar |\n|---|---|\n| a | b |\n' >"$tmp/header.md"
rc2 "wrong table header" "$tmp/header.md" "$tmp/bestand.txt"
printf '{"a": 1}' >"$tmp/obj.json"; rc2 "inventory JSON object" "$tmp/ok.md" "$tmp/obj.json"
printf '[1, 2]' >"$tmp/nums.json"; rc2 "inventory JSON numbers" "$tmp/ok.md" "$tmp/nums.json"
printf '[]' >"$tmp/empty.json"; rc2 "inventory empty list" "$tmp/ok.md" "$tmp/empty.json"
# round counter: red, red, red = runde 1, 2, 3 and rc 1, 1, 3; green resets; a bad input does not count
mkdir -p "$tmp/r"; printf "$head| Gantt | timeline | gantt | - | familie | |\n" >"$tmp/r/d.md"; cp "$tmp/ok.md" "$tmp/ok-green.md"
round() { # round <want rc> <want runde> <name> <dossier file to copy>
  cp "$4" "$tmp/r/d.md"
  out="$("${tool[@]}" --dossier "$tmp/r/d.md" --bestand "$tmp/bestand.txt" --blinc "$tmp/blinc.txt" 2>&1)"; rc=$?
  if [ "$rc" -eq "$1" ] && grep -qxF "runde=$2" <<<"$out"; then echo "ok   $3"; else echo "FAIL $3: rc=$rc (want $1), runde '$2'"; printf '    %s\n' "$out"; fails=$((fails+1)); fi
}
printf "$head| Gantt | timeline | gantt | - | familie | |\n" >"$tmp/red.md"
round 1 1 "counter: first red round = runde 1, rc 1" "$tmp/red.md"
round 1 2 "second red round = runde 2, rc 1" "$tmp/red.md"
round 3 3 "third red round = runde 3, rc 3 (escalate)" "$tmp/red.md"
round 3 4 "fourth red round stays rc 3" "$tmp/red.md"
round 0 0 "green resets the counter" "$tmp/ok-green.md"
round 1 1 "red after green starts again at runde 1" "$tmp/red.md"
printf 'broken' >"$tmp/r/bad.md"; cp "$tmp/r/bad.md" "$tmp/r/d.md"
"${tool[@]}" --dossier "$tmp/r/d.md" --bestand "$tmp/bestand.txt" --blinc "$tmp/blinc.txt" >/dev/null 2>&1
round 1 2 "bad input (rc 2) does not touch the counter" "$tmp/red.md"
[ "$fails" -eq 0 ] && echo "all green" || { echo "$fails failed"; exit 1; }
