//! Pattern: none (test). The workflow files in `workflows/`: schema check (`scripts/check_workflows.py`), step names equal to
//! the code, and mutations (broken YAML, missing actor, duplicate name, ...) that must turn the check red.

mod support;
use fs_release::lift::{self, LiftOpts};
use std::fs;
use std::path::PathBuf;
use support::workflows::{assert_red, read, root, script, step_names, FILES};
use support::{have_yaml, python};

#[test]
fn the_real_files_pass_the_schema_check() {
    for f in FILES {
        assert_ne!(read(f), "");
    }
    let (code, out) = python(&script(), &[&root().join("workflows")]);
    assert_eq!(code, if have_yaml() { 0 } else { 2 }, "{out}");
}

#[test]
fn the_list_equals_the_files_in_the_folder_and_all_carry_the_crate_version() {
    let mut on_disk: Vec<String> = fs::read_dir(root().join("workflows"))
        .unwrap()
        .filter_map(|e| {
            let name = e.unwrap().file_name().into_string().unwrap();
            name.strip_suffix(".yaml").map(str::to_string)
        })
        .collect();
    on_disk.sort();
    assert_eq!(on_disk, FILES);
    let version = format!("  version: {}\n", env!("CARGO_PKG_VERSION"));
    for f in FILES {
        assert!(read(f).contains(&version), "{f}.yaml: document.version");
    }
}

#[test]
fn every_catalog_call_and_the_chain_file_carry_the_crate_version() {
    let version = env!("CARGO_PKG_VERSION");
    for f in FILES {
        for line in read(f).lines() {
            if let Some((_, call)) = line.trim().split_once("call: ") {
                let call = call.trim();
                assert!(
                    call.ends_with(&format!(":{version}@fs")),
                    "{f}.yaml: catalog call {call:?} is not at {version}"
                );
            }
        }
    }
    let chain = fs::read_to_string(root().join("chains/release.chain.yaml")).unwrap();
    assert!(
        chain.contains(&format!("\nversion: {version}\n")),
        "chain version"
    );
    assert!(
        chain.contains(&format!("\ncatalog: fs/{version}\n")),
        "chain catalog"
    );
}

#[test]
fn nobody_pushes_or_merges_as_an_agent_and_the_coordinator_is_the_person() {
    for f in FILES {
        assert!(
            !read(f).contains("gate: coordinator"),
            "{f}.yaml: the coordinator is `gate: human`"
        );
    }
    for (file, step) in [
        ("publish-doc", "push"),
        ("adr-write", "push"),
        ("workflow-pflege", "to-main"),
    ] {
        let text = read(file);
        let body = text.split(&format!("  - {step}:")).nth(1).unwrap();
        let body = body.split("\n  - ").next().unwrap();
        assert!(
            body.contains("actor: human") && body.contains("gate: human"),
            "{file}.yaml step {step}: push and merge are the person's"
        );
    }
}

#[test]
fn the_cost_figures_are_a_script_and_the_agent_only_explains_a_deviation() {
    let text = read("cost-reconcile");
    for step in ["compare", "unknown", "correction", "limit"] {
        let body = text.split(&format!("  - {step}:")).nth(1).unwrap();
        let body = body.split("\n  - ").next().unwrap();
        assert!(
            body.contains("actor: machine"),
            "cost-reconcile step {step} is a script"
        );
    }
    let body = text.split("  - explain:").nth(1).unwrap();
    assert!(
        body.contains("actor: ai") && body.contains("if: "),
        "the agent only on a deviation"
    );
}

#[test]
fn picture_acceptance_keeps_the_user_verdict_human_and_subjective() {
    let text = read("picture-acceptance");
    let step = text.split("  - user-verdict:").nth(1).unwrap();
    let step = step.split("\n  - ").next().unwrap();
    assert!(step.contains("actor: human") && step.contains("kind: subjective"));
}

