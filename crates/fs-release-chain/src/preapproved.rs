//! Pattern: Specification (which repos the owner has pre-approved for the chain, and how far; a fixed list in the repo).

use serde::Deserialize;

use crate::ctx::Ctx;
use crate::git;
use crate::meta::Reach;
use crate::origin::configured_slug;
use crate::outcome::Stop;

/// The only repo whose `.fs/preapproved.toml` counts.
pub const RELEASE_SLUG: &str = "FreeSynergy/fs-release";

/// The one ref the list is read from, whatever `--base` the run targets (not settable from the command line).
pub const LIST_BRANCH: &str = "main";

/// `.fs/preapproved.toml` of fs-release, read from `origin/main` of the checkout, not from `HEAD`.
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
    /// Reads the list from `origin/main` ([`LIST_BRANCH`]) of the fs-release checkout (`--release-dir`): the checkout's origin
    /// must be exactly `FreeSynergy/fs-release`, the ref is fetched first (when that fails, an existing ref is used below
    /// public reach, public reach is red), and the working tree, `HEAD`, any local branch and the run's `--base` count for
    /// nothing. A release of fs-release itself, or any other branch pushed to its origin, cannot bring its own list.
    ///
    /// # Errors
    /// [`Stop::Red`] when the checkout is not fs-release, the file is damaged or (public reach) the fetch failed; [`Stop::Halt`] `not-preapproved` when
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
        let fetched = git::fetch_raw(ctx, dir, LIST_BRANCH)?;
        if !fetched.ok() {
            if ctx.reach == Reach::Public {
                return Err(Stop::red(format!(
                    "public reach needs the fresh list: git fetch origin {LIST_BRANCH} of {} failed ({}); no local ref is trusted",
                    dir.display(),
                    fetched.first_error()
                )));
            }
            eprintln!(
                "warning: git fetch origin {LIST_BRANCH} failed, the existing refs/remotes/origin/{LIST_BRANCH} is used: {}",
                fetched.first_error()
            );
        }
        let spec = format!("refs/remotes/origin/{LIST_BRANCH}:.fs/preapproved.toml");
        let o = git::raw(ctx, dir, &["show", &spec])?;
        if !o.ok() {
            return Err(Stop::halt(
                "not-preapproved",
                format!(".fs/preapproved.toml on origin/{LIST_BRANCH} of the fs-release checkout"),
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
