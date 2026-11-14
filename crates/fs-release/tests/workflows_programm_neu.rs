//! Pattern: none (test). The flow `programm-neu`: the 17 steps in the phases P0 to P6, an ADR reference and a `kind` on every step,
//! every objective agent step followed by an objective script check, jumps back as `switch`, the human stays human (server run,
//! acceptance), the missing flow `facette-anlegen` only as a TODO stub. Each rule has a counter-test on a changed copy.

mod support;
use support::workflows::{assert_red, read, step_body, step_names};

const FLOW: &str = "programm-neu";
const MAIN: [&str; 19] = [
    "wessen-daten",
    "katalog-suche",
    "standards",
    "kern-crate",
    "umzug",
    "datenmodell",
    "schnittstelle",
    "manifest",
    "engine",
    "grenz-adapter",
    "admin-frontend",
    "vorbild-dossier",
    "frontend",
    "sprache",
    "hub-adapter",
    "sicherheit",
    "pakete",
    "release",
    "abnahme",
];

fn position(text: &str, step: &str) -> usize {
    step_names(text)
        .iter()
        .position(|n| n == step)
        .unwrap_or_else(|| panic!("no step {step}"))
}

/// Every step names at least one ADR (`freesynergy.net:adr: '0076, 0061'`) and its phase.
fn all_have_adr(text: &str) -> bool {
    step_names(text).iter().all(|s| {
        let b = step_body(text, s);
        b.lines().any(|l| {
            l.trim()
                .strip_prefix("freesynergy.net:adr: '")
                .is_some_and(|v| v.chars().next().is_some_and(|c| c.is_ascii_digit()))
        }) && b.contains("freesynergy.net:phase: P")
    })
}

/// Every objective agent step is followed by a machine step of kind objective (the objective check).
fn agent_steps_are_checked(text: &str) -> bool {
    let names = step_names(text);
    names.iter().enumerate().all(|(i, n)| {
        let b = step_body(text, n);
        if !(b.contains("actor: ai") && b.contains("kind: objective")) {
            return true;
        }
        names.get(i + 1).is_some_and(|next| {
            let nb = step_body(text, next);
            nb.contains("actor: machine") && nb.contains("kind: objective")
        })
    })
}

#[test]
fn the_17_steps_come_in_the_order_of_the_phases() {
    let text = read(FLOW);
    let at: Vec<usize> = MAIN.iter().map(|s| position(&text, s)).collect();
    assert!(at.windows(2).all(|p| p[0] < p[1]), "{at:?}");
    let phase = |s: &str| {
        let b = step_body(&text, s);
        let l = b
            .lines()
            .find(|l| l.contains("freesynergy.net:phase: P"))
            .unwrap()
            .to_string();
        l.trim().chars().last().unwrap()
    };
    let phases: Vec<char> = MAIN.iter().map(|s| phase(s)).collect();
    assert!(phases.windows(2).all(|p| p[0] <= p[1]), "{phases:?}");
    assert_eq!((phases[0], phases[18]), ('0', '6'));
}

#[test]
fn the_border_adapter_step_and_the_formats_name_adr_0085() {
    let text = read(FLOW);
    for step in [
        "umzug",
        "datenmodell",
        "grenz-adapter",
        "grenz-check",
        "grenz-again",
    ] {
        assert!(step_body(&text, step).contains("0085"), "{step}");
    }
    // counter-test: the reference removed from that one step only (the other steps keep theirs), the rule then fails
    let adr = |t: &str, s: &str| {
        step_body(t, s)
            .lines()
            .any(|l| l.trim().starts_with("freesynergy.net:adr:") && l.contains("0085"))
    };
    let broken = text.replacen("'0085, 0014, 0046'", "'0014, 0046'", 1);
    assert_ne!(broken, text);
    assert!(adr(&text, "grenz-adapter") && !adr(&broken, "grenz-adapter"));
    assert!(adr(&broken, "umzug"));
}

#[test]
fn counter_two_steps_swapped_are_caught() {
    let text = read(FLOW);
    let swapped = text
        .replacen("  - engine:", "  - zz-tmp:", 1)
        .replacen("  - datenmodell:", "  - engine:", 1)
        .replacen("  - zz-tmp:", "  - datenmodell:", 1);
    let at: Vec<usize> = MAIN.iter().map(|s| position(&swapped, s)).collect();
    assert!(!at.windows(2).all(|p| p[0] < p[1]));
}

