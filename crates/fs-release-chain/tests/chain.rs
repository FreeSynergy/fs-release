//! Test of the whole chain against local remotes, a fake `gh` and a virtual clock (nothing here touches GitHub): the paths that publish or fail late.

mod support;

use std::fs;

use fs_release_chain::meta::Reach;
use fs_release_chain::Outcome;
use support::flow::{all_green, stopped_at};
use support::{git, run_json, World, SLUG};

#[test]
fn green_chain_runs_to_done_and_publishes_everything() {
    let w = World::new();
    let sha = w.head();
    w.approve(&sha, "JA");
    all_green(&w, &sha);
    let o = w.run(Reach::Public);
    assert!(matches!(o, Outcome::Done { .. }), "{}", o.line());
    assert_eq!(o.code(), 0);
    assert!(o.line().starts_with("DONE ") && !o.line().contains('\n'));
    assert_eq!(
        git(&w.origin(), &["rev-parse", "main"]),
        sha,
        "main moved to the checked commit"
    );
    assert_eq!(
        git(&w.origin(), &["rev-parse", "v0.0.9^{commit}"]),
        sha,
        "tag is on that commit"
    );
    assert!(fs::read_to_string(w.root().join("catalog/packages.toml"))
        .unwrap()
        .contains("tag = \"v0.0.9\""));
    assert_eq!(
        git(&w.cat_origin(), &["log", "-1", "--format=%s"]),
        "catalog: fs-demo 0.0.9"
    );
    assert!(
        fs::read_to_string(w.fake().join("release-Org_fs-demo-v0.0.9.json"))
            .unwrap()
            .contains("\"isDraft\":false")
    );
    let report = fs::read_to_string(w.root().join("state/report-0.0.9.txt")).unwrap();
    assert!(report.lines().count() <= 10 && report.contains("tag:") && report.contains("sign:"));
}

#[test]
fn red_ci_halts_before_anything_public() {
    let w = World::new();
    let sha = w.head();
    w.approve(&sha, "JA");
    all_green(&w, &sha);
    w.runs(
        SLUG,
        &run_json(&sha, &[(1, "check", "feat", "completed", "failure")]),
    );
    fs::write(
        w.fake().join("log-1.txt"),
        "step\terror: test parser::tests::it failed\n",
    )
    .unwrap();
    let o = w.run(Reach::Public);
    assert!(
        matches!(&o, Outcome::Red { step, line } if step == "wait-ci" && line.contains("parser::tests")),
        "{}",
        o.line()
    );
    assert_eq!(o.code(), 1);
    assert_ne!(
        git(&w.origin(), &["rev-parse", "main"]),
        sha,
        "main must not move"
    );
    assert!(git(&w.origin(), &["tag", "-l"]).is_empty(), "no tag");
    assert_eq!(w.count("release edit"), 0);
    assert_eq!(w.count("rerun"), 0, "a code failure is not retried");
}

#[test]
fn repeat_after_a_partial_run_goes_on_and_a_full_repeat_changes_nothing() {
    let w = World::new();
    let sha = w.head();
    w.approve(&sha, "JA");
    all_green(&w, &sha);
    let o = w.run(Reach::Shared);
    assert_eq!(stopped_at(&o), "halt merge-ff reach-limit", "{}", o.line());
    assert_eq!(git(&w.origin(), &["rev-parse", "feat"]), sha);
    let polls = w.count("run list");
    let o = w.run(Reach::Public);
    assert!(matches!(o, Outcome::Done { .. }), "{}", o.line());
    assert_eq!(w.count("gates-ran"), 0);
    assert_eq!(
        fs::read_to_string(w.fake().join("gates.log"))
            .unwrap()
            .lines()
            .count(),
        1,
        "gates ran once for this commit"
    );
    assert!(w.count("run list") > polls);
    let (edits, tags, cat) = (
        w.count("release edit"),
        git(&w.origin(), &["tag", "-l"]),
        git(&w.cat_origin(), &["rev-list", "--count", "main"]),
    );
    let again = w.run(Reach::Public);
    assert!(matches!(again, Outcome::Done { .. }), "{}", again.line());
    assert_eq!(
        (
            w.count("release edit"),
            git(&w.origin(), &["tag", "-l"]),
            git(&w.cat_origin(), &["rev-list", "--count", "main"])
        ),
        (edits, tags, cat)
    );
    assert_eq!(w.count("workflow run"), 0, "sign is not started again");
}

#[test]
fn runner_dns_failure_is_retried_once_and_then_green() {
    let w = World::new();
    let sha = w.head();
    w.approve(&sha, "JA");
    all_green(&w, &sha);
    w.runs(
        SLUG,
        &run_json(&sha, &[(1, "check", "feat", "completed", "failure")]),
    );
    fs::write(
        w.fake().join("runs-Org_fs-demo.json.rerun"),
        run_json(
            &sha,
            &[
                (1, "check", "feat", "completed", "success"),
                (2, "release", "v0.0.9", "completed", "success"),
            ],
        ),
    )
    .unwrap();
    fs::write(
        w.fake().join("log-1.txt"),
        "apt\tTemporary failure in name resolution\n",
    )
    .unwrap();
    let o = w.run(Reach::Public);
    assert!(matches!(o, Outcome::Done { .. }), "{}", o.line());
    assert_eq!(w.count("run rerun"), 1);
}

#[test]
fn tag_on_another_commit_is_red_and_never_moved() {
    let w = World::new();
    let sha = w.head();
    git(
        &w.work(),
        &["push", "--quiet", "origin", "main:refs/tags/v0.0.9"],
    );
    w.approve(&sha, "JA");
    all_green(&w, &sha);
    let o = w.run(Reach::Public);
    assert!(
        matches!(&o, Outcome::Red { step, line } if step == "tag" && line.contains("exists")),
        "{}",
        o.line()
    );
}

#[test]
fn draft_release_in_the_catalog_step_is_red_and_check_needs_a_good_signature() {
    let w = World::new();
    let sha = w.head();
    w.approve(&sha, "JA");
    all_green(&w, &sha);
    fs::write(w.root().join("catalog/catalog.xml.asc"), "BAD").unwrap();
    git(
        &w.root().join("catalog"),
        &["commit", "--quiet", "-am", "bad sig"],
    );
    git(
        &w.root().join("catalog"),
        &["push", "--quiet", "origin", "main"],
    );
    let o = w.run(Reach::Public);
    assert!(
        matches!(&o, Outcome::Red { step, line } if step == "check" && line.contains("signature")),
        "{}",
        o.line()
    );
}
