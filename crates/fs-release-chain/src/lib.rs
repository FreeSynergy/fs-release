//! Pattern: none (module list and re-exports of the release chain library).
//!
//! The release chain as one command: gates, review, push, wait-ci, merge-ff, tag, release, bump, sign, check, report.
//! Output of a run is one line: `DONE`, `HALT` (somebody else has to act) or `RED` (a check failed); exit 0, 10, 1.

pub mod approval;
pub mod audit;
pub mod chain;
pub mod cli;
pub mod ctx;
pub mod gh;
pub mod git;
pub mod ledger;
pub mod meta;
pub mod origin;
pub mod outcome;
pub mod packages;
pub mod poll;
pub mod preapproved;
pub mod profile;
pub mod registry;
pub mod runner;
pub mod slug;
pub mod steps;
pub mod sys;

pub use ctx::{Ctx, Target, Timing};
pub use outcome::Outcome;
