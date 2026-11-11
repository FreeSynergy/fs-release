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
fn the_list_is_read_from_the_commit_not_from_the_file_lying_around() {
    let (w, _) = approved_world();
    fs::write(
        w.root().join("rel/.fs/preapproved.toml"),
        "repos = [\"Org/fs-demo\"]\npublic_ok = \"forged\"\n",
    )
    .unwrap();
    w.set_preapproved(&["Org/fs-demo"], "");
    fs::write(
        w.root().join("rel/.fs/preapproved.toml"),
        "repos = [\"Org/fs-demo\"]\npublic_ok = \"forged\"\n",
    )
    .unwrap();
    let o = w.run_names(Reach::Public, &names(&["review"]));
    assert_eq!(
        stopped_at(&o),
        "halt preflight public-locked",
        "{}",
        o.line()
    );
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
        let l = lines
            .iter()
            .find(|l| l.contains(&format!("step={step} ")))
            .unwrap_or_else(|| panic!("no audit line for {step}"));
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
        6,
        "one line per publishing command, none for the others"
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
