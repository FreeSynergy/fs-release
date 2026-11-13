#!/usr/bin/env python3
"""Measure what the agent orders cost: mean prompt size and mean cost per kind of run (build, fix, rev, dossier).

usage: python3 -I measure-prompts.py [--prompts DIR] [--costs FILE]
defaults: /srv/FreeSynergy/.scratch/api-runner/prompts and /srv/FreeSynergy/.scratch/api-kosten.md

Prompts: `<kind>-<name>.txt`, the size is the byte count; kind is the file name prefix (`build-`, `fix-`, `rev-`,
`dossier-`; other prefixes are not counted; a `fix` word in the name makes it a fix). Costs: the markdown table `| Datum | Aufgabe | Modell | USD | Tokens | Ergebnis |`,
one row per run; the kind comes from the task (`rev-...` = rev, `dossier-...` = dossier, a task with `fix` as a word = fix,
anything else = build); rows without a figure (USD 0 and 0 tokens, an aborted run) are left out.
exit 0 with the table, 1 when a source is missing.
"""
import os
import re
import sys

KINDS = ("build", "fix", "rev", "dossier")
PROMPTS = "/srv/FreeSynergy/.scratch/api-runner/prompts"
COSTS = "/srv/FreeSynergy/.scratch/api-kosten.md"


def prompt_kind(file_name):
    """Kind of a prompt file; a `-fix` word anywhere in the name makes it a fix, like for the cost rows."""
    if file_name.split("-", 1)[0] not in KINDS:
        return None
    return task_kind(file_name.removesuffix(".txt"))


def task_kind(task):
    if task.startswith("rev-"):
        return "rev"
    if task.startswith("dossier-"):
        return "dossier"
    if re.search(r"(^|-)fix(-|$)", task):
        return "fix"
    return "build"


def read_sizes(folder):
    """Prompt sizes in bytes per kind."""
    sizes = {k: [] for k in KINDS}
    for name in sorted(os.listdir(folder)):
        kind = prompt_kind(name)
        if kind and name.endswith(".txt"):
            sizes[kind].append(os.path.getsize(os.path.join(folder, name)))
    return sizes


def read_costs(path):
    """(USD, tokens) per run, by kind; header, separator and rows without a figure are skipped."""
    runs = {k: [] for k in KINDS}
    with open(path, encoding="utf-8") as f:
        for line in f:
            cells = [c.strip() for c in line.split("|")]
            if len(cells) < 7 or not re.fullmatch(r"\d{4}-\d{2}-\d{2}", cells[1]):
                continue
            try:
                usd, tokens = float(cells[4]), int(cells[5])
            except ValueError:
                continue
            if usd == 0 and tokens == 0:
                continue
            runs[task_kind(cells[2])].append((usd, tokens))
    return runs


def mean(values):
    return sum(values) / len(values) if values else None


def cell(value, fmt):
    return "-" if value is None else format(value, fmt)


def table(sizes, runs):
    rows = ["| kind | prompts | mean prompt bytes | runs | mean USD | mean tokens |", "|---|---|---|---|---|---|"]
    for kind in KINDS:
        costs = runs[kind]
        rows.append(
            f"| {kind} | {len(sizes[kind])} | {cell(mean(sizes[kind]), '.0f')} | {len(costs)} "
            f"| {cell(mean([c[0] for c in costs]), '.4f')} | {cell(mean([c[1] for c in costs]), '.0f')} |"
        )
    return "\n".join(rows)


def main(argv):
    prompts, costs = PROMPTS, COSTS
    args = iter(argv[1:])
    for a in args:
        if a == "--prompts":
            prompts = next(args, prompts)
        elif a == "--costs":
            costs = next(args, costs)
        else:
            print(f"unknown argument {a!r}")
            return 1
    for path, check in ((prompts, os.path.isdir), (costs, os.path.isfile)):
        if not check(path):
            print(f"{path}: missing")
            return 1
    print(table(read_sizes(prompts), read_costs(costs)))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
