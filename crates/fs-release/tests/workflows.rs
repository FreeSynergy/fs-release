//! Pattern: none (test). The workflow files in `workflows/`: schema check (`scripts/check_workflows.py`), step names equal to
//! the code, and mutations (broken YAML, missing actor, duplicate name, ...) that must turn the check red.

mod support;

use std::fs;
use std::path::{Path, PathBuf};

use fs_release::lift::{self, LiftOpts};
use support::{have_yaml, python};

const FILES: [&str; 23] = [
    "adr-write",
    "agent-run",
    "bugfix",
    "catalog-bump",
    "cleanup-targets",
    "coordination-round",
    "cost-reconcile",
    "dossier",
    "download-readonly",
    "frontend-round",
    "handoff-update",
    "icon-round",
    "lift",
    "picture-acceptance",
    "publish-doc",
    "release-flow",
    "report-to-strategy",
    "restart-handoff",
    "review",
    "slot-check",
    "user-decision",
    "wip-continue",
    "workflow-pflege",
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn script() -> PathBuf {
    root().join("scripts/check_workflows.py")
}

fn read(name: &str) -> String {
    fs::read_to_string(root().join("workflows").join(format!("{name}.yaml"))).unwrap()
}

/// The step names of a file (`  - name:` lines of the `do` list; the files keep that layout).
fn step_names(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|l| l.strip_prefix("  - ")?.strip_suffix(':'))
        .map(str::to_string)
        .collect()
}

/// A copy of `workflows/` in a temp folder with `file` changed by `edit`; returns exit code and output of the check.
fn mutated(file: &str, edit: impl Fn(String) -> String) -> (i32, String) {
    let tmp = tempfile::tempdir().unwrap();
    for f in FILES {
        let text = read(f);
        let text = if f == file { edit(text) } else { text };
        fs::write(tmp.path().join(format!("{f}.yaml")), text).unwrap();
    }
    python(&script(), &[tmp.path()])
}

/// A mutation must turn the check red and say `needle`; without `PyYAML` the check says `skipped` (exit 2) instead.
fn assert_red(file: &str, needle: &str, edit: impl Fn(String) -> String) {
    let (code, out) = mutated(file, edit);
    if have_yaml() {
        assert_eq!(code, 1, "{out}");
        assert!(out.contains(needle), "wanted {needle:?} in:\n{out}");
    } else {
        assert_eq!(code, 2, "{out}");
    }
}

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
        t.replacen("  version: 0.0.3\n", "", 1)
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
        t.replacen("      call: deps:0.1.0@fs\n", "", 1)
    });
}
