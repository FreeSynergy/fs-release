//! Test support for the flow: release checkout with the pre-approved list, bare remote of the program, approvals, targets.

use std::fs;
use std::path::Path;

use fs_release_chain::approval::parse_date;
use fs_release_chain::ctx::Target;
use fs_release_chain::sys::SystemRunner;

use super::{git, World};

pub const SLUG: &str = "Org/demo";

impl World {
    pub fn remote(&self) -> std::path::PathBuf {
        self.root().join("demo-origin.git")
    }

    /// `origin` of the program and of the release checkout: GitHub URLs that git rewrites to local bare repos.
    pub fn flow_setup(&self, public_ok: &str) {
        let r = self.root();
        git(
            &r,
            &[
                "init",
                "--quiet",
                "--bare",
                "-b",
                "main",
                &self.remote().display().to_string(),
            ],
        );
        let w = self.work();
        let url = "https://github.com/Org/demo.git";
        git(&w, &["config", "remote.origin.url", url]);
        git(
            &w,
            &[
                "config",
                &format!("url.{}.insteadOf", self.remote().display()),
                url,
            ],
        );
        git(&w, &["push", "--quiet", "origin", "main"]);
        // The release checkout: a repo whose origin is exactly FreeSynergy/fs-release, with the list on origin/main.
        let bare = r.join("rel-origin.git");
        git(
            &r,
            &[
                "init",
                "--quiet",
                "--bare",
                "-b",
                "main",
                &bare.display().to_string(),
            ],
        );
        let rel = self.rel();
        git(&rel, &["init", "--quiet", "-b", "main"]);
        fs::create_dir_all(rel.join(".fs")).unwrap();
        fs::write(
            rel.join(".fs/preapproved.toml"),
            format!("repos = [\"{SLUG}\"]\ncatalog_targets = [\"FreeSynergy/fs-catalog\"]\npublic_ok = \"{public_ok}\"\n"),
        )
        .unwrap();
        git(&rel, &["add", ".fs"]);
        git(&rel, &["commit", "--quiet", "-m", "list"]);
        let rurl = "https://github.com/FreeSynergy/fs-release.git";
        git(&rel, &["remote", "add", "origin", rurl]);
        git(
            &rel,
            &["config", &format!("url.{}.insteadOf", bare.display()), rurl],
        );
        git(&rel, &["push", "--quiet", "origin", "main"]);
    }

    pub fn target(&self) -> Target {
        let today = parse_date("2026-11-12").unwrap();
        Target {
            repo_dir: self.work(),
            slug: SLUG.into(),
            branch: "feat".into(),
            base: "main".into(),
            version: "0.0.9".into(),
            release_dir: self.rel(),
            catalog_dir: None,
            catalog_slug: "FreeSynergy/fs-catalog".into(),
            approval_dir: self.root().join("freigaben"),
            approval_days: 7,
            today_days: today,
            now_secs: today * 86_400 + 3_600,
            audit_log: self.root().join("audit/AUDIT.log"),
            fake: false,
            state_dir: self.root().join("state"),
            prerelease: true,
            wrapper: vec![],
        }
    }

    #[allow(clippy::unused_self)] // kept as a method: every test asks the world for its runner
    pub fn runner(&self) -> SystemRunner {
        let mut env: Vec<(String, String)> = super::ident()
            .into_iter()
            .map(|(k, v)| (k.into(), v.into()))
            .collect();
        env.push(("GIT_CONFIG_GLOBAL".into(), "/dev/null".into()));
        env.push(("GIT_CONFIG_NOSYSTEM".into(), "1".into()));
        SystemRunner { env }
    }

    /// Writes `<freigaben>/demo/<sha>.ok` as Opus would: findings file, its hash, verdict.
    pub fn approve(&self, sha: &str, verdict: &str) {
        let dir = self.root().join("freigaben/demo");
        fs::create_dir_all(&dir).unwrap();
        let findings = self.root().join("findings.md");
        fs::write(&findings, "no findings\n").unwrap();
        let hash = fs_release_chain_sha(&findings);
        fs::write(
            dir.join(format!("{sha}.ok")),
            format!("repo: {SLUG}\nsha: {sha}\ndate: \"2026-11-12\"\nverdict: {verdict}\nby: opus\nfindings: {}\nfindings_sha256: {hash}\n", findings.display()),
        )
        .unwrap();
    }

    pub fn audit(&self) -> String {
        fs::read_to_string(self.root().join("audit/AUDIT.log")).unwrap_or_default()
    }

    pub fn remote_has(&self, name: &str) -> bool {
        let o = std::process::Command::new("git")
            .args([
                "--git-dir",
                &self.remote().display().to_string(),
                "rev-parse",
                "--verify",
                "--quiet",
                name,
            ])
            .output()
            .unwrap();
        o.status.success()
    }
}

fn fs_release_chain_sha(path: &Path) -> String {
    let o = std::process::Command::new("sha256sum")
        .arg(path)
        .output()
        .unwrap();
    String::from_utf8_lossy(&o.stdout)
        .split_whitespace()
        .next()
        .unwrap()
        .to_string()
}
