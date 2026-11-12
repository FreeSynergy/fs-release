//! Pattern: none (test). `fs-release lift` against throw-away repos: green, wrong tag, two Blinc sources, red gates, mutation.

mod support;

use fs_release::lift::{self, Report};
use fs_release_chain::sys::SystemRunner;
use std::fs;
use support::{git, World};

fn step<'a>(r: &'a Report, name: &str) -> &'a fs_release::task::Record {
    r.steps
        .iter()
        .find(|s| s.name == name)
        .unwrap_or_else(|| panic!("no step {name}"))
}

fn run(w: &World, o: &lift::LiftOpts) -> (Report, i32) {
    let _ = w;
    lift::run(o, &SystemRunner::default())
}

#[test]
fn green_lift_moves_every_crate_to_the_given_tags_and_writes_the_report() {
    let w = World::new();
    let (r, code) = run(&w, &w.opts());
    assert_eq!(code, 0, "{}", r.markdown());
    assert!(r.steps.iter().all(|s| s.status == "ok"), "{}", r.markdown());
    assert_eq!(r.steps.len(), 7);
    for f in ["Cargo.toml", "crates/a/Cargo.toml", "crates/b/Cargo.toml"] {
        let t = w.read(f);
        assert!(!t.contains("v0.1.0"), "{f} still on the old tag:\n{t}");
    }
    assert!(
        w.read("Cargo.toml").contains("v0.2.0") && w.read("crates/b/Cargo.toml").contains("v0.2.0")
    );
    assert!(w.read("crates/a/Cargo.toml").contains("v0.2.0"));
    let lock = w.read("Cargo.lock");
    assert!(
        lock.contains("fs-components?tag=v0.2.0#") && lock.contains("fs-interfaces?tag=v0.2.0#")
    );
    assert!(r
        .pins
        .iter()
        .any(|p| p.repo == "fs-components" && p.sha == w.sha("fs-components", "v0.2.0")));
    assert!(r
        .pins
        .iter()
        .any(|p| p.repo == "Blinc" && p.tag == "fs-blinc-3"));
    let json: serde_json::Value =
        serde_json::from_str(&w.read(".fs-release/lift-report.json")).unwrap();
    assert_eq!(json["ok"], true);
    assert_eq!(json["head"], git(&w.work(), &["rev-parse", "HEAD"]));
    assert!(w.read(".fs-release/lift-report.md").contains("GREEN"));
    // Not committed: Cargo.toml and Cargo.lock are modified, the report folder ignores itself.
    let status = git(&w.work(), &["status", "--porcelain"]);
    assert!(
        status.contains("Cargo.lock")
            && status.contains("Cargo.toml")
            && !status.contains(".fs-release"),
        "{status}"
    );
}

#[test]
fn a_tag_that_does_not_exist_is_red_at_the_lock_and_the_rest_is_skipped() {
    let w = World::new();
    let mut o = w.opts();
    o.components = Some("v9.9.9".into());
    let (r, code) = run(&w, &o);
    assert_eq!(code, 1);
    assert_eq!(step(&r, "deps").status, "ok");
    let lock = step(&r, "lock");
    assert_eq!(lock.status, "red");
    assert_ne!(lock.first_error.clone().unwrap(), "");
    assert_eq!(step(&r, "tests").status, "skipped");
    assert!(w.read(".fs-release/lift-report.md").contains("RED"));
}

#[test]
fn a_tag_that_is_not_vx_y_z_is_red_at_deps_and_nothing_is_edited() {
    let w = World::new();
    let mut o = w.opts();
    o.components = Some("main".into());
    let (r, code) = run(&w, &o);
    assert_eq!(code, 1);
    assert_eq!(step(&r, "deps").status, "red");
    assert_eq!(git(&w.work(), &["status", "--porcelain"]), "");
}

#[test]
fn nothing_to_lift_is_red() {
    let w = World::new();
    let mut o = w.opts();
    o.components = None;
    o.interfaces = None;
    assert_eq!(run(&w, &o).1, 1);
}

