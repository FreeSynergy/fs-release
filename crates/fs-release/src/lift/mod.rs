//! Pattern: Facade (one call `run` lifts a worktree: the steps in fixed order, then the report; no LLM, no network by default).

use std::path::{Path, PathBuf};

use fs_release_chain::outcome::Stop;
use fs_release_chain::sys::{Cmd, Out, Runner};

use crate::task::{run_all, Task};

mod checks;
mod commit;
mod report;
mod steps;

pub use report::{Pin, Report};

/// The shared build slots: every cargo call of a lift goes through this script when it exists.
pub const BUILD_SLOT: &str = "/srv/FreeSynergy/.scratch/build-slot.sh";
/// Throw-away git config that maps the GitHub URLs of fs-components / fs-interfaces to the local repos (tags not pushed yet).
pub const LOCAL_TAGS: &str = "/srv/FreeSynergy/.scratch/gitconfig-local-tags";

/// What one lift is asked to do.
#[derive(Debug, Clone)]
pub struct LiftOpts {
    /// Worktree of the repo (the one that gets lifted).
    pub repo_dir: PathBuf,
    /// `vX.Y.Z` for the fs-components dependency (None: leave it).
    pub components: Option<String>,
    /// `vX.Y.Z` for the fs-interfaces dependency (None: leave it).
    pub interfaces: Option<String>,
    /// Real tags from GitHub. Without it the local-tags git config is used (nothing leaves this machine).
    pub online: bool,
    /// The fs-release checkout (scripts, `RUST_VERSION`).
    pub release_dir: PathBuf,
    /// Git config for local tags.
    pub git_config: PathBuf,
    /// Program (and arguments) put in front of every cargo call.
    pub wrapper: Vec<String>,
    /// Alternative list for the Blinc lock gate (tests); default: the one next to the script.
    pub approved: Option<PathBuf>,
    /// Extra environment for every program (tests).
    pub env: Vec<(String, String)>,
    /// After green steps: commit the changed `Cargo.toml` files (and `Cargo.lock` when online).
    pub commit: bool,
}

impl LiftOpts {
    /// Options with the production defaults.
    #[must_use]
    pub fn new(repo_dir: PathBuf, release_dir: PathBuf) -> LiftOpts {
        LiftOpts {
            repo_dir,
            components: None,
            interfaces: None,
            online: false,
            release_dir,
            git_config: PathBuf::from(LOCAL_TAGS),
            wrapper: if Path::new(BUILD_SLOT).is_file() {
                vec![BUILD_SLOT.to_string()]
            } else {
                vec![]
            },
            approved: None,
            env: vec![],
            commit: false,
        }
    }

    /// Where the report goes.
    #[must_use]
    pub fn report_dir(&self) -> PathBuf {
        self.repo_dir.join(".fs-release")
    }
}

/// What a lift step sees.
pub struct LiftEnv<'a> {
    pub opts: &'a LiftOpts,
    pub runner: &'a dyn Runner,
}

impl LiftEnv<'_> {
    /// Runs a program in the worktree with the extra environment.
    ///
    /// # Errors
    /// [`Stop::Red`] when the program cannot be started.
    pub fn exec(
        &self,
        program: &str,
        args: Vec<String>,
        env: Vec<(String, String)>,
    ) -> Result<Out, Stop> {
        let mut all = self.opts.env.clone();
        all.extend(env);
        let cmd = Cmd {
            program: program.into(),
            args,
            cwd: Some(self.opts.repo_dir.clone()),
            env: all,
        };
        self.runner
            .run(&cmd)
            .map_err(|e| Stop::red(format!("cannot start {program}: {e}")))
    }

    /// Runs `cargo <args>` through the wrapper, with the pinned toolchain and (offline) the local tags.
    ///
    /// # Errors
    /// [`Stop::Red`] when cargo cannot be started.
    pub fn cargo(&self, args: &[&str]) -> Result<Out, Stop> {
        let mut words = self.opts.wrapper.clone();
        // The slot script adds `-j<N>` to a cargo call; only the compiling subcommands accept it. For the others (update,
        // generate-lockfile, fmt) `env` in front keeps the slot (and its memory limit) but hides the call from that rule.
        if !matches!(
            args.first().copied(),
            Some("build" | "check" | "clippy" | "test")
        ) && !words.is_empty()
        {
            words.push("env".into());
        }
        words.push("cargo".into());
        words.extend(args.iter().map(|a| (*a).to_string()));
        let program = words.remove(0);
        let mut env = Vec::new();
        if let Ok(v) = std::fs::read_to_string(self.opts.release_dir.join("RUST_VERSION")) {
            env.push(("RUSTUP_TOOLCHAIN".to_string(), v.trim().to_string()));
        }
        if !self.opts.online {
            env.push((
                "GIT_CONFIG_GLOBAL".into(),
                self.opts.git_config.display().to_string(),
            ));
            env.push(("CARGO_NET_GIT_FETCH_WITH_CLI".into(), "true".into()));
        }
        self.exec(&program, words, env)
    }

    /// Path of a script of the fs-release checkout.
    #[must_use]
    pub fn script(&self, name: &str) -> String {
        self.opts
            .release_dir
            .join("scripts")
            .join(name)
            .display()
            .to_string()
    }
}

/// The steps in order. The checks after the lock are independent: all of them run, so one report shows every red.
#[must_use]
pub fn tasks<'a>(opts: &LiftOpts) -> Vec<Box<dyn Task<LiftEnv<'a>>>> {
    let mut v: Vec<Box<dyn Task<LiftEnv<'a>>>> = vec![
        Box::new(steps::Deps),
        Box::new(steps::Lock),
        Box::new(steps::LockGate),
        Box::new(checks::Fmt),
        Box::new(checks::Clippy),
        Box::new(checks::Rules),
        Box::new(checks::Tests),
    ];
    if opts.commit {
        v.push(Box::new(commit::Commit));
    }
    v
}

/// Runs the whole lift and writes `<repo>/.fs-release/lift-report.{md,json}`. Exit code 0 green, 1 red.
#[must_use]
pub fn run(opts: &LiftOpts, runner: &dyn Runner) -> (Report, i32) {
    let started = std::time::Instant::now();
    let mut rep = report::start(opts, runner);
    let ok = if opts.components.is_none() && opts.interfaces.is_none() {
        rep.steps = vec![crate::task::Record {
            name: "setup".into(),
            status: "red".into(),
            detail: String::new(),
            first_error: Some("nothing to lift: give --components and/or --interfaces".into()),
            secs: 0.0,
        }];
        false
    } else {
        let env = LiftEnv { opts, runner };
        let list = tasks(opts);
        let (records, first) = run_all(&list, &env);
        rep.steps = records;
        first.is_none()
    };
    rep.finish(opts, runner, ok, started.elapsed().as_secs_f64());
    let code = i32::from(!ok);
    (rep, code)
}
