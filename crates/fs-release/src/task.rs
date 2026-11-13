//! Pattern: Strategy + Template method (one unit of work behind a trait; one loop runs any list of them and keeps records).

use std::time::Instant;

use fs_release_chain::outcome::Stop;
use serde::Serialize;

/// A step returns `Ok(format!("{SKIPPED}<why>"))` when it could not do its check (a missing tool): recorded as `skipped`
/// with the reason, never as `ok`.
pub const SKIPPED: &str = "skipped: ";

/// Detail of a record that never ran because an earlier step was not green (not a step that skipped its own check).
pub const NOT_RUN: &str = "an earlier step was not green";

/// One step of a run. The same trait serves the lift steps and the flow steps, so a workflow file can call either by name.
pub trait Task<C> {
    /// Fixed name (`deps`, `lock`, `fmt`, `tag`, ...).
    fn name(&self) -> &'static str;

    /// True: a red result is recorded and the following steps still run (independent checks, one report shows all reds).
    /// False: a red result stops the run and the rest is recorded as skipped.
    fn continue_on_red(&self) -> bool {
        false
    }

    /// True: the task only runs when every step before it was green (a commit must not follow a red check).
    fn needs_green(&self) -> bool {
        false
    }

    /// Does the work and returns a short note.
    ///
    /// # Errors
    /// A [`Stop`]: red (a check failed) or halt (somebody else has to act).
    fn run(&self, ctx: &C) -> Result<String, Stop>;
}

/// What happened to one task.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Record {
    pub name: String,
    /// `ok`, `red`, `halt` or `skipped`.
    pub status: String,
    pub detail: String,
    /// First error line of a red or halted step.
    pub first_error: Option<String>,
    pub secs: f64,
}

impl Record {
    /// True for `ok`.
    #[must_use]
    pub fn is_ok(&self) -> bool {
        self.status == "ok"
    }
}

/// Runs the tasks in order. Returns one record per task and the first stop (None when everything is green).
pub fn run_all<C>(tasks: &[Box<dyn Task<C>>], ctx: &C) -> (Vec<Record>, Option<(String, Stop)>) {
    let mut records = Vec::new();
    let mut first: Option<(String, Stop)> = None;
    let mut halted = false;
    let mut any_red = false;
    for t in tasks {
        let name = t.name().to_string();
        if halted || (any_red && t.needs_green()) {
            records.push(Record {
                name,
                status: "skipped".into(),
                detail: NOT_RUN.into(),
                first_error: None,
                secs: 0.0,
            });
            continue;
        }
        let start = Instant::now();
        let result = t.run(ctx);
        let secs = start.elapsed().as_secs_f64();
        match result {
            Ok(detail) => {
                // A step that could not check anything says so; the run is not red, but the record is never `ok`.
                let (status, detail) = match detail.strip_prefix(SKIPPED) {
                    Some(why) => ("skipped", why.to_string()),
                    None => ("ok", detail),
                };
                records.push(Record {
                    name,
                    status: status.into(),
                    detail,
                    first_error: None,
                    secs,
                });
            }
            Err(stop) => {
                let (status, line) = match &stop {
                    Stop::Red { line } => ("red", line.clone()),
                    Stop::Halt { reason, needs } => ("halt", format!("{reason}: {needs}")),
                };
                records.push(Record {
                    name: name.clone(),
                    status: status.into(),
                    detail: String::new(),
                    first_error: Some(line),
                    secs,
                });
                any_red = true;
                halted = !(matches!(stop, Stop::Red { .. }) && t.continue_on_red());
                first.get_or_insert((name, stop));
            }
        }
    }
    (records, first)
}
