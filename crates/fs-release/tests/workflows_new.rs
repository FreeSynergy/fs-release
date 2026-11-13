//! Pattern: none (test). The flows `frontend-umbau`, `upstream-anfrage`, `wrapper-einbindung`: who does which step, the order of
//! the steps, and that every sub-workflow a flow runs exists at the crate version. Each rule has a counter-test on a changed copy.
//! The 0.0.5 rules (one review, data flow, control flow, channel, license) are in `workflows_pflege5.rs`.

mod support;
use support::workflows::{read, step_body, step_names, FILES};

fn position(text: &str, step: &str) -> usize {
    step_names(text)
        .iter()
        .position(|n| n == step)
        .unwrap_or_else(|| panic!("no step {step}"))
}

/// The person approves (`actor: human`, `gate: human`) before the script sends; the rules check comes before the approval.
fn approval_before_send(text: &str) -> bool {
    let approval = step_body(text, "approval");
    approval.contains("actor: human")
        && approval.contains("gate: human")
        && position(text, "rules") < position(text, "approval")
        && position(text, "approval") < position(text, "send")
}

/// The counts, the pixel comparison and the gates are scripts (`actor: machine`), never the agent's word.
fn counts_are_scripts(text: &str) -> bool {
    ["count", "recount", "before", "gates", "same-pictures"]
        .iter()
        .all(|s| step_body(text, s).contains("actor: machine"))
}

/// The license step is in the flow before the gates and names the GitHub CI (cargo-deny runs there, not locally).
fn license_gate_in_ci(text: &str) -> bool {
    position(text, "pin") < position(text, "wrapper")
        && position(text, "wrapper") < position(text, "license")
        && position(text, "license") < position(text, "gates")
        && step_body(text, "license").contains("GitHub CI")
}

#[test]
fn upstream_anfrage_sends_only_after_the_rules_and_the_person_approved() {
    let text = read("upstream-anfrage");
    assert!(approval_before_send(&text));
    assert!(step_body(&text, "send").contains("actor: machine"));
    assert!(step_body(&text, "rules").contains("CLAUDE.md"));
}

#[test]
fn counter_upstream_anfrage_an_agent_approval_or_a_swapped_order_is_caught() {
    let text = read("upstream-anfrage");
    let ai = text.replacen(
        "  - approval:\n      call: approve:0.0.6@fs\n      with:\n        draft: ${ $context.draft }\n      metadata:\n        freesynergy.net:actor: human",
        "  - approval:\n      call: approve:0.0.6@fs\n      with:\n        draft: ${ $context.draft }\n      metadata:\n        freesynergy.net:actor: ai",
        1,
    );
    assert_ne!(ai, text);
    assert!(!approval_before_send(&ai));
    let swapped = text
        .replacen("  - send:", "  - tmp:", 1)
        .replacen("  - approval:", "  - send:", 1)
        .replacen("  - tmp:", "  - approval:", 1);
    assert!(!approval_before_send(&swapped));
}

#[test]
fn frontend_umbau_counts_and_compares_by_script() {
    let text = read("frontend-umbau");
    assert!(counts_are_scripts(&text));
    let release = step_body(&text, "release");
    assert!(release.contains("actor: human") && release.contains("gate: human"));
    assert!(position(&text, "recount") > position(&text, "convert"));
    assert!(position(&text, "same-pictures") > position(&text, "convert"));
}

#[test]
fn counter_frontend_umbau_a_count_by_the_agent_is_caught() {
    let text = read("frontend-umbau");
    let changed = text.replacen(
        "  - recount:\n      call: count-fixed-values:0.0.6@fs\n      with:\n        group: ${ .input.group }\n      export:\n        as: '${ $context + { recount: . } }'\n      metadata:\n        freesynergy.net:actor: machine\n        freesynergy.net:actor-type: Service",
        "  - recount:\n      call: count-fixed-values:0.0.6@fs\n      with:\n        group: ${ .input.group }\n      export:\n        as: '${ $context + { recount: . } }'\n      metadata:\n        freesynergy.net:actor: ai\n        freesynergy.net:actor-type: Application",
        1,
    );
    assert_ne!(changed, text);
    assert!(!counts_are_scripts(&changed));
}

#[test]
fn wrapper_einbindung_orders_pin_wrapper_license_gates_and_names_the_ci() {
    let text = read("wrapper-einbindung");
    assert!(license_gate_in_ci(&text));
    assert!(step_body(&text, "lock").contains("Cargo.lock"));
}

#[test]
fn counter_wrapper_einbindung_a_local_deny_run_or_a_late_license_step_is_caught() {
    let text = read("wrapper-einbindung");
    let local = text.replace("GitHub CI", "local run");
    assert_ne!(local, text);
    assert!(!license_gate_in_ci(&local));
    let late = text
        .replacen("  - license:", "  - tmp:", 1)
        .replacen("  - gates:", "  - license:", 1)
        .replacen("  - tmp:", "  - gates:", 1);
    assert!(!license_gate_in_ci(&late));
}

/// Every `name:` under a `workflow:` key of a file, with its version line.
fn sub_workflows(text: &str) -> Vec<(String, String)> {
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    let mut out = vec![];
    for (i, l) in lines.iter().enumerate() {
        if *l == "workflow:" {
            let name = lines[i + 2].strip_prefix("name: ").unwrap();
            let version = lines[i + 3].strip_prefix("version: ").unwrap();
            out.push((name.to_string(), version.to_string()));
        }
    }
    out
}

fn all_sub_workflows_exist(read_file: impl Fn(&str) -> String) -> bool {
    let version = env!("CARGO_PKG_VERSION");
    FILES.iter().all(|f| {
        sub_workflows(&read_file(f))
            .iter()
            .all(|(n, v)| v == version && FILES.contains(&n.as_str()))
    })
}

#[test]
fn every_sub_workflow_exists_at_the_crate_version() {
    assert!(all_sub_workflows_exist(read));
    let seen: usize = FILES.iter().map(|f| sub_workflows(&read(f)).len()).sum();
    assert!(seen >= 8, "found only {seen} sub-workflow references");
}

#[test]
fn counter_a_missing_sub_workflow_is_caught() {
    let broken = |f: &str| read(f).replace("name: review\n", "name: no-such-flow\n");
    assert!(!all_sub_workflows_exist(broken));
}
