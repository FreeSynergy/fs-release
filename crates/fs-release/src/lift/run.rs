//! Pattern: Facade (one call `run` lifts a worktree: the steps in fixed order, then the report; no LLM, no network by default).

use fs_release_chain::sys::Runner;

use super::{checks, commit, steps, LiftEnv, LiftOpts, Report};
use crate::task::{run_all, Task};

/// The steps in order. The checks after the lock are independent: all of them run, so one report shows every red.
#[must_use]
pub fn tasks<'a>(opts: &LiftOpts) -> Vec<Box<dyn Task<LiftEnv<'a>>>> {
    let mut v: Vec<Box<dyn Task<LiftEnv<'a>>>> = vec![
        Box::new(steps::Deps),
        Box::new(steps::Lock),
        Box::new(steps::LockGate),
        Box::new(checks::Fmt),
        Box::new(checks::Clippy),
        Box::new(checks::Rules),
        Box::new(checks::Tests),
    ];
    if opts.commit {
        v.push(Box::new(commit::Commit));
    }
    v
}

/// Runs the whole lift and writes `<repo>/.fs-release/lift-report.{md,json}`. Exit code 0 green, 1 red.
#[must_use]
pub fn run(opts: &LiftOpts, runner: &dyn Runner) -> (Report, i32) {
    let started = std::time::Instant::now();
    let mut rep = super::report::start(opts, runner);
    let ok = if opts.components.is_none() && opts.interfaces.is_none() {
        rep.steps = vec![crate::task::Record {
            name: "setup".into(),
            status: "red".into(),
            detail: String::new(),
            first_error: Some("nothing to lift: give --components and/or --interfaces".into()),
            secs: 0.0,
        }];
        false
    } else {
        let env = LiftEnv { opts, runner };
        let list = tasks(opts);
        let (records, first) = run_all(&list, &env);
        rep.steps = records;
        first.is_none()
    };
    rep.finish(opts, runner, ok, started.elapsed().as_secs_f64());
    let code = i32::from(!ok);
    (rep, code)
}
