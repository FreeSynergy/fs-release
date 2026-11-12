//! Pattern: Strategy (the review-request step: writes the file Opus reads before it writes a JA).

use std::fs;

use fs_release_chain::ctx::Ctx;
use fs_release_chain::git;
use fs_release_chain::outcome::Stop;

use super::{green_report, guard, report_path};
use crate::task::Task;

/// Writes `<worktree>/.fs-release/review-request.md`: what to review (repo, branch, head, diff head), the lift report to read
/// and where the approval file has to be written. Needs a green lift report for exactly this head. Changes nothing else.
pub struct ReviewRequest;

/// Lines of the diff stat that go into the request.
const DIFF_LINES: usize = 40;

impl Task<Ctx<'_>> for ReviewRequest {
    fn name(&self) -> &'static str {
        "review-request"
    }

    fn run(&self, ctx: &Ctx) -> Result<String, Stop> {
        let head = guard(ctx)?;
        green_report(ctx, &head)?;
        let t = &ctx.target;
        let range = format!("{}...HEAD", t.base);
        let remote_range = format!("origin/{}...HEAD", t.base);
        let stat = [remote_range, range]
            .iter()
            .find_map(|r| {
                let o = git::raw(ctx, &t.repo_dir, &["diff", "--stat", r]).ok()?;
                o.ok().then_some(o.stdout)
            })
            .unwrap_or_else(|| "(no diff against the base branch available)\n".into());
        let diff: Vec<&str> = stat.lines().collect();
        let shown = diff
            .iter()
            .take(DIFF_LINES)
            .copied()
            .collect::<Vec<_>>()
            .join("\n");
        let more = diff.len().saturating_sub(DIFF_LINES);
        let approval = t
            .approval_dir
            .join(fs_release_chain::slug::name(&t.slug))
            .join(format!("{head}.ok"));
        let text = format!(
            "# Review request: {slug} {tag}\n\n- branch `{branch}`, head `{head}`\n- lift report: {report}\n- approval file to write (JA): {approval}\n  fields: repo, sha, date, verdict (JA), by, findings, findings_sha256\n\n## Diff against {base}\n\n```\n{shown}\n```\n{more_note}",
            slug = t.slug,
            tag = t.tag(),
            branch = t.branch,
            report = report_path(ctx).display(),
            approval = approval.display(),
            base = t.base,
            more_note = if more > 0 { format!("\n({more} more lines)\n") } else { String::new() },
        );
        let out = t.repo_dir.join(".fs-release/review-request.md");
        fs::write(&out, text)?;
        Ok(format!("review request {}", out.display()))
    }
}
