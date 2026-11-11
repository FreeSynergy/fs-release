//! Pattern: Adapter (the `gh` command line behind small typed calls; the only place that knows its flags and JSON).

use serde::Deserialize;

use crate::ctx::Ctx;
use crate::outcome::Stop;
use crate::sys::{Cmd, Out};

/// One workflow run as `gh run list --json` reports it.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Run {
    #[serde(rename = "databaseId")]
    pub id: u64,
    #[serde(rename = "workflowName", default)]
    pub workflow: String,
    pub status: String,
    #[serde(default)]
    pub conclusion: String,
    #[serde(rename = "headSha", default)]
    pub head_sha: String,
    #[serde(rename = "headBranch", default)]
    pub head_branch: String,
}

/// Draft and pre-release flags of a release.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
pub struct ReleaseState {
    #[serde(rename = "isDraft")]
    pub draft: bool,
    #[serde(rename = "isPrerelease")]
    pub prerelease: bool,
}

fn gh(ctx: &Ctx, args: &[&str]) -> Result<Out, Stop> {
    let cmd = Cmd {
        program: "gh".into(),
        args: args.iter().map(|a| (*a).to_string()).collect(),
        ..Cmd::default()
    };
    Ok(ctx.runner.run(&cmd)?)
}

/// Waits until the rate limit allows another query, then records the time. At most one run query per `poll` time.
fn throttle(ctx: &Ctx) {
    let now = ctx.clock.now();
    if let Some(last) = ctx.last_query.get() {
        let next = last + ctx.timing.poll;
        if next > now {
            ctx.clock.sleep(next.saturating_sub(now));
        }
    }
    ctx.last_query.set(Some(ctx.clock.now()));
}

/// The newest runs of a repo (one query, throttled).
///
/// # Errors
/// [`Stop::Red`] when `gh` fails or answers something that is not the expected JSON.
pub fn list_runs(ctx: &Ctx, slug: &str) -> Result<Vec<Run>, Stop> {
    throttle(ctx);
    let fields = "databaseId,workflowName,status,conclusion,headSha,headBranch";
    let o = gh(
        ctx,
        &[
            "run", "list", "--repo", slug, "--limit", "50", "--json", fields,
        ],
    )?;
    if !o.ok() {
        return Err(Stop::red(format!("gh run list: {}", o.first_error())));
    }
    serde_json::from_str(&o.stdout).map_err(|e| Stop::red(format!("gh run list: bad answer ({e})")))
}

/// First error line of the failed steps of a run (what the log says is wrong), or a fixed hint when there is none.
///
/// # Errors
/// [`Stop::Red`] when `gh` cannot be started.
pub fn failed_line(ctx: &Ctx, slug: &str, id: u64) -> Result<String, Stop> {
    let o = gh(
        ctx,
        &[
            "run",
            "view",
            &id.to_string(),
            "--repo",
            slug,
            "--log-failed",
        ],
    )?;
    Ok(if o.ok() {
        o.first_error()
    } else {
        format!("run {id} failed (log not readable)")
    })
}

/// Starts the failed jobs of a run again.
///
/// # Errors
/// [`Stop::Red`] when `gh` refuses.
pub fn rerun(ctx: &Ctx, slug: &str, id: u64) -> Result<(), Stop> {
    let o = gh(
        ctx,
        &["run", "rerun", &id.to_string(), "--repo", slug, "--failed"],
    )?;
    if o.ok() {
        Ok(())
    } else {
        Err(Stop::red(format!("gh run rerun: {}", o.first_error())))
    }
}

/// Draft and pre-release state of a release, or `None` when the tag has no release yet.
///
/// # Errors
/// [`Stop::Red`] when the answer is not JSON.
pub fn release_state(ctx: &Ctx, slug: &str, tag: &str) -> Result<Option<ReleaseState>, Stop> {
    let o = gh(
        ctx,
        &[
            "release",
            "view",
            tag,
            "--repo",
            slug,
            "--json",
            "isDraft,isPrerelease",
        ],
    )?;
    if !o.ok() {
        return Ok(None);
    }
    serde_json::from_str(&o.stdout)
        .map(Some)
        .map_err(|e| Stop::red(format!("gh release view: bad answer ({e})")))
}

/// Publishes a draft release (as pre-release when asked).
///
/// # Errors
/// [`Stop::Red`] when `gh` refuses.
pub fn publish(ctx: &Ctx, slug: &str, tag: &str, prerelease: bool) -> Result<(), Stop> {
    let mut args = vec!["release", "edit", tag, "--repo", slug, "--draft=false"];
    if prerelease {
        args.push("--prerelease");
    }
    let o = gh(ctx, &args)?;
    if o.ok() {
        Ok(())
    } else {
        Err(Stop::red(format!("gh release edit: {}", o.first_error())))
    }
}

/// Starts a workflow by hand (`workflow_dispatch`).
///
/// # Errors
/// [`Stop::Red`] when `gh` refuses.
pub fn dispatch(ctx: &Ctx, slug: &str, workflow: &str) -> Result<(), Stop> {
    let o = gh(ctx, &["workflow", "run", workflow, "--repo", slug])?;
    if o.ok() {
        Ok(())
    } else {
        Err(Stop::red(format!("gh workflow run: {}", o.first_error())))
    }
}
