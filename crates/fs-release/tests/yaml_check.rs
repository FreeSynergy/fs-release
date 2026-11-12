//! Pattern: none (test). The `yaml-check` step of the lift and `scripts/check_yaml.py`: a workflow file that is not YAML is red
//! with `file:line`, a valid one is green, a missing `PyYAML` is `skipped` (never green).

mod support;

use std::fs;
use std::path::{Path, PathBuf};

use fs_release::lift;
use fs_release::task::{run_all, Task, SKIPPED};
use fs_release_chain::outcome::Stop;
use support::{have_yaml, python, World};

const GOOD: &str = "name: ci\non: push\njobs:\n  a:\n    runs-on: ubuntu-latest\n    steps:\n      - name: Build (debug, the test x)\n        run: echo hi\n";
/// The finding: an unquoted `: ` inside a step name (line 7).
const BAD: &str = "name: ci\non: push\njobs:\n  a:\n    runs-on: ubuntu-latest\n    steps:\n      - name: Build (debug, the test: x)\n        run: echo hi\n";

fn script() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/check_yaml.py")
}

fn put_workflow(root: &Path, text: &str) {
    let dir = root.join(".github/workflows");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("ci.yml"), text).unwrap();
}

fn step<'a>(r: &'a lift::Report, name: &str) -> &'a fs_release::task::Record {
    r.steps.iter().find(|s| s.name == name).unwrap()
}

#[test]
fn script_green_on_a_valid_file_and_red_with_file_and_line_on_the_broken_one() {
    let tmp = tempfile::tempdir().unwrap();
    put_workflow(tmp.path(), GOOD);
    let (code, out) = python(&script(), &[tmp.path()]);
    if !have_yaml() {
        assert_eq!(code, 2, "{out}");
        return;
    }
    assert_eq!(code, 0, "{out}");
    put_workflow(tmp.path(), BAD);
    let (code, out) = python(&script(), &[tmp.path()]);
    assert_eq!(code, 1, "{out}");
    assert!(out.starts_with(".github/workflows/ci.yml:7:"), "{out}");
}

#[test]
fn script_without_the_yaml_module_is_skipped_with_a_warning_not_green() {
    let tmp = tempfile::tempdir().unwrap();
    put_workflow(tmp.path(), BAD);
    let s = script();
    let arg = Path::new("--simulate-missing-yaml");
    let (code, out) = python(&s, &[tmp.path(), arg]);
    assert_eq!(code, 2, "{out}");
    assert!(out.starts_with("skipped:") && out.contains("NOT checked"), "{out}");
}

#[test]
fn a_repo_without_workflow_files_is_green() {
    let tmp = tempfile::tempdir().unwrap();
    let (code, out) = python(&script(), &[tmp.path()]);
    assert_eq!(code, if have_yaml() { 0 } else { 2 }, "{out}");
}

#[test]
fn lift_is_red_at_yaml_check_for_the_broken_file_and_green_for_the_valid_one() {
    let w = World::new();
    put_workflow(&w.work(), GOOD);
    let (r, code) = lift::run(&w.opts(), &fs_release_chain::sys::SystemRunner::default());
    assert_eq!(code, 0, "{}", r.markdown());
    let s = step(&r, "yaml-check");
    assert_eq!(s.status, if have_yaml() { "ok" } else { "skipped" }, "{s:?}");
    assert_eq!(r.steps.len(), 9);
    if !have_yaml() {
        return;
    }
    put_workflow(&w.work(), BAD);
    let mut o = w.opts();
    o.commit = true;
    let (r, code) = lift::run(&o, &fs_release_chain::sys::SystemRunner::default());
    assert_eq!(code, 1, "{}", r.markdown());
    let s = step(&r, "yaml-check");
    assert_eq!(s.status, "red");
    assert!(
        s.first_error.clone().unwrap().starts_with(".github/workflows/ci.yml:7:"),
        "{s:?}"
    );
    assert_eq!(step(&r, "tests").status, "ok");
    assert_eq!(step(&r, "commit").status, "skipped");
}

struct Unable;

impl Task<()> for Unable {
    fn name(&self) -> &'static str {
        "unable"
    }

    fn run(&self, (): &()) -> Result<String, Stop> {
        Ok(format!("{SKIPPED}no tool"))
    }
}

#[test]
fn a_step_that_returns_the_skipped_marker_is_recorded_skipped_and_does_not_stop_the_run() {
    let list: Vec<Box<dyn Task<()>>> = vec![Box::new(Unable)];
    let (records, first) = run_all(&list, &());
    assert!(first.is_none());
    assert_eq!(records[0].status, "skipped");
    assert_eq!(records[0].detail, "no tool");
    assert!(!records[0].is_ok());
}
