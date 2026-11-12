//! Pattern: Strategy (the three steps that change and judge the dependencies: deps, lock, lock-gate).

use std::fs;

use fs_release_chain::outcome::Stop;

use super::LiftEnv;
use crate::task::Task;

/// The repos whose dependency is lifted, with the tag the caller asked for.
fn wanted(env: &LiftEnv) -> Vec<(&'static str, String)> {
    let o = env.opts;
    [
        ("fs-components", &o.components),
        ("fs-interfaces", &o.interfaces),
    ]
    .into_iter()
    .filter_map(|(n, t)| t.clone().map(|t| (n, t)))
    .collect()
}

/// `vX.Y.Z` and nothing else: no branch, no rev, no range.
fn valid_tag(tag: &str) -> bool {
    let Some(rest) = tag.strip_prefix('v') else {
        return false;
    };
    let parts: Vec<&str> = rest.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
}

/// Sets the git tag of every dependency on fs-components / fs-interfaces, in every crate of the workspace, to exactly the
/// given one (the helper script edits line by line and lets the TOML parser judge the result).
pub struct Deps;

impl Task<LiftEnv<'_>> for Deps {
    fn name(&self) -> &'static str {
        "deps"
    }

    fn run(&self, env: &LiftEnv) -> Result<String, Stop> {
        let mut notes = Vec::new();
        let mut found = 0;
        for (repo, tag) in wanted(env) {
            if !valid_tag(&tag) {
                return Err(Stop::red(format!(
                    "{tag:?} is not a tag like v0.0.12 ({repo})"
                )));
            }
            let args = vec![
                "-I".to_string(),
                env.script("set_components_tag.py"),
                tag.clone(),
                env.opts.repo_dir.display().to_string(),
                repo.to_string(),
            ];
            let o = env.exec("python3", args, vec![])?;
            match o.code {
                0 => {
                    found += 1;
                    notes.push(format!("{repo} {tag}: {}", o.stdout.trim()));
                }
                3 => notes.push(format!("{repo}: no dependency, left alone")),
                _ => return Err(Stop::red(format!("{repo}: {}", o.first_error()))),
            }
        }
        if found == 0 {
            return Err(Stop::red(
                "no dependency on fs-components / fs-interfaces found in the workspace",
            ));
        }
        Ok(notes.join("; "))
    }
}

/// Brings `Cargo.lock` in line with the new tags (`cargo update --workspace`: only what the manifests changed is re-resolved, so
/// nothing else moves), then checks that the lock really holds the asked tags.
/// Offline (default) the local tags are read through the throw-away git config; the lock is not committed by this step.
pub struct Lock;

impl Task<LiftEnv<'_>> for Lock {
    fn name(&self) -> &'static str {
        "lock"
    }

    fn run(&self, env: &LiftEnv) -> Result<String, Stop> {
        if !env.opts.online && !env.opts.git_config.is_file() {
            return Err(Stop::red(format!(
                "local-tags git config {} is missing (use --online for real tags)",
                env.opts.git_config.display()
            )));
        }
        if !env.opts.repo_dir.join("Cargo.lock").is_file() {
            let o = env.cargo(&["generate-lockfile"])?;
            if !o.ok() {
                return Err(Stop::red(format!("generate-lockfile: {}", o.first_error())));
            }
        }
        let o = env.cargo(&["update", "--workspace"])?;
        if !o.ok() {
            return Err(Stop::red(format!(
                "cargo update --workspace: {}",
                o.first_error()
            )));
        }
        let lock = fs::read_to_string(env.opts.repo_dir.join("Cargo.lock")).unwrap_or_default();
        let mut done = Vec::new();
        for (repo, tag) in wanted(env) {
            if depends_on(env, repo) {
                if !lock.contains(&format!("github.com/FreeSynergy/{repo}?tag={tag}#")) {
                    return Err(Stop::red(format!(
                        "Cargo.lock does not hold {repo} {tag} after the update"
                    )));
                }
                done.push(format!("{repo} {tag}"));
            }
        }
        let mode = if env.opts.online {
            "online"
        } else {
            "local tags"
        };
        Ok(format!(
            "Cargo.lock regenerated ({mode}): {}",
            done.join(", ")
        ))
    }
}

/// Does the workspace still name this repo in any Cargo.toml (the deps step said so)? A cheap text look, no parsing.
fn depends_on(env: &LiftEnv, repo: &str) -> bool {
    fn walk(dir: &std::path::Path, repo: &str) -> bool {
        let Ok(rd) = fs::read_dir(dir) else {
            return false;
        };
        rd.filter_map(Result::ok).any(|e| {
            let p = e.path();
            let name = e.file_name().to_string_lossy().into_owned();
            if p.is_dir() {
                !(name == "target" || name.starts_with('.')) && walk(&p, repo)
            } else {
                name == "Cargo.toml"
                    && fs::read_to_string(&p).is_ok_and(|t| t.contains(&format!("/{repo}")))
            }
        })
    }
    walk(&env.opts.repo_dir, repo)
}

/// The Blinc lock gate: exactly one Blinc source in `Cargo.lock`, and it is in `blinc-approved.txt`.
pub struct LockGate;

impl Task<LiftEnv<'_>> for LockGate {
    fn name(&self) -> &'static str {
        "lock-gate"
    }

    fn run(&self, env: &LiftEnv) -> Result<String, Stop> {
        let mut args = vec!["-I".to_string(), env.script("check_blinc_lock.py")];
        if let Some(f) = &env.opts.approved {
            args.extend(["--approved".to_string(), f.display().to_string()]);
        }
        args.push(env.opts.repo_dir.display().to_string());
        let o = env.exec("python3", args, vec![])?;
        if o.ok() {
            Ok(o.stdout
                .lines()
                .last()
                .unwrap_or("blinc lock ok")
                .trim()
                .to_string())
        } else {
            Err(Stop::red(format!("blinc-lock: {}", o.first_error())))
        }
    }
}
