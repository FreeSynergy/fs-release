//! Test support: a throw-away world (local bare remotes instead of GitHub, a fake `gh` and `gpg` first in PATH, a virtual clock).

#![allow(dead_code)]

pub mod flow;

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use fs_release_chain::chain::Chain;
use fs_release_chain::ctx::{Ctx, Target, Timing};
use fs_release_chain::ledger::Ledger;
use fs_release_chain::meta::Reach;
use fs_release_chain::sys::{FakeClock, SystemRunner};
use fs_release_chain::{runner, Outcome};
use tempfile::TempDir;

pub const SLUG: &str = "Org/fs-demo";
pub const CAT: &str = "Org/fs-catalog";
const GH: &str = include_str!("fake-gh.sh");

pub struct World {
    pub tmp: TempDir,
    pub clock: FakeClock,
}

pub fn git(dir: &Path, args: &[&str]) -> String {
    let o = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .envs(ident())
        .output()
        .expect("git");
    assert!(
        o.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&o.stderr)
    );
    String::from_utf8_lossy(&o.stdout).trim().to_string()
}

fn ident() -> Vec<(&'static str, &'static str)> {
    vec![
        ("GIT_AUTHOR_NAME", "t"),
        ("GIT_AUTHOR_EMAIL", "t@e"),
        ("GIT_COMMITTER_NAME", "t"),
        ("GIT_COMMITTER_EMAIL", "t@e"),
    ]
}

