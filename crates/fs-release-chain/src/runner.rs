//! Pattern: Template method (the fixed frame around every command: reach, approval, run, log; the commands only fill in the work).

use std::collections::BTreeMap;

use crate::approval;
use crate::audit;
use crate::ctx::Ctx;
use crate::git::{self, short};
use crate::ledger::{Entry, Ledger};
use crate::meta::{Reach, Risk};
use crate::outcome::{Note, Outcome, Stop};
use crate::preapproved::Preapproved;
use crate::registry;
use crate::steps::Frame;

fn preflight(ctx: &Ctx) -> Result<(), Stop> {
    let t = &ctx.target;
    crate::slug::validate_branch(&t.base).map_err(Stop::red)?;
    if t.branch == t.base {
        return Err(Stop::red(format!(
            "{} is the base branch: the chain releases a branch, main only moves by merge-ff",
            t.base
        )));
    }
    if t.fake && ctx.reach != Reach::Local {
        return Err(Stop::red(
            "fake gates mode cannot publish: use --reach local",
        ));
    }
    if ctx.reach != Reach::Local {
        Preapproved::load(ctx)?.allows(&t.slug, &t.catalog_slug, ctx.reach)?;
    }
    let on = git::out(ctx, &t.repo_dir, &["symbolic-ref", "--short", "HEAD"])?;
    if on == t.branch {
        Ok(())
    } else {
        Err(Stop::red(format!(
            "{} has {on} checked out, not {}",
            t.repo_dir.display(),
            t.branch
        )))
    }
}

/// A command that finished: its note and the commit it ran for.
struct Finished {
    note: Note,
    head: String,
}

/// One command inside the frame.
fn one(ctx: &Ctx, ledger: &Ledger, name: &str) -> Result<Finished, (String, Stop)> {
    let at = |s: Stop| (String::new(), s);
    let Some(step) = registry::find(name) else {
        return Err(at(Stop::red(format!("unknown command {name}"))));
    };
    let meta = step.meta();
    if meta.reach > ctx.reach {
        return Err(at(Stop::halt(
            "reach-limit",
            format!(
                "--reach {} for {} (this run may touch {})",
                meta.reach.word(),
                name,
                ctx.reach.word()
            ),
        )));
    }
    let head = git::rev(ctx, &ctx.target.repo_dir, "HEAD").map_err(at)?;
    if meta.reach >= Reach::Shared
        && !git::out(
            ctx,
            &ctx.target.repo_dir,
            &["status", "--porcelain", "--untracked-files=no"],
        )
        .map_err(at)?
        .is_empty()
    {
        return Err((
            head,
            Stop::red(format!(
                "worktree {} has uncommitted changes: what is released must be what was checked",
                ctx.target.repo_dir.display()
            )),
        ));
    }
    let approved = if meta.risk == Risk::Publish {
        Some(approval::verify(ctx, &head).map_err(|why| (head.clone(), why))?)
    } else {
        None
    };
    if let Some(a) = &approved {
        // The intention is on record BEFORE the step runs; when the log cannot be written the step does not run.
        audit::record(ctx, &audit::line(ctx, a, &head, &meta, "start"))
            .map_err(|why| (head.clone(), why))?;
    }
    let frame = Frame {
        ctx,
        ledger,
        head: &head,
    };
    let result = step.run(&frame);
    if let Some(a) = approved {
        let word = match &result {
            Ok(_) => "ok",
            Err(Stop::Halt { .. }) => "halt",
            Err(Stop::Red { .. }) => "red",
        };
        if let Err(why) = audit::record(ctx, &audit::line(ctx, &a, &head, &meta, word)) {
            return Err((head, why));
        }
    }
    match result {
        Ok(note) => Ok(Finished { note, head }),
        Err(why) => Err((head, why)),
    }
}

fn log(
    ledger: &Ledger,
    step: &str,
    sha: &str,
    result: &str,
    detail: &str,
    data: BTreeMap<String, String>,
) -> Result<(), Stop> {
    if sha.is_empty() {
        return Ok(());
    }
    ledger.append(&Entry {
        step: step.into(),
        sha: sha.into(),
        result: result.into(),
        detail: detail.into(),
        data,
    })
}

/// Runs the named commands in order and stops at the first halt or red. Every result is written to the run log.
#[must_use]
pub fn execute(ctx: &Ctx, ledger: &Ledger, names: &[String]) -> Outcome {
    if let Err(stop) = preflight(ctx) {
        return Outcome::from_stop("preflight", stop);
    }
    let mut last = (String::new(), String::new());
    for name in names {
        match one(ctx, ledger, name) {
            Ok(Finished { note, head }) => {
                if let Err(stop) = log(ledger, name, &head, "ok", &note.text, note.data) {
                    return Outcome::from_stop(name, stop);
                }
                last = (head, note.text);
            }
            Err((head, stop)) => {
                let (result, detail) = match &stop {
                    Stop::Halt { reason, needs } => ("halt", format!("{reason}: {needs}")),
                    Stop::Red { line } => ("red", line.clone()),
                };
                let _ = log(ledger, name, &head, result, &detail, BTreeMap::new());
                return Outcome::from_stop(name, stop);
            }
        }
    }
    let t = &ctx.target;
    let summary = if names.len() == 1 {
        format!("{} {}", names[0], last.1)
    } else {
        format!(
            "{} v{} {} steps={} next: user-test",
            t.slug,
            t.version,
            short(&last.0),
            names.len()
        )
    };
    Outcome::Done { summary }
}