#[test]
fn every_step_names_its_adrs_and_a_kind() {
    assert!(all_have_adr(&read(FLOW)));
    assert_red(FLOW, "kind", |t| {
        t.replacen("        freesynergy.net:kind: subjective\n", "", 1)
    });
}

#[test]
fn counter_a_step_without_an_adr_reference_is_caught() {
    let text = read(FLOW);
    let broken = text.replacen("        freesynergy.net:adr: '0076, 0061'\n", "", 1);
    assert_ne!(broken, text);
    assert!(!all_have_adr(&broken));
}

#[test]
fn objective_agent_steps_have_an_objective_script_check() {
    assert!(agent_steps_are_checked(&read(FLOW)));
}

#[test]
fn counter_a_missing_check_step_is_caught() {
    let text = read(FLOW);
    let broken = text.replacen("  - standards-check:", "  - standards-note:", 1).replacen(
        "freesynergy.net:actor: machine\n        freesynergy.net:actor-type: Service\n        freesynergy.net:gate: none\n        freesynergy.net:kind: objective\n        freesynergy.net:phase: P1\n        freesynergy.net:adr: '0014'",
        "freesynergy.net:actor: human\n        freesynergy.net:actor-type: Person\n        freesynergy.net:gate: human\n        freesynergy.net:kind: subjective\n        freesynergy.net:phase: P1\n        freesynergy.net:adr: '0014'",
        1,
    );
    assert_ne!(broken, text);
    assert!(!agent_steps_are_checked(&broken));
}

#[test]
fn red_checks_jump_back_by_switch_and_the_person_runs_real_servers() {
    let text = read(FLOW);
    for (step, target) in [
        ("einordnung-again", "then: wessen-daten"),
        ("standards-again", "then: standards"),
        ("kern-again", "then: kern-crate"),
        ("modell-again", "then: datenmodell"),
        ("schnittstelle-again", "then: datenmodell"),
        ("engine-again", "then: schnittstelle"),
        ("admin-again", "then: admin-frontend"),
        ("sprache-again", "then: sprache"),
        ("adapter-again", "then: hub-adapter"),
        ("sicherheit-again", "then: sicherheit"),
        ("pakete-again", "then: pakete"),
        ("abnahme-again", "then: befunde"),
        ("umzug-again", "then: umzug"),
        ("grenz-again", "then: grenz-adapter"),
        ("grenz-again", "then: datenmodell"),
        ("manifest-again", "then: manifest"),
        ("engine-again", "then: engine"),
        ("katalog-weiter", "then: vorhanden-bericht"),
        ("katalog-weiter", "then: katalog-fehler"),
        ("einordnung-again", "then: eskalation"),
    ] {
        let b = step_body(&text, step);
        assert!(
            b.contains("switch:") && b.contains(target) && b.contains("actor: machine"),
            "{step}"
        );
    }
    let server = step_body(&text, "server-lauf");
    assert!(
        server.contains("actor: human")
            && server.contains("gate: human")
            && server.contains("no agent ever has SSH or root")
    );
    let hub = step_body(&text, "hub-adapter");
    assert!(hub.contains("actor: ai") && hub.contains("nothing on a real server"));
    let abnahme = step_body(&text, "abnahme");
    assert!(abnahme.contains("actor: human") && abnahme.contains("kind: subjective"));
    assert!(step_body(&text, "befunde").contains("name: bugfix"));
}

#[test]
fn facette_and_modul_leave_at_a_todo_stub_and_facette_anlegen_is_not_invented() {
    let text = read(FLOW);
    for stub in ["facette-abzweig", "modul-abzweig", "suite-abzweig"] {
        assert!(ends_the_flow(&text, stub), "{stub}");
        assert!(step_body(&text, stub).contains("actor: human"), "{stub}");
    }
    // every dead end of the flow really ends (a `then: continue` would run on into the next stub)
    for end in ["eskalation", "vorhanden-bericht", "katalog-fehler"] {
        assert!(ends_the_flow(&text, end), "{end}");
    }
    // counter-tests: a stub with `then: continue`, a stub without any `then`
    for stub in ["facette-abzweig", "modul-abzweig", "suite-abzweig"] {
        let body = step_body(&text, stub);
        let cont = text.replacen(
            &body,
            &body.replace("      then: end", "      then: continue"),
            1,
        );
        assert_ne!(cont, text);
        assert!(!ends_the_flow(&cont, stub), "{stub}");
        let none = text.replacen(&body, &body.replace("      then: end\n", ""), 1);
        assert!(!ends_the_flow(&none, stub), "{stub}");
    }
    assert!(step_body(&text, "facette-abzweig").contains("does NOT exist yet"));
    assert!(!text.contains("name: facette-anlegen"));
    assert!(!text.contains("Gesicht"));
    let again = step_body(&text, "einordnung-again");
    for t in [
        "then: facette-abzweig",
        "then: modul-abzweig",
        "then: suite-abzweig",
    ] {
        assert!(again.contains(t), "{t}");
    }
}

