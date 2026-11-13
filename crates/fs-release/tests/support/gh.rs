//! Test support: a runner that answers `gh` itself (nothing leaves the machine) and hands every other program to the real one.

use std::cell::RefCell;
use std::io;

use fs_release_chain::sys::{Cmd, Out, Runner, SystemRunner};

/// `release view` answers the given state, `workflow run` succeeds, any other `gh` call is an error. Every `gh` call is kept.
pub struct MockGh {
    pub inner: SystemRunner,
    /// `None`: the tag has no release; `Some(draft)`.
    pub release: Option<bool>,
    /// True: the release answers `isPrerelease: false` (a full release, not the Pre-release the bump follows).
    pub full: bool,
    pub calls: RefCell<Vec<String>>,
}

impl MockGh {
    pub fn new(inner: SystemRunner, release: Option<bool>) -> MockGh {
        MockGh {
            inner,
            release,
            full: false,
            calls: RefCell::new(Vec::new()),
        }
    }

    /// The `gh` calls that started a workflow.
    pub fn dispatched(&self) -> Vec<String> {
        self.calls
            .borrow()
            .iter()
            .filter(|c| c.starts_with("workflow run"))
            .cloned()
            .collect()
    }
}

impl Runner for MockGh {
    fn run(&self, cmd: &Cmd) -> io::Result<Out> {
        if cmd.program != "gh" {
            return self.inner.run(cmd);
        }
        let line = cmd.args.join(" ");
        self.calls.borrow_mut().push(line.clone());
        let ok = |stdout: &str| {
            Ok(Out {
                code: 0,
                stdout: stdout.into(),
                stderr: String::new(),
            })
        };
        if line.starts_with("release view") {
            return match self.release {
                Some(draft) => ok(&format!(
                    "{{\"isDraft\":{draft},\"isPrerelease\":{}}}",
                    !self.full
                )),
                None => Ok(Out {
                    code: 1,
                    stdout: String::new(),
                    stderr: "release not found".into(),
                }),
            };
        }
        if line.starts_with("workflow run") {
            return ok("");
        }
        Ok(Out {
            code: 1,
            stdout: String::new(),
            stderr: format!("mock gh: unexpected call {line}"),
        })
    }
}
