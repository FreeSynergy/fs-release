//! Pattern: none (pure functions of the `origin` URL and the wall clock: slug of the origin, slug check, today).

use std::time::{SystemTime, UNIX_EPOCH};

/// `Owner/name` of a GitHub URL (https or ssh form), if it is one.
#[must_use]
pub fn slug_of(url: &str) -> Option<String> {
    let rest = url.trim().trim_end_matches('/').trim_end_matches(".git");
    let tail = rest
        .rsplit_once("github.com")
        .map(|(_, t)| t.trim_start_matches([':', '/']))?;
    (tail.matches('/').count() == 1).then(|| tail.to_string())
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
