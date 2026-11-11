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
    /// Publishes `.fs/preapproved.toml` of fs-release: committed in the checkout `rel` AND pushed to its origin (a bare
    /// repo that stands in for GitHub; the configured origin URL is the real GitHub one, mapped with `insteadOf`).
    pub fn set_preapproved(&self, repos: &[&str], public_ok: &str) {
        self.write_preapproved(repos, &["Org/fs-catalog"], public_ok, true);
    }

    /// Writes and commits the list; pushes it to the origin only when `push` is set (a local-only commit must count for nothing).
    pub fn write_preapproved(
        &self,
        repos: &[&str],
        catalogs: &[&str],
        public_ok: &str,
        push: bool,
    ) {
        let rel = self.root().join("rel");
        if !rel.join(".git").exists() {
            let bare = self.root().join("rel-origin.git");
            super::git(
                &self.root(),
                &[
                    "init",
                    "--quiet",
                    "--bare",
                    "-b",
                    "main",
                    &bare.display().to_string(),
                ],
            );
            super::git(&rel, &["init", "--quiet", "-b", "main"]);
            let url = "https://github.com/FreeSynergy/fs-release.git";
            super::git(&rel, &["remote", "add", "origin", url]);
            super::git(
                &rel,
                &["config", &format!("url.{}.insteadOf", bare.display()), url],
            );
        }
        fs::create_dir_all(rel.join(".fs")).unwrap();
        let q = |l: &[&str]| {
            l.iter()
                .map(|r| format!("\"{r}\""))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let text = format!(
            "repos = [{}]\ncatalog_targets = [{}]\npublic_ok = \"{public_ok}\"\n",
            q(repos),
            q(catalogs)
        );
        fs::write(rel.join(".fs/preapproved.toml"), text).unwrap();
        super::git(&rel, &["add", "."]);
        super::git(
            &rel,
            &["commit", "--quiet", "--allow-empty", "-m", "preapproved"],
        );
        if push {
            super::git(&rel, &["push", "--quiet", "origin", "main"]);
        }
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
