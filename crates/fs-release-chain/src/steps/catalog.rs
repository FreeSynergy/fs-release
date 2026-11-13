//! Pattern: Strategy, part of the chain commands (the three commands that work on the catalog: bump, sign, check).

use std::fs;
use std::path::PathBuf;

use crate::ctx::Ctx;
use crate::gh;
use crate::git::{self, short};
use crate::meta::{Meta, Reach, Risk};
use crate::origin::configured_slug;
use crate::outcome::{Note, StepResult, Stop};
use crate::packages::set_tag;
use crate::poll::Filter;
use crate::steps::ci::{settle, Retry};
use crate::steps::{Frame, Step};
use crate::sys::Cmd;

/// Primary fingerprint of the project signing key (fs-catalog README). The key file in the checkout is only imported;
/// a catalog signed by any other key is red, whatever that file contains.
pub const PROJECT_KEY_FPR: &str = "16A4A3EE563AEF51FCF0C55DED7B57C4A1904342";

/// The catalog checkout, and proof that its origin is the catalog repo the run was told about (fetch and push URL).
fn catalog_dir(ctx: &Ctx) -> Result<PathBuf, Stop> {
    let dir = catalog_path(ctx)?;
    let who = configured_slug(ctx.runner, &dir).map_err(Stop::red)?;
    if !who.eq_ignore_ascii_case(&ctx.target.catalog_slug) {
        return Err(Stop::red(format!(
            "catalog checkout {} is {who}, not {}",
            dir.display(),
            ctx.target.catalog_slug
        )));
    }
    Ok(dir)
}

fn catalog_path(ctx: &Ctx) -> Result<PathBuf, Stop> {
    ctx.target
        .catalog_dir
        .clone()
        .ok_or_else(|| Stop::halt("catalog-dir", "--catalog <checkout of the catalog repo>"))
}

/// Puts the new tag of the package into the catalog's `packages.toml` and pushes that commit.
pub struct Bump;

impl Step for Bump {
    fn meta(&self) -> Meta {
        Meta {
            name: "bump",
            risk: Risk::Publish,
            reach: Reach::Public,
            preview: "The catalog gets {{repo}} {{version}}.",
            undo: "bump to the old version",
        }
    }

    fn run(&self, f: &Frame) -> StepResult {
        let t = &f.ctx.target;
        let dir = catalog_dir(f.ctx)?;
        let tag = t.tag();
        if gh::release_state(f.ctx, &t.slug, &tag)?.is_none_or(|r| r.draft) {
            return Err(Stop::red(format!(
                "release {tag} is not published: the catalog generator would stop on it"
            )));
        }
        if !git::out(
            f.ctx,
            &dir,
            &["status", "--porcelain", "--untracked-files=no"],
        )?
        .is_empty()
        {
            return Err(Stop::red(format!(
                "{} has uncommitted changes",
                dir.display()
            )));
        }
        git::out(
            f.ctx,
            &dir,
            &["fetch", "--quiet", "--end-of-options", "origin", &t.base],
        )?;
        let (local, remote) = (
            git::rev(f.ctx, &dir, "HEAD")?,
            git::rev(f.ctx, &dir, &format!("refs/remotes/origin/{}", t.base))?,
        );
        if local != remote || git::out(f.ctx, &dir, &["symbolic-ref", "--short", "HEAD"])? != t.base
        {
            return Err(Stop::red(format!(
                "{} is not exactly origin/{} (HEAD {}, origin {}): local commits are never pushed by the chain, clean up first",
                dir.display(),
                t.base,
                short(&local),
                short(&remote)
            )));
        }
        let file = dir.join("packages.toml");
        let edit = set_tag(&fs::read_to_string(&file)?, &t.slug, &tag).map_err(Stop::red)?;
        if !edit.changed {
            // HEAD is origin/<base>, so the entry is on the remote.
            return Ok(Note::text(format!("catalog has {} {tag} already", edit.id))
                .with("catalog_sha", local)
                .with("unchanged", "1"));
        }
        fs::write(&file, &edit.text)?;
        git::out(f.ctx, &dir, &["add", "packages.toml"])?;
        git::out(
            f.ctx,
            &dir,
            &[
                "commit",
                "--quiet",
                "-m",
                &format!("catalog: {} {}", edit.id, t.version),
            ],
        )?;
        let spec = format!("HEAD:refs/heads/{}", t.base);
        let o = git::raw(f.ctx, &dir, &["push", "origin", &spec])?;
        if !o.ok() {
            git::out(f.ctx, &dir, &["reset", "--hard", "--quiet", &local])?;
            return Err(Stop::red(format!(
                "catalog push rejected (own commit taken back): {}",
                o.first_error()
            )));
        }
        let sha = git::rev(f.ctx, &dir, "HEAD")?;
        Ok(
            Note::text(format!("catalog {} {tag} at {}", edit.id, short(&sha)))
                .with("catalog_sha", sha),
        )
    }
}

/// Waits for the catalog's `sign` run on the bump commit (started by hand once when none was triggered).
pub struct Sign;

impl Step for Sign {
    fn meta(&self) -> Meta {
        Meta {
            name: "sign",
            risk: Risk::Publish,
            reach: Reach::Public,
            preview: "The catalog is signed.",
            undo: "none",
        }
    }

