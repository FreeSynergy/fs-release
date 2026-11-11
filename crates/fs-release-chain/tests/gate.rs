//! Test of the doors in front of the chain: pre-approved repos, public lock, audit log, slug, clean worktree, profile from the commit.

mod support;

use std::fs;

use fs_release_chain::cli::main_with;
use fs_release_chain::meta::Reach;
use fs_release_chain::Outcome;
use support::flow::{all_green, stopped_at};
use support::{git, World};

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
fn repos_outside_the_list_halt_and_public_stays_locked_until_the_entry_exists() {
    let (w, _) = approved_world();
    w.set_preapproved(&["Org/other"], "test-ok");
    let o = w.run_names(Reach::Shared, &names(&["gates", "wait-ci"]));
    assert_eq!(
        stopped_at(&o),
        "halt preflight not-preapproved",
        "{}",
        o.line()
    );
    assert!(
        matches!(
            w.run_names(Reach::Local, &names(&["gates"])),
            Outcome::Done { .. }
        ),
        "a local run needs no list"
    );
    w.set_preapproved(&["Org/fs-demo"], "");
    let o = w.run_names(Reach::Public, &names(&["gates", "review", "push"]));
    assert_eq!(
        stopped_at(&o),
        "halt preflight public-locked",
        "{}",
        o.line()
    );
    assert_eq!(git(&w.origin(), &["branch", "-l", "feat"]), "");
    w.set_preapproved(&["Org/fs-demo"], "opus-ok-ref");
    assert!(matches!(
        w.run_names(Reach::Public, &names(&["gates", "review", "push"])),
        Outcome::Done { .. }
    ));
}

#[test]
fn the_list_comes_from_origin_not_from_a_local_commit_or_file() {
    let (w, _) = approved_world();
    w.set_preapproved(&["Org/fs-demo"], "");
    w.write_preapproved(&["Org/fs-demo"], &["Org/fs-catalog"], "forged", false);
    let o = w.run_names(Reach::Public, &names(&["review"]));
    assert_eq!(
        stopped_at(&o),
        "halt preflight public-locked",
        "a local commit counts for nothing: {}",
        o.line()
    );
    w.set_preapproved(&["Org/other"], "ok");
    w.write_preapproved(
        &["Org/fs-demo", "Org/other"],
        &["Org/fs-catalog"],
        "ok",
        false,
    );
    let o = w.run_names(Reach::Shared, &names(&["review"]));
    assert_eq!(
        stopped_at(&o),
        "halt preflight not-preapproved",
        "a local list counts for nothing: {}",
        o.line()
    );
}

#[test]
fn a_release_dir_that_is_not_fs_release_is_red() {
    let (w, _) = approved_world();
    let rel = w.root().join("rel");
    git(
        &rel,
        &[
            "config",
            "remote.origin.url",
            "https://github.com/Evil/fs-release.git",
        ],
    );
    let o = w.run_names(Reach::Shared, &names(&["review"]));
    assert!(
        matches!(&o, Outcome::Red { line, .. } if line.contains("not FreeSynergy/fs-release")),
        "{}",
        o.line()
    );
    git(
        &rel,
        &[
            "config",
            "remote.origin.url",
            "https://github.com/FreeSynergy/fs-release.git",
        ],
    );
    git(
        &rel,
        &[
            "config",
            "remote.origin.pushurl",
            "https://github.com/Evil/fs-release.git",
        ],
    );
    let o = w.run_names(Reach::Shared, &names(&["review"]));
    assert!(
        matches!(&o, Outcome::Red { line, .. } if line.contains("two repos")),
        "{}",
        o.line()
    );
}

#[test]
fn the_catalog_repo_must_be_a_listed_target_and_the_checkout_its_origin() {
    let (w, _) = approved_world();
    w.write_preapproved(&["Org/fs-demo"], &["Org/not-the-catalog"], "ok", true);
    let o = w.run_names(Reach::Public, &names(&["review"]));
    assert_eq!(
        stopped_at(&o),
        "halt preflight catalog-not-preapproved",
        "{}",
        o.line()
    );
    w.set_preapproved(&["Org/fs-demo"], "ok");
    let cat = w.root().join("catalog");
    git(
        &cat,
        &[
            "config",
            "remote.origin.pushurl",
            "https://github.com/Evil/fs-catalog.git",
        ],
    );
    let up = [
        "gates", "review", "push", "wait-ci", "merge-ff", "tag", "release", "bump",
    ];
    let o = w.run_names(Reach::Public, &names(&up));
    assert!(
        matches!(&o, Outcome::Red { step, line } if step == "bump" && line.contains("two repos")),
        "{}",
        o.line()
    );
    assert_eq!(
        git(&w.cat_origin(), &["log", "-1", "--format=%s"]),
        "base",
        "nothing pushed to the catalog"
    );
}

