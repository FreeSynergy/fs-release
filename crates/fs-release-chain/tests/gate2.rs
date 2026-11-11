//! Test of the doors in front of the chain, part 2: approval links, base branch, clean worktree, profile from the commit, verifier interface.

mod support;

use std::fs;

use fs_release_chain::approval::ApprovalVerifier;
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
