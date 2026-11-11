//! Pattern: Facade (command line in, one run out: builds the context from the arguments and prints the one-line result).

use std::path::{Path, PathBuf};
use std::time::Duration;

use clap::{Args, Parser, Subcommand};

use crate::chain::Chain;
use crate::ctx::{Ctx, Target, Timing, MIN_POLL};
use crate::ledger::Ledger;
use crate::meta::Reach;
use crate::outcome::Outcome;
use crate::registry;
use crate::runner;
use crate::sys::{Cmd, Runner, SystemClock, SystemRunner};

#[derive(Parser)]
#[command(
    name = "fs-release-chain",
    version,
    about = "The release chain as one command: DONE, HALT or RED in one line"
)]
struct Cli {
    #[command(subcommand)]
    command: Sub,
}

#[derive(Subcommand)]
enum Sub {
    /// Run the whole chain from the chain file; stops at the first halt or red. Run again to go on.
    Run(Common),
    /// Run one command of the chain (gates, push, wait-ci, ...).
    Step {
        name: String,
        #[command(flatten)]
        common: Common,
    },
    /// List the commands with risk and reach.
    List,
    /// Check the chain file against the commands in the program.
    Verify {
        #[arg(long, default_value = "chains/release.chain.yaml")]
        chain: PathBuf,
    },
}

#[derive(Args)]
struct Common {
    /// Release version without `v` (0.0.9).
    #[arg(long)]
    version: String,
    /// Worktree of the branch to release.
    #[arg(long, default_value = ".")]
    repo_dir: PathBuf,
    /// Owner/name on GitHub; default: from the `origin` URL.
    #[arg(long)]
    slug: Option<String>,
    /// Branch to release; default: the one checked out.
    #[arg(long)]
    branch: Option<String>,
    #[arg(long, default_value = "main")]
    base: String,
    /// The fs-release checkout (scripts, signing key, chains).
    #[arg(long, default_value = ".")]
    release_dir: PathBuf,
    #[arg(long)]
    chain: Option<PathBuf>,
    /// Checkout of the catalog repo (for bump, sign, check).
    #[arg(long)]
    catalog: Option<PathBuf>,
    #[arg(long, default_value = "FreeSynergy/fs-catalog")]
    catalog_slug: String,
    /// The approval file; default `<repo-dir>/.opus-ok`.
    #[arg(long)]
    approval: Option<PathBuf>,
    #[arg(long)]
    state_dir: Option<PathBuf>,
    /// Highest reach this run may touch: local, shared or public.
    #[arg(long, default_value = "local")]
    reach: String,
    /// Program put in front of the gate run (the shared build-slot script); words are split at spaces.
    #[arg(long, env = "FS_BUILD_WRAPPER")]
    wrap: Option<String>,
    /// Seconds between GitHub run queries; values below 30 are raised to 30.
    #[arg(long, default_value_t = 30)]
    poll_secs: u64,
    /// Publish the release as a full release, not a pre-release.
    #[arg(long)]
    full_release: bool,
}

/// Parses the command line, runs it and returns the exit code.
#[must_use]
pub fn main_with(args: Vec<String>) -> i32 {
    let cli = match Cli::try_parse_from(args) {
        Ok(c) => c,
        Err(e) => {
            let _ = e.print();
            return if e.use_stderr() { 2 } else { 0 };
        }
    };
    match cli.command {
        Sub::List => list(),
        Sub::Verify { chain } => verify(&chain),
        Sub::Run(c) => go(&c, None),
        Sub::Step { name, common } => go(&common, Some(name)),
    }
}

fn list() -> i32 {
    for s in registry::all() {
        let m = s.meta();
        println!(
            "{} risk={} reach={} undo={}",
            m.name,
            m.risk.word(),
            m.reach.word(),
            m.undo
        );
    }
    0
}

fn verify(path: &Path) -> i32 {
    match Chain::load(path) {
        Ok(c) => {
            println!(
                "DONE chain {} {} matches the program ({} commands)",
                c.chain,
                c.version,
                c.steps.len()
            );
            0
        }
        Err(e) => {
            println!("RED verify {e}");
            1
        }
    }
}

