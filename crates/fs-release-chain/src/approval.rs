//! Pattern: Specification (the one predicate "this approval covers exactly this commit of this repo").

use std::fs;
use std::path::Path;

use serde::Deserialize;

use crate::outcome::Stop;

/// The content of `.opus-ok`: who looked at which commit of which repo, when, and what they said.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Approval {
    pub repo: String,
    /// Full 40-character commit sha. A new commit on the branch makes the approval worthless.
    pub sha: String,
    pub date: String,
    /// `JA` or `NEIN`.
    pub verdict: String,
    #[serde(default)]
    pub by: Option<String>,
}

impl Approval {
    /// Reads the approval file.
    ///
    /// # Errors
    /// [`Stop::Halt`] `review` when the file is missing (nobody approved yet); [`Stop::Red`] when it is damaged.
    pub fn load(path: &Path) -> Result<Approval, Stop> {
        let text = fs::read_to_string(path).map_err(|_| {
            Stop::halt(
                "review",
                format!(
                    "approval file {} (repo, sha, date, verdict: JA)",
                    path.display()
                ),
            )
        })?;
        serde_norway::from_str(&text).map_err(|e| Stop::red(format!("{}: {e}", path.display())))
    }

    /// True when the approval is for `repo`, says yes, and names `head` in full.
    ///
    /// # Errors
    /// [`Stop::Halt`] with the reason: `review-no` (verdict is not JA), `approval-repo` or `approval-stale`.
    pub fn covers(&self, repo: &str, head: &str) -> Result<(), Stop> {
        if !self.verdict.trim().eq_ignore_ascii_case("ja") {
            return Err(Stop::halt(
                "review-no",
                format!(
                    "a new review of {} (verdict was {})",
                    short(head),
                    self.verdict
                ),
            ));
        }
        if self.repo != repo {
            return Err(Stop::halt(
                "approval-repo",
                format!("an approval for {repo}, this one is for {}", self.repo),
            ));
        }
        if self.sha != head {
            return Err(Stop::halt(
                "approval-stale",
                format!("a new JA for {head} (approved: {})", self.sha),
            ));
        }
        Ok(())
    }
}

fn short(sha: &str) -> &str {
    sha.get(..7).unwrap_or(sha)
}
