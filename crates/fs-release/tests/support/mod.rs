//! Test support: a throw-away world in a temp folder (local git repos stand in for GitHub, real cargo on tiny crates).

#![allow(dead_code)]

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use fs_release::lift::LiftOpts;
use tempfile::TempDir;

pub mod flow;
pub mod gh;

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

/// Does `python3` have `PyYAML`? The YAML checks are `skipped` (exit 2) when it has not; the tests then expect exactly that.
pub fn have_yaml() -> bool {
    Command::new("python3")
        .args(["-I", "-c", "import yaml"])
        .output()
        .is_ok_and(|o| o.status.success())
}

/// Runs `python3 -I <script> <args>`; returns exit code and stdout.
pub fn python(script: &Path, args: &[&Path]) -> (i32, String) {
    let o = Command::new("python3")
        .arg("-I")
        .arg(script)
        .args(args)
        .output()
        .expect("python3");
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).trim().to_string(),
    )
}

pub fn ident() -> Vec<(&'static str, &'static str)> {
    vec![
        ("GIT_AUTHOR_NAME", "t"),
        ("GIT_AUTHOR_EMAIL", "t@e"),
        ("GIT_COMMITTER_NAME", "t"),
        ("GIT_COMMITTER_EMAIL", "t@e"),
    ]
}

/// A tiny crate that passes fmt, clippy pedantic, the rules and its test.
fn crate_files(root: &Path, name: &str, deps: &str) {
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("Cargo.toml"), format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\n{deps}")).unwrap();
    fs::write(
        root.join("src/lib.rs"),
        "//! Pattern: none (test crate).\n\npub mod answer;\n",
    )
    .unwrap();
    fs::write(
        root.join("src/answer.rs"),
        "//! Pattern: none (test crate).\n\n/// The answer.\n#[must_use]\npub fn answer() -> u32 {\n    42\n}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn is_42() {\n        assert_eq!(super::answer(), 42);\n    }\n}\n",
    )
    .unwrap();
}

/// Repo with one tagged commit per entry (`tag`, crate name, dependencies text), made with `git init` + tags.
fn upstream(dir: &Path, versions: &[(&str, &str, &str)]) {
    fs::create_dir_all(dir).unwrap();
    git(dir, &["init", "--quiet", "-b", "main"]);
    for (tag, name, deps) in versions {
        crate_files(dir, name, deps);
        git(dir, &["add", "-A"]);
        git(dir, &["commit", "--quiet", "--allow-empty", "-m", tag]);
        git(dir, &["tag", tag]);
    }
}

pub struct World {
    pub tmp: TempDir,
}

impl World {
    /// `blinc`: which Blinc tags the world has: components (v0.1.0, v0.2.0) depend on `blinc-core` at `fs-blinc-3`.
    pub fn new() -> World {
        let w = World {
            tmp: TempDir::new().unwrap(),
        };
        let r = w.root();
        let blinc = r.join("up/Blinc");
        upstream(
            &blinc,
            &[
                ("fs-blinc-3", "blinc-core", ""),
                ("fs-blinc-4", "blinc-fork", ""),
            ],
        );
        let dep = "blinc-core = { git = \"https://github.com/FreeSynergy/Blinc\", tag = \"fs-blinc-3\" }\n";
        upstream(
            &r.join("up/fs-components"),
            &[
                ("v0.1.0", "fs-components", dep),
                ("v0.2.0", "fs-components", dep),
            ],
        );
        upstream(
            &r.join("up/fs-interfaces"),
            &[
                ("v0.1.0", "fs-interfaces", ""),
                ("v0.2.0", "fs-interfaces", ""),
            ],
        );
        let mut cfg = String::new();
        for n in ["fs-components", "fs-interfaces", "Blinc"] {
            let _ = writeln!(
                cfg,
                "[url \"{}\"]\n\tinsteadOf = https://github.com/FreeSynergy/{n}",
                r.join("up").join(n).display()
            );
        }
        fs::write(r.join("gitconfig"), cfg).unwrap();
        w.release_dir();
        w.program();
        w
    }

    pub fn root(&self) -> PathBuf {
        self.tmp.path().to_path_buf()
    }

    pub fn work(&self) -> PathBuf {
        self.root().join("work")
    }

    pub fn rel(&self) -> PathBuf {
        self.root().join("rel")
    }

    pub fn sha(&self, repo: &str, rev: &str) -> String {
        git(&self.root().join("up").join(repo), &["rev-parse", rev])
    }

