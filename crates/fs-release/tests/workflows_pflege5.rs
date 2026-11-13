//! Pattern: none (test). The 0.0.5 rules of the workflow files: one review per head, task outputs through `export.as` and `$context`,
//! loops as `switch` with `then`, the upstream channel before the send, the license compare as a script, the compact order
//! (a `prompt-template.md` and the `rules.md` blocks named in the steps). Each rule has a counter-test on a changed copy.

mod support;
use support::workflows::{read, step_body, step_names, FILES};

fn position(text: &str, step: &str) -> usize {
    step_names(text)
        .iter()
        .position(|n| n == step)
        .unwrap_or_else(|| panic!("no step {step}"))
}

/// The one Opus review of a head sits in `release-flow`: no own `review` step, and the release step runs that flow.
fn single_review(text: &str) -> bool {
    !step_names(text).contains(&"review".to_string())
        && step_body(text, "release").contains("name: release-flow")
}

#[test]
fn flows_that_release_have_no_second_review() {
    for f in [
        "frontend-umbau",
        "wrapper-einbindung",
        "bugfix",
        "frontend-round",
    ] {
        assert!(single_review(&read(f)), "{f}");
    }
}

#[test]
fn counter_an_own_review_step_before_the_release_is_caught() {
    let text = read("frontend-umbau");
    let doubled = text.replacen(
        "  - release:",
        "  - review:\n      call: review-ai:0.0.6@fs\n  - release:",
        1,
    );
    assert_ne!(doubled, text);
    assert!(!single_review(&doubled));
}

/// Every read of an earlier task output is `$context.<key>` and the file exports that key (`{ <key>: . }`); a read of
/// `${ .<name> }` other than `.input` cannot be resolved in Serverless Workflow 1.0. The export comes before the read (in file order).
fn data_flow_resolvable(text: &str) -> bool {
    if text.split("${ .").skip(1).any(|r| !r.starts_with("input")) {
        return false;
    }
    text.match_indices("$context.").all(|(at, _)| {
        let key: String = text[at + "$context.".len()..]
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        text[..at].contains(&format!("{{ {key}: . }}"))
    })
}

#[test]
fn task_outputs_are_exported_before_they_are_read() {
    for f in FILES {
        assert!(data_flow_resolvable(&read(f)), "{f}");
    }
    assert!(read("upstream-anfrage").contains("$context.send.id"));
}

#[test]
fn counter_a_read_without_export_is_caught() {
    let text = read("frontend-umbau");
    assert!(!data_flow_resolvable(
        &text.replace("{ assign: . }", "{ other: . }")
    ));
    assert!(!data_flow_resolvable(
        &text.replace("$context.count", ".count")
    ));
    // read before the export: the export of `assign` stays, but a read of it comes first
    assert!(!data_flow_resolvable(&text.replacen(
        "do:\n",
        "do:\n  # ${ $context.assign }\n",
        1
    )));
}

#[test]
fn the_loops_back_are_switch_with_then() {
    for (f, step, target) in [
        ("frontend-umbau", "repeat", "then: convert"),
        ("wrapper-einbindung", "repeat", "then: wrapper"),
        ("upstream-anfrage", "back-to-draft", "then: draft"),
    ] {
        let body = step_body(&read(f), step);
        assert!(body.contains("switch:") && body.contains(target), "{f}");
        assert!(
            body.contains("actor: machine"),
            "{f}: the loop decision is a script"
        );
    }
}

#[test]
fn counter_a_loop_to_a_missing_step_is_red() {
    support::workflows::assert_red("frontend-umbau", "no step of this flow", |t| {
        t.replacen("then: convert", "then: nowhere", 1)
    });
    support::workflows::assert_red("upstream-anfrage", "no step of this flow", |t| {
        t.replacen("then: draft", "then: drfat", 1)
    });
}

/// The person chooses the channel after the approval and before the send; the send runs for the other channels only.
fn channel_before_send(text: &str) -> bool {
    let choose = step_body(text, "choose-channel");
    choose.contains("actor: human")
        && choose.contains("gate: human")
        && position(text, "approval") < position(text, "choose-channel")
        && position(text, "choose-channel") < position(text, "send")
        && step_body(text, "send").contains("!= \"direct\"")
}

