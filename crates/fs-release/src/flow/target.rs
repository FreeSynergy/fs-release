//! Pattern: Factory (builds the chain's run target from a worktree: slug from `origin`, fixed approval folder and audit log).

use std::path::{Path, PathBuf};

use fs_release_chain::approval::{APPROVAL_DAYS, APPROVAL_DIR};
use fs_release_chain::audit::AUDIT_LOG;
use fs_release_chain::ctx::Target;
use fs_release_chain::origin::{configured_slug, now_secs, today_days};
use fs_release_chain::sys::{Cmd, Runner};

/// The target of a flow run. The approval folder and the audit log are the fixed ones of the chain: there is no way to
/// point a run at other approvals.
///
/// # Errors
/// A sentence when the worktree has no branch, no GitHub origin or the version is not `N.N.N`.
pub fn build_target(
    runner: &dyn Runner,
    repo_dir: &Path,
    release_dir: &Path,
    version: &str,
    base: &str,
    branch: Option<&str>,
) -> Result<Target, String> {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() != 3
        || parts
            .iter()
            .any(|p| p.is_empty() || !p.chars().all(|c| c.is_ascii_digit()))
    {
        return Err(format!("--version {version}: use N.N.N without the v"));
    }
    fs_release_chain::slug::validate_branch(base).map_err(|e| format!("--base: {e}"))?;
    let git = |args: &[&str]| -> Result<String, String> {
        let cmd = Cmd {
            program: "git".into(),
            args: args.iter().map(|a| (*a).into()).collect(),
            cwd: Some(repo_dir.to_path_buf()),
            ..Cmd::default()
        };
        let o = runner.run(&cmd).map_err(|e| e.to_string())?;
        if o.ok() {
            Ok(o.stdout.trim().to_string())
        } else {
            Err(o.first_error())
        }
    };
    let branch = match branch {
        Some(b) => b.to_string(),
        None => git(&["symbolic-ref", "--short", "HEAD"])?,
    };
    let slug = configured_slug(runner, repo_dir)?;
    let state_dir =
        PathBuf::from(git(&["rev-parse", "--absolute-git-dir"])?).join("fs-release-chain");
    Ok(Target {
        repo_dir: repo_dir.to_path_buf(),
        slug,
        branch,
        base: base.to_string(),
        version: version.to_string(),
        release_dir: release_dir.to_path_buf(),
        catalog_dir: None,
        catalog_slug: "FreeSynergy/fs-catalog".into(),
        approval_dir: PathBuf::from(APPROVAL_DIR),
        approval_days: APPROVAL_DAYS,
        today_days: today_days(),
        now_secs: now_secs(),
        audit_log: PathBuf::from(AUDIT_LOG),
        fake: false,
        state_dir,
        prerelease: true,
        wrapper: vec![],
    })
}
