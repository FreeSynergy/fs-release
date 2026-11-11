//! Test of what the chain trusts: signing key, fake gates mode, approval rules, slug, sign without restart.

mod support;

use std::fs;

use fs_release_chain::cli::check_slug;
use fs_release_chain::meta::Reach;
use fs_release_chain::Outcome;
use support::flow::{all_green, stopped_at};
use support::{git, run_json, World, CAT};

fn names(list: &[&str]) -> Vec<String> {
    list.iter().map(ToString::to_string).collect()
}

fn approved_world() -> (World, String) {
    let w = World::new();
    let sha = w.head();
    w.approve(&sha, "JA");
    all_green(&w, &sha);
    (w, sha)
}

#[test]
fn sign_that_fails_is_red_at_once_without_a_new_start() {
    let (w, _) = approved_world();
    w.runs(
        CAT,
        &run_json("@MAIN@", &[(3, "sign", "main", "completed", "failure")]),
    );
    let o = w.run(Reach::Public);
    assert!(
        matches!(&o, Outcome::Red { step, .. } if step == "sign"),
        "{}",
        o.line()
    );
    assert_eq!(w.count("run rerun"), 0);
    assert_eq!(w.count("workflow run"), 0);
}

#[test]
fn check_wants_the_pinned_key_and_a_good_fs_store() {
    let (w, _) = approved_world();
    fs::write(w.root().join("catalog/catalog.xml.asc"), "SIG-OTHER-KEY").unwrap();
    git(
        &w.root().join("catalog"),
        &["commit", "--quiet", "-am", "other key"],
    );
    git(
        &w.root().join("catalog"),
        &["push", "--quiet", "origin", "main"],
    );
    let o = w.run(Reach::Public);
    assert!(
        matches!(&o, Outcome::Red { step, line } if step == "check" && line.contains("not by the project key")),
        "{}",
        o.line()
    );
    fs::write(w.root().join("catalog/catalog.xml.asc"), "SIG-OK").unwrap();
    git(
        &w.root().join("catalog"),
        &["commit", "--quiet", "-am", "good key"],
    );
    git(
        &w.root().join("catalog"),
        &["push", "--quiet", "origin", "main"],
    );
    fs::write(w.fake().join("store-red"), "").unwrap();
    let o = w.run_names(Reach::Public, &names(&["check"]));
    assert!(
        matches!(&o, Outcome::Red { line, .. } if line.contains("fs-store check")),
        "{}",
        o.line()
    );
    fs::remove_file(w.fake().join("store-red")).unwrap();
    assert!(matches!(
        w.run_names(Reach::Public, &names(&["check"])),
        Outcome::Done { .. }
    ));
}

#[test]
fn fake_gates_cannot_publish_and_count_for_nothing() {
    let w = World::new();
    let mut t = w.target();
    t.fake = true;
    let o = w.run_target(t.clone(), Reach::Public, &names(&["gates"]));
    assert!(
        matches!(&o, Outcome::Red { line, .. } if line.contains("fake")),
        "{}",
        o.line()
    );
    assert_eq!(w.count("gates-ran"), 0);
    let o = w.run_target(t, Reach::Local, &names(&["gates"]));
    assert!(
        matches!(&o, Outcome::Done { summary } if summary.contains("FAKE")),
        "{}",
        o.line()
    );
    let real = w.run_names(Reach::Local, &names(&["gates"]));
    assert!(matches!(real, Outcome::Done { .. }));
    let ran = fs::read_to_string(w.fake().join("gates.log"))
        .unwrap()
        .lines()
        .count();
    assert_eq!(ran, 2, "the real run did not trust the fake one");
}

#[test]
fn approval_rules_expiry_findings_author_and_the_fixed_folder() {
    let w = World::new();
    let sha = w.head();
    w.approve_on(&sha, "JA", "2026-11-01");
    assert_eq!(
        stopped_at(&w.run_names(Reach::Local, &names(&["review"]))),
        "halt review approval-expired"
    );
    w.approve_on(&sha, "JA", "2026-12-01");
    assert_eq!(
        stopped_at(&w.run_names(Reach::Local, &names(&["review"]))),
        "halt review approval-expired",
        "from the future"
    );
    w.approve(&sha, "JA");
    fs::write(w.root().join("findings.md"), "changed after approval\n").unwrap();
    assert_eq!(
        stopped_at(&w.run_names(Reach::Local, &names(&["review"]))),
        "halt review approval-findings"
    );
    w.approve(&sha, "JA");
    let f = w.root().join("freigaben/fs-demo").join(format!("{sha}.ok"));
    let text = fs::read_to_string(&f).unwrap();
    fs::write(&f, text.replace("by: opus\n", "")).unwrap();
    assert!(
        matches!(
            w.run_names(Reach::Local, &names(&["review"])),
            Outcome::Red { .. }
        ),
        "`by` is required"
    );
    fs::write(&f, text.replace("by: opus", "by: \"\"")).unwrap();
    assert_eq!(
        stopped_at(&w.run_names(Reach::Local, &names(&["review"]))),
        "halt review approval-incomplete"
    );
    fs::write(&f, &text).unwrap();
    fs::write(w.work().join(".opus-ok"), &text).unwrap();
    assert!(matches!(
        w.run_names(Reach::Local, &names(&["review"])),
        Outcome::Done { .. }
    ));
    fs::remove_file(&f).unwrap();
    assert_eq!(
        stopped_at(&w.run_names(Reach::Local, &names(&["review"]))),
        "halt review review",
        "a file in the worktree counts for nothing"
    );
}

#[test]
fn slug_must_be_the_origin() {
    assert!(check_slug(
        "FreeSynergy/fs-store",
        "https://github.com/FreeSynergy/fs-store.git"
    )
    .is_ok());
    assert!(check_slug(
        "FreeSynergy/fs-store",
        "git@github.com:FreeSynergy/fs-store.git"
    )
    .is_ok());
    assert!(check_slug(
        "FreeSynergy/fs-store",
        "https://github.com/FreeSynergy/fs-tasks"
    )
    .is_err());
    assert!(check_slug("FreeSynergy/fs-store", "/tmp/some/path").is_err());
}