#[test]
fn upstream_anfrage_has_the_channel_choice_before_the_send() {
    let text = read("upstream-anfrage");
    assert!(channel_before_send(&text));
    let choose = step_body(&text, "choose-channel");
    for word in [
        "tracker",
        "git-bug",
        "direct",
        "write access",
        "the user writes it themselves",
    ] {
        assert!(choose.contains(word), "{word}");
    }
    let hand = step_body(&text, "hand-text");
    assert!(hand.contains("actor: human") && hand.contains("== \"direct\""));
}

#[test]
fn counter_channel_as_machine_or_after_the_send_is_caught() {
    let text = read("upstream-anfrage");
    let machine = text.replacen(
        "  - choose-channel:\n      call: choose-channel:0.0.6@fs\n      with:\n        upstream: ${ .input.upstream }\n      export:\n        as: '${ $context + { channel: . } }'\n      metadata:\n        freesynergy.net:actor: human",
        "  - choose-channel:\n      call: choose-channel:0.0.6@fs\n      with:\n        upstream: ${ .input.upstream }\n      export:\n        as: '${ $context + { channel: . } }'\n      metadata:\n        freesynergy.net:actor: machine",
        1,
    );
    assert_ne!(machine, text);
    assert!(!channel_before_send(&machine));
    let late = text
        .replacen("  - send:", "  - tmp:", 1)
        .replacen("  - choose-channel:", "  - send:", 1)
        .replacen("  - tmp:", "  - choose-channel:", 1);
    assert!(!channel_before_send(&late));
}

/// The license compare is a script, the agent only writes the reasons of the exceptions after it.
fn license_is_a_script(text: &str) -> bool {
    step_body(text, "license").contains("actor: machine")
        && step_body(text, "license-reasons").contains("actor: ai")
        && position(text, "license") < position(text, "license-reasons")
        && position(text, "license-reasons") < position(text, "gates")
}

#[test]
fn wrapper_einbindung_compares_licenses_by_script() {
    assert!(license_is_a_script(&read("wrapper-einbindung")));
}

#[test]
fn counter_a_license_compare_by_the_agent_is_caught() {
    let text = read("wrapper-einbindung");
    let changed = text.replacen(
        "call: license-compare:0.0.6@fs\n      with:\n        crate: ${ .input.crate }\n      export:\n        as: '${ $context + { license: . } }'\n      metadata:\n        freesynergy.net:actor: machine",
        "call: license-compare:0.0.6@fs\n      with:\n        crate: ${ .input.crate }\n      export:\n        as: '${ $context + { license: . } }'\n      metadata:\n        freesynergy.net:actor: ai",
        1,
    );
    assert_ne!(changed, text);
    assert!(!license_is_a_script(&changed));
}

#[test]
fn frontend_umbau_names_the_adr_file() {
    let name = "conventions/decisions/0074-stil-bausteine-bewegung-klang-menues.md";
    assert!(read("frontend-umbau").contains(name));
    assert!(!read("frontend-umbau")
        .replace(name, "")
        .contains("0074-stil"));
}

/// The compact order: the flow names the template and the `rules.md` blocks instead of the agent order repeating them.
fn compact_order(text: &str) -> bool {
    text.contains("docs/prompt-template.md")
        && ["[GATES]", "[BERICHT]", "[COMMIT]", "[REVIEW]"]
            .iter()
            .any(|b| text.contains(b))
}

#[test]
fn the_flows_for_agents_name_the_template_and_the_rules_blocks() {
    for f in [
        "agent-run",
        "bugfix",
        "review",
        "frontend-round",
        "frontend-umbau",
        "dossier",
    ] {
        assert!(compact_order(&read(f)), "{f}");
    }
}

#[test]
fn counter_without_the_template_or_the_blocks_is_caught() {
    let text = read("review");
    assert!(!compact_order(&text.replace("docs/prompt-template.md", "")));
    let bare = text
        .replace("[GATES]", "")
        .replace("[BERICHT]", "")
        .replace("[COMMIT]", "")
        .replace("[REVIEW]", "");
    assert!(!compact_order(&bare));
}
