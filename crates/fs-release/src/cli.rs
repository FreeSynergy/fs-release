//! Pattern: Facade (command line in, one run out: `lift` prints the report path and the verdict, the flow commands print one line).

use std::path::{Path, PathBuf};

use clap::{Args, Parser, Subcommand};
use fs_release_chain::ctx::{Ctx, Timing};
use fs_release_chain::meta::Reach;
use fs_release_chain::outcome::Outcome;
use fs_release_chain::sys::{SystemClock, SystemRunner};

use crate::flow;
use crate::lift::{self, LiftOpts};
use crate::task::run_all;

#[derive(Parser)]
#[command(
    name = "fs-release",
    version,
    about = "Lift a program onto new tags and walk it through review, tag and push"
)]
struct Cli {
    #[command(subcommand)]
    command: Sub,
}

#[derive(Subcommand)]
enum Sub {
    /// Lift dependencies, regenerate the lock, run lock gate, fmt, clippy pedantic, rules, tests; write the report.
    Lift(LiftArgs),
    /// Write the review request for Opus (needs a green lift report for the head).
    ReviewRequest(StepArgs),
    /// Set the local tag (needs the approval of the head).
    Tag(StepArgs),
    /// Push branch (--reach shared) and tag (--reach public); needs the approval and the pre-approved list.
    Push(StepArgs),
    /// After the Pre-release: start bump.yml of the catalog (--reach public); needs the approval and the pre-approved list.
    Bump(StepArgs),
    /// lift --commit, review-request, tag, push, bump in one run; halts until the approval and the Pre-release exist, run again to go on.
    Flow {
        #[command(flatten)]
        lift: LiftArgs,
        #[command(flatten)]
        flow: FlowArgs,
    },
}

#[derive(Args, Clone)]
struct LiftArgs {
    /// Worktree of the repo.
    repo: PathBuf,
    /// Tag of fs-components, `vX.Y.Z`.
    #[arg(long)]
    components: Option<String>,
    /// Tag of fs-interfaces, `vX.Y.Z`.
    #[arg(long)]
    interfaces: Option<String>,
    /// Use the real tags on GitHub instead of the local-tags git config.
    #[arg(long)]
    online: bool,
    /// Commit the changed Cargo.toml files after green steps (the flow does).
    #[arg(long)]
    commit: bool,
    /// The fs-release checkout (scripts, `RUST_VERSION`).
    #[arg(long, default_value = ".")]
    release_dir: PathBuf,
    /// Throw-away git config for local tags.
    #[arg(long, default_value = lift::LOCAL_TAGS)]
    git_config: PathBuf,
}

#[derive(Args, Clone)]
struct StepArgs {
    /// Worktree of the repo.
    repo: PathBuf,
    /// The fs-release checkout (pre-approved list).
    #[arg(long, default_value = ".")]
    release_dir: PathBuf,
    #[command(flatten)]
    flow: FlowArgs,
}

#[derive(Args, Clone)]
struct FlowArgs {
    /// Release version without `v` (0.0.9).
    #[arg(long)]
    version: String,
    #[arg(long, default_value = "main")]
    base: String,
    /// Highest reach: local, shared or public.
    #[arg(long, default_value = "local")]
    reach: String,
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
        Sub::Lift(a) => lift_cmd(&a),
        Sub::ReviewRequest(s) => one(&s, 0),
        Sub::Tag(s) => one(&s, 1),
        Sub::Push(s) => one(&s, 2),
        Sub::Bump(s) => one(&s, 3),
        Sub::Flow { lift, flow } => full(&lift, &flow),
    }
}

fn opts(a: &LiftArgs) -> LiftOpts {
    let mut o = LiftOpts::new(a.repo.clone(), a.release_dir.clone());
    o.components.clone_from(&a.components);
    o.interfaces.clone_from(&a.interfaces);
    o.online = a.online;
    o.commit = a.commit;
    o.git_config.clone_from(&a.git_config);
    o
}

fn lift_cmd(a: &LiftArgs) -> i32 {
    let o = opts(a);
    let (rep, code) = lift::run(&o, &SystemRunner::default());
    let red: Vec<String> = rep
        .steps
        .iter()
        .filter(|s| s.status == "red")
        .map(|s| format!("{}: {}", s.name, s.first_error.as_deref().unwrap_or("")))
        .collect();
    println!(
        "{} lift {} {:.0}s report {}/lift-report.md",
        if rep.ok { "GREEN" } else { "RED" },
        rep.repo,
        rep.secs,
        o.report_dir().display()
    );
    for r in red {
        println!("  {r}");
    }
    for s in rep.steps.iter().filter(|s| s.status == "skipped" && s.secs > 0.0) {
        println!("  WARNING {} skipped: {}", s.name, s.detail);
    }
    code
}

fn with_ctx(
    f: &FlowArgs,
    dir: &Path,
    release_dir: &Path,
    body: impl FnOnce(&Ctx) -> Outcome,
) -> i32 {
    let runner = SystemRunner::default();
    let clock = SystemClock::new();
    let line = |o: Outcome| {
        println!("{}", o.line());
        o.code()
    };
    let red = |why: String| {
        line(Outcome::Red {
            step: "setup".into(),
            line: why,
        })
    };
    let Some(reach) = Reach::parse(&f.reach) else {
        return red(format!("--reach {}: use local, shared or public", f.reach));
    };
    let target = match flow::build_target(&runner, dir, release_dir, &f.version, &f.base, None) {
        Ok(t) => t,
        Err(e) => return red(e),
    };
    let ctx = Ctx::new(target, reach, Timing::standard(), &runner, &clock);
    line(body(&ctx))
}

/// `which`: 0 review-request, 1 tag, 2 push, 3 bump (the one step, with all that must hold before it).
fn one(s: &StepArgs, which: usize) -> i32 {
    with_ctx(&s.flow, &s.repo, &s.release_dir, |ctx| {
        let mut all = flow::tasks();
        let list = vec![all.remove(which)];
        let (records, first) = run_all(&list, ctx);
        match first {
            None => Outcome::Done {
                summary: records[0].detail.clone(),
            },
            Some((name, stop)) => Outcome::from_stop(&name, stop),
        }
    })
}

fn full(l: &LiftArgs, f: &FlowArgs) -> i32 {
    let mut a = l.clone();
    a.commit = true;
    let o = opts(&a);
    with_ctx(f, &l.repo, &l.release_dir, |ctx| {
        flow::run(&o, ctx, &SystemRunner::default())
    })
}