#[test]
fn counter_a_jump_to_a_missing_step_is_red() {
    assert_red(FLOW, "no step of this flow", |t| {
        t.replacen("then: facette-abzweig", "then: facette-weg", 1)
    });
}

// ---- structure rules: graph, loops, order, inputs (each with a counter-test; the prose `contains` tests above are no proof of behaviour)

fn swapped(text: &str, a: &str, b: &str) -> String {
    text.replacen(&format!("  - {a}:"), "  - zz-tmp:", 1)
        .replacen(&format!("  - {b}:"), &format!("  - {a}:"), 1)
        .replacen("  - zz-tmp:", &format!("  - {b}:"), 1)
}

/// The steps a step can go to: switch cases (12 spaces), else the step `then` (6 spaces), else the next step.
fn successors(text: &str, i: usize) -> Vec<usize> {
    let names = step_names(text);
    let body = step_body(text, &names[i]);
    let want = if body.contains("      switch:\n") {
        12
    } else {
        6
    };
    let mut thens: Vec<String> = body
        .lines()
        .filter(|l| l.len() - l.trim_start().len() == want)
        .filter_map(|l| l.trim().strip_prefix("then: ").map(str::to_string))
        .collect();
    if want == 6 && thens.is_empty() {
        thens.push("continue".into());
    }
    let mut out = vec![];
    for t in thens {
        match t.as_str() {
            "continue" if i + 1 < names.len() => out.push(i + 1),
            "continue" | "end" | "exit" => {}
            name => out.push(names.iter().position(|n| n == name).unwrap_or(usize::MAX)),
        }
    }
    out
}

/// Every step is reachable from the first one (a stub or a branch that nothing leads to is dead).
fn all_reachable(text: &str) -> bool {
    let n = step_names(text).len();
    let mut seen = vec![false; n];
    let mut todo = vec![0];
    while let Some(i) = todo.pop() {
        if i >= n || seen[i] {
            continue;
        }
        seen[i] = true;
        todo.extend(successors(text, i));
    }
    seen.iter().all(|s| *s)
}

/// A jump back must land on a step that can change the thing the check looks at: an agent or a person, never a machine step.
fn back_jumps_land_on_workers(text: &str) -> bool {
    let names = step_names(text);
    (0..names.len()).all(|i| {
        successors(text, i)
            .into_iter()
            .filter(|&j| j <= i)
            .all(|j| j < names.len() && !step_body(text, &names[j]).contains("actor: machine"))
    })
}

/// Every red loop of the flow stops in the third red round: its switch has a case whose `when` tests `runde` against exactly `>= 3`,
/// that case goes to `escalate` (a human step), and it stands before the first case that jumps back. `skip` = loops without a counter by design.
fn red_loops_escalate(text: &str, escalate: &str, skip: &[&str]) -> bool {
    let names = step_names(text);
    (0..names.len()).all(|i| {
        let back = successors(text, i).into_iter().any(|j| j <= i);
        let b = step_body(text, &names[i]);
        if !back || !b.contains("      switch:\n") || skip.contains(&names[i].as_str()) {
            return true;
        }
        let lines: Vec<&str> = b
            .split("      switch:\n")
            .nth(1)
            .unwrap_or("")
            .lines()
            .collect();
        // the case that counts: `when` with `runde` and `>= 3`, directly followed by `then: <escalate>`
        let Some(k) = lines
            .iter()
            .position(|l| l.contains("runde") && threshold_is_3(l))
        else {
            return false;
        };
        let goes_to_escalate = lines
            .get(k + 1)
            .is_some_and(|l| l.trim() == format!("then: {escalate}"));
        let first_back = lines.iter().position(|l| {
            l.trim()
                .strip_prefix("then: ")
                .and_then(|t| names.iter().position(|n| n == t))
                .is_some_and(|j| j <= i)
        });
        goes_to_escalate && first_back.map_or(true, |f| k < f) && human_step(text, escalate)
    })
}

