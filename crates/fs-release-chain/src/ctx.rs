//! Pattern: Value object (everything one run knows: where, which version, how far it may reach, which clock and runner).

use std::cell::Cell;
use std::path::PathBuf;
use std::time::Duration;

use crate::meta::Reach;
use crate::sys::{Clock, Runner};

/// The GitHub rate limit is tight: never more than one run query per this time.
pub const MIN_POLL: Duration = Duration::from_secs(30);

/// Waiting rules for everything that polls GitHub.
#[derive(Debug, Clone, Copy)]
pub struct Timing {
    /// First wait between two run queries; grows by half each time up to `poll_max`.
    pub poll: Duration,
    pub poll_max: Duration,
    /// No run shows up for the commit at all: halt after this.
    pub start_limit: Duration,
    /// Runs exist but all only queue (no job started): halt after this.
    pub queue_limit: Duration,
    /// Whole wait of one step: halt after this.
    pub total_limit: Duration,
}

impl Timing {
    /// The values the tool uses for real. The poll time can only be raised, never lowered below [`MIN_POLL`].
    #[must_use]
    pub fn standard() -> Timing {
        Timing {
            poll: MIN_POLL,
            poll_max: Duration::from_secs(120),
            start_limit: Duration::from_mins(10),
            queue_limit: Duration::from_mins(20),
            total_limit: Duration::from_mins(90),
        }
    }
}

/// What a run is about.
#[derive(Debug, Clone)]
pub struct Target {
    /// Worktree of the branch to release (never the main checkout).
    pub repo_dir: PathBuf,
    /// `Owner/name` on GitHub.
    pub slug: String,
    pub branch: String,
    /// Branch that gets the fast-forward (`main`).
    pub base: String,
    /// Version without `v`, e.g. `0.0.9`.
    pub version: String,
    /// The fs-release checkout (scripts, signing key); the chain file lives in its `chains/`.
    pub release_dir: PathBuf,
    /// Checkout of the catalog repo used for the bump, and its slug.
    pub catalog_dir: Option<PathBuf>,
    pub catalog_slug: String,
    /// Folder of the approvals (`<dir>/<repo name>/<sha>.ok`); outside every worktree.
    pub approval_dir: PathBuf,
    /// Days an approval stays valid.
    pub approval_days: i64,
    /// Today as days since 1970-01-01.
    pub today_days: i64,
    /// Test mode of the gates (a fake wrapper stands in for them): nothing may be published and nothing is remembered as green.
    pub fake: bool,
    /// Where the run log and the report go.
    pub state_dir: PathBuf,
    /// Published as pre-release (versions 0.0.N are).
    pub prerelease: bool,
    /// Program (and arguments) put in front of the heavy gate run, e.g. the shared build-slot script.
    pub wrapper: Vec<String>,
}

impl Target {
    /// The release tag, `v` plus version.
    #[must_use]
    pub fn tag(&self) -> String {
        format!("v{}", self.version)
    }
}

/// The run context handed to every command.
pub struct Ctx<'a> {
    pub target: Target,
    /// Highest reach this run may touch.
    pub reach: Reach,
    pub timing: Timing,
    pub runner: &'a dyn Runner,
    pub clock: &'a dyn Clock,
    /// When the last GitHub run query went out (for the rate limit).
    pub last_query: Cell<Option<Duration>>,
}

impl<'a> Ctx<'a> {
    #[must_use]
    pub fn new(
        target: Target,
        reach: Reach,
        timing: Timing,
        runner: &'a dyn Runner,
        clock: &'a dyn Clock,
    ) -> Ctx<'a> {
        Ctx {
            target,
            reach,
            timing,
            runner,
            clock,
            last_query: Cell::new(None),
        }
    }
}
