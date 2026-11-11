//! Pattern: Strategy, part of the chain commands (the two commands that wait for GitHub runs: wait-ci, release).

use std::collections::BTreeMap;

use crate::gh::{self, Run};
use crate::git::short;
use crate::ledger::Entry;
use crate::meta::{Meta, Reach, Risk};
use crate::outcome::{Note, StepResult, Stop};
use crate::poll::{self, Filter, Verdict};
use crate::steps::{Frame, Step};

/// Log lines that mean the runner or its network failed, not the code: worth one second try.
const INFRA: [&str; 5] = [
    "temporary failure in name resolution",
    "could not resolve host",
    "temporary failure resolving",
    "e: failed to fetch",
    "connection timed out",
];

fn is_infra(line: &str) -> bool {
    let low = line.to_lowercase();
    INFRA.iter().any(|p| low.contains(p))
}

/// Waits for the matching runs. A red run is started again once (only when its log shows a runner or network problem if
/// `infra_only`), then it is red for good.
pub(super) fn settle(
    f: &Frame,
    step: &str,
    slug: &str,
    filter: &Filter,
    infra_only: bool,
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
                let again =
                    !f.ledger.has_marker(step, f.head, "rerun") && (!infra_only || is_infra(&line));
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
        let filter = Filter {
            sha: f.head.to_string(),
            branch: Some(t.branch.clone()),
            ..Filter::default()
        };
        let runs = settle(f, "wait-ci", &t.slug, &filter, true)?;
        Ok(Note::text(format!(
            "CI green at {} ({} runs)",
            short(f.head),
            runs.len()
        )))
    }
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
        settle(f, "release", &t.slug, &filter, false)?;
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
