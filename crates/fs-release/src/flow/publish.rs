//! Pattern: Strategy (tag and push: both need the approval of exactly this head, both go to the audit log).

use fs_release_chain::ctx::Ctx;
use fs_release_chain::git::{self, short};
use fs_release_chain::meta::Reach;
use fs_release_chain::outcome::Stop;
use fs_release_chain::preapproved::Preapproved;

use super::{audited, green_report, guard, meta};
use crate::task::Task;

/// Sets the annotated tag `v<version>` on the approved head, locally. Nothing leaves the machine.
pub struct TagStep;

impl Task<Ctx<'_>> for TagStep {
    fn name(&self) -> &'static str {
        "tag"
    }

    fn run(&self, ctx: &Ctx) -> Result<String, Stop> {
        let head = guard(ctx)?;
        green_report(ctx, &head)?;
        let t = &ctx.target;
        let tag = t.tag();
        audited(ctx, &meta("tag", Reach::Local), &head, || {
            match git::rev(ctx, &t.repo_dir, &format!("refs/tags/{tag}^{{commit}}")) {
                Ok(at) if at == head => Ok(format!("{tag} is on {} already", short(&head))),
                Ok(at) => Err(Stop::red(format!(
                    "tag {tag} exists on {}, not on {}",
                    short(&at),
                    short(&head)
                ))),
                Err(_) => {
                    git::out(ctx, &t.repo_dir, &["tag", "-a", &tag, &head, "-m", &tag])?;
                    Ok(format!("tag {tag} set on {}", short(&head)))
                }
            }
        })
    }
}

/// Pushes the branch (reach shared) and the tag (reach public) to `origin`, never forced. Shared and public reach need the
/// repo on the owner's pre-approved list; public also needs the lifted lock (`public_ok`, written by the strategy).
pub struct PushStep;

impl Task<Ctx<'_>> for PushStep {
    fn name(&self) -> &'static str {
        "push"
    }

    fn run(&self, ctx: &Ctx) -> Result<String, Stop> {
        if ctx.reach < Reach::Shared {
            return Err(Stop::halt(
                "reach-limit",
                "--reach shared (branch) or public (branch and tag) for push",
            ));
        }
        let t = &ctx.target;
        Preapproved::load(ctx)?.allows(&t.slug, &t.catalog_slug, ctx.reach)?;
        let head = guard(ctx)?;
        green_report(ctx, &head)?;
        let tag = t.tag();
        let at = git::rev(ctx, &t.repo_dir, &format!("refs/tags/{tag}^{{commit}}"))
            .map_err(|_| Stop::red(format!("no local tag {tag}: run tag first")))?;
        if at != head {
            return Err(Stop::red(format!(
                "tag {tag} is on {}, not on the head {}",
                short(&at),
                short(&head)
            )));
        }
        audited(ctx, &meta("push", ctx.reach), &head, || {
            let spec = format!("{head}:refs/heads/{}", t.branch);
            let o = git::raw(ctx, &t.repo_dir, &["push", "origin", &spec])?;
            if !o.ok() {
                return Err(Stop::red(format!("push rejected: {}", o.first_error())));
            }
            if ctx.reach < Reach::Public {
                return Ok(format!(
                    "pushed {} at {}; tag not pushed (reach shared)",
                    t.branch,
                    short(&head)
                ));
            }
            let o = git::raw(
                ctx,
                &t.repo_dir,
                &["push", "origin", &format!("refs/tags/{tag}")],
            )?;
            if o.ok() {
                Ok(format!("pushed {} and {tag} at {}", t.branch, short(&head)))
            } else {
                Err(Stop::red(format!("tag push rejected: {}", o.first_error())))
            }
        })
    }
}
