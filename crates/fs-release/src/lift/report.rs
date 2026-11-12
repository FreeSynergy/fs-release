//! Pattern: Builder (collects heads, pins and step records into one report; renders Markdown and JSON; writes both files).

use std::fs;
use std::path::Path;

use fs_release_chain::sys::{Cmd, Runner};
use serde::Serialize;

use super::LiftOpts;
use crate::task::Record;

/// A git dependency the lock resolved: repo, tag and the commit the tag points at.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Pin {
    pub repo: String,
    pub tag: String,
    pub sha: String,
}

/// The lift report: what an agent reads when (and only when) something is red.
#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub repo: String,
    pub repo_dir: String,
    pub branch: String,
    /// Commit of the worktree head after the run.
    pub head: String,
    /// Files with uncommitted changes after the run (the lift edits `Cargo.toml`, the lock stays uncommitted).
    pub dirty: Vec<String>,
    pub components: Option<String>,
    pub interfaces: Option<String>,
    pub online: bool,
    pub ok: bool,
    pub secs: f64,
    /// fs-components / fs-interfaces / Blinc commits as the lock holds them.
    pub pins: Vec<Pin>,
    pub steps: Vec<Record>,
}

fn git(runner: &dyn Runner, dir: &Path, args: &[&str]) -> String {
    let cmd = Cmd {
        program: "git".into(),
        args: args.iter().map(|a| (*a).to_string()).collect(),
        cwd: Some(dir.to_path_buf()),
        ..Cmd::default()
    };
    runner
        .run(&cmd)
        .map(|o| o.stdout.trim().to_string())
        .unwrap_or_default()
}

/// A report with the request filled in; steps and heads come later.
pub fn start(opts: &LiftOpts, runner: &dyn Runner) -> Report {
    let dir = &opts.repo_dir;
    let name = fs::canonicalize(dir)
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()));
    Report {
        repo: name.unwrap_or_else(|| dir.display().to_string()),
        repo_dir: dir.display().to_string(),
        branch: git(runner, dir, &["symbolic-ref", "--short", "HEAD"]),
        head: String::new(),
        dirty: vec![],
        components: opts.components.clone(),
        interfaces: opts.interfaces.clone(),
        online: opts.online,
        ok: false,
        secs: 0.0,
        pins: vec![],
        steps: vec![],
    }
}

/// `git+https://github.com/<owner>/<repo>?tag=<tag>#<sha>` lines of a lock, for the repos that matter.
fn pins_of(lock: &str) -> Vec<Pin> {
    let mut pins: Vec<Pin> = Vec::new();
    for line in lock.lines() {
        let Some(src) = line
            .trim()
            .strip_prefix("source = \"git+https://github.com/FreeSynergy/")
        else {
            continue;
        };
        let src = src.trim_end_matches('"');
        let Some((repo, rest)) = src.split_once("?tag=") else {
            continue;
        };
        let Some((tag, sha)) = rest.split_once('#') else {
            continue;
        };
        let pin = Pin {
            repo: repo.to_string(),
            tag: tag.to_string(),
            sha: sha.to_string(),
        };
        if !pins.contains(&pin) {
            pins.push(pin);
        }
    }
    pins
}

impl Report {
    /// Fills in the heads, the verdict and the duration, then writes both files.
    pub fn finish(&mut self, opts: &LiftOpts, runner: &dyn Runner, ok: bool, secs: f64) {
        let dir = &opts.repo_dir;
        self.head = git(runner, dir, &["rev-parse", "HEAD"]);
        self.dirty = git(
            runner,
            dir,
            &["status", "--porcelain", "--untracked-files=no"],
        )
        .lines()
        .map(|l| l.trim().to_string())
        .collect();
        self.pins = fs::read_to_string(dir.join("Cargo.lock"))
            .map(|t| pins_of(&t))
            .unwrap_or_default();
        self.ok = ok;
        self.secs = secs;
        let out = opts.report_dir();
        if fs::create_dir_all(&out).is_ok() {
            // The folder ignores itself: the report is never committed.
            let _ = fs::write(out.join(".gitignore"), "*\n");
            let _ = fs::write(out.join("lift-report.md"), self.markdown());
            let _ = fs::write(out.join("lift-report.json"), self.json());
        }
    }

    /// Machine-readable form.
    #[must_use]
    pub fn json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".into()) + "\n"
    }

    /// Human form (an agent reads this first).
    #[must_use]
    pub fn markdown(&self) -> String {
        let mode = if self.online { "online" } else { "local tags" };
        let mut m = vec![
            format!("# Lift report: {} {}\n", self.repo, if self.ok { "GREEN" } else { "RED" }),
            format!("- branch `{}`, head `{}`", self.branch, self.head),
            format!(
                "- components {}, interfaces {}, {mode}",
                self.components.as_deref().unwrap_or("-"),
                self.interfaces.as_deref().unwrap_or("-")
            ),
            format!("- duration {:.1} s", self.secs),
        ];
        if !self.dirty.is_empty() {
            m.push(format!("- uncommitted: {}", self.dirty.join(", ")));
        }
        for p in &self.pins {
            m.push(format!("- pin {} {} `{}`", p.repo, p.tag, p.sha));
        }
        m.push("\n| step | result | s | first error / note |\n|---|---|---|---|".into());
        for s in &self.steps {
            let text = s.first_error.clone().unwrap_or_else(|| s.detail.clone()).replace('|', "/");
            m.push(format!("| {} | {} | {:.1} | {} |", s.name, s.status, s.secs, text));
        }
        m.join("\n") + "\n"
    }
}
