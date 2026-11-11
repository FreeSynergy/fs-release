//! Pattern: none (pure functions of the `origin` URL and the wall clock: slug of the origin, slug check, today).

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::sys::{Cmd, Runner};

/// `Owner/name` of a GitHub URL. Only two exact forms count: `https://github.com/<owner>/<name>[.git]` and
/// `git@github.com:<owner>/<name>[.git]`. No substring match, no other host, no user part, no trailing junk; the slug must
/// also pass [`crate::slug::validate`] (so no Unicode, `%`, NUL, `..`).
#[must_use]
pub fn slug_of(url: &str) -> Option<String> {
    let rest = ["https://github.com/", "git@github.com:"]
        .iter()
        .find_map(|p| url.strip_prefix(p))?;
    let slug = rest.strip_suffix(".git").unwrap_or(rest);
    crate::slug::validate(slug).ok().map(|()| slug.to_string())
}

/// The repo `origin` of `dir` is configured for: the raw `remote.origin.url` (not rewritten by `insteadOf`) and every
/// `remote.origin.pushurl` must be GitHub URLs of one and the same repo.
///
/// # Errors
/// A sentence when there is no origin, a URL is not an exact GitHub URL, or push and fetch URL differ.
pub fn configured_slug(runner: &dyn Runner, dir: &Path) -> Result<String, String> {
    let ask = |key: &str| -> Result<Vec<String>, String> {
        let cmd = Cmd {
            program: "git".into(),
            args: vec!["config".into(), "--get-all".into(), key.into()],
            cwd: Some(dir.to_path_buf()),
            ..Cmd::default()
        };
        let o = runner.run(&cmd).map_err(|e| e.to_string())?;
        Ok(o.stdout.lines().map(str::to_string).collect())
    };
    let fetch = ask("remote.origin.url")?;
    let pushes = ask("remote.origin.pushurl")?;
    let mut found: Option<String> = None;
    for url in fetch.iter().chain(pushes.iter()) {
        let slug = slug_of(url).ok_or_else(|| format!("origin URL {url:?} of {} is not https://github.com/<owner>/<name> or git@github.com:<owner>/<name>", dir.display()))?;
        if found.as_ref().is_some_and(|f| *f != slug) {
            return Err(format!(
                "origin of {} points at two repos (fetch and push URL differ)",
                dir.display()
            ));
        }
        found = Some(slug);
    }
    found.ok_or_else(|| format!("{} has no origin", dir.display()))
}

/// `--slug` must be the repo `origin` points at; otherwise the checks would run for one repo and publish to another.
///
/// # Errors
/// A sentence when the origin URL names another repo (or none).
pub fn check_slug(given: &str, origin_url: &str) -> Result<(), String> {
    crate::slug::validate(given)?;
    match slug_of(origin_url) {
        Some(o) if o.eq_ignore_ascii_case(given) => Ok(()),
        Some(o) => Err(format!("--slug {given} but origin is {o}")),
        None => Err(format!(
            "--slug {given} cannot be checked: origin {origin_url} is not a GitHub URL"
        )),
    }
}

/// Now as unix seconds.
#[must_use]
pub fn now_secs() -> i64 {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    i64::try_from(secs).unwrap_or(0)
}

/// Today as days since 1970-01-01.
#[must_use]
pub fn today_days() -> i64 {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    i64::try_from(secs / 86_400).unwrap_or(0)
}
