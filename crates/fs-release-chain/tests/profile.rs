//! The chain file and the program say the same; the printed forms are one short line.

use std::path::Path;
use std::process::Command;

use fs_release_chain::chain::Chain;
use fs_release_chain::outcome::Outcome;
use fs_release_chain::registry;

fn file() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../chains/release.chain.yaml")
}

#[test]
fn chain_file_equals_the_commands_in_the_program() {
    let chain = Chain::load(&file()).unwrap();
    let code: Vec<&str> = registry::all().iter().map(|s| s.meta().name).collect();
    assert_eq!(chain.names(), code);
}

#[test]
fn a_chain_file_that_differs_is_refused() {
    let text = std::fs::read_to_string(file()).unwrap().replace(
        "risk: publish, reach: shared",
        "risk: view,    reach: shared",
    );
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("x.yaml");
    std::fs::write(&p, text).unwrap();
    assert!(Chain::load(&p).unwrap_err().contains("push"));
}

#[test]
fn every_command_has_risk_reach_and_a_whole_sentence_preview() {
    for s in registry::all() {
        let m = s.meta();
        assert!(m.preview.ends_with('.') && !m.undo.is_empty(), "{}", m.name);
    }
}

#[test]
fn outcome_lines_are_one_line_with_the_exit_codes() {
    let halt = Outcome::Halt {
        step: "review".into(),
        reason: "approval-stale".into(),
        needs: "a new\nJA".into(),
    };
    let red = Outcome::Red {
        step: "gates".into(),
        line: "x".repeat(500),
    };
    let done = Outcome::Done {
        summary: "ok".into(),
    };
    assert_eq!((done.code(), halt.code(), red.code()), (0, 10, 1));
    assert!(halt
        .line()
        .starts_with("HALT review approval-stale needs: a new JA"));
    assert!(red.line().chars().count() <= 220 && !red.line().contains('\n'));
}

#[test]
fn binary_verifies_the_chain_file() {
    let o = Command::new(env!("CARGO_BIN_EXE_fs-release-chain"))
        .arg("verify")
        .arg("--chain")
        .arg(file())
        .output()
        .unwrap();
    assert!(o.status.success());
    assert!(String::from_utf8_lossy(&o.stdout).starts_with("DONE chain release"));
}
