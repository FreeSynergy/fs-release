//! Pattern: none (test). `scripts/gates-files.sh`, the part of `gates.sh` that evaluates the exit codes of `check_yaml.py` and
//! `check_workflows.py`: a broken workflow file or `workflows/` file makes it exit 1 and say which check failed. Limit: the
//! "no `PyYAML`" path (exit 2 of the scripts, warning only) is not run here; the scripts' own tests cover their exit 2.

mod support;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use support::have_yaml;

const GOOD: &str = "name: ci\non: push\njobs:\n  a:\n    runs-on: ubuntu-latest\n    steps:\n      - name: Build\n        run: echo hi\n";
const BAD: &str = "name: ci\non: [push\n";

fn script() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/gates-files.sh")
}

/// A repo root with a good `.github/workflows/ci.yml`.
fn repo() -> tempfile::TempDir {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join(".github/workflows");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("ci.yml"), GOOD).unwrap();
    tmp
}

/// Exit code and stderr of `gates-files.sh <root>`.
fn run(root: &Path) -> (i32, String) {
    let o = Command::new("bash")
        .arg(script())
        .arg(root)
        .output()
        .expect("bash");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stderr).into_owned(),
    )
}

#[test]
fn a_good_repo_passes() {
    let tmp = repo();
    let (code, err) = run(tmp.path());
    assert_eq!(code, 0, "{err}");
}

#[test]
fn a_broken_workflow_file_is_red_at_yaml_check() {
    if !have_yaml() {
        return;
    }
    let tmp = repo();
    fs::write(tmp.path().join(".github/workflows/ci.yml"), BAD).unwrap();
    let (code, err) = run(tmp.path());
    assert_eq!(code, 1, "{err}");
    assert!(err.contains("gates FAILED: yaml-check"), "{err}");
}

#[test]
fn a_broken_file_in_workflows_is_red_at_the_schema_check() {
    if !have_yaml() {
        return;
    }
    let tmp = repo();
    fs::create_dir_all(tmp.path().join("workflows")).unwrap();
    fs::write(tmp.path().join("workflows/x.yaml"), BAD).unwrap();
    let (code, err) = run(tmp.path());
    assert_eq!(code, 1, "{err}");
    assert!(err.contains("gates FAILED: workflows/ schema check"), "{err}");
}
