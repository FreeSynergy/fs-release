//! Test of the stops of the chain: approval, reach, gates, Blinc lock, CI queue (nothing here touches GitHub).

mod support;

use std::fs;
use std::time::Duration;

use fs_release_chain::meta::Reach;
use fs_release_chain::Outcome;
use support::flow::{all_green, stopped_at};
use support::{git, run_json, World, SLUG};

#[test]
fn new_commit_after_approval_voids_it() {
    let w = World::new();
    w.approve(&w.head(), "JA");
    fs::write(w.work().join("more"), "y\n").unwrap();
    git(&w.work(), &["add", "."]);
    git(&w.work(), &["commit", "--quiet", "-m", "one more"]);
    all_green(&w, &w.head());
    let o = w.run(Reach::Public);
    assert_eq!(stopped_at(&o), "halt review approval-stale", "{}", o.line());
    assert_eq!(o.code(), 10);
    assert!(
        git(&w.origin(), &["branch", "-l", "feat"]).is_empty(),
        "nothing was pushed"
    );
}

#[test]
fn approval_missing_or_no_halts_and_a_commit_in_the_middle_halts_the_next_publish() {
    let w = World::new();
    assert_eq!(stopped_at(&w.run(Reach::Public)), "halt review review");
    w.approve(&w.head(), "NEIN");
    assert_eq!(stopped_at(&w.run(Reach::Public)), "halt review review-no");
    let sha = w.head();
    w.approve(&sha, "JA");
    all_green(&w, &sha);
    let first = w.run_names(
        Reach::Public,
        &["gates".into(), "review".into(), "push".into()],
    );
    assert!(matches!(first, Outcome::Done { .. }));
    fs::write(w.work().join("late"), "z\n").unwrap();
    git(&w.work(), &["add", "."]);
    git(&w.work(), &["commit", "--quiet", "-m", "late"]);
    let o = w.run_names(Reach::Public, &["merge-ff".into()]);
    assert_eq!(
        stopped_at(&o),
        "halt merge-ff approval-stale",
        "{}",
        o.line()
    );
}

#[test]
fn default_reach_local_never_publishes() {
    let w = World::new();
    let sha = w.head();
    w.approve(&sha, "JA");
    let o = w.run(Reach::Local);
    assert_eq!(stopped_at(&o), "halt push reach-limit", "{}", o.line());
    assert_eq!(git(&w.origin(), &["branch", "-l", "feat"]), "");
}

#[test]
fn blinc_lock_red_stops_at_gates() {
    let w = World::new();
    let lock = "version = 4\n\n[[package]]\nname = \"blinc-app\"\nversion = \"0.1.0\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\n";
    fs::write(w.work().join("Cargo.lock"), lock).unwrap();
    git(&w.work(), &["add", "."]);
    git(&w.work(), &["commit", "--quiet", "-m", "lock"]);
    w.approve(&w.head(), "JA");
    let o = w.run(Reach::Public);
    assert!(
        matches!(&o, Outcome::Red { step, line } if step == "gates" && line.starts_with("blinc-lock")),
        "{}",
        o.line()
    );
    assert!(
        !w.fake().join("gates.log").exists(),
        "the slow gates did not even start"
    );
    assert_eq!(git(&w.origin(), &["branch", "-l", "feat"]), "");
}

#[test]
fn red_gates_stop_with_the_first_error_line_and_a_log() {
    let w = World::new();
    w.set_gates(false);
    w.approve(&w.head(), "JA");
    let o = w.run(Reach::Public);
    assert!(
        matches!(&o, Outcome::Red { step, line } if step == "gates" && line.contains("clippy found 1 problem") && line.contains(".log")),
        "{}",
        o.line()
    );
}

#[test]
fn queue_without_jobs_halts_after_the_limit_and_polls_slowly() {
    let w = World::new();
    let sha = w.head();
    w.approve(&sha, "JA");
    w.runs(
        SLUG,
        &run_json(
            &sha,
            &[
                (1, "check", "feat", "queued", ""),
                (4, "lint", "feat", "queued", ""),
            ],
        ),
    );
    let o = w.run(Reach::Public);
    assert_eq!(stopped_at(&o), "halt wait-ci ci-queue", "{}", o.line());
    let slept = w.clock.slept();
    assert!(slept >= Duration::from_mins(20));
    let polls = u32::try_from(w.count("run list")).unwrap();
    assert!(
        Duration::from_secs(30) * (polls - 1) <= slept,
        "{polls} polls in {slept:?}: more than one per 30 s"
    );
    assert!(
        polls < 40,
        "back-off keeps the number of queries low ({polls})"
    );
}

#[test]
fn run_that_never_appears_halts() {
    let w = World::new();
    w.approve(&w.head(), "JA");
    let o = w.run(Reach::Public);
    assert_eq!(stopped_at(&o), "halt wait-ci ci-missing", "{}", o.line());
}
