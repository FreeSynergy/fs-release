//! Helpers of the chain tests: what GitHub would answer for a green release, and a short name for where a run stopped.

use fs_release_chain::sys::{Cmd, Runner};
use fs_release_chain::Outcome;

use std::fs;

use super::{run_json, World, CAT, SLUG};

/// Everything GitHub would answer for a fully green release of the current head.
pub fn all_green(w: &World, sha: &str) {
    w.runs(SLUG, &run_json(sha, &release_rows()));
    w.runs(
        CAT,
        &run_json("@MAIN@", &[(3, "sign", "main", "completed", "success")]),
    );
    w.release_draft();
}

/// `halt <step> <reason>`, `red <step>` or `done`.
pub fn stopped_at(o: &Outcome) -> String {
    match o {
        Outcome::Halt { step, reason, .. } => format!("halt {step} {reason}"),
        Outcome::Red { step, .. } => format!("red {step}"),
        Outcome::Done { .. } => "done".into(),
    }
}

/// The rows of a CI that is green or red in the required runs `check` and `lint` (workflow `lint` is `lint_result`).
pub fn ci_rows(
    conclusion: &str,
    lint: &str,
) -> Vec<(u64, &'static str, &'static str, &'static str, &'static str)> {
    let c: &'static str = Box::leak(conclusion.to_string().into_boxed_str());
    let l: &'static str = Box::leak(lint.to_string().into_boxed_str());
    vec![
        (1, "check", "feat", "completed", c),
        (4, "lint", "feat", "completed", l),
    ]
}

impl World {
    /// Commits `.fs/preapproved.toml` in the fs-release checkout (`rel`): the list of repos and the public lock.
    pub fn set_preapproved(&self, repos: &[&str], public_ok: &str) {
        let rel = self.root().join("rel");
        if !rel.join(".git").exists() {
            super::git(&rel, &["init", "--quiet", "-b", "main"]);
        }
        fs::create_dir_all(rel.join(".fs")).unwrap();
        let list: Vec<String> = repos.iter().map(|r| format!("\"{r}\"")).collect();
        fs::write(
            rel.join(".fs/preapproved.toml"),
            format!(
                "repos = [{}]\npublic_ok = \"{public_ok}\"\n",
                list.join(", ")
            ),
        )
        .unwrap();
        super::git(&rel, &["add", "."]);
        super::git(
            &rel,
            &["commit", "--quiet", "--allow-empty", "-m", "preapproved"],
        );
    }

    /// A sequence of answers: each poll takes the next one, the last stays.
    pub fn runs_seq(&self, repo: &str, answers: &[&str]) {
        let dir = self
            .fake()
            .join(format!("runs-{}.json.seq", repo.replace('/', "_")));
        fs::create_dir_all(&dir).unwrap();
        for (i, a) in answers.iter().enumerate() {
            fs::write(dir.join(format!("{i:03}")), a).unwrap();
        }
    }

    pub fn calls(&self) -> String {
        fs::read_to_string(self.fake().join("calls.log")).unwrap_or_default()
    }

    pub fn count(&self, needle: &str) -> usize {
        self.calls().lines().filter(|l| l.contains(needle)).count()
    }

    /// Writes a complete approval for `sha` in the fixed folder, dated `date`, with a real findings file and its hash.
    pub fn approve_on(&self, sha: &str, verdict: &str, date: &str) {
        let findings = self.root().join("findings.md");
        fs::write(&findings, "no findings\n").unwrap();
        let hash = self
            .runner()
            .run(&Cmd {
                program: "sha256sum".into(),
                args: vec![findings.display().to_string()],
                ..Cmd::default()
            })
            .unwrap();
        let hash = hash.stdout.split_whitespace().next().unwrap().to_string();
        let dir = self.root().join("freigaben/fs-demo");
        fs::create_dir_all(&dir).unwrap();
        let text = format!("repo: {SLUG}\nsha: {sha}\ndate: {date}\nverdict: {verdict}\nby: opus\nfindings: {}\nfindings_sha256: {hash}\n", findings.display());
        fs::write(dir.join(format!("{sha}.ok")), text).unwrap();
    }

    pub fn approve(&self, sha: &str, verdict: &str) {
        self.approve_on(sha, verdict, "2026-11-12");
    }
}

fn release_rows() -> Vec<(u64, &'static str, &'static str, &'static str, &'static str)> {
    let mut rows = ci_rows("success", "success");
    rows.push((2, "release", "v0.0.9", "completed", "success"));
    rows
}
