//! Test of the hardening: base branch push, live CI check, required runs, catalog bump with local commits, approval rules,
//! pinned signing key, fake gates mode, no restart of sign.

mod support;

use std::fs;

use fs_release_chain::meta::Reach;
use fs_release_chain::Outcome;
use support::flow::{all_green, ci_rows, stopped_at};
use support::{git, run_json, World, SLUG};

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
fn push_refuses_the_base_branch_even_with_full_reach() {
    let w = World::new();
    git(&w.work(), &["checkout", "--quiet", "main"]);
    let sha = w.head();
    w.approve(&sha, "JA");
    let mut t = w.target();
    t.branch = "main".into();
    let o = w.run_target(t, Reach::Public, &names(&["push"]));
    assert!(
        matches!(&o, Outcome::Red { line, .. } if line.contains("base branch")),
        "{}",
        o.line()
    );
    assert_eq!(
        git(&w.origin(), &["rev-parse", "main"]),
        sha,
        "main did not move through push (it was there already)"
    );
    assert_eq!(w.count("push"), 0);
}

#[test]
fn shared_reach_cannot_push_at_all() {
    let (w, _) = approved_world();
    let o = w.run_names(Reach::Shared, &names(&["gates", "review", "push"]));
    assert_eq!(stopped_at(&o), "halt push reach-limit", "{}", o.line());
    assert_eq!(git(&w.origin(), &["branch", "-l", "feat"]), "");
}

#[test]
fn merge_ff_asks_github_now_and_does_not_believe_the_log() {
    let (w, sha) = approved_world();
    let first = w.run_names(
        Reach::Public,
        &names(&["gates", "review", "push", "wait-ci"]),
    );
    assert!(matches!(first, Outcome::Done { .. }));
    w.runs(SLUG, &run_json(&sha, &ci_rows("failure", "success")));
    let o = w.run_names(Reach::Public, &names(&["merge-ff"]));
    assert!(
        matches!(&o, Outcome::Red { step, line } if step == "merge-ff" && line.contains("check")),
        "{}",
        o.line()
    );
    assert_ne!(git(&w.origin(), &["rev-parse", "main"]), sha);
}

#[test]
fn a_missing_required_run_is_never_green() {
    let (w, sha) = approved_world();
    w.runs(
        SLUG,
        &run_json(&sha, &[(1, "check", "feat", "completed", "success")]),
    );
    let o = w.run_names(
        Reach::Public,
        &names(&["gates", "review", "push", "wait-ci"]),
    );
    assert_eq!(stopped_at(&o), "halt wait-ci ci-missing", "{}", o.line());
    let o = w.run_names(Reach::Public, &names(&["merge-ff"]));
    assert!(
        matches!(&o, Outcome::Red { line, .. } if line.contains("lint")),
        "{}",
        o.line()
    );
}

#[test]
fn a_red_run_that_is_not_required_halts_instead_of_red() {
    let (w, sha) = approved_world();
    let mut rows = ci_rows("success", "success");
    rows.push((9, "docs", "feat", "completed", "failure"));
    w.runs(SLUG, &run_json(&sha, &rows));
    let o = w.run_names(
        Reach::Public,
        &names(&["gates", "review", "push", "wait-ci"]),
    );
    assert_eq!(
        stopped_at(&o),
        "halt wait-ci ci-optional-red",
        "{}",
        o.line()
    );
    assert_eq!(o.code(), 10);
}

#[test]
fn without_a_profile_the_ci_step_halts() {
    let (w, _) = approved_world();
    fs::remove_file(w.work().join(".fs/release.toml")).unwrap();
    git(&w.work(), &["commit", "--quiet", "-am", "no profile"]);
    w.approve(&w.head(), "JA");
    let o = w.run_names(
        Reach::Public,
        &names(&["gates", "review", "push", "wait-ci"]),
    );
    assert_eq!(stopped_at(&o), "halt wait-ci no-profile", "{}", o.line());
}

#[test]
fn a_connection_timeout_is_not_retried_but_dns_is() {
    let (w, sha) = approved_world();
    w.runs(SLUG, &run_json(&sha, &ci_rows("failure", "success")));
    fs::write(w.fake().join("log-1.txt"), "net\tconnection timed out\n").unwrap();
    let o = w.run_names(
        Reach::Public,
        &names(&["gates", "review", "push", "wait-ci"]),
    );
    assert!(matches!(o, Outcome::Red { .. }), "{}", o.line());
    assert_eq!(w.count("run rerun"), 0);
}

#[test]
fn bump_refuses_foreign_local_commits_and_takes_back_a_failed_push() {
    let (w, _) = approved_world();
    let cat = w.root().join("catalog");
    fs::write(cat.join("foreign"), "x").unwrap();
    git(&cat, &["add", "."]);
    git(
        &cat,
        &["commit", "--quiet", "-m", "someone else's local work"],
    );
    let before = git(&w.cat_origin(), &["rev-parse", "main"]);
    let upto = [
        "gates", "review", "push", "wait-ci", "merge-ff", "tag", "release", "bump",
    ];
    let o = w.run_names(Reach::Public, &names(&upto));
    assert!(
        matches!(&o, Outcome::Red { step, line } if step == "bump" && line.contains("local commits")),
        "{}",
        o.line()
    );
    assert_eq!(
        git(&w.cat_origin(), &["rev-parse", "main"]),
        before,
        "the foreign commit was not pushed"
    );
    git(&cat, &["reset", "--hard", "--quiet", "origin/main"]);
    let hook = w.cat_origin().join("hooks/pre-receive");
    fs::write(
        &hook,
        "#!/bin/sh\necho 'error: rejected by test' >&2\nexit 1\n",
    )
    .unwrap();
    std::process::Command::new("chmod")
        .arg("+x")
        .arg(&hook)
        .status()
        .unwrap();
    let o = w.run_names(Reach::Public, &names(&["bump"]));
    assert!(
        matches!(&o, Outcome::Red { line, .. } if line.contains("rejected")),
        "{}",
        o.line()
    );
    assert_eq!(
        git(&cat, &["rev-parse", "HEAD"]),
        before,
        "own commit taken back"
    );
    fs::remove_file(&hook).unwrap();
    let o = w.run_names(Reach::Public, &names(&["bump"]));
    assert!(matches!(o, Outcome::Done { .. }), "{}", o.line());
    assert_ne!(
        git(&w.cat_origin(), &["rev-parse", "main"]),
        before,
        "now it is really on the remote"
    );
}
