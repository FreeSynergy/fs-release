//! Pattern: Specification (the one predicate "this approval covers exactly this commit of this repo, is young enough and
//! names the findings it was based on").

use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::ctx::Ctx;
use crate::outcome::Stop;
use crate::sys::Cmd;

/// The only place approvals are read from: outside every worktree, one file per commit,
/// `<dir>/<repo name>/<full sha>.ok`. There is no option to read one from somewhere else.
pub const APPROVAL_DIR: &str = "/srv/FreeSynergy/.scratch/freigaben";

/// Days an approval stays valid.
pub const APPROVAL_DAYS: i64 = 7;

/// The content of `<sha>.ok`: who looked at which commit of which repo, when, based on which findings, and what they said.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Approval {
    pub repo: String,
    /// Full 40-character commit sha. A new commit on the branch makes the approval worthless.
    pub sha: String,
    /// `YYYY-MM-DD`; the approval expires after [`APPROVAL_DAYS`].
    pub date: String,
    /// `JA` or `NEIN`.
    pub verdict: String,
    /// Who approved. Required.
    pub by: String,
    /// The findings file of the review and its sha256 (a changed file voids the approval).
    pub findings: PathBuf,
    pub findings_sha256: String,
}

/// Days since 1970-01-01 of a `YYYY-MM-DD` date (civil calendar), or `None` when it is not one.
#[must_use]
pub fn parse_date(text: &str) -> Option<i64> {
    let mut p = text.split('-');
    let (y, m, d): (i64, i64, i64) = (
        p.next()?.parse().ok()?,
        p.next()?.parse().ok()?,
        p.next()?.parse().ok()?,
    );
    if p.next().is_some() || !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146_097 + doe - 719_468)
}

impl Approval {
    /// Reads `<dir>/<repo name>/<head>.ok`. The slug and the sha are validated first (no path leaves the folder), and a
    /// symbolic link for the repo folder or the file is refused.
    ///
    /// # Errors
    /// [`Stop::Halt`] `approval-stale` when approvals exist for other commits only, `review` when there is none at all;
    /// [`Stop::Red`] when the slug or sha is not valid, a link is in the way, or the file is damaged.
    pub fn load(dir: &Path, slug: &str, head: &str) -> Result<(Approval, PathBuf), Stop> {
        crate::slug::validate(slug).map_err(Stop::red)?;
        if head.len() != 40 || !head.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(Stop::red(format!("not a full commit sha: {head:?}")));
        }
        let repo_dir = dir.join(crate::slug::name(slug));
        let path = repo_dir.join(format!("{head}.ok"));
        let is_link = |p: &Path| fs::symlink_metadata(p).is_ok_and(|m| m.file_type().is_symlink());
        if is_link(&repo_dir) || is_link(&path) {
            return Err(Stop::red(format!(
                "{} is a symbolic link: approvals must be plain files",
                path.display()
            )));
        }
        let Ok(text) = fs::read_to_string(&path) else {
            let others = fs::read_dir(&repo_dir).map_or(0, |d| {
                d.filter_map(Result::ok)
                    .filter(|e| e.path().extension().is_some_and(|x| x == "ok"))
                    .count()
            });
            return Err(if others > 0 {
                Stop::halt(
                    "approval-stale",
                    format!("a new JA for {head} (approvals exist for other commits only)"),
                )
            } else {
                Stop::halt("review", format!("approval file {} (repo, sha, date, verdict: JA, by, findings, findings_sha256)", path.display()))
            });
        };
        let a = serde_norway::from_str(&text)
            .map_err(|e| Stop::red(format!("{}: {e}", path.display())))?;
        Ok((a, path))
    }

    /// True when the approval is for `repo`, says yes, names `head` in full, has an author, a findings hash, and is not
    /// older than `days` nor from the future.
    ///
    /// # Errors
    /// [`Stop::Halt`] with the reason: `review-no`, `approval-repo`, `approval-stale`, `approval-expired`, `approval-incomplete`.
    pub fn covers(&self, repo: &str, head: &str, today: i64, days: i64) -> Result<(), Stop> {
        if !self.verdict.trim().eq_ignore_ascii_case("ja") {
            return Err(Stop::halt(
                "review-no",
                format!("a new review of {head} (verdict was {})", self.verdict),
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
        let hash_ok = self.findings_sha256.len() == 64
            && self.findings_sha256.chars().all(|c| c.is_ascii_hexdigit());
        if self.by.trim().is_empty() || !hash_ok {
            return Err(Stop::halt(
                "approval-incomplete",
                "an approval with `by` and the 64-hex findings_sha256",
            ));
        }
        match parse_date(&self.date) {
            Some(d) if d <= today && today - d <= days => Ok(()),
            Some(_) => Err(Stop::halt(
                "approval-expired",
                format!("a new JA (dated {}, valid {days} days)", self.date),
            )),
            None => Err(Stop::halt("approval-incomplete", "a date as YYYY-MM-DD")),
        }
    }
}

/// One way to tell that an approval is real. Today only the findings hash is checked ([`FindingsHash`]); a signature
/// check (`ssh-keygen -Y verify` against a fixed `allowed_signers` file) is another implementation added to [`verifiers`],
/// nothing else changes.
pub trait ApprovalVerifier {
    /// # Errors
    /// [`Stop::Halt`] when the approval is not proven by this verifier.
    fn check(&self, ctx: &Ctx, approval: &Approval, file: &Path) -> Result<(), Stop>;
}

/// Compares the findings file with the hash the approval names. Protects against mistakes (a changed or wrong findings
/// file), not against somebody who writes both files: whoever can write the approval can write the hash.
pub struct FindingsHash;

impl ApprovalVerifier for FindingsHash {
    fn check(&self, ctx: &Ctx, a: &Approval, _file: &Path) -> Result<(), Stop> {
        let cmd = Cmd {
            program: "sha256sum".into(),
            args: vec![a.findings.display().to_string()],
            ..Cmd::default()
        };
        let o = ctx.runner.run(&cmd)?;
        let have = o.stdout.split_whitespace().next().unwrap_or("");
        if o.ok() && have.eq_ignore_ascii_case(&a.findings_sha256) {
            Ok(())
        } else {
            Err(Stop::halt(
                "approval-findings",
                format!(
                    "the findings file {} as it was when approved",
                    a.findings.display()
                ),
            ))
        }
    }
}

/// The verifiers every approval has to pass, in order.
#[must_use]
pub fn verifiers() -> Vec<Box<dyn ApprovalVerifier>> {
    vec![Box::new(FindingsHash)]
}

/// Loads and checks the approval of `head`: the file, its content, and every verifier.
///
/// # Errors
/// [`Stop::Halt`] for everything [`Approval::load`], [`Approval::covers`] and the verifiers say.
pub fn verify(ctx: &Ctx, head: &str) -> Result<Approval, Stop> {
    let t = &ctx.target;
    let (a, file) = Approval::load(&t.approval_dir, &t.slug, head)?;
    a.covers(&t.slug, head, t.today_days, t.approval_days)?;
    for v in verifiers() {
        v.check(ctx, &a, &file)?;
    }
    Ok(a)
}
