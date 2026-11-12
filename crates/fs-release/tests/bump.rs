//! Pattern: none (test). The bump step after the Pre-release: behind the approval, public reach only, `gh` is a mock (no real call).

mod support;

use fs_release::flow::BumpStep;
use fs_release::lift;
use fs_release::task::Task;
use fs_release_chain::ctx::{Ctx, Timing};
use fs_release_chain::meta::Reach;
use fs_release_chain::outcome::Stop;
use fs_release_chain::sys::FakeClock;
use support::gh::MockGh;
use support::{git, World};

const START: &str = "workflow run bump.yml --repo FreeSynergy/fs-catalog";

/// A lifted, committed world whose head carries a JA; the flow list allows public reach.
fn approved() -> (World, String) {
    let w = World::new();
    w.flow_setup("opus-ja-1");
    let mut o = w.online_opts();
    o.commit = true;
    let (r, code) = lift::run(&o, &w.runner());
    assert_eq!(code, 0, "{}", r.markdown());
    let head = git(&w.work(), &["rev-parse", "HEAD"]);
    w.approve(&head, "JA");
    (w, head)
}

fn bump(w: &World, reach: Reach, release: Option<bool>) -> (Result<String, Stop>, Vec<String>) {
    let (mock, clock) = (MockGh::new(w.runner(), release), FakeClock::default());
    let ctx = Ctx::new(w.target(), reach, Timing::standard(), &mock, &clock);
    let out = BumpStep.run(&ctx);
    (out, mock.dispatched())
}

fn halt(r: &Result<String, Stop>) -> &str {
    match r {
        Err(Stop::Halt { reason, .. }) => reason,
        other => panic!("expected a halt, got {other:?}"),
    }
}

#[test]
fn a_published_pre_release_and_the_approval_start_bump_yml_of_the_catalog_once() {
    let (w, head) = approved();
    let (r, started) = bump(&w, Reach::Public, Some(false));
    assert!(r.as_ref().unwrap().contains("bump.yml started"), "{r:?}");
    assert_eq!(started, [START]);
    let log = w.audit();
    assert!(
        log.contains("step=bump") && log.contains("result=ok") && log.contains(&head),
        "{log}"
    );
}

#[test]
fn below_public_reach_nothing_is_asked_and_nothing_started() {
    let (w, _) = approved();
    for reach in [Reach::Local, Reach::Shared] {
        let (r, started) = bump(&w, reach, Some(false));
        assert_eq!(halt(&r), "reach-limit");
        assert_eq!(started, Vec::<String>::new());
    }
}

#[test]
fn no_release_or_a_draft_halts_for_the_release_and_starts_nothing() {
    let (w, _) = approved();
    for release in [None, Some(true)] {
        let (r, started) = bump(&w, Reach::Public, release);
        assert_eq!(halt(&r), "release", "{release:?}");
        assert_eq!(started, Vec::<String>::new());
    }
}

#[test]
fn without_the_approval_of_this_head_nothing_is_started() {
    let (w, head) = approved();
    std::fs::remove_file(w.root().join(format!("freigaben/demo/{head}.ok"))).unwrap();
    let (r, started) = bump(&w, Reach::Public, Some(false));
    assert_eq!(halt(&r), "review");
    assert_eq!(started, Vec::<String>::new());
}

#[test]
fn a_locked_public_reach_starts_nothing() {
    let w = World::new();
    w.flow_setup("");
    let mut o = w.online_opts();
    o.commit = true;
    assert_eq!(lift::run(&o, &w.runner()).1, 0);
    let head = git(&w.work(), &["rev-parse", "HEAD"]);
    w.approve(&head, "JA");
    let (r, started) = bump(&w, Reach::Public, Some(false));
    assert_eq!(halt(&r), "public-locked");
    assert_eq!(started, Vec::<String>::new());
}
