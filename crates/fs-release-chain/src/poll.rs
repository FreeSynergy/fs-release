//! Pattern: State (the waiting state of a run set: missing, queued, running, green, red) with a back-off timer.

use std::time::Duration;

use crate::ctx::Ctx;
use crate::gh::{self, Run};
use crate::outcome::Stop;

/// Which runs belong to the wait.
#[derive(Debug, Clone, Default)]
pub struct Filter {
    /// Commit of the run; empty means any.
    pub sha: String,
    /// Branch or tag name the run was started for.
    pub branch: Option<String>,
    /// Part of the workflow name, any case.
    pub workflow: Option<String>,
    /// Only runs newer than this id (a run started by hand after a dispatch).
    pub min_id: u64,
    /// Workflow names that must all be present and green; other runs do not decide (empty: every matching run decides).
    pub required: Vec<String>,
}

impl Filter {
    pub(crate) fn matches(&self, r: &Run) -> bool {
        (self.sha.is_empty() || r.head_sha == self.sha)
            && r.id > self.min_id
            && self.branch.as_ref().is_none_or(|b| &r.head_branch == b)
            && self
                .workflow
                .as_ref()
                .is_none_or(|w| r.workflow.to_lowercase().contains(&w.to_lowercase()))
    }
}

/// How a wait ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Every matching run finished and none failed.
    Green(Vec<Run>),
    /// This run failed (or was cancelled).
    Red(Run),
    /// No matching run showed up within the start limit.
    Missing,
}

enum Seen {
    Green,
    Red(Run),
    Pending { queued_only: bool },
    Missing,
}

fn classify(all: &[Run], required: &[String]) -> Seen {
    let named = |r: &&Run, n: &String| r.workflow.eq_ignore_ascii_case(n);
    let runs: Vec<Run> = if required.is_empty() {
        all.to_vec()
    } else {
        all.iter()
            .filter(|r| required.iter().any(|n| named(r, n)))
            .cloned()
            .collect()
    };
    let runs = runs.as_slice();
    let absent = required.iter().any(|n| !runs.iter().any(|r| named(&r, n)));
    if runs.is_empty()
        || (absent
            && !runs.iter().any(|r| {
                r.status == "completed"
                    && !matches!(r.conclusion.as_str(), "success" | "skipped" | "neutral")
            }))
    {
        return Seen::Missing;
    }
    if let Some(bad) = runs.iter().find(|r| {
        r.status == "completed"
            && !matches!(r.conclusion.as_str(), "success" | "skipped" | "neutral")
    }) {
        return Seen::Red(bad.clone());
    }
    let pending: Vec<&Run> = runs.iter().filter(|r| r.status != "completed").collect();
    if pending.is_empty() {
        return Seen::Green;
    }
    Seen::Pending {
        queued_only: pending.iter().all(|r| {
            matches!(
                r.status.as_str(),
                "queued" | "waiting" | "pending" | "requested"
            )
        }),
    }
}

fn grow(d: Duration, max: Duration) -> Duration {
    (d + d / 2).min(max)
}

/// Polls until the matching runs are green or one is red.
///
/// One query per wait, spaced by the throttle in `gh` (at least `timing.poll`), growing by half up to `poll_max`.
///
/// # Errors
/// [`Stop::Halt`] `ci-queue` when the runs only queue for the queue limit, `ci-timeout` when the whole wait exceeds the
/// total limit; [`Stop::Red`] when `gh` fails.
pub fn wait(ctx: &Ctx, slug: &str, filter: &Filter) -> Result<Verdict, Stop> {
    let began = ctx.clock.now();
    let mut missing_since = Some(began);
    let mut queued_since: Option<Duration> = None;
    let mut gap = ctx.timing.poll;
    loop {
        let runs: Vec<Run> = gh::list_runs(ctx, slug)?
            .into_iter()
            .filter(|r| filter.matches(r))
            .collect();
        let now = ctx.clock.now();
        match classify(&runs, &filter.required) {
            Seen::Green => return Ok(Verdict::Green(runs)),
            Seen::Red(r) => return Ok(Verdict::Red(r)),
            Seen::Missing => {
                queued_since = None;
                if now.saturating_sub(missing_since.unwrap_or(began)) >= ctx.timing.start_limit {
                    return Ok(Verdict::Missing);
                }
            }
            Seen::Pending { queued_only } => {
                missing_since = Some(now);
                queued_since = if queued_only {
                    queued_since.or(Some(now))
                } else {
                    None
                };
                if queued_since.is_some_and(|q| now.saturating_sub(q) >= ctx.timing.queue_limit) {
                    return Err(Stop::halt(
                        "ci-queue",
                        format!(
                            "a free runner for {slug} (queued {} min without a job)",
                            ctx.timing.queue_limit.as_secs() / 60
                        ),
                    ));
                }
            }
        }
        if now.saturating_sub(began) >= ctx.timing.total_limit {
            return Err(Stop::halt(
                "ci-timeout",
                format!(
                    "a look at the runs of {slug} (waited {} min)",
                    ctx.timing.total_limit.as_secs() / 60
                ),
            ));
        }
        ctx.clock.sleep(gap);
        gap = grow(gap, ctx.timing.poll_max);
    }
}