    fn run(&self, f: &Frame) -> StepResult {
        let t = &f.ctx.target;
        // The entry of the run that pushed the commit wins over a later "already there" entry.
        let oks: Vec<_> = f
            .ledger
            .entries()
            .into_iter()
            .filter(|e| e.step == "bump" && e.sha == f.head && e.result == "ok")
            .collect();
        let Some(bump) = oks
            .iter()
            .rev()
            .find(|e| !e.data.contains_key("unchanged"))
            .or(oks.last())
        else {
            return Err(Stop::red(
                "no bump recorded for this commit: run bump first",
            ));
        };
        let sha = bump.data.get("catalog_sha").cloned().unwrap_or_default();
        let mut filter = Filter {
            sha,
            workflow: Some("sign".into()),
            ..Filter::default()
        };
        let seen = gh::list_runs(f.ctx, &t.catalog_slug)?;
        if bump.data.contains_key("unchanged") && !seen.iter().any(|r| filter.matches(r)) {
            // Nothing was pushed by us and no run exists for the commit: start one and take only a newer run.
            filter.min_id = seen.iter().map(|r| r.id).max().unwrap_or(0);
            gh::dispatch(f.ctx, &t.catalog_slug, "sign")?;
            filter.sha = String::new();
        }
        let runs = settle(f, "sign", &t.catalog_slug, &filter, Retry::Never)?;
        Ok(Note::text(format!(
            "catalog signed by run #{}",
            runs.first().map_or(0, |r| r.id)
        )))
    }
}

/// Looks at the published catalog like a user: fresh clone, signature against the project key, the version is in it.
pub struct Check;

impl Step for Check {
    fn meta(&self) -> Meta {
        Meta {
            name: "check",
            risk: Risk::View,
            reach: Reach::Local,
            preview: "The catalog is checked the way a user sees it.",
            undo: "none",
        }
    }

    fn run(&self, f: &Frame) -> StepResult {
        let t = &f.ctx.target;
        let dir = catalog_dir(f.ctx)?;
        let url = git::out(f.ctx, &dir, &["remote", "get-url", "origin"])?;
        let work = t.state_dir.join(format!("check-{}", t.version));
        let _ = fs::remove_dir_all(&work);
        fs::create_dir_all(&work)?;
        let clone = work.join("catalog");
        git::out(
            f.ctx,
            &work,
            &[
                "clone",
                "--quiet",
                "--depth",
                "1",
                &url,
                &clone.display().to_string(),
            ],
        )?;
        let home = work.join("gnupg");
        fs::create_dir_all(&home)?;
        let gpg = |args: Vec<String>| {
            f.ctx.runner.run(&Cmd {
                program: "gpg".into(),
                args,
                env: vec![("GNUPGHOME".into(), home.display().to_string())],
                ..Cmd::default()
            })
        };
        let key = t
            .release_dir
            .join("release-signing-key.asc")
            .display()
            .to_string();
        let imported = gpg(vec!["--batch".into(), "--import".into(), key])?;
        if !imported.ok() {
            return Err(Stop::red(format!(
                "project key not importable: {}",
                imported.first_error()
            )));
        }
        let (asc, xml) = (clone.join("catalog.xml.asc"), clone.join("catalog.xml"));
        let verified = gpg(vec![
            "--batch".into(),
            "--status-fd".into(),
            "1".into(),
            "--verify".into(),
            asc.display().to_string(),
            xml.display().to_string(),
        ])?;
        if !verified.ok() {
            return Err(Stop::red(format!(
                "catalog signature: {}",
                verified.first_error()
            )));
        }
        // VALIDSIG <fpr of the signing key> <date> <ts> <expire> <ver> <res> <pk-algo> <hash-algo> <class> <primary fpr>
        let primary = verified
            .stdout
            .lines()
            .find(|l| l.contains("VALIDSIG"))
            .and_then(|l| l.split_whitespace().last())
            .unwrap_or("");
        if !primary.eq_ignore_ascii_case(PROJECT_KEY_FPR) {
            return Err(Stop::red(format!(
                "catalog is signed by {primary:?}, not by the project key {PROJECT_KEY_FPR}"
            )));
        }
        let version = format!("<release version=\"{}\"", t.version);
        let text = fs::read_to_string(&xml)?;
        let id = t.slug.rsplit('/').next().unwrap_or(&t.slug);
        if !text.contains(&version)
            || !text.contains(&format!("/{id}/releases/download/{}/", t.tag()))
        {
            return Err(Stop::red(format!(
                "catalog.xml has no release {} of {id} (signature is fine)",
                t.version
            )));
        }
        // Last: the store itself, in an empty store root (the real installed state is never touched).
        let root = work.join("store-root");
        fs::create_dir_all(&root)?;
        let store = f.ctx.runner.run(&Cmd {
            program: "fs-store".into(),
            args: vec!["check".into()],
            env: vec![("FS_STORE_ROOT".into(), root.display().to_string())],
            ..Cmd::default()
        })?;
        if !store.ok() {
            return Err(Stop::red(format!(
                "fs-store check: {}",
                store.first_error()
            )));
        }
        let _ = fs::remove_dir_all(&work);
        Ok(Note::text(format!(
            "catalog signed and lists {id} {}",
            t.version
        )))
    }
}
