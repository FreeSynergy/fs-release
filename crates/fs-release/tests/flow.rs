//! Pattern: none (test). The chain lift -> review-request -> tag -> push: nothing without the approval of exactly this head.

mod support;

use fs_release::flow;
use fs_release::lift;
use fs_release::task::run_all;
use fs_release_chain::ctx::{Ctx, Timing};
use fs_release_chain::meta::Reach;
use fs_release_chain::outcome::Outcome;
use fs_release_chain::sys::{FakeClock, Runner};
use support::gh::MockGh;
use support::{git, World};

fn ctx<'a>(w: &World, runner: &'a dyn Runner, clock: &'a FakeClock, reach: Reach) -> Ctx<'a> {
    Ctx::new(w.target(), reach, Timing::standard(), runner, clock)
}

/// Lift with commit in the world: green, one commit on `feat`.
fn lifted(w: &World) -> String {
    let mut o = w.online_opts();
    o.commit = true;
    let (r, code) = lift::run(&o, &w.runner());
    assert_eq!(code, 0, "{}", r.markdown());
    git(&w.work(), &["rev-parse", "HEAD"])
}

/// The whole flow; `gh` is the mock (the Pre-release is published), so no call leaves the machine. Returns the `gh` workflow starts.
fn run_flow(w: &World, reach: Reach) -> (Outcome, Vec<String>) {
    let (mock, clock) = (MockGh::new(w.runner(), Some(false)), FakeClock::default());
    let c = ctx(w, &mock, &clock, reach);
    let mut o = w.online_opts();
    o.commit = true;
    let out = flow::run(&o, &c, &mock);
    (out, mock.dispatched())
}

fn one(w: &World, reach: Reach, which: usize) -> Result<String, fs_release_chain::outcome::Stop> {
    let (runner, clock) = (w.runner(), FakeClock::default());
    let c = ctx(w, &runner, &clock, reach);
    let mut all = flow::tasks();
    let list = vec![all.remove(which)];
    let (records, first) = run_all(&list, &c);
    match first {
        None => Ok(records[0].detail.clone()),
        Some((_, stop)) => Err(stop),
    }
}

fn halt_reason(r: Result<String, fs_release_chain::outcome::Stop>) -> String {
    match r {
        Err(fs_release_chain::outcome::Stop::Halt { reason, .. }) => reason,
        other => panic!("expected a halt, got {other:?}"),
    }
}

#[test]
fn review_request_names_head_report_and_approval_file() {
    let w = World::new();
    w.flow_setup("");
    let head = lifted(&w);
    let note = one(&w, Reach::Local, 0).unwrap();
    let text = std::fs::read_to_string(w.work().join(".fs-release/review-request.md")).unwrap();
    assert!(note.contains("review-request.md"));
    assert!(
        text.contains(&head)
            && text.contains("lift-report.json")
            && text.contains(&format!("{head}.ok")),
        "{text}"
    );
    assert!(
        text.contains("crates/a/Cargo.toml"),
        "diff head missing:\n{text}"
    );
}

#[test]
fn review_request_needs_a_green_lift_report_for_this_head() {
    let w = World::new();
    w.flow_setup("");
    assert!(
        matches!(
            one(&w, Reach::Local, 0),
            Err(fs_release_chain::outcome::Stop::Red { .. })
        ),
        "no report"
    );
    lifted(&w);
    std::fs::write(w.work().join("new"), "x").unwrap();
    git(&w.work(), &["add", "new"]);
    git(&w.work(), &["commit", "--quiet", "-m", "later"]);
    let r = one(&w, Reach::Local, 0);
    assert!(
        matches!(&r, Err(fs_release_chain::outcome::Stop::Red { line }) if line.contains("lift again")),
        "{r:?}"
    );
}

#[test]
fn tag_halts_without_approval_and_works_with_it_and_audits() {
    let w = World::new();
    w.flow_setup("");
    let head = lifted(&w);
    assert_eq!(halt_reason(one(&w, Reach::Local, 1)), "review");
    assert_eq!(git(&w.work(), &["tag", "-l", "v0.0.9"]), "");
    w.approve(&head, "NEIN");
    assert_eq!(halt_reason(one(&w, Reach::Local, 1)), "review-no");
    w.approve(&head, "JA");
    one(&w, Reach::Local, 1).unwrap();
    assert_eq!(git(&w.work(), &["rev-parse", "v0.0.9^{commit}"]), head);
    let log = w.audit();
    assert!(
        log.contains("step=tag")
            && log.contains("result=start")
            && log.contains("result=ok")
            && log.contains(&head),
        "{log}"
    );
}

