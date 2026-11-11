//! Pattern: Strategy, part of the chain commands (the two commands that wait for GitHub runs: wait-ci, release).

use std::collections::BTreeMap;

use crate::gh::{self, Run};
use crate::git::short;
use crate::ledger::Entry;
use crate::meta::{Meta, Reach, Risk};
use crate::outcome::{Note, StepResult, Stop};
use crate::poll::{self, Filter, Verdict};
use crate::profile::Profile;
use crate::steps::{Frame, Step};

/// Log lines that mean the runner's DNS or apt failed, not the code: worth one second try. Nothing broader.
const INFRA: [&str; 4] = [
    "temporary failure in name resolution",
    "could not resolve host",
    "temporary failure resolving",
    "e: failed to fetch",
];

fn is_infra(line: &str) -> bool {
    let low = line.to_lowercase();
    INFRA.iter().any(|p| low.contains(p))
}

/// When a red run is started again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Retry {
    /// Never: red is red.
    Never,
    /// Once, only when the log shows a runner DNS or apt problem.
    Infra,
    /// Once, whatever the log says (artefact runs).
    Once,
}

/// Waits for the matching runs. A red run is started again once (only when its log shows a runner or network problem if
/// `infra_only`), then it is red for good.
pub(super) fn settle(
    f: &Frame,
    step: &str,
    slug: &str,
    filter: &Filter,
    retry: Retry,
) -> Result<Vec<Run>, Stop> {
    loop {
        match poll::wait(f.ctx, slug, filter)? {
            Verdict::Green(runs) => return Ok(runs),
            Verdict::Missing => {
                return Err(Stop::halt(
                    "ci-missing",
                    format!(
                        "a run for {} in {slug} (none started within {} min)",
                        short(f.head),
                        f.ctx.timing.start_limit.as_secs() / 60
                    ),
                ));
            }
            Verdict::Red(run) => {
                let line = gh::failed_line(f.ctx, slug, run.id)?;
                let again = retry != Retry::Never
                    && !f.ledger.has_marker(step, f.head, "rerun")
                    && (retry == Retry::Once || is_infra(&line));
                if !again {
                    return Err(Stop::red(format!("{} #{}: {line}", run.workflow, run.id)));
                }
                gh::rerun(f.ctx, slug, run.id)?;
                let mark = Entry {
                    step: step.to_string(),
                    sha: f.head.to_string(),
                    result: "note".into(),
                    detail: format!("rerun of #{}", run.id),
                    data: BTreeMap::from([("rerun".to_string(), run.id.to_string())]),
                };
                f.ledger.append(&mark)?;
            }
        }
    }
}

/// Waits for the CI of the pushed commit.
pub struct WaitCi;

impl Step for WaitCi {
    fn meta(&self) -> Meta {
        Meta {
            name: "wait-ci",
            risk: Risk::View,
            reach: Reach::Shared,
            preview: "The CI for {{sha}} is awaited (at most {{t}}).",
            undo: "none",
        }
    }

    fn run(&self, f: &Frame) -> StepResult {
        let t = &f.ctx.target;
        let profile = Profile::load(f.ctx, f.head)?;
        let filter = Filter {
            sha: f.head.to_string(),
            branch: Some(t.branch.clone()),
            required: profile.required.clone(),
            ..Filter::default()
        };
        let runs = settle(f, "wait-ci", &t.slug, &filter, Retry::Infra)?;
        optional_red(&runs, &profile)?;
        if pending_optional(&runs, &profile).is_some() {
            // Not required, but still running: wait for them too (up to the limits), a red one halts.
            let all = Filter {
                required: Vec::new(),
                ..filter
            };
            if let Verdict::Red(r) = poll::wait(f.ctx, &t.slug, &all)? {
                let required = profile
                    .required
                    .iter()
                    .any(|n| r.workflow.eq_ignore_ascii_case(n));
                return Err(if required {
                    Stop::red(format!("required run {} #{} turned red", r.workflow, r.id))
                } else {
                    Stop::halt(
                        "ci-optional-red",
                        format!(
                            "a look at the not required run {} #{} ({})",
                            r.workflow, r.id, r.conclusion
                        ),
                    )
                });
            }
        }
        Ok(Note::text(format!(
            "CI green at {} (required: {})",
            short(f.head),
            profile.required.join(", ")
        )))
    }
}

