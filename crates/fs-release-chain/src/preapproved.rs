//! Pattern: Specification (which repos the owner has pre-approved for the chain, and how far; a fixed list in the repo).

use serde::Deserialize;

use crate::ctx::Ctx;
use crate::git;
use crate::meta::Reach;
use crate::outcome::Stop;

/// `.fs/preapproved.toml` of fs-release, read from its committed `HEAD` (a change is a commit, and shows in review).
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Preapproved {
    /// `Owner/name` of every repo the owner pre-approved for versions Opus has checked.
    pub repos: Vec<String>,
    /// Empty: public reach stays locked for everybody. Filled with the reference of the Opus JA that lifts the lock.
    #[serde(default)]
    pub public_ok: String,
}

impl Preapproved {
    /// Reads the list from the fs-release checkout.
    ///
    /// # Errors
    /// [`Stop::Halt`] `not-preapproved` when the file is not in the committed `HEAD`; [`Stop::Red`] when it is damaged.
    pub fn load(ctx: &Ctx) -> Result<Preapproved, Stop> {
        let o = git::raw(
            ctx,
            &ctx.target.release_dir,
            &["show", "HEAD:.fs/preapproved.toml"],
        )?;
        if !o.ok() {
            return Err(Stop::halt(
                "not-preapproved",
                ".fs/preapproved.toml committed in the fs-release checkout",
            ));
        }
        toml::from_str(&o.stdout).map_err(|e| Stop::red(format!(".fs/preapproved.toml: {e}")))
    }

    /// A run that touches more than this machine needs the repo on the list; public reach also needs the lifted lock.
    ///
    /// # Errors
    /// [`Stop::Halt`] `not-preapproved` or `public-locked`.
    pub fn allows(&self, slug: &str, reach: Reach) -> Result<(), Stop> {
        if reach == Reach::Local {
            return Ok(());
        }
        if !self.repos.iter().any(|r| r == slug) {
            return Err(Stop::halt(
                "not-preapproved",
                format!("{slug} on the list in .fs/preapproved.toml (a commit) or the owner's say"),
            ));
        }
        if reach == Reach::Public && self.public_ok.trim().is_empty() {
            return Err(Stop::halt(
                "public-locked",
                "the Opus JA entry (`public_ok`) in .fs/preapproved.toml: public reach is locked",
            ));
        }
        Ok(())
    }
}
