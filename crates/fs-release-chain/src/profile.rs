//! Pattern: Value object (the release profile of a repo, `.fs/release.toml`: which runs must be green).

use serde::Deserialize;

use crate::ctx::Ctx;
use crate::git;
use crate::outcome::Stop;

/// What a repo says about its own release: the workflow runs that must exist and be green for its head commit.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    /// Workflow names (`check`, `lint`, `typing-test` for UI repos).
    pub required: Vec<String>,
}

impl Profile {
    /// Reads `.fs/release.toml` as it is IN the commit `sha` (`git show`), not from the working tree: what is released is
    /// what was checked, whatever lies around.
    ///
    /// # Errors
    /// [`Stop::Halt`] `no-profile` when the commit has no such file, `no-required` when it lists no run (a chain without
    /// required runs would call anything green); [`Stop::Red`] when it is damaged.
    pub fn load(ctx: &Ctx, sha: &str) -> Result<Profile, Stop> {
        let spec = format!("{sha}:.fs/release.toml");
        let o = git::raw(ctx, &ctx.target.repo_dir, &["show", &spec])?;
        if !o.ok() {
            return Err(Stop::halt(
                "no-profile",
                format!(
                    "{} in the commit with required = [\"check\", \"lint\"]",
                    ".fs/release.toml"
                ),
            ));
        }
        let p: Profile = toml::from_str(&o.stdout)
            .map_err(|e| Stop::red(format!(".fs/release.toml at {sha}: {e}")))?;
        if p.required.is_empty() {
            return Err(Stop::halt(
                "no-required",
                "at least one required run in .fs/release.toml (empty list)",
            ));
        }
        Ok(p)
    }
}