#[test]
fn lift_yaml_has_exactly_the_steps_of_the_code_in_order() {
    let mut o = LiftOpts::new(PathBuf::new(), PathBuf::new());
    o.commit = true;
    let code: Vec<&str> = lift::tasks(&o).iter().map(|t| t.name()).collect();
    assert_eq!(step_names(&read("lift")), code);
}

#[test]
fn release_flow_yaml_names_the_flow_steps_in_order_and_bumps_after_the_release() {
    let names = step_names(&read("release-flow"));
    let code: Vec<&str> = fs_release::flow::tasks().iter().map(|t| t.name()).collect();
    let at = |n: &str| {
        names
            .iter()
            .position(|x| x == n)
            .unwrap_or_else(|| panic!("no step {n}"))
    };
    let positions: Vec<usize> = code.iter().map(|n| at(n)).collect();
    assert!(
        positions.windows(2).all(|p| p[0] < p[1]),
        "{names:?} vs {code:?}"
    );
    assert_eq!(names.first().map(String::as_str), Some("lift"));
    assert!(at("push") < at("release") && at("release") < at("bump") && at("bump") < at("sign"));
    let text = read("release-flow");
    assert!(text.contains("workflow: bump.yml") && text.contains("repo: FreeSynergy/fs-catalog"));
}

#[test]
fn the_files_hold_no_secrets() {
    for f in FILES {
        let t = read(f).to_lowercase();
        for bad in [
            "ghp_",
            "github_pat_",
            "-----begin",
            "password:",
            "token:",
            "secret:",
        ] {
            assert!(!t.contains(bad), "{f}.yaml contains {bad:?}");
        }
    }
}

#[test]
fn mutation_broken_yaml_is_red_with_file_and_line() {
    assert_red("lift", "lift.yaml:", |t| format!("{t}  - [unclosed\n"));
}

#[test]
fn mutation_missing_actor_is_red() {
    assert_red("lift", "actor", |t| {
        t.replacen("        freesynergy.net:actor: machine\n", "", 1)
    });
}

#[test]
fn mutation_unknown_actor_value_is_red() {
    assert_red("agent-run", "'robot'", |t| {
        t.replacen(
            "freesynergy.net:actor: human",
            "freesynergy.net:actor: robot",
            1,
        )
    });
}

#[test]
fn mutation_actor_type_that_does_not_match_the_actor_is_red() {
    assert_red("agent-run", "actor-type must be Person", |t| {
        t.replacen("actor-type: Person", "actor-type: Service", 1)
    });
}

#[test]
fn mutation_missing_gate_is_red() {
    assert_red("release-flow", "gate", |t| {
        t.replacen("        freesynergy.net:gate: none\n", "", 1)
    });
}

#[test]
fn mutation_duplicate_step_name_is_red() {
    assert_red("lift", "duplicate step name 'deps'", |t| {
        t.replacen("  - lock:\n", "  - deps:\n", 1)
    });
}

#[test]
fn mutation_missing_document_version_is_red() {
    assert_red("release-flow", "document.version", |t| {
        t.replacen("  version: 0.0.6\n", "", 1)
    });
}

#[test]
fn mutation_missing_or_unknown_kind_is_red() {
    assert_red("bugfix", "kind", |t| {
        t.replacen("        freesynergy.net:kind: objective\n", "", 1)
    });
    assert_red("bugfix", "'maybe'", |t| {
        t.replacen("kind: objective", "kind: maybe", 1)
    });
}

#[test]
fn mutation_wrong_dsl_and_empty_do_are_red() {
    assert_red("lift", "document.dsl", |t| {
        t.replacen("dsl: 1.0.3", "dsl: 0.8", 1)
    });
    assert_red("cleanup-targets", "`do`", |t| {
        let head = t.split("\ndo:").next().unwrap().to_string();
        format!("{head}\ndo: []\n")
    });
}

#[test]
fn a_step_without_a_task_type_is_red() {
    assert_red("lift", "no task type", |t| {
        t.replacen("      call: deps:0.0.6@fs\n", "", 1)
    });
}
