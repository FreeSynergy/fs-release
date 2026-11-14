//! Test support for the workflow files in `workflows/`: the list of files, reading, step bodies, and mutated copies.

use std::fs;
use std::path::{Path, PathBuf};

use super::{have_yaml, python};

pub const FILES: [&str; 28] = [
    "adr-write",
    "agent-run",
    "bugfix",
    "catalog-bump",
    "cleanup-targets",
    "coordination-round",
    "cost-reconcile",
    "dossier",
    "download-readonly",
    "entscheidung",
    "frontend-round",
    "frontend-umbau",
    "handoff-update",
    "icon-round",
    "lift",
    "picture-acceptance",
    "programm-neu",
    "publish-doc",
    "release-flow",
    "report-to-strategy",
    "restart-handoff",
    "review",
    "slot-check",
    "upstream-anfrage",
    "user-decision",
    "wip-continue",
    "workflow-pflege",
    "wrapper-einbindung",
];

pub fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn script() -> PathBuf {
    root().join("scripts/check_workflows.py")
}

pub fn read(name: &str) -> String {
    fs::read_to_string(root().join("workflows").join(format!("{name}.yaml"))).unwrap()
}

/// The step names of a file (`  - name:` lines of the `do` list; the files keep that layout).
pub fn step_names(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|l| l.strip_prefix("  - ")?.strip_suffix(':'))
        .map(str::to_string)
        .collect()
}

/// The text of one step of a file (from its `  - name:` line to the next step).
pub fn step_body(text: &str, step: &str) -> String {
    let body = text.split(&format!("  - {step}:\n")).nth(1).unwrap();
    body.split("\n  - ").next().unwrap().to_string()
}

/// A copy of `workflows/` in a temp folder with `file` changed by `edit`; returns exit code and output of the check.
pub fn mutated(file: &str, edit: impl Fn(String) -> String) -> (i32, String) {
    let tmp = tempfile::tempdir().unwrap();
    for f in FILES {
        let text = read(f);
        let text = if f == file { edit(text) } else { text };
        fs::write(tmp.path().join(format!("{f}.yaml")), text).unwrap();
    }
    python(&script(), &[tmp.path()])
}

/// A mutation must turn the check red and say `needle`; without `PyYAML` the check says `skipped` (exit 2) instead.
pub fn assert_red(file: &str, needle: &str, edit: impl Fn(String) -> String) {
    let (code, out) = mutated(file, edit);
    if have_yaml() {
        assert_eq!(code, 1, "{out}");
        assert!(out.contains(needle), "wanted {needle:?} in:\n{out}");
    } else {
        assert_eq!(code, 2, "{out}");
    }
}