#[test]
fn two_blinc_sources_in_the_lock_are_red_even_when_both_are_approved() {
    let w = World::new();
    // The program takes a second Blinc source on its own (a fork crate from another approved tag).
    let t = w.read("crates/b/Cargo.toml");
    let extra =
        "blinc-fork = { git = \"https://github.com/FreeSynergy/Blinc\", tag = \"fs-blinc-4\" }\n";
    fs::write(w.work().join("crates/b/Cargo.toml"), format!("{t}{extra}")).unwrap();
    git(&w.work(), &["commit", "--quiet", "-am", "second blinc"]);
    let (r, code) = run(&w, &w.opts());
    assert_eq!(code, 1);
    assert_eq!(step(&r, "lock").status, "ok");
    let gate = step(&r, "lock-gate");
    assert_eq!(gate.status, "red", "{}", r.markdown());
    assert!(gate
        .first_error
        .clone()
        .unwrap()
        .to_lowercase()
        .contains("blinc"));
}

#[test]
fn a_blinc_tag_that_is_not_approved_is_red_and_the_same_world_is_green_when_it_is() {
    let w = World::new();
    w.approve_blinc(&["fs-blinc-4"]);
    let (r, code) = run(&w, &w.opts());
    assert_eq!(code, 1);
    assert_eq!(step(&r, "lock-gate").status, "red");
    w.approve_blinc(&["fs-blinc-3"]);
    git(&w.work(), &["checkout", "--quiet", "."]);
    assert_eq!(run(&w, &w.opts()).1, 0);
}

#[test]
fn red_checks_are_all_reported_with_their_first_error_line() {
    let w = World::new();
    // fmt red (bad layout), rules red (no module doc), tests red (failing test); clippy is red through the compile error? no: it stays green.
    fs::write(w.work().join("crates/a/src/answer.rs"), "//! Pattern: none (test crate).\n\n/// The answer.\n#[must_use]\npub fn answer() -> u32 {\n42\n}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn is_42() {\n        assert_eq!(super::answer(), 41);\n    }\n}\n").unwrap();
    fs::write(
        w.work().join("crates/b/src/extra.rs"),
        "pub const X: u32 = 1;\n",
    )
    .unwrap();
    let lib = w.read("crates/b/src/lib.rs");
    fs::write(
        w.work().join("crates/b/src/lib.rs"),
        format!("{lib}pub mod extra;\n"),
    )
    .unwrap();
    let (r, code) = run(&w, &w.opts());
    assert_eq!(code, 1);
    for name in ["fmt", "rules", "tests"] {
        let s = step(&r, name);
        assert_eq!(s.status, "red", "{name}: {}", r.markdown());
        assert_ne!(s.first_error.clone().unwrap(), "");
    }
    assert!(
        step(&r, "tests")
            .first_error
            .clone()
            .unwrap()
            .contains("FAILED")
            || step(&r, "tests")
                .first_error
                .clone()
                .unwrap()
                .contains("panicked")
    );
    assert_eq!(step(&r, "lock-gate").status, "ok");
    let md = w.read(".fs-release/lift-report.md");
    assert!(md.contains("RED") && md.contains("| fmt | red"));
}

#[test]
fn commit_after_green_commits_the_manifests_but_not_the_local_tags_lock() {
    let w = World::new();
    let mut o = w.opts();
    o.commit = true;
    let (r, code) = run(&w, &o);
    assert_eq!(code, 0, "{}", r.markdown());
    let files = git(&w.work(), &["show", "--name-only", "--format=", "HEAD"]);
    assert!(
        files.contains("crates/a/Cargo.toml") && !files.contains("Cargo.lock"),
        "{files}"
    );
    assert_eq!(r.head, git(&w.work(), &["rev-parse", "HEAD"]));
}

#[test]
fn a_red_check_blocks_the_commit() {
    let w = World::new();
    fs::write(w.work().join("crates/a/src/answer.rs"), "pub fn x() {}\n").unwrap();
    let mut o = w.opts();
    o.commit = true;
    let before = git(&w.work(), &["rev-parse", "HEAD"]);
    let (r, code) = run(&w, &o);
    assert_eq!(code, 1);
    assert_eq!(step(&r, "commit").status, "skipped");
    assert_eq!(git(&w.work(), &["rev-parse", "HEAD"]), before);
}

#[test]
fn online_commit_includes_the_lock() {
    let w = World::new();
    let mut o = w.online_opts();
    o.commit = true;
    let (r, code) = run(&w, &o);
    assert_eq!(code, 0, "{}", r.markdown());
    assert!(git(&w.work(), &["show", "--name-only", "--format=", "HEAD"]).contains("Cargo.lock"));
    assert_eq!(
        git(
            &w.work(),
            &["status", "--porcelain", "--untracked-files=no"]
        ),
        ""
    );
}
