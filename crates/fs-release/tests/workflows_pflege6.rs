//! Pattern: none (test). The 0.0.6 rules of the workflow files: the [EFFIZIENZ] step rules, the place of `rules.md`, the decision flow
//! (ADR search first), texts before tokens in `frontend-umbau`, jumps back as `switch` with `then`. Each rule has a counter-test.

mod support;
use support::workflows::{assert_red, read, step_body, step_names};

const RULES_MD: &str = "/srv/FreeSynergy/.scratch/api-runner/rules.md";

fn position(text: &str, step: &str) -> usize {
    step_names(text)
        .iter()
        .position(|n| n == step)
        .unwrap_or_else(|| panic!("no step {step}"))
}

/// Swaps the names of two steps (the bodies stay): the same flow with the two steps in the other order.
fn swapped(text: &str, a: &str, b: &str) -> String {
    text.replacen(&format!("  - {a}:"), "  - zz-tmp:", 1)
        .replacen(&format!("  - {b}:"), &format!("  - {a}:"), 1)
        .replacen("  - zz-tmp:", &format!("  - {b}:"), 1)
}

/// The [EFFIZIENZ] rules as step rules: every phrase is in the file.
fn has_rules(text: &str, phrases: &[&str]) -> bool {
    text.contains("[EFFIZIENZ]") && phrases.iter().all(|p| text.contains(p))
}

const BUILD_RULES: [&str; 6] = [
    "one gate cycle",
    "once per head",
    "head hash",
    "files the order names",
    "script",
    "rc=124",
];
const REVIEW_RULES: [&str; 6] = [
    "delta",
    "file list",
    "once per head",
    "head hash",
    "files the order names",
    "rc=124",
];

#[test]
fn agent_run_and_review_carry_the_efficiency_rules() {
    assert!(has_rules(&read("agent-run"), &BUILD_RULES));
    assert!(has_rules(&read("review"), &REVIEW_RULES));
}

#[test]
fn counter_a_missing_efficiency_rule_is_caught() {
    for phrase in BUILD_RULES {
        let text = read("agent-run").replace(phrase, "");
        assert!(!has_rules(&text, &BUILD_RULES), "{phrase}");
    }
    for phrase in REVIEW_RULES {
        let text = read("review").replace(phrase, "");
        assert!(!has_rules(&text, &REVIEW_RULES), "{phrase}");
    }
}

#[test]
fn the_place_of_rules_md_is_named_where_the_orders_are_explained() {
    for text in [
        read("agent-run"),
        read("review"),
        std::fs::read_to_string(support::workflows::root().join("workflows/README.md")).unwrap(),
        std::fs::read_to_string(support::workflows::root().join("docs/prompt-template.md")).unwrap(),
    ] {
        assert!(text.contains(RULES_MD));
        assert!(!text.replace(RULES_MD, "").contains(RULES_MD));
    }
}

/// Look up first, evaluate, ask only on `open`, write down; the question step of `user-decision` also starts with the search.
fn decision_flow(text: &str) -> bool {
    let ask = step_body(text, "ask");
    step_names(text) == ["adr-search", "evaluate", "ask", "record"]
        && step_body(text, "adr-search").contains("actor: machine")
        && step_body(text, "adr-search").contains("call: adr-search:")
        && step_body(text, "evaluate").contains("actor: ai")
        && ask.contains("actor: human")
        && ask.contains("gate: human")
        && ask.contains("name: user-decision")
        && ask.contains("if: ${ $context.verdict.open }")
        && step_body(text, "record").contains("actor: ai")
}

#[test]
fn entscheidung_searches_the_adrs_before_it_asks() {
    assert!(decision_flow(&read("entscheidung")));
    let first = &step_names(&read("user-decision"))[0];
    assert_eq!(first, "adr-search");
    assert!(step_body(&read("user-decision"), first).contains("actor: machine"));
}

#[test]
fn counter_asking_first_or_a_search_by_the_agent_is_caught() {
    let text = read("entscheidung");
    assert!(!decision_flow(&swapped(&text, "adr-search", "ask")));
    assert!(!decision_flow(&text.replacen(
        "actor: machine\n        freesynergy.net:actor-type: Service",
        "actor: ai\n        freesynergy.net:actor-type: Application",
        1
    )));
    assert!(!decision_flow(&text.replace("if: ${ $context.verdict.open }", "")));
    let decision = read("user-decision");
    assert_ne!(step_names(&swapped(&decision, "adr-search", "question"))[0], "adr-search");
}

/// First the texts (ADR 0057, Words/MF2, de and en, gate `texts-check`), then the tokens; the check is a script.
fn texts_before_tokens(text: &str) -> bool {
    let texte = step_body(text, "texte");
    ["texte", "texts-check", "texts-again"]
        .iter()
        .all(|s| position(text, s) < position(text, "count"))
        && texte.contains("actor: ai")
        && texte.contains("ADR 0057")
        && texte.contains("MF2")
        && texte.contains("German and English")
        && step_body(text, "texts-check").contains("actor: machine")
        && step_body(text, "texts-check").contains("call: texts-check:")
}

#[test]
fn frontend_umbau_converts_the_texts_before_the_tokens() {
    assert!(texts_before_tokens(&read("frontend-umbau")));
}

#[test]
fn counter_texts_after_tokens_or_without_the_gate_are_caught() {
    let text = read("frontend-umbau");
    assert!(!texts_before_tokens(&swapped(&text, "texte", "count")));
    assert!(!texts_before_tokens(&text.replace("MF2", "Fluent")));
    assert!(!texts_before_tokens(&text.replace("call: texts-check:", "call: other:")));
}

#[test]
fn the_jumps_back_that_were_only_in_the_summary_are_switch_with_then() {
    for (f, step, target) in [
        ("frontend-round", "picture-again", "then: gap-list"),
        ("frontend-round", "gates-again", "then: build"),
        ("frontend-round", "acceptance-again", "then: build"),
        ("bugfix", "red-back", "then: fix"),
        ("frontend-umbau", "texts-again", "then: texte"),
    ] {
        let body = step_body(&read(f), step);
        assert!(body.contains("switch:") && body.contains(target), "{f} {step}");
        assert!(body.contains("actor: machine"), "{f} {step}");
    }
}

#[test]
fn counter_a_jump_to_a_missing_step_is_red() {
    assert_red("bugfix", "no step of this flow", |t| t.replacen("then: fix", "then: fx", 1));
    assert_red("frontend-round", "no step of this flow", |t| {
        t.replacen("then: gap-list", "then: gap-lst", 1)
    });
}
