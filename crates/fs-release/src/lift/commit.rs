//! Pattern: Strategy (the optional last step: commit what the lift changed, so a review has a commit to look at).

use fs_release_chain::outcome::Stop;

use super::LiftEnv;
use crate::task::Task;

/// Commits the changed `Cargo.toml` files, and `Cargo.lock` only after an online lift (a local-tags lock is not committed).
/// Runs only when every step before it was green.
pub struct Commit;

impl Task<LiftEnv<'_>> for Commit {
    fn name(&self) -> &'static str {
        "commit"
    }

    fn needs_green(&self) -> bool {
        true
    }

    fn run(&self, env: &LiftEnv) -> Result<String, Stop> {
        let git = |args: &[&str]| {
            env.exec(
                "git",
                args.iter().map(|a| (*a).to_string()).collect(),
                vec![],
            )
        };
        let list = git(&["diff", "--name-only"])?;
        let files: Vec<&str> = list
            .stdout
            .lines()
            .filter(|f| f.ends_with("Cargo.toml") || (env.opts.online && *f == "Cargo.lock"))
            .collect();
        if files.is_empty() {
            return Ok("nothing to commit (already on these tags)".into());
        }
        let mut add = vec!["add", "--"];
        add.extend(files.iter().copied());
        let o = git(&add)?;
        if !o.ok() {
            return Err(Stop::red(format!("git add: {}", o.first_error())));
        }
        let (c, i) = (
            env.opts.components.as_deref(),
            env.opts.interfaces.as_deref(),
        );
        let what: Vec<String> = [("fs-components", c), ("fs-interfaces", i)]
            .into_iter()
            .filter_map(|(n, t)| t.map(|t| format!("{n} {t}")))
            .collect();
        let o = git(&[
            "commit",
            "--quiet",
            "-m",
            &format!("lift to {}", what.join(", ")),
        ])?;
        if !o.ok() {
            return Err(Stop::red(format!("git commit: {}", o.first_error())));
        }
        Ok(format!("committed {} file(s)", files.len()))
    }
}
