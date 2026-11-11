//! Pattern: Specification (which repos the owner has pre-approved for the chain, and how far; a fixed list in the repo).

use serde::Deserialize;

use crate::ctx::Ctx;
use crate::git;
use crate::meta::Reach;
use crate::origin::configured_slug;
use crate::outcome::Stop;

/// The only repo whose `.fs/preapproved.toml` counts.
pub const RELEASE_SLUG: &str = "FreeSynergy/fs-release";

/// `.fs/preapproved.toml` of fs-release, read from `origin/<base>` of the checkout, not from `HEAD`.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Preapproved {
    /// `Owner/name` of every repo the owner pre-approved for versions Opus has checked.
    pub repos: Vec<String>,
    /// Catalog repos `bump` and `sign` may push to (public reach only).
    #[serde(default)]
    pub catalog_targets: Vec<String>,
    /// Empty: public reach stays locked for everybody. Filled with the reference of the Opus JA that lifts the lock.
    #[serde(default)]
    pub public_ok: String,
}

impl Preapproved {
    /// Reads the list from `origin/<base>` of the fs-release checkout (`--release-dir`): the checkout's origin must be exactly
    /// `FreeSynergy/fs-release`, the ref is fetched first (an existing one is used when the fetch fails), and the working
    /// tree, `HEAD` and any local branch count for nothing. A release of fs-release itself therefore cannot bring its own list.
    ///
    /// # Errors
    /// [`Stop::Red`] when the checkout is not fs-release or the file is damaged; [`Stop::Halt`] `not-preapproved` when
    /// `origin/<base>` has no such file.
    pub fn load(ctx: &Ctx) -> Result<Preapproved, Stop> {
        let dir = &ctx.target.release_dir;
        let who = configured_slug(ctx.runner, dir).map_err(Stop::red)?;
        if who != RELEASE_SLUG {
            return Err(Stop::red(format!(
                "--release-dir {} is {who}, not {RELEASE_SLUG}",
                dir.display()
            )));
        }
        let base = &ctx.target.base;
        let _ = git::raw(ctx, dir, &["fetch", "--quiet", "origin", base])?;
        let spec = format!("refs/remotes/origin/{base}:.fs/preapproved.toml");
        let o = git::raw(ctx, dir, &["show", &spec])?;
        if !o.ok() {
            return Err(Stop::halt(
                "not-preapproved",
                format!(".fs/preapproved.toml on origin/{base} of the fs-release checkout"),
            ));
        }
        toml::from_str(&o.stdout).map_err(|e| Stop::red(format!(".fs/preapproved.toml: {e}")))
    }

    /// A run that touches more than this machine needs the repo on the list; public reach also needs the lifted lock and
    /// the catalog repo among the catalog targets.
    ///
    /// # Errors
    /// [`Stop::Halt`] `not-preapproved`, `public-locked` or `catalog-not-preapproved`.
    pub fn allows(&self, slug: &str, catalog_slug: &str, reach: Reach) -> Result<(), Stop> {
        if reach == Reach::Local {
            return Ok(());
        }
        if !self.repos.iter().any(|r| r == slug) {
            return Err(Stop::halt(
                "not-preapproved",
                format!("{slug} on the list in .fs/preapproved.toml (on origin of fs-release)"),
            ));
        }
        if reach == Reach::Public && self.public_ok.trim().is_empty() {
            return Err(Stop::halt(
                "public-locked",
                "the Opus JA entry (`public_ok`) in .fs/preapproved.toml: public reach is locked",
            ));
        }
        if reach == Reach::Public && !self.catalog_targets.iter().any(|r| r == catalog_slug) {
            return Err(Stop::halt(
                "catalog-not-preapproved",
                format!("{catalog_slug} in `catalog_targets` of .fs/preapproved.toml"),
            ));
        }
        Ok(())
    }
}
