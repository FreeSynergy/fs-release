//! Pattern: Facade (command line in, one run out: `lift` prints the report path and the verdict, the flow commands print one line).

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use fs_release_chain::sys::SystemRunner;

use crate::lift::{self, LiftOpts};

#[derive(Parser)]
#[command(
    name = "fs-release",
    version,
    about = "Lift a program onto new fs-components / fs-interfaces tags"
)]
struct Cli {
    #[command(subcommand)]
    command: Sub,
}

#[derive(Subcommand)]
enum Sub {
    /// Lift dependencies, regenerate the lock, run lock gate, fmt, clippy pedantic, rules, tests; write the report.
    Lift(LiftArgs),
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
    /// Commit the changed Cargo.toml files after green steps.
    #[arg(long)]
    commit: bool,
    /// The fs-release checkout (scripts, RUST_VERSION).
    #[arg(long, default_value = ".")]
    release_dir: PathBuf,
    /// Throw-away git config for local tags.
    #[arg(long, default_value = lift::LOCAL_TAGS)]
    git_config: PathBuf,
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
    code
}