/// A run that is not required and has not finished.
fn pending_optional<'a>(runs: &'a [Run], profile: &Profile) -> Option<&'a Run> {
    runs.iter().find(|r| {
        r.status != "completed"
            && !profile
                .required
                .iter()
                .any(|n| r.workflow.eq_ignore_ascii_case(n))
    })
}

/// A red run that is not required does not make the commit red, but somebody has to look at it.
fn optional_red(runs: &[Run], profile: &Profile) -> Result<(), Stop> {
    let bad = runs.iter().find(|r| {
        r.status == "completed"
            && !matches!(r.conclusion.as_str(), "success" | "skipped" | "neutral")
            && !profile
                .required
                .iter()
                .any(|n| r.workflow.eq_ignore_ascii_case(n))
    });
    match bad {
        Some(r) => Err(Stop::halt(
            "ci-optional-red",
            format!(
                "a look at the not required run {} #{} ({})",
                r.workflow, r.id, r.conclusion
            ),
        )),
        None => Ok(()),
    }
}

/// Asks GitHub now (one query) whether every required run of the head commit exists and is green. The run log is not asked.
pub(super) fn verify_green(f: &Frame) -> Result<(), Stop> {
    let t = &f.ctx.target;
    let profile = Profile::load(f.ctx, f.head)?;
    let filter = Filter {
        sha: f.head.to_string(),
        branch: Some(t.branch.clone()),
        required: profile.required.clone(),
        ..Filter::default()
    };
    let runs: Vec<Run> = gh::list_runs(f.ctx, &t.slug)?
        .into_iter()
        .filter(|r| filter.matches(r))
        .collect();
    for name in &profile.required {
        let mine: Vec<&Run> = runs
            .iter()
            .filter(|r| r.workflow.eq_ignore_ascii_case(name))
            .collect();
        if mine.is_empty() {
            return Err(Stop::red(format!(
                "required run {name} not found for {} on GitHub",
                short(f.head)
            )));
        }
        if let Some(r) = mine.iter().find(|r| {
            r.status != "completed"
                || !matches!(r.conclusion.as_str(), "success" | "skipped" | "neutral")
        }) {
            return Err(Stop::red(format!(
                "required run {name} #{} is {} {} on GitHub",
                r.id, r.status, r.conclusion
            )));
        }
    }
    if let Some(r) = pending_optional(&runs, &profile) {
        return Err(Stop::halt(
            "ci-pending-optional",
            format!(
                "the not required run {} #{} to finish (or wait-ci again)",
                r.workflow, r.id
            ),
        ));
    }
    optional_red(&runs, &profile)
}

/// Waits for the release run on the tag, then turns the draft into a (pre-)release.
pub struct Release;

impl Step for Release {
    fn meta(&self) -> Meta {
        Meta {
            name: "release",
            risk: Risk::Publish,
            reach: Reach::Public,
            preview: "The release {{tag}} is published as a pre-release.",
            undo: "back to draft",
        }
    }

    fn run(&self, f: &Frame) -> StepResult {
        let t = &f.ctx.target;
        let tag = t.tag();
        let published = |s: Option<gh::ReleaseState>| s.filter(|r| !r.draft);
        if published(gh::release_state(f.ctx, &t.slug, &tag)?).is_some() {
            return Ok(Note::text(format!("release {tag} is published already")));
        }
        let filter = Filter {
            sha: f.head.to_string(),
            branch: Some(tag.clone()),
            workflow: Some("release".into()),
            ..Filter::default()
        };
        settle(f, "release", &t.slug, &filter, Retry::Once)?;
        match gh::release_state(f.ctx, &t.slug, &tag)? {
            None => {
                return Err(Stop::red(format!(
                    "release run is green but {tag} has no release"
                )))
            }
            Some(s) if !s.draft => return Ok(Note::text(format!("release {tag} is published"))),
            Some(_) => {}
        }
        gh::publish(f.ctx, &t.slug, &tag, t.prerelease)?;
        if published(gh::release_state(f.ctx, &t.slug, &tag)?).is_none() {
            return Err(Stop::red(format!(
                "release {tag} is still a draft after publishing"
            )));
        }
        Ok(Note::text(format!(
            "release {tag} published{}",
            if t.prerelease { " as pre-release" } else { "" }
        )))
    }
}
