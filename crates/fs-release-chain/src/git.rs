//! Pattern: Facade (the few git calls the chain needs, each with its error already turned into a stop).

use std::path::Path;

use crate::ctx::Ctx;
use crate::outcome::Stop;
use crate::sys::{Cmd, Out};

/// Runs `git args` in `dir`.
///
/// # Errors
/// [`Stop::Red`] when git cannot be started.
pub fn raw(ctx: &Ctx, dir: &Path, args: &[&str]) -> Result<Out, Stop> {
    let cmd = Cmd {
        program: "git".into(),
        args: args.iter().map(|a| (*a).to_string()).collect(),
        cwd: Some(dir.to_path_buf()),
        ..Cmd::default()
    };
    Ok(ctx.runner.run(&cmd)?)
}

/// Runs git and returns trimmed stdout.
///
/// # Errors
/// [`Stop::Red`] with the first error line when git exits non-zero.
pub fn out(ctx: &Ctx, dir: &Path, args: &[&str]) -> Result<String, Stop> {
    let o = raw(ctx, dir, args)?;
    if o.ok() {
        Ok(o.stdout.trim().to_string())
    } else {
        Err(Stop::red(format!(
            "git {}: {}",
            args.first().unwrap_or(&""),
            o.first_error()
        )))
    }
}

/// `git fetch origin <branch>` (the branch after `--end-of-options`: it never counts as an option), result as it is.
///
/// # Errors
/// [`Stop::Red`] when git cannot be started.
pub fn fetch_raw(ctx: &Ctx, dir: &Path, branch: &str) -> Result<Out, Stop> {
    let args = ["fetch", "--quiet", "--end-of-options", "origin", branch];
    raw(ctx, dir, &args)
}

/// As [`fetch_raw`], but a failed fetch is a stop.
///
/// # Errors
/// [`Stop::Red`] with the first error line when the fetch fails.
pub fn fetch(ctx: &Ctx, dir: &Path, branch: &str) -> Result<(), Stop> {
    let o = fetch_raw(ctx, dir, branch)?;
    if o.ok() {
        Ok(())
    } else {
        Err(Stop::red(format!("git fetch: {}", o.first_error())))
    }
}

/// Full sha of `rev` (`HEAD`, a branch, `v1^{commit}`).
///
/// # Errors
/// [`Stop::Red`] when the revision does not exist.
pub fn rev(ctx: &Ctx, dir: &Path, rev: &str) -> Result<String, Stop> {
    out(ctx, dir, &["rev-parse", "--verify", "--quiet", rev])
}

/// Sha a ref has on the remote `origin` (a tag is peeled to its commit), or `None` when the remote has no such ref.
///
/// # Errors
/// [`Stop::Red`] when the remote cannot be asked.
pub fn remote_ref(ctx: &Ctx, dir: &Path, name: &str) -> Result<Option<String>, Stop> {
    let plain = format!("refs/{name}");
    let peeled = format!("{plain}^{{}}");
    let text = out(ctx, dir, &["ls-remote", "origin", &plain, &peeled])?;
    let find = |want: &str| {
        text.lines().find_map(|l| {
            l.split_once('\t')
                .filter(|(_, r)| *r == want)
                .map(|(s, _)| s.to_string())
        })
    };
    Ok(find(&peeled).or_else(|| find(&plain)))
}

/// Remote branch head.
///
/// # Errors
/// As [`remote_ref`].
pub fn remote_branch(ctx: &Ctx, dir: &Path, branch: &str) -> Result<Option<String>, Stop> {
    remote_ref(ctx, dir, &format!("heads/{branch}"))
}

/// True when `ancestor` is reachable from `descendant` in the local object store.
///
/// # Errors
/// [`Stop::Red`] when git cannot be started.
pub fn is_ancestor(ctx: &Ctx, dir: &Path, ancestor: &str, descendant: &str) -> Result<bool, Stop> {
    Ok(raw(
        ctx,
        dir,
        &["merge-base", "--is-ancestor", ancestor, descendant],
    )?
    .ok())
}

/// First seven characters of a sha, for lines a person reads.
#[must_use]
pub fn short(sha: &str) -> &str {
    sha.get(..7).unwrap_or(sha)
}
