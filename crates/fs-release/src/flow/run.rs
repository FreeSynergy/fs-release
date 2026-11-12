//! Pattern: Facade (the chain as one run: lift, review-request, tag, push; stops at the first red or halt, run again to go on).

use std::fs;
use std::path::PathBuf;

use fs_release_chain::approval;
use fs_release_chain::audit;
use fs_release_chain::ctx::Ctx;
use fs_release_chain::git;
use fs_release_chain::meta::{Meta, Reach, Risk};
use fs_release_chain::outcome::{Outcome, Stop};
use fs_release_chain::sys::Runner;

use super::{BumpStep, PushStep, ReviewRequest, TagStep};
use crate::lift::{self, LiftOpts};
use crate::task::{run_all, Task};

/// The flow steps after the lift, in order. Tag comes before push: what is pushed is already tagged and approved. The bump
/// comes last: it needs the Pre-release that the pushed tag makes `release.yml` publish.
#[must_use]
pub fn tasks<'a>() -> Vec<Box<dyn Task<Ctx<'a>>>> {
    vec![
        Box::new(ReviewRequest),
        Box::new(TagStep),
        Box::new(PushStep),
        Box::new(BumpStep),
    ]
}

/// Where the lift report of the worktree lives.
#[must_use]
pub fn report_path(ctx: &Ctx) -> PathBuf {
    ctx.target.repo_dir.join(".fs-release/lift-report.json")
}

/// The lift report must be green and be about exactly the commit at the head.
///
/// # Errors
/// [`Stop::Red`] when the report is missing, red, or for another commit.
pub fn green_report(ctx: &Ctx, head: &str) -> Result<(), Stop> {
    let path = report_path(ctx);
    let text = fs::read_to_string(&path).map_err(|_| {
        Stop::red(format!(
            "no lift report {}: run `fs-release lift ... --commit` first",
            path.display()
        ))
    })?;
    let v: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| Stop::red(format!("{}: {e}", path.display())))?;
    if v["ok"] != true {
        return Err(Stop::red(format!("lift report {} is red", path.display())));
    }
    if v["head"].as_str() != Some(head) {
        return Err(Stop::red(format!(
            "lift report is for {}, the head is {}: lift again",
            v["head"].as_str().map_or("?", git::short),
            git::short(head)
        )));
    }
    Ok(())
}

/// The common guard of every flow step: a release branch (never the base), checked out, nothing uncommitted.
///
/// # Errors
/// [`Stop::Red`] naming the first violation.
pub fn guard(ctx: &Ctx) -> Result<String, Stop> {
    let t = &ctx.target;
    if t.branch == t.base {
        return Err(Stop::red(format!(
            "{} is the base branch: the flow releases a branch",
            t.base
        )));
    }
    let on = git::out(ctx, &t.repo_dir, &["symbolic-ref", "--short", "HEAD"])?;
    if on != t.branch {
        return Err(Stop::red(format!(
            "{} has {on} checked out, not {}",
            t.repo_dir.display(),
            t.branch
        )));
    }
    if !git::out(
        ctx,
        &t.repo_dir,
        &["status", "--porcelain", "--untracked-files=no"],
    )?
    .is_empty()
    {
        return Err(Stop::red("worktree has uncommitted changes: what is released must be what was checked (lift --commit; a local-tags Cargo.lock is never committed, use lift --online --commit)"));
    }
    git::rev(ctx, &t.repo_dir, "HEAD")
}

/// Runs `body` as a publishing step: the approval is verified, the intention goes to the audit log BEFORE the work and the
/// result AFTER it. Without a JA for exactly this head nothing happens.
///
/// # Errors
/// Whatever the approval, the audit log or the body say.
pub fn audited(
    ctx: &Ctx,
    meta: &Meta,
    head: &str,
    body: impl FnOnce() -> Result<String, Stop>,
) -> Result<String, Stop> {
    let a = approval::verify(ctx, head)?;
    audit::record(ctx, &audit::line(ctx, &a, head, meta, "start"))?;
    let result = body();
    let word = match &result {
        Ok(_) => "ok",
        Err(Stop::Halt { .. }) => "halt",
        Err(Stop::Red { .. }) => "red",
    };
    audit::record(ctx, &audit::line(ctx, &a, head, meta, word))?;
    result
}

/// Fixed facts of the two publishing steps.
#[must_use]
pub fn meta(name: &'static str, reach: Reach) -> Meta {
    Meta {
        name,
        risk: Risk::Publish,
        reach,
        preview: "see the step",
        undo: "none",
    }
}

/// The whole flow: lift (with commit), then the flow steps. One line out, like the chain: DONE (0), HALT (10), RED (1).
#[must_use]
pub fn run(opts: &LiftOpts, ctx: &Ctx, runner: &dyn Runner) -> Outcome {
    let (report, code) = lift::run(opts, runner);
    if code != 0 {
        let bad = report.steps.iter().find(|s| s.status == "red");
        return Outcome::Red {
            step: format!("lift/{}", bad.map_or("setup", |s| s.name.as_str())),
            line: bad.and_then(|s| s.first_error.clone()).unwrap_or_default(),
        };
    }
    let (_records, first) = run_all(&tasks(), ctx);
    match first {
        None => Outcome::Done {
            summary: format!(
                "{} v{} {} lift review-request tag push bump",
                ctx.target.slug,
                ctx.target.version,
                git::short(&report.head)
            ),
        },
        Some((name, stop)) => Outcome::from_stop(&name, stop),
    }
}
