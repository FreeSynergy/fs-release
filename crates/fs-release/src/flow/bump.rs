//! Pattern: Strategy (the bump after the Pre-release: start the catalog's `bump.yml`, behind the same approval as tag and push).

use fs_release_chain::ctx::Ctx;
use fs_release_chain::gh;
use fs_release_chain::meta::Reach;
use fs_release_chain::outcome::Stop;
use fs_release_chain::preapproved::Preapproved;

use super::{audited, green_report, guard, meta};
use crate::task::Task;

/// The catalog workflow that raises the tag in `packages.toml` (`gh workflow run bump.yml -R <catalog>`).
pub const BUMP_WORKFLOW: &str = "bump.yml";

/// After the Pre-release is published: starts `bump.yml` of the catalog repo. Public reach, the repo and the catalog must be on the
/// pre-approved list, and the approval of exactly this head is verified and audited like for tag and push. The catalog's own
/// `sign` and `test` runs follow by themselves; this step does not poll them (the chain's `sign` / `check` steps do).
pub struct BumpStep;

impl Task<Ctx<'_>> for BumpStep {
    fn name(&self) -> &'static str {
        "bump"
    }

    fn run(&self, ctx: &Ctx) -> Result<String, Stop> {
        if ctx.reach < Reach::Public {
            return Err(Stop::halt(
                "reach-limit",
                "--reach public for bump (the catalog is public)",
            ));
        }
        let t = &ctx.target;
        Preapproved::load(ctx)?.allows(&t.slug, &t.catalog_slug, ctx.reach)?;
        let head = guard(ctx)?;
        green_report(ctx, &head)?;
        let tag = t.tag();
        if gh::release_state(ctx, &t.slug, &tag)?.is_none_or(|r| r.draft) {
            return Err(Stop::halt(
                "release",
                format!(
                    "release.yml to publish the Pre-release {tag} of {}: run again after it",
                    t.slug
                ),
            ));
        }
        audited(ctx, &meta("bump", Reach::Public), &head, || {
            gh::dispatch(ctx, &t.catalog_slug, BUMP_WORKFLOW)?;
            Ok(format!(
                "{BUMP_WORKFLOW} started in {} for {tag}; its sign and test runs follow there",
                t.catalog_slug
            ))
        })
    }
}
