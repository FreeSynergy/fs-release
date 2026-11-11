//! Pattern: Repository over an append-only file (one line per publication: who approved, which commit, which findings, when, how far).

use std::fs::{self, OpenOptions};
use std::io::Write;

use crate::approval::Approval;
use crate::ctx::Ctx;
use crate::meta::Meta;
use crate::outcome::Stop;

/// The audit log; lives next to the approvals, outside every worktree.
pub const AUDIT_LOG: &str = "/srv/FreeSynergy/.scratch/freigaben/AUDIT.log";

/// `YYYY-MM-DDTHH:MM:SSZ` of a unix time (UTC, civil calendar).
#[must_use]
pub fn iso(secs: i64) -> String {
    let (days, rest) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        rest / 3_600,
        rest % 3_600 / 60,
        rest % 60
    )
}

/// The line for one publishing command: time, who wrote the approval (`by`), repo, commit, hash of the findings,
/// the command, the reach of the run and the result (`ok`, `red`, `halt`).
#[must_use]
pub fn line(ctx: &Ctx, a: &Approval, head: &str, meta: &Meta, result: &str) -> String {
    format!(
        "{} by={} repo={} sha={head} findings={} step={} reach={} result={result}",
        iso(ctx.target.now_secs),
        a.by.trim(),
        ctx.target.slug,
        a.findings_sha256,
        meta.name,
        ctx.reach.word()
    )
}

/// Appends the line to the audit log (the file is only ever opened for appending) and keeps it for the output.
///
/// # Errors
/// [`Stop::Red`] when the log cannot be written: a publication nobody can trace is not acceptable.
pub fn record(ctx: &Ctx, line: &str) -> Result<(), Stop> {
    let path = &ctx.target.audit_log;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| Stop::red(format!("audit log {}: {e}", path.display())))?;
    writeln!(f, "{line}").map_err(|e| Stop::red(format!("audit log {}: {e}", path.display())))?;
    ctx.audit.borrow_mut().push(line.to_string());
    Ok(())
}