fn script(path: &Path, body: &str) {
    fs::write(path, body).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

impl World {
    pub fn new() -> World {
        let w = World {
            tmp: TempDir::new().unwrap(),
            clock: FakeClock::default(),
        };
        let (r, d) = (w.root(), w.fake());
        fs::create_dir_all(&d).unwrap();
        fs::create_dir_all(r.join("bin")).unwrap();
        script(&r.join("bin/gh"), GH);
        script(&r.join("bin/gpg"), include_str!("fake-gpg.sh"));
        script(&r.join("bin/fs-store"), "#!/usr/bin/env bash\necho \"fs-store $*\" >> \"$FAKE_DIR/calls.log\"\n[ -f \"$FAKE_DIR/store-red\" ] && { echo 'error: catalog not readable' >&2; exit 1; }\nexit 0\n");
        w.program_repo();
        w.catalog_repo();
        w.release_dir();
        w.set_preapproved(&["Org/fs-demo"], "test-ok");
        w.set_gates(true);
        w
    }

    pub fn root(&self) -> PathBuf {
        self.tmp.path().to_path_buf()
    }

    pub fn fake(&self) -> PathBuf {
        self.root().join("fake")
    }

    pub fn work(&self) -> PathBuf {
        self.root().join("work")
    }

    pub fn origin(&self) -> PathBuf {
        self.root().join("origin.git")
    }

    pub fn cat_origin(&self) -> PathBuf {
        self.root().join("cat-origin.git")
    }

    fn init_pair(&self, bare: &Path, clone: &Path, files: &[(&str, &str)]) {
        let r = self.root();
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
        git(
            &r,
            &[
                "clone",
                "--quiet",
                &bare.display().to_string(),
                &clone.display().to_string(),
            ],
        );
        git(clone, &["checkout", "--quiet", "-B", "main"]);
        for (n, c) in files {
            fs::write(clone.join(n), c).unwrap();
        }
        git(clone, &["add", "."]);
        git(clone, &["commit", "--quiet", "-m", "base"]);
        git(clone, &["push", "--quiet", "origin", "main"]);
    }

    fn program_repo(&self) {
        self.init_pair(&self.origin(), &self.work(), &[("README", "demo\n")]);
        git(&self.work(), &["checkout", "--quiet", "-b", "feat"]);
        fs::write(self.work().join("feature"), "x\n").unwrap();
        fs::create_dir_all(self.work().join(".fs")).unwrap();
        fs::write(
            self.work().join(".fs/release.toml"),
            "required = [\"check\", \"lint\"]\n",
        )
        .unwrap();
        git(&self.work(), &["add", "."]);
        git(&self.work(), &["commit", "--quiet", "-m", "feature"]);
    }

    fn catalog_repo(&self) {
        let pk = format!(
            "# catalog\n[[package]]\nid = \"fs-demo\"\nrepo = \"{SLUG}\"\ntag = \"v0.0.1\"\n"
        );
        let xml = format!("<releases><release version=\"0.0.9\"><location>https://x/{SLUG}/releases/download/v0.0.9/a</location></release></releases>\n");
        let xml = xml.replace("Org/fs-demo", "fs-demo");
        self.init_pair(
            &self.cat_origin(),
            &self.root().join("catalog"),
            &[
                ("packages.toml", &pk),
                ("catalog.xml", &xml),
                ("catalog.xml.asc", "SIG-OK"),
            ],
        );
    }

    fn release_dir(&self) {
        let dir = self.root().join("rel");
        fs::create_dir_all(dir.join("scripts")).unwrap();
        let real = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts");
        fs::copy(
            real.join("check_blinc_lock.py"),
            dir.join("scripts/check_blinc_lock.py"),
        )
        .unwrap();
        fs::write(
            dir.join("scripts/blinc-approved.txt"),
            "# nothing approved\n",
        )
        .unwrap();
        fs::write(dir.join("release-signing-key.asc"), "KEY\n").unwrap();
        script(&dir.join("scripts/gates.sh"), "#!/usr/bin/env bash\necho gates-ran >> \"$FAKE_DIR/gates.log\"\n[ -f \"$FAKE_DIR/gates-red\" ] && { echo 'error: clippy found 1 problem' >&2; exit 1; }\nexit 0\n");
    }

    pub fn set_gates(&self, green: bool) {
        let f = self.fake().join("gates-red");
        if green {
            let _ = fs::remove_file(f);
        } else {
            fs::write(f, "").unwrap();
        }
    }

    pub fn head(&self) -> String {
        git(&self.work(), &["rev-parse", "HEAD"])
    }

    /// Replaces what `gh run list` answers for `repo` (JSON text; `@MAIN@` becomes the main sha of the catalog remote).
    pub fn runs(&self, repo: &str, json: &str) {
        fs::write(
            self.fake()
                .join(format!("runs-{}.json", repo.replace('/', "_"))),
            json,
        )
        .unwrap();
    }

    pub fn release_draft(&self) {
        fs::write(
            self.fake().join("release-Org_fs-demo-v0.0.9.json"),
            "{\"isDraft\":true,\"isPrerelease\":false}",
        )
        .unwrap();
    }

    pub fn target(&self) -> Target {
        Target {
            repo_dir: self.work(),
            slug: SLUG.into(),
            branch: "feat".into(),
            base: "main".into(),
            version: "0.0.9".into(),
            release_dir: self.root().join("rel"),
            catalog_dir: Some(self.root().join("catalog")),
            catalog_slug: CAT.into(),
            approval_dir: self.root().join("freigaben"),
            approval_days: 7,
            today_days: fs_release_chain::approval::parse_date("2026-11-12").unwrap(),
            now_secs: fs_release_chain::approval::parse_date("2026-11-12").unwrap() * 86_400
                + 3_600,
            audit_log: self.root().join("audit/AUDIT.log"),
            fake: false,
            state_dir: self.root().join("state"),
            prerelease: true,
            wrapper: vec![],
        }
    }

    pub fn runner(&self) -> SystemRunner {
        let path = format!(
            "{}:{}",
            self.root().join("bin").display(),
            std::env::var("PATH").unwrap_or_default()
        );
        let mut env: Vec<(String, String)> = ident()
            .into_iter()
            .map(|(k, v)| (k.into(), v.into()))
            .collect();
        env.extend([
            ("PATH".into(), path),
            ("FAKE_DIR".into(), self.fake().display().to_string()),
            (
                "FAKE_CAT_ORIGIN".into(),
                self.cat_origin().display().to_string(),
            ),
            ("GIT_CONFIG_GLOBAL".into(), "/dev/null".into()),
            ("GIT_CONFIG_NOSYSTEM".into(), "1".into()),
        ]);
        SystemRunner { env }
    }

    /// Runs the real chain file (all commands) with the given reach.
    pub fn run(&self, reach: Reach) -> Outcome {
        let chain = Chain::load(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../chains/release.chain.yaml"),
        )
        .unwrap();
        self.run_names(reach, &chain.names())
    }

    pub fn run_names(&self, reach: Reach, names: &[String]) -> Outcome {
        self.run_target(self.target(), reach, names)
    }

    /// Like `run_names`, with a changed target.
    pub fn run_target(&self, t: Target, reach: Reach, names: &[String]) -> Outcome {
        let runner = self.runner();
        let ledger = Ledger::new(&t.state_dir, &t.version);
        let ctx = Ctx::new(t, reach, Timing::standard(), &runner, &self.clock);
        runner::execute(&ctx, &ledger, names)
    }

    pub fn ledger(&self) -> Ledger {
        Ledger::new(&self.root().join("state"), "0.0.9")
    }
}

/// `gh run list` answer: one run per (id, workflow, branch, status, conclusion).
pub fn run_json(sha: &str, rows: &[(u64, &str, &str, &str, &str)]) -> String {
    let items: Vec<String> = rows.iter().map(|(id, wf, br, st, co)| {
        format!("{{\"databaseId\":{id},\"workflowName\":\"{wf}\",\"status\":\"{st}\",\"conclusion\":\"{co}\",\"headSha\":\"{sha}\",\"headBranch\":\"{br}\"}}")
    }).collect();
    format!("[{}]", items.join(","))
}
