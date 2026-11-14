//! Pattern: none (test). The style gate of the theme bridge: in `frontend-umbau` the order is contract (`bruecke`, `alle-stile-erweitern`),
//! completeness, then `zwei-stile`, all before `convert` (the replacement of painted copies); `frontend-round` saves a style
//! (`stil-sichern`) and checks completeness. Each rule has a counter-test on a changed copy.

mod support;
use support::workflows::{assert_red, read, step_body, step_names};

fn position(text: &str, step: &str) -> usize {
    step_names(text)
        .iter()
        .position(|n| n == step)
        .unwrap_or_else(|| panic!("no step {step}"))
}

fn swapped(text: &str, a: &str, b: &str) -> String {
    text.replacen(&format!("  - {a}:"), "  - zz-tmp:", 1)
        .replacen(&format!("  - {b}:"), &format!("  - {a}:"), 1)
        .replacen("  - zz-tmp:", &format!("  - {b}:"), 1)
}

fn gate_before_convert(text: &str) -> bool {
    let order = [
        "assign",
        "bruecke",
        "alle-stile-erweitern",
        "stil-vollstaendigkeit",
        "vollstaendig-again",
        "zwei-stile",
        "zwei-stile-again",
        "convert",
    ];
    order.windows(2).all(|w| position(text, w[0]) < position(text, w[1]))
        && ["stil-vollstaendigkeit", "zwei-stile"].iter().all(|s| {
            let b = step_body(text, s);
            b.contains("actor: machine") && b.contains("kind: objective") && b.contains("call: ")
        })
        && step_body(text, "zwei-stile").contains("scripts/zwei-stile")
        && step_body(text, "zwei-stile").contains("content")
        && step_body(text, "stil-vollstaendigkeit").contains("scripts/stil-vollstaendigkeit")
        && step_body(text, "alle-stile-erweitern").contains("new version of the theme contract")
        && step_body(text, "bruecke").contains("actor: ai")
}

#[test]
fn frontend_umbau_has_the_style_gate_before_the_replacement() {
    assert!(gate_before_convert(&read("frontend-umbau")));
}

#[test]
fn counter_gate_after_convert_or_by_the_agent_is_caught() {
    let text = read("frontend-umbau");
    assert!(!gate_before_convert(&swapped(&text, "zwei-stile", "convert")));
    assert!(!gate_before_convert(&swapped(&text, "stil-vollstaendigkeit", "zwei-stile")));
    let ai = text.replacen(
        "  - zwei-stile:\n      call: zwei-stile:0.0.6@fs\n      with:\n        group: ${ .input.group }\n      export:\n        as: '${ $context + { stile2: . } }'\n      metadata:\n        freesynergy.net:actor: machine",
        "  - zwei-stile:\n      call: zwei-stile:0.0.6@fs\n      with:\n        group: ${ .input.group }\n      export:\n        as: '${ $context + { stile2: . } }'\n      metadata:\n        freesynergy.net:actor: ai",
        1,
    );
    assert_ne!(ai, text);
    assert!(!gate_before_convert(&ai));
}

#[test]
fn the_jumps_back_of_the_style_gate_are_switch_with_then() {
    let u = read("frontend-umbau");
    for (step, target) in [
        ("vollstaendig-again", "then: alle-stile-erweitern"),
        ("zwei-stile-again", "then: bruecke"),
    ] {
        let b = step_body(&u, step);
        assert!(b.contains("switch:") && b.contains(target) && b.contains("actor: machine"));
    }
    let r = step_body(&read("frontend-round"), "stil-sichern-again");
    assert!(r.contains("then: stil-sichern") && r.contains("then: stil-eskalation"));
    assert!(r.contains(">= 3"));
}

#[test]
fn counter_a_jump_of_the_style_gate_to_a_missing_step_is_red() {
    assert_red("frontend-umbau", "no step of this flow", |t| {
        t.replacen("then: alle-stile-erweitern", "then: alle-stile", 1)
    });
    assert_red("frontend-round", "no step of this flow", |t| {
        t.replacen("then: stil-sichern\n", "then: stil-sichen\n", 1)
    });
}

fn style_saved_and_complete(text: &str) -> bool {
    let save = step_body(text, "stil-sichern");
    ["stil-analyse", "stil-check", "stil-again", "stil-sichern", "stil-sichern-check", "stil-vollstaendigkeit", "stil-sichern-again", "images"]
        .windows(2)
        .all(|w| position(text, w[0]) < position(text, w[1]))
        && save.contains("actor: ai")
        && save.contains("English name")
        && save.contains("licence")
        && save.contains("ADR 0013")
        && save.contains("EVERY block")
        && step_body(text, "stil-sichern-check").contains("actor: machine")
        && step_body(text, "stil-vollstaendigkeit").contains("actor: machine")
        && step_body(text, "stil-vollstaendigkeit").contains("scripts/stil-vollstaendigkeit")
        && step_body(text, "stil-vollstaendigkeit").contains("self test")
}

#[test]
fn frontend_round_saves_the_style_and_checks_it_is_complete() {
    assert!(style_saved_and_complete(&read("frontend-round")));
}

#[test]
fn counter_style_save_without_licence_or_after_images_is_caught() {
    let text = read("frontend-round");
    assert!(!style_saved_and_complete(&text.replace("licence", "origin")));
    assert!(!style_saved_and_complete(&swapped(&text, "stil-sichern", "images")));
    let by_ai = text.replacen(
        "  - stil-vollstaendigkeit:\n      call: stil-vollstaendigkeit:0.0.6@fs\n      with:\n        contract: contract\n      export:\n        as: '${ $context + { vollstaendig: . } }'\n      metadata:\n        freesynergy.net:actor: machine",
        "  - stil-vollstaendigkeit:\n      call: stil-vollstaendigkeit:0.0.6@fs\n      with:\n        contract: contract\n      export:\n        as: '${ $context + { vollstaendig: . } }'\n      metadata:\n        freesynergy.net:actor: ai",
        1,
    );
    assert_ne!(by_ai, text);
    assert!(!style_saved_and_complete(&by_ai));
}
