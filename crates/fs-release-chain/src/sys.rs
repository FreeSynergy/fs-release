//! Pattern: Strategy (how programs are run and how time passes; the real ones here, a virtual clock in the tests).

use std::cell::Cell;
use std::io;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// One program call. Arguments are never joined into a shell line.
#[derive(Debug, Clone, Default)]
pub struct Cmd {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: Option<PathBuf>,
    pub env: Vec<(String, String)>,
}

/// What a program call returned.
#[derive(Debug, Clone, Default)]
pub struct Out {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl Out {
    /// True when the program exited with 0.
    #[must_use]
    pub fn ok(&self) -> bool {
        self.code == 0
    }

    /// First line that carries an error, else the first non-empty line of stderr, else of stdout.
    #[must_use]
    pub fn first_error(&self) -> String {
        let all = format!("{}\n{}", self.stderr, self.stdout);
        let lines = || all.lines().map(str::trim).filter(|l| !l.is_empty());
        let pick = lines().find(|l| {
            let low = l.to_lowercase();
            low.contains("error")
                || low.contains("failed")
                || low.contains("fatal")
                || low.contains("rejected")
        });
        pick.or_else(|| lines().next())
            .unwrap_or("no output")
            .to_string()
    }
}

/// Runs programs. The chain never starts a process any other way, so a test can steer all of it.
pub trait Runner {
    /// # Errors
    /// When the program cannot be started at all (not when it exits non-zero).
    fn run(&self, cmd: &Cmd) -> io::Result<Out>;
}

/// The real thing: `std::process`, with extra environment (the tests put a fake `gh` first in `PATH`).
#[derive(Debug, Clone, Default)]
pub struct SystemRunner {
    pub env: Vec<(String, String)>,
}

impl Runner for SystemRunner {
    fn run(&self, cmd: &Cmd) -> io::Result<Out> {
        let mut c = Command::new(&cmd.program);
        c.args(&cmd.args)
            .stdin(Stdio::null())
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GH_PROMPT_DISABLED", "1");
        if let Some(dir) = &cmd.cwd {
            c.current_dir(dir);
        }
        for (k, v) in self.env.iter().chain(cmd.env.iter()) {
            c.env(k, v);
        }
        let out = c.output()?;
        Ok(Out {
            code: out.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        })
    }
}

/// Time as the chain sees it: a reading and a wait.
pub trait Clock {
    /// Time since an arbitrary start; only differences mean something.
    fn now(&self) -> Duration;
    fn sleep(&self, d: Duration);
}

/// Real time.
#[derive(Debug)]
pub struct SystemClock {
    start: Instant,
}

impl SystemClock {
    #[must_use]
    pub fn new() -> SystemClock {
        SystemClock {
            start: Instant::now(),
        }
    }
}

impl Default for SystemClock {
    fn default() -> SystemClock {
        SystemClock::new()
    }
}

impl Clock for SystemClock {
    fn now(&self) -> Duration {
        self.start.elapsed()
    }

    fn sleep(&self, d: Duration) {
        std::thread::sleep(d);
    }
}

/// Virtual time for tests: `sleep` only moves the reading forward and remembers how long it was asked to wait.
#[derive(Debug, Default)]
pub struct FakeClock {
    now: Cell<Duration>,
    slept: Cell<Duration>,
}

impl FakeClock {
    /// Total time the chain asked to wait.
    #[must_use]
    pub fn slept(&self) -> Duration {
        self.slept.get()
    }
}

impl Clock for FakeClock {
    fn now(&self) -> Duration {
        self.now.get()
    }

    fn sleep(&self, d: Duration) {
        self.now.set(self.now.get() + d);
        self.slept.set(self.slept.get() + d);
    }
}
