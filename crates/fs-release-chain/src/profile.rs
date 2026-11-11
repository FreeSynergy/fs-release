//! Pattern: Value object (the release profile of a repo, `.fs/release.toml`: which runs must be green).

use std::fs;
use std::path::Path;

use serde::Deserialize;

use crate::outcome::Stop;

/// What a repo says about its own release: the workflow runs that must exist and be green for its head commit.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    /// Workflow names (`check`, `lint`, `typing-test` for UI repos).
    pub required: Vec<String>,
}

impl Profile {
    /// Reads `<repo>/.fs/release.toml`.
    ///
    /// # Errors
    /// [`Stop::Halt`] `no-profile` when the file is missing or lists nothing (a chain without required runs would call
    /// anything green); [`Stop::Red`] when it is damaged.
    pub fn load(repo: &Path) -> Result<Profile, Stop> {
        let path = repo.join(".fs/release.toml");
        let Ok(text) = fs::read_to_string(&path) else {
            return Err(Stop::halt(
                "no-profile",
                format!("{} with required = [\"check\", \"lint\"]", path.display()),
            ));
        };
        let p: Profile =
            toml::from_str(&text).map_err(|e| Stop::red(format!("{}: {e}", path.display())))?;
        if p.required.is_empty() {
            return Err(Stop::halt(
                "no-profile",
                format!("at least one required run in {}", path.display()),
            ));
        }
        Ok(p)
    }
}
