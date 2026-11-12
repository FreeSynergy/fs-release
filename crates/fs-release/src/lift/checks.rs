//! Pattern: Strategy (the four independent checks after the lock: fmt, clippy pedantic, coding rules, tests).

use fs_release_chain::outcome::Stop;
use fs_release_chain::sys::Out;

use super::LiftEnv;
use crate::task::Task;

/// The line a person (or an agent) should see first: a compiler error, a failed test, a panic, a format diff.
pub(super) fn red_line(o: &Out) -> String {
    let all = format!("{}\n{}", o.stderr, o.stdout);
    let lines = || all.lines().map(str::trim);
    // A failed test or a format diff says more than the closing `error: test failed`, so those come first.
    let hit = lines()
        .find(|l| l.contains("FAILED") || l.contains("panicked") || l.starts_with("Diff in"))
        .or_else(|| lines().find(|l| l.starts_with("error")));
    let line = hit.map_or_else(|| o.first_error(), str::to_string);
    line.chars().take(200).collect()
}

fn verdict(name: &str, o: &Out) -> Result<String, Stop> {
    if o.ok() {
        Ok(format!("{name} green"))
    } else {
        Err(Stop::red(red_line(o)))
    }
}

/// `cargo fmt --check`.
pub struct Fmt;

impl Task<LiftEnv<'_>> for Fmt {
    fn name(&self) -> &'static str {
        "fmt"
    }

    fn continue_on_red(&self) -> bool {
        true
    }

    fn run(&self, env: &LiftEnv) -> Result<String, Stop> {
        verdict("fmt", &env.cargo(&["fmt", "--check"])?)
    }
}

/// The same clippy call as `scripts/gates.sh`.
pub struct Clippy;

impl Task<LiftEnv<'_>> for Clippy {
    fn name(&self) -> &'static str {
        "clippy"
    }

    fn continue_on_red(&self) -> bool {
        true
    }

    fn run(&self, env: &LiftEnv) -> Result<String, Stop> {
        let args = [
            "clippy",
            "--workspace",
            "--all-targets",
            "--all-features",
            "--",
            "-D",
            "warnings",
            "-W",
            "clippy::pedantic",
        ];
        verdict("clippy pedantic", &env.cargo(&args)?)
    }
}

/// `scripts/check_rust_rules.py` (file length, module docs, pattern line, Blinc rules).
pub struct Rules;

impl Task<LiftEnv<'_>> for Rules {
    fn name(&self) -> &'static str {
        "rules"
    }

    fn continue_on_red(&self) -> bool {
        true
    }

    fn run(&self, env: &LiftEnv) -> Result<String, Stop> {
        let args = vec![
            "-I".to_string(),
            env.script("check_rust_rules.py"),
            env.opts.repo_dir.display().to_string(),
        ];
        verdict("rules", &env.exec("python3", args, vec![])?)
    }
}

/// `cargo test --workspace --all-features`.
pub struct Tests;

impl Task<LiftEnv<'_>> for Tests {
    fn name(&self) -> &'static str {
        "tests"
    }

    fn continue_on_red(&self) -> bool {
        true
    }

    fn run(&self, env: &LiftEnv) -> Result<String, Stop> {
        verdict(
            "tests",
            &env.cargo(&["test", "--workspace", "--all-features"])?,
        )
    }
}