#[test]
fn an_approval_for_another_commit_is_stale_and_a_wrong_tag_is_red() {
    let w = World::new();
    w.flow_setup("");
    let head = lifted(&w);
    let old = git(&w.work(), &["rev-parse", "HEAD~1"]);
    w.approve(&old, "JA");
    assert_eq!(halt_reason(one(&w, Reach::Local, 1)), "approval-stale");
    git(&w.work(), &["tag", "-a", "v0.0.9", &old, "-m", "x"]);
    w.approve(&head, "JA");
    let r = one(&w, Reach::Local, 1);
    assert!(
        matches!(&r, Err(fs_release_chain::outcome::Stop::Red { line }) if line.contains("exists on")),
        "{r:?}"
    );
}

#[test]
fn push_needs_reach_approval_and_the_lifted_public_lock() {
    let w = World::new();
    w.flow_setup("");
    let head = lifted(&w);
    w.approve(&head, "JA");
    one(&w, Reach::Local, 1).unwrap();
    assert_eq!(halt_reason(one(&w, Reach::Local, 2)), "reach-limit");
    // Shared: branch only.
    one(&w, Reach::Shared, 2).unwrap();
    assert!(w.remote_has("refs/heads/feat") && !w.remote_has("refs/tags/v0.0.9"));
    // Public: locked until the strategy writes `public_ok`.
    assert_eq!(halt_reason(one(&w, Reach::Public, 2)), "public-locked");
    assert!(!w.remote_has("refs/tags/v0.0.9"));
    let log = w.audit();
    assert!(
        log.contains("step=push") && log.contains("reach=shared"),
        "{log}"
    );
}

#[test]
fn push_without_the_approval_does_nothing() {
    let w = World::new();
    w.flow_setup("opus-ja-1");
    let head = lifted(&w);
    w.approve(&head, "JA");
    one(&w, Reach::Local, 1).unwrap();
    std::fs::remove_file(w.root().join(format!("freigaben/demo/{head}.ok"))).unwrap();
    assert_eq!(halt_reason(one(&w, Reach::Public, 2)), "review");
    assert!(!w.remote_has("refs/heads/feat") && !w.remote_has("refs/tags/v0.0.9"));
}

#[test]
fn whole_flow_halts_for_the_approval_then_pushes_branch_and_tag_when_public_is_lifted() {
    let w = World::new();
    w.flow_setup("opus-ja-1");
    // First run: lift and review-request happen, tag halts: no approval yet.
    let (first, started) = run_flow(&w, Reach::Public);
    assert_eq!(started, Vec::<String>::new());
    assert!(
        matches!(&first, Outcome::Halt { step, reason, .. } if step == "tag" && reason == "review"),
        "{}",
        first.line()
    );
    assert!(w.work().join(".fs-release/review-request.md").is_file());
    let head = git(&w.work(), &["rev-parse", "HEAD"]);
    w.approve(&head, "JA");
    let (second, started) = run_flow(&w, Reach::Public);
    assert!(matches!(second, Outcome::Done { .. }), "{}", second.line());
    assert_eq!(
        started,
        ["workflow run bump.yml --repo FreeSynergy/fs-catalog"]
    );
    assert!(w.remote_has("refs/heads/feat") && w.remote_has("refs/tags/v0.0.9"));
}

#[test]
fn a_red_lift_stops_the_flow_before_any_review() {
    let w = World::new();
    w.flow_setup("");
    let mut o = w.online_opts();
    o.commit = true;
    o.components = Some("v9.9.9".into());
    let (runner, clock) = (w.runner(), FakeClock::default());
    let c = ctx(&w, &runner, &clock, Reach::Local);
    let out = flow::run(&o, &c, &runner);
    assert!(
        matches!(&out, Outcome::Red { step, .. } if step == "lift/lock"),
        "{}",
        out.line()
    );
    assert!(!w.work().join(".fs-release/review-request.md").exists());
}

#[test]
fn a_local_tags_lock_cannot_go_through_the_flow() {
    let w = World::new();
    w.flow_setup("");
    let mut o = w.opts();
    o.commit = true;
    let (_, code) = lift::run(&o, &w.runner());
    assert_eq!(code, 0);
    let r = one(&w, Reach::Local, 0);
    assert!(
        matches!(&r, Err(fs_release_chain::outcome::Stop::Red { line }) if line.contains("uncommitted")),
        "{r:?}"
    );
}