    /// The release checkout: the real scripts, the real toolchain pin, and an approved list for the test Blinc tags.
    fn release_dir(&self) {
        let dir = self.rel();
        fs::create_dir_all(dir.join("scripts")).unwrap();
        let real = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts");
        for f in [
            "check_blinc_lock.py",
            "check_rust_rules.py",
            "set_components_tag.py",
            "check_yaml.py",
        ] {
            fs::copy(real.join(f), dir.join("scripts").join(f)).unwrap();
        }
        fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../RUST_VERSION"),
            dir.join("RUST_VERSION"),
        )
        .unwrap();
        self.approve_blinc(&["fs-blinc-3", "fs-blinc-4"]);
    }

    /// Writes the approved list: the given tags with their real commits.
    pub fn approve_blinc(&self, tags: &[&str]) {
        let mut text = String::new();
        for t in tags {
            let _ = writeln!(text, "{t} {}", self.sha("Blinc", t));
        }
        fs::write(self.rel().join("scripts/blinc-approved.txt"), text).unwrap();
    }

    fn program(&self) {
        let w = self.work();
        fs::create_dir_all(w.join("crates")).unwrap();
        let comp = "fs-components = { git = \"https://github.com/FreeSynergy/fs-components\", tag = \"v0.1.0\" }";
        fs::write(
            w.join("Cargo.toml"),
            format!("[workspace]\nresolver = \"2\"\nmembers = [\"crates/a\", \"crates/b\"]\n\n[workspace.dependencies]\n{comp}\n"),
        )
        .unwrap();
        crate_files(&w.join("crates/a"), "demo-a", "fs-components.workspace = true\nfs-interfaces = { git = \"https://github.com/FreeSynergy/fs-interfaces\", tag = \"v0.1.0\" }\n");
        crate_files(&w.join("crates/b"), "demo-b", &format!("{comp}\n"));
        fs::write(w.join(".gitignore"), "target/\n").unwrap();
        git(&w, &["init", "--quiet", "-b", "main"]);
        let o = self.cargo_env_cmd(&w, &["generate-lockfile"]);
        assert!(o, "generate-lockfile failed");
        git(&w, &["add", "-A"]);
        git(&w, &["commit", "--quiet", "-m", "base"]);
        git(&w, &["checkout", "--quiet", "-b", "feat"]);
    }

    fn cargo_env_cmd(&self, dir: &Path, args: &[&str]) -> bool {
        let mut c = Command::new("cargo");
        c.args(args).current_dir(dir);
        for (k, v) in self.env() {
            c.env(k, v);
        }
        c.env("GIT_CONFIG_GLOBAL", self.root().join("gitconfig"))
            .env("CARGO_NET_GIT_FETCH_WITH_CLI", "true");
        c.status().unwrap().success()
    }

    /// Environment of every program: own cargo home (the fake repos must not mix with the real git cache), toolchain, identity.
    pub fn env(&self) -> Vec<(String, String)> {
        let home = std::env::var("HOME").unwrap_or_default();
        let rustup = std::env::var("RUSTUP_HOME").unwrap_or_else(|_| format!("{home}/.rustup"));
        let mut v: Vec<(String, String)> = ident()
            .into_iter()
            .map(|(k, x)| (k.into(), x.into()))
            .collect();
        v.push((
            "CARGO_HOME".into(),
            self.root().join("cargo-home").display().to_string(),
        ));
        v.push(("RUSTUP_HOME".into(), rustup));
        v.push((
            "RUSTUP_TOOLCHAIN".into(),
            fs::read_to_string(self.rel().join("RUST_VERSION"))
                .unwrap()
                .trim()
                .into(),
        ));
        v
    }

    pub fn opts(&self) -> LiftOpts {
        let mut o = LiftOpts::new(self.work(), self.rel());
        o.git_config = self.root().join("gitconfig");
        o.wrapper = vec![];
        o.approved = Some(self.rel().join("scripts/blinc-approved.txt"));
        o.env = self.env();
        o.components = Some("v0.2.0".into());
        o.interfaces = Some("v0.2.0".into());
        o
    }

    /// Like `opts`, but "online": no local-tags config of the tool; the test's own git config stands in for GitHub.
    pub fn online_opts(&self) -> LiftOpts {
        let mut o = self.opts();
        o.online = true;
        o.env.push((
            "GIT_CONFIG_GLOBAL".into(),
            self.root().join("gitconfig").display().to_string(),
        ));
        o.env
            .push(("CARGO_NET_GIT_FETCH_WITH_CLI".into(), "true".into()));
        o
    }

    pub fn read(&self, rel: &str) -> String {
        fs::read_to_string(self.work().join(rel)).unwrap()
    }
}
