#!/usr/bin/env python3
"""Schema check of the workflow files in `workflows/` (Serverless Workflow 1.0 YAML, FS profile).

usage: python3 -I check_workflows.py <folder with the workflow files> [--simulate-missing-yaml]
exit 0 green, 1 red (every problem on its own line `file: reason`), 2 skipped (no PyYAML: nothing was checked).

Per file: parses; `document` has dsl (1.0.x), namespace, name, version; `do` is a non-empty list; every step is one
`name: task` pair with a task type, a unique name, `freesynergy.net:actor` (human | ai | machine), the matching
`freesynergy.net:actor-type` (Person | Application | Service, the ActivityStreams types) and `freesynergy.net:gate` and `freesynergy.net:kind` (objective = repeat until the goal is reached; subjective = change the
state whatever the result, e.g. show images, ask for a verdict). A `then` (task or `switch` case) is continue, exit, end
or the name of a step of the same flow.
"""
import glob
import os
import re
import sys

ACTORS = {"human": "Person", "ai": "Application", "machine": "Service"}
TASK_TYPES = {"call", "do", "emit", "for", "fork", "listen", "raise", "run", "set", "switch", "try", "wait"}
KINDS = ("objective", "subjective")
FLOW_WORDS = {"continue", "exit", "end"}
ACTOR, ATYPE, GATE = "freesynergy.net:actor", "freesynergy.net:actor-type", "freesynergy.net:gate"
KIND = "freesynergy.net:kind"


def check_step(item, seen, where):
    if not isinstance(item, dict) or len(item) != 1:
        return [f"{where}: a step is exactly one `name: task` pair"]
    ((name, task),) = item.items()
    problems = []
    if name in seen:
        problems.append(f"{where}: duplicate step name {name!r}")
    seen.add(name)
    where = f"{where} step {name!r}"
    if not isinstance(task, dict):
        return problems + [f"{where}: the task is not a mapping"]
    if not TASK_TYPES & set(task):
        problems.append(f"{where}: no task type (one of {', '.join(sorted(TASK_TYPES))})")
    meta = task.get("metadata")
    if not isinstance(meta, dict):
        return problems + [f"{where}: no metadata (actor and gate are required)"]
    actor = meta.get(ACTOR)
    if actor not in ACTORS:
        problems.append(f"{where}: {ACTOR} is {actor!r}, allowed: {', '.join(ACTORS)}")
    elif meta.get(ATYPE) != ACTORS[actor]:
        problems.append(f"{where}: {ATYPE} must be {ACTORS[actor]} for actor {actor}")
    if not isinstance(meta.get(GATE), str) or not meta[GATE].strip():
        problems.append(f"{where}: {GATE} is missing (a name, or `none`)")
    if meta.get(KIND) not in KINDS:
        problems.append(f"{where}: {KIND} is {meta.get(KIND)!r}, allowed: {', '.join(KINDS)}")
    return problems


def check_doc(doc):
    if not isinstance(doc, dict):
        return ["top level is not a mapping"]
    problems = []
    head = doc.get("document")
    if not isinstance(head, dict):
        problems.append("no `document` block")
    else:
        for key in ("dsl", "namespace", "name", "version"):
            if not isinstance(head.get(key), str) or not head[key].strip():
                problems.append(f"document.{key} is missing (a non-empty string)")
        if isinstance(head.get("dsl"), str) and not re.fullmatch(r"1\.0\.\d+", head["dsl"]):
            problems.append(f"document.dsl is {head['dsl']!r}, expected 1.0.x")
    steps = doc.get("do")
    if not isinstance(steps, list) or not steps:
        return problems + ["`do` is not a non-empty list"]
    seen = set()
    for i, item in enumerate(steps):
        problems += check_step(item, seen, f"do[{i}]")
    return problems + check_then(steps, seen)


def check_then(steps, names):
    """Every flow directive `then` (on a task or in a `switch` case) is continue, exit, end or the name of a step."""
    problems = []
    for item in steps:
        if not isinstance(item, dict) or len(item) != 1:
            continue
        ((name, task),) = item.items()
        if not isinstance(task, dict):
            continue
        targets = [task.get("then")]
        if isinstance(task.get("switch"), list):
            for case in task["switch"]:
                if isinstance(case, dict):
                    targets += [c.get("then") for c in case.values() if isinstance(c, dict)]
        for target in targets:
            if target is not None and target not in FLOW_WORDS and target not in names:
                problems.append(f"step {name!r}: `then` names {target!r}, which is no step of this flow")
    return problems


def event_types(doc):
    """(type, is_emit) of every `emit` step and every `listen` / `schedule.on` event of a flow."""
    found = []

    def walk(node, emit):
        if isinstance(node, dict):
            for key, val in node.items():
                if key == "type" and isinstance(val, str) and val.startswith("net."):
                    found.append((val, emit))
                walk(val, emit or key == "emit")
        elif isinstance(node, list):
            for val in node:
                walk(val, emit)

    walk(doc.get("do"), False)
    walk(doc.get("schedule"), False)
    return found


def check_events(doc, name, readme):
    """Event rules: every type is `net.freesynergy.*` and in the README catalog; the coordinator starts with `rot-pruefen`."""
    problems = []
    for typ, _ in event_types(doc):
        if not typ.startswith("net.freesynergy."):
            problems.append(f"event type {typ!r} does not start with net.freesynergy.")
        elif readme is not None and f"`{typ}`" not in readme:
            problems.append(f"event type {typ!r} is not in the event catalog of README.md")
    if name == "coordination-round":
        first = next(iter(doc["do"][0]), None) if doc.get("do") else None
        if first != "rot-pruefen":
            problems.append("the first step must be `rot-pruefen` (the coordinator acts only on red or at gates)")
    return problems


def main(argv):
    try:
        if "--simulate-missing-yaml" in argv:
            raise ImportError("simulated")
        import yaml
    except ImportError:
        print("skipped: PyYAML (python3 yaml module) is not installed, the workflow files were NOT checked")
        return 2
    folder = argv[1]
    files = sorted(glob.glob(os.path.join(folder, "*.yaml")) + glob.glob(os.path.join(folder, "*.yml")))
    if not files:
        print(f"{folder}: no workflow files")
        return 1
    bad = 0
    readme_path = os.path.join(folder, "README.md")
    readme = open(readme_path, encoding="utf-8").read() if os.path.exists(readme_path) else None
    for path in files:
        rel = os.path.basename(path)
        try:
            with open(path, encoding="utf-8") as f:
                doc = yaml.safe_load(f)
        except yaml.YAMLError as e:
            mark = getattr(e, "problem_mark", None)
            print(f"{rel}:{mark.line + 1 if mark else 0}: {getattr(e, 'problem', None) or 'not YAML'}")
            bad += 1
            continue
        problems = check_doc(doc)
        if not problems:
            problems = check_events(doc, rel.rsplit(".", 1)[0], readme)
        for p in problems:
            print(f"{rel}: {p}")
            bad += 1
    if not bad:
        print(f"{len(files)} workflow file(s) ok")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