/// The `when` line compares against exactly `>= 3` (not 30, not 99).
fn threshold_is_3(line: &str) -> bool {
    line.split(">= ").nth(1).is_some_and(|rest| {
        let mut c = rest.chars();
        c.next() == Some('3') && !c.next().is_some_and(|d| d.is_ascii_digit())
    })
}

fn human_step(text: &str, step: &str) -> bool {
    let b = step_body(text, step);
    b.contains("actor: human") && b.contains("gate: human")
}

/// The step has the flow directive `then: end` at task level (6 spaces), so it ends the flow and does not fall into the next step.
fn ends_the_flow(text: &str, step: &str) -> bool {
    let thens: Vec<String> = step_body(text, step)
        .lines()
        .filter(|l| l.len() - l.trim_start().len() == 6)
        .filter_map(|l| l.trim().strip_prefix("then: ").map(str::to_string))
        .collect();
    thens == ["end"]
}

fn at(text: &str, s: &str) -> usize {
    position(text, s)
}

/// Security, packages, release, server run, in this order; the Frontend round does not release by itself.
fn release_comes_last(text: &str) -> bool {
    at(text, "sprache") < at(text, "release")
        && at(text, "sicherheit-again") < at(text, "pakete")
        && at(text, "pakete-again") < at(text, "release")
        && at(text, "release") < at(text, "server-lauf")
        && at(text, "server-lauf") < at(text, "abnahme")
        && step_body(text, "frontend").contains("          release: false\n")
}

/// Step 5 checks only what step 5 makes; the commands and the round trip belong to step 9a.
fn formats_before_commands(text: &str) -> bool {
    let u = step_body(text, "umzug-check");
    let g = step_body(text, "grenz-check");
    u.contains("call: format-check:")
        && g.contains("call: boundary-check:")
        && !u.contains("round trip over the fixtures keeps")
        && u.contains("NO import or export command")
        && g.contains("round trip")
}

fn core_outcome_is_checkable(text: &str) -> bool {
    step_body(text, "kern-crate").contains("eigener-kern")
        && step_body(text, "kern-check").contains("no pin is asked for")
}

fn p0_exits_are_reported(text: &str) -> bool {
    let w = step_body(text, "katalog-weiter");
    w.contains("when: '${ $context.katalog.fehler }'")
        && w.contains("then: vorhanden-bericht")
        && !w.contains("then: end")
        && ["vorhanden-bericht", "katalog-fehler"].iter().all(|s| {
            let b = step_body(text, s);
            b.contains("gate: human") && b.contains("actor: human")
        })
}

/// Every `.input.X` that the called flow reads is passed by the calling step.
fn run_inputs_are_given(caller: &str) -> Vec<String> {
    run_inputs_missing(caller, &read(caller))
}

fn run_inputs_missing(caller: &str, text: &str) -> Vec<String> {
    let mut missing = vec![];
    for s in step_names(text) {
        let b = step_body(text, &s);
        let Some(callee) = b
            .lines()
            .skip_while(|l| !l.contains("workflow:"))
            .find_map(|l| l.trim().strip_prefix("name: ").map(str::to_string))
        else {
            continue;
        };
        let given: Vec<String> = b
            .lines()
            .skip_while(|l| l.trim() != "input:")
            .skip(1)
            .filter_map(|l| l.trim().split_once(": ").map(|(k, _)| k.to_string()))
            .collect();
        let callee_text = read(&callee);
        let mut used: Vec<String> = callee_text
            .split(".input.")
            .skip(1)
            .map(|r| {
                r.chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                    .collect()
            })
            .collect();
        used.sort();
        used.dedup();
        for u in used {
            if !given.contains(&u) {
                missing.push(format!("{caller} step {s} -> {callee}: {u}"));
            }
        }
    }
    missing
}

