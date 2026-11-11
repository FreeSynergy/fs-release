//! Pattern: Strategy, part of the chain commands (the three commands that stay on this machine: gates, review, report).

use std::fs;

use crate::approval;
use crate::git::short;
use crate::ledger::Ledger;
use crate::meta::{Meta, Reach, Risk};
use crate::outcome::{Note, StepResult, Stop};
use crate::steps::{Frame, Step};
use crate::sys::Cmd;

/// Blinc lock check, then the one measure (`scripts/gates.sh`: fmt, clippy pedantic, tests, rules).
pub struct Gates;

impl Step for Gates {
    fn meta(&self) -> Meta {
        Meta {
            name: "gates",
            risk: Risk::Start,
            reach: Reach::Local,
            preview: "The local checks run in {{wt}}.",
            undo: "none",
        }
    }

    fn run(&self, f: &Frame) -> StepResult {
        if f.ledger.entries().iter().any(|e| {
            e.step == "gates" && e.sha == f.head && e.result == "ok" && !e.data.contains_key("fake")
        }) {
            return Ok(Note::text("gates were green for this commit already"));
        }
        let t = &f.ctx.target;
        let scripts = t.release_dir.join("scripts");
        let lock = Cmd {
            program: "python3".into(),
            args: vec![
                "-I".into(),
                scripts.join("check_blinc_lock.py").display().to_string(),
                t.repo_dir.display().to_string(),
            ],
            ..Cmd::default()
        };
        let o = f.ctx.runner.run(&lock)?;
        if !o.ok() {
            return Err(Stop::red(format!("blinc-lock: {}", o.first_error())));
        }
        let mut argv = t.wrapper.clone();
        argv.extend([
            "bash".into(),
            scripts.join("gates.sh").display().to_string(),
            t.repo_dir.display().to_string(),
        ]);
        let program = argv.remove(0);
        let o = f.ctx.runner.run(&Cmd {
            program,
            args: argv,
            ..Cmd::default()
        })?;
        if o.ok() {
            let note = Note::text(format!(
                "gates ok at {}{}",
                short(f.head),
                if t.fake {
                    " (FAKE wrapper, counts for nothing)"
                } else {
                    ""
                }
            ));
            return Ok(if t.fake { note.with("fake", "1") } else { note });
        }
        let log = t.state_dir.join(format!("gates-{}.log", short(f.head)));
        fs::create_dir_all(&t.state_dir)?;
        fs::write(&log, format!("{}\n{}", o.stdout, o.stderr))?;
        Err(Stop::red(format!(
            "gates: {} (log {})",
            o.first_error(),
            log.display()
        )))
    }
}

/// The approval point: the chain goes on only with a JA for exactly this commit.
pub struct Review;

impl Step for Review {
    fn meta(&self) -> Meta {
        Meta {
            name: "review",
            risk: Risk::View,
            reach: Reach::Local,
            preview: "The approval of {{sha}} is checked.",
            undo: "none",
        }
    }

    fn run(&self, f: &Frame) -> StepResult {
        let a = approval::verify(f.ctx, f.head)?;
        Ok(Note::text(format!(
            "approved {} by {} on {}",
            short(f.head),
            a.by,
            a.date
        )))
    }
}

/// Writes the short report (at most ten lines) of the run so far.
pub struct Report;

/// The last `ok` line of every command for this commit, one line each, at most ten lines.
fn report_lines(ledger: &Ledger, head: &str, slug: &str, version: &str) -> Vec<String> {
    let mut lines = vec![format!("{slug} v{version} at {}", short(head))];
    for e in ledger
        .entries()
        .iter()
        .filter(|e| e.sha == head && e.result == "ok" && e.step != "report")
    {
        let line = format!("{}: {}", e.step, e.detail);
        if let Some(old) = lines
            .iter()
            .position(|l| l.starts_with(&format!("{}:", e.step)))
        {
            lines[old] = line;
        } else {
            lines.push(line);
        }
    }
    lines.truncate(10);
    lines
}

impl Step for Report {
    fn meta(&self) -> Meta {
        Meta {
            name: "report",
            risk: Risk::View,
            reach: Reach::Local,
            preview: "The report is written.",
            undo: "none",
        }
    }

    fn run(&self, f: &Frame) -> StepResult {
        let t = &f.ctx.target;
        let lines = report_lines(f.ledger, f.head, &t.slug, &t.version);
        fs::create_dir_all(&t.state_dir)?;
        let path = t.state_dir.join(format!("report-{}.txt", t.version));
        fs::write(&path, lines.join("\n") + "\n")?;
        Ok(Note::text(format!("report {}", path.display())))
    }
}