#[test]
fn an_audit_log_that_cannot_be_written_stops_before_the_step() {
    let (w, _) = approved_world();
    fs::write(w.root().join("audit"), "a file where the folder should be").unwrap();
    let o = w.run_names(Reach::Public, &names(&["gates", "review", "push"]));
    assert!(
        matches!(&o, Outcome::Red { step, line } if step == "push" && line.contains("audit")),
        "{}",
        o.line()
    );
    assert_eq!(
        git(&w.origin(), &["branch", "-l", "feat"]),
        "",
        "the step did not run"
    );
    assert_eq!(w.count("push"), 0);
}

#[test]
fn every_publication_leaves_an_audit_line_and_the_log_only_grows() {
    let (w, sha) = approved_world();
    let log = w.root().join("audit/AUDIT.log");
    fs::create_dir_all(log.parent().unwrap()).unwrap();
    fs::write(&log, "older line\n").unwrap();
    let o = w.run(Reach::Public);
    assert!(matches!(o, Outcome::Done { .. }), "{}", o.line());
    let text = fs::read_to_string(&log).unwrap();
    assert!(text.starts_with("older line\n"));
    let lines: Vec<&str> = text.lines().skip(1).collect();
    for step in ["push", "merge-ff", "tag", "release", "bump", "sign"] {
        let start = lines
            .iter()
            .position(|l| l.contains(&format!("step={step} ")) && l.contains("result=start"))
            .unwrap_or_else(|| panic!("no start line for {step}"));
        let l = lines
            .iter()
            .skip(start + 1)
            .find(|l| l.contains(&format!("step={step} ")))
            .unwrap_or_else(|| panic!("no result line for {step}"));
        assert!(
            l.contains("by=opus")
                && l.contains(&format!("sha={sha}"))
                && l.contains("reach=public")
                && l.contains("result=ok")
                && l.contains("findings="),
            "{l}"
        );
        assert!(l.starts_with("2026-11-12T01:00:00Z "), "{l}");
    }
    assert_eq!(
        lines.len(),
        12,
        "a start and a result line per publishing command, none for the others"
    );
}

#[test]
fn a_failed_publication_is_audited_too() {
    let (w, _) = approved_world();
    git(
        &w.work(),
        &["push", "--quiet", "origin", "main:refs/tags/v0.0.9"],
    );
    let o = w.run_names(
        Reach::Public,
        &names(&["gates", "review", "push", "wait-ci", "merge-ff", "tag"]),
    );
    assert!(matches!(o, Outcome::Red { .. }), "{}", o.line());
    let text = fs::read_to_string(w.root().join("audit/AUDIT.log")).unwrap();
    assert!(
        text.contains("step=tag ") && text.contains("result=red"),
        "{text}"
    );
}

#[test]
fn slug_with_dots_or_slashes_is_red_and_never_reaches_a_path() {
    for bad in [
        "x/..",
        "../x",
        "a/b/c",
        "x/.hidden",
        "x",
        "a b/c",
        "x/y;z",
        "",
    ] {
        assert!(fs_release_chain::slug::validate(bad).is_err(), "{bad:?}");
    }
    assert!(fs_release_chain::slug::validate("FreeSynergy/fs-store").is_ok());
    let w = World::new();
    let code = main_with(
        [
            "fs-release-chain",
            "step",
            "review",
            "--version",
            "0.0.9",
            "--slug",
            "x/..",
            "--repo-dir",
            &w.work().display().to_string(),
        ]
        .map(String::from)
        .to_vec(),
    );
    assert_eq!(code, 1);
    let code = main_with(
        [
            "fs-release-chain",
            "step",
            "review",
            "--version",
            "0.0.9",
            "--slug",
            "Other/fs-demo",
            "--repo-dir",
            &w.work().display().to_string(),
        ]
        .map(String::from)
        .to_vec(),
    );
    assert_eq!(code, 1, "a slug that is not the origin");
}
