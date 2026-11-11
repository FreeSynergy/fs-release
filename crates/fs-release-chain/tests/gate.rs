//! Test of the doors in front of the chain: pre-approved repos, public lock, audit log, slug, clean worktree, profile from the commit.

mod support;

use std::fs;

use fs_release_chain::approval::ApprovalVerifier;
use fs_release_chain::cli::main_with;
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

#[test]
fn approval_files_that_are_links_are_refused() {
    let w = World::new();
    let sha = w.head();
    w.approve(&sha, "JA");
    let dir = w.root().join("freigaben/fs-demo");
    let real = dir.join(format!("{sha}.ok"));
    let other = w.root().join("elsewhere.ok");
    fs::rename(&real, &other).unwrap();
    std::os::unix::fs::symlink(&other, &real).unwrap();
    let o = w.run_names(Reach::Local, &names(&["review"]));
    assert!(
        matches!(&o, Outcome::Red { line, .. } if line.contains("symbolic link")),
        "{}",
        o.line()
    );
    fs::remove_file(&real).unwrap();
    fs::rename(&other, &real).unwrap();
    let moved = w.root().join("freigaben/real-folder");
    fs::rename(&dir, &moved).unwrap();
    std::os::unix::fs::symlink(&moved, &dir).unwrap();
    let o = w.run_names(Reach::Local, &names(&["review"]));
    assert!(
        matches!(&o, Outcome::Red { line, .. } if line.contains("symbolic link")),
        "{}",
        o.line()
    );
}

#[test]
fn release_branch_equal_to_the_base_is_red_before_anything_runs() {
    let w = World::new();
    git(&w.work(), &["checkout", "--quiet", "main"]);
    let mut t = w.target();
    t.branch = "main".into();
    let o = w.run_target(t, Reach::Local, &names(&["gates"]));
    assert!(
        matches!(&o, Outcome::Red { step, .. } if step == "preflight"),
        "{}",
        o.line()
    );
    assert_eq!(w.count("gates-ran"), 0);
}

#[test]
fn a_dirty_worktree_stops_every_command_from_shared_reach_up() {
    let (w, _) = approved_world();
    fs::write(w.work().join("feature"), "changed after the review\n").unwrap();
    let o = w.run_names(Reach::Shared, &names(&["wait-ci"]));
    assert!(
        matches!(&o, Outcome::Red { line, .. } if line.contains("uncommitted")),
        "{}",
        o.line()
    );
    assert!(
        matches!(
            w.run_names(Reach::Local, &names(&["report"])),
            Outcome::Done { .. }
        ),
        "local commands do not care"
    );
}

#[test]
fn the_profile_comes_from_the_commit_not_from_the_working_tree() {
    let (w, sha) = approved_world();
    git(
        &w.work(),
        &["update-index", "--assume-unchanged", ".fs/release.toml"],
    );
    fs::write(
        w.work().join(".fs/release.toml"),
        "required = [\"check\"]\n",
    )
    .unwrap();
    w.runs(
        SLUG,
        &run_json(&sha, &[(1, "check", "feat", "completed", "success")]),
    );
    let o = w.run_names(Reach::Shared, &names(&["wait-ci"]));
    assert_eq!(
        stopped_at(&o),
        "halt wait-ci ci-missing",
        "the commit still demands lint: {}",
        o.line()
    );
}

#[test]
fn an_empty_required_list_halts() {
    let (w, _) = approved_world();
    fs::write(w.work().join(".fs/release.toml"), "required = []\n").unwrap();
    git(&w.work(), &["commit", "--quiet", "-am", "empty"]);
    let o = w.run_names(Reach::Shared, &names(&["wait-ci"]));
    assert_eq!(stopped_at(&o), "halt wait-ci no-required", "{}", o.line());
}

#[test]
fn a_running_optional_run_is_waited_for_and_merge_ff_halts_on_it() {
    let (w, sha) = approved_world();
    let mut running = ci_rows("success", "success");
    running.push((9, "docs", "feat", "in_progress", ""));
    let mut done = ci_rows("success", "success");
    done.push((9, "docs", "feat", "completed", "success"));
    w.runs_seq(
        SLUG,
        &[
            &run_json(&sha, &running),
            &run_json(&sha, &running),
            &run_json(&sha, &done),
        ],
    );
    let o = w.run_names(Reach::Shared, &names(&["wait-ci"]));
    assert!(matches!(o, Outcome::Done { .. }), "{}", o.line());
    assert!(w.clock.slept().as_secs() >= 30, "it waited");
    w.runs(SLUG, &run_json(&sha, &running));
    let _ = fs::remove_dir_all(w.fake().join("runs-Org_fs-demo.json.seq"));
    let first = w.run_names(Reach::Public, &names(&["gates", "review", "push"]));
    assert!(matches!(first, Outcome::Done { .. }));
    let o = w.run_names(Reach::Public, &names(&["merge-ff"]));
    assert_eq!(
        stopped_at(&o),
        "halt merge-ff ci-pending-optional",
        "{}",
        o.line()
    );
}

struct Never;
impl ApprovalVerifier for Never {
    fn check(
        &self,
        _: &fs_release_chain::Ctx,
        _: &fs_release_chain::approval::Approval,
        _: &std::path::Path,
    ) -> Result<(), fs_release_chain::outcome::Stop> {
        Err(fs_release_chain::outcome::Stop::halt(
            "signature",
            "a signature",
        ))
    }
}

#[test]
fn another_verifier_can_be_plugged_in_without_touching_the_rest() {
    let w = World::new();
    let sha = w.head();
    w.approve(&sha, "JA");
    let runner = w.runner();
    let ctx = fs_release_chain::Ctx::new(
        w.target(),
        Reach::Local,
        fs_release_chain::Timing::standard(),
        &runner,
        &w.clock,
    );
    let (a, file) =
        fs_release_chain::approval::Approval::load(&ctx.target.approval_dir, SLUG, &sha).unwrap();
    assert!(fs_release_chain::approval::verifiers()
        .iter()
        .all(|v| v.check(&ctx, &a, &file).is_ok()));
    assert!(Never.check(&ctx, &a, &file).is_err());
}
