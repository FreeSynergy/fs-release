//! Pattern: Strategy (one type per command behind the `Step` trait; the registry hands them out in chain order).

use crate::ctx::Ctx;
use crate::ledger::Ledger;
use crate::meta::Meta;
use crate::outcome::StepResult;

mod catalog;
mod ci;
mod local;
mod publish;

pub use catalog::{Bump, Check, Sign};
pub use ci::{Release, WaitCi};
pub use local::{Gates, Report, Review};
pub use publish::{MergeFf, Push, Tag};

/// What a command sees: the run context, the log, and the commit the branch head is at right now.
pub struct Frame<'a> {
    pub ctx: &'a Ctx<'a>,
    pub ledger: &'a Ledger,
    pub head: &'a str,
}

/// One command of the chain. Every command is idempotent: it first checks whether its result already exists and then
/// reports it as done without doing it a second time.
pub trait Step {
    /// Fixed facts: name, risk, reach, preview sentence, undo.
    fn meta(&self) -> Meta;

    /// Does the work or finds it done.
    ///
    /// # Errors
    /// A [`Stop`](crate::outcome::Stop): halt (somebody else has to act) or red (a check failed).
    fn run(&self, f: &Frame) -> StepResult;
}