/// Every `.input.X` of programm-neu is named in its header (the caller knows what to give).
fn inputs_are_declared(text: &str) -> bool {
    let header: String = text
        .lines()
        .take_while(|l| l.starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");
    let mut used: Vec<String> = text
        .split(".input.")
        .skip(1)
        .map(|r| {
            r.chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect()
        })
        .collect();
    used.dedup();
    used.iter().all(|u| header.contains(u.as_str()))
}

#[test]
fn the_graph_has_no_dead_step_and_the_back_jumps_land_on_workers() {
    let text = read(FLOW);
    assert!(all_reachable(&text));
    assert!(back_jumps_land_on_workers(&text));
    // counter-tests
    assert!(!all_reachable(&text.replacen(
        "then: facette-abzweig",
        "then: continue",
        1
    )));
    assert!(!back_jumps_land_on_workers(&text.replacen(
        "then: wessen-daten",
        "then: ausgang-check",
        1
    )));
    assert!(!back_jumps_land_on_workers(&text.replacen(
        "then: standards\n",
        "then: standards-check\n",
        1
    )));
}

#[test]
fn every_red_loop_of_programm_neu_stops_in_the_third_red_round() {
    let text = read(FLOW);
    assert!(red_loops_escalate(&text, "eskalation", &["befunde"]));
    let one = "        - runden:\n            when: '${ ($context.standards.red and ($context.standards.runde // 0) >= 3) }'\n            then: eskalation\n";
    assert!(text.contains(one));
    // counter-tests: the case removed, the threshold 99 or 30, the case after the jump back, the target not a human step
    assert!(!red_loops_escalate(
        &text.replacen(one, "", 1),
        "eskalation",
        &["befunde"]
    ));
    assert!(!red_loops_escalate(
        &text.replacen(
            ">= 3) }'\n            then: eskalation",
            ">= 99) }'\n            then: eskalation",
            1
        ),
        "eskalation",
        &["befunde"]
    ));
    assert!(!red_loops_escalate(
        &text.replacen(
            ">= 3) }'\n            then: eskalation",
            ">= 30) }'\n            then: eskalation",
            1
        ),
        "eskalation",
        &["befunde"]
    ));
    assert!(!red_loops_escalate(
        &text.replacen(
            one,
            &format!("{}", one.replace("then: eskalation", "then: standards")),
            1
        ),
        "eskalation",
        &["befunde"]
    ));
    assert!(!red_loops_escalate(
        &text.replacen("then: eskalation", "then: katalog-fehler", 1),
        "eskalation",
        &["befunde"]
    ));
    assert!(step_body(&text, "eskalation").contains("actor: human"));
}

#[test]
fn the_red_loops_of_dossier_and_frontend_round_stop_where_they_have_a_counter() {
    // dossier: the block assignment (rc 3 of the script); frontend-round: only the style analysis has a counter, the loops
    // `gates`, `menue-vergleich` and `acceptance` have none yet (open point, see CHANGELOG), so they are named as skipped
    let dossier = read("dossier");
    assert!(red_loops_escalate(&dossier, "eingabe-fehler", &[]));
    let case = "        - runden:\n            when: '${ $context.bausteine.red and ($context.bausteine.runde // 0) >= 3 }'\n            then: eingabe-fehler\n";
    assert!(dossier.contains(case));
    assert!(!red_loops_escalate(
        &dossier.replacen(case, "", 1),
        "eingabe-fehler",
        &[]
    ));
    assert!(!red_loops_escalate(
        &dossier.replacen(">= 3 }'", ">= 99 }'", 1),
        "eingabe-fehler",
        &[]
    ));
    let round = read("frontend-round");
    let open = [
        "picture-again",
        "gates-again",
        "menue-vergleich-again",
        "acceptance-again",
    ];
    assert!(red_loops_escalate(&round, "stil-eskalation", &open));
    let case = "        - runden:\n            when: '${ $context.stil.red and ($context.stil.runde // 0) >= 3 }'\n            then: stil-eskalation\n";
    assert!(round.contains(case));
    assert!(!red_loops_escalate(
        &round.replacen(case, "", 1),
        "stil-eskalation",
        &open
    ));
    assert!(!red_loops_escalate(
        &round.replacen(">= 3 }'", ">= 99 }'", 1),
        "stil-eskalation",
        &open
    ));
    // the open loops are exactly those without a counter: with no skip list the check is red
    assert!(!red_loops_escalate(&round, "stil-eskalation", &[]));
}

#[test]
fn security_packages_release_server_run_come_in_this_order() {
    let text = read(FLOW);
    assert!(release_comes_last(&text));
    // counter-tests: the old order, the round releasing by itself
    assert!(!release_comes_last(&swapped(
        &text,
        "release",
        "sicherheit"
    )));
    assert!(!release_comes_last(&swapped(
        &text,
        "server-lauf",
        "release"
    )));
    assert!(!release_comes_last(&text.replacen(
        "          release: false\n",
        "          release: true\n",
        1
    )));
}

#[test]
fn step_5_checks_formats_only_and_the_round_trip_is_checked_in_9a() {
    let text = read(FLOW);
    assert!(formats_before_commands(&text));
    let old = text.replacen(
        "NO import or export command and NO round trip here",
        "for every named format an import and an export command exist and a round trip over the fixtures keeps the data",
        1,
    );
    assert_ne!(old, text);
    assert!(!formats_before_commands(&old));
    // counter-tests: step 5 calls the boundary check of step 9a (or the old name), step 9a calls the format check
    assert!(!formats_before_commands(&text.replacen(
        "call: format-check:",
        "call: roundtrip-check:",
        1
    )));
    assert!(!formats_before_commands(&text.replacen(
        "call: boundary-check:",
        "call: format-check:",
        1
    )));
}

#[test]
fn an_own_core_can_pass_the_check_and_the_exits_of_p0_are_reported() {
    let text = read(FLOW);
    assert!(core_outcome_is_checkable(&text));
    assert!(!core_outcome_is_checkable(
        &text.replace("no pin is asked for", "a pin exists")
    ));
    assert!(p0_exits_are_reported(&text));
    assert!(!p0_exits_are_reported(&text.replacen(
        "then: vorhanden-bericht",
        "then: end",
        1
    )));
}

#[test]
fn sub_workflows_get_every_input_they_read_and_the_dossier_is_the_models() {
    for caller in ["programm-neu", "frontend-round"] {
        assert_eq!(run_inputs_are_given(caller), Vec::<String>::new());
    }
    let text = read(FLOW);
    assert!(inputs_are_declared(&text));
    // step 11 and step 12 ask for the same dossier of the MODEL program (`vorbild`), so step 12 finds it green; the `frontend`
    // of step 12 is our own program (`programm`), never the model
    let line = |s: &str, key: &str| {
        step_body(&text, s)
            .lines()
            .find(|l| l.trim().starts_with(key))
            .map(str::to_string)
    };
    assert!(line("vorbild-dossier", "vorbild: ").is_some());
    assert_eq!(
        line("vorbild-dossier", "vorbild: "),
        line("frontend", "vorbild: ")
    );
    assert!(line("vorbild-dossier", "frontend: ").is_none());
    assert!(line("frontend", "frontend: ")
        .unwrap()
        .contains("input.programm"));
    // counter-tests: an input the callee reads dropped (every sub-workflow call), a name changed
    for key in ["vorbild: ", "blinc: ", "bestand: "] {
        let body = step_body(&text, "vorbild-dossier");
        let line = body.lines().find(|l| l.trim().starts_with(key)).unwrap();
        let dropped = text.replacen(&body, &body.replace(&format!("{line}\n"), ""), 1);
        assert_ne!(dropped, text, "{key}");
        assert!(!run_inputs_missing(FLOW, &dropped).is_empty(), "{key}");
    }
    let body = step_body(&text, "frontend");
    for key in ["frontend: ", "vorbild: ", "blinc: "] {
        let line = body.lines().find(|l| l.trim().starts_with(key)).unwrap();
        let dropped = text.replacen(&body, &body.replace(&format!("{line}\n"), ""), 1);
        assert!(
            !run_inputs_missing(FLOW, &dropped).is_empty(),
            "frontend {key}"
        );
    }
    let round = read("frontend-round");
    for key in ["vorbild: ", "blinc: "] {
        let body = step_body(&round, "dossier");
        let line = body.lines().find(|l| l.trim().starts_with(key)).unwrap();
        let dropped = round.replacen(&body, &body.replace(&format!("{line}\n"), ""), 1);
        assert!(
            !run_inputs_missing("frontend-round", &dropped).is_empty(),
            "{key}"
        );
    }
    // counter-tests: an input dropped, a name changed
    let dropped = text.replacen("          release: false\n", "", 1);
    assert!(!release_comes_last(&dropped));
    assert!(!inputs_are_declared(&text.replacen(
        "${ .input.programm }",
        "${ .input.unbekannt }",
        1
    )));
}