fn slug_of(url: &str) -> Option<String> {
    let rest = url.trim().trim_end_matches('/').trim_end_matches(".git");
    let tail = rest
        .rsplit_once("github.com")
        .map(|(_, t)| t.trim_start_matches([':', '/']))?;
    (tail.matches('/').count() == 1).then(|| tail.to_string())
}

fn build_target(c: &Common, runner: &dyn Runner) -> Result<Target, String> {
    let git = |args: &[&str]| -> Result<String, String> {
        let cmd = Cmd {
            program: "git".into(),
            args: args.iter().map(|a| (*a).into()).collect(),
            cwd: Some(c.repo_dir.clone()),
            ..Cmd::default()
        };
        let o = runner.run(&cmd).map_err(|e| e.to_string())?;
        if o.ok() {
            Ok(o.stdout.trim().to_string())
        } else {
            Err(o.first_error())
        }
    };
    let branch = match &c.branch {
        Some(b) => b.clone(),
        None => git(&["symbolic-ref", "--short", "HEAD"])?,
    };
    let slug = match &c.slug {
        Some(s) => s.clone(),
        None => slug_of(&git(&["remote", "get-url", "origin"])?)
            .ok_or("cannot read owner/name from the origin URL: give --slug")?,
    };
    let state_dir = match &c.state_dir {
        Some(d) => d.clone(),
        None => PathBuf::from(git(&["rev-parse", "--absolute-git-dir"])?).join("fs-release-chain"),
    };
    Ok(Target {
        repo_dir: c.repo_dir.clone(),
        slug,
        branch,
        base: c.base.clone(),
        version: c.version.clone(),
        release_dir: c.release_dir.clone(),
        catalog_dir: c.catalog.clone(),
        catalog_slug: c.catalog_slug.clone(),
        approval: c
            .approval
            .clone()
            .unwrap_or_else(|| c.repo_dir.join(".opus-ok")),
        state_dir,
        prerelease: !c.full_release,
        wrapper: c
            .wrap
            .as_deref()
            .map(|w| w.split_whitespace().map(str::to_string).collect())
            .unwrap_or_default(),
    })
}

fn go(c: &Common, one: Option<String>) -> i32 {
    let runner = SystemRunner::default();
    let clock = SystemClock::new();
    let outcome = start(c, one, &runner, &clock);
    println!("{}", outcome.0.line());
    if let Some(log) = outcome.1 {
        println!("log: {}", log.display());
    }
    outcome.0.code()
}

fn names_of(c: &Common, one: Option<String>) -> Result<Vec<String>, String> {
    if let Some(n) = one {
        return Ok(vec![n]);
    }
    let file = c
        .chain
        .clone()
        .unwrap_or_else(|| c.release_dir.join("chains/release.chain.yaml"));
    Chain::load(&file).map(|ch| ch.names())
}

fn start(
    c: &Common,
    one: Option<String>,
    runner: &SystemRunner,
    clock: &SystemClock,
) -> (Outcome, Option<PathBuf>) {
    let red = |line: String| {
        (
            Outcome::Red {
                step: "setup".into(),
                line,
            },
            None,
        )
    };
    let Some(reach) = Reach::parse(&c.reach) else {
        return red(format!("--reach {}: use local, shared or public", c.reach));
    };
    let target = match build_target(c, runner) {
        Ok(t) => t,
        Err(e) => return red(e),
    };
    let names = match names_of(c, one) {
        Ok(list) => list,
        Err(e) => return red(e),
    };
    let mut timing = Timing::standard();
    timing.poll = Duration::from_secs(c.poll_secs).max(MIN_POLL);
    let ledger = Ledger::new(&target.state_dir, &target.version);
    let path = ledger.path().to_path_buf();
    let ctx = Ctx::new(target, reach, timing, runner, clock);
    (runner::execute(&ctx, &ledger, &names), Some(path))
}
