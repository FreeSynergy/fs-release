//! Pattern: Value object (a GitHub `Owner/name`, valid or not at all: it becomes a folder name and a command argument).

/// Checks a slug: exactly one `/`, both parts non-empty, only letters, digits, `.`, `_`, `-`, no `..`, no part starting
/// with a dot. Nothing else gets near a path or a command line.
///
/// # Errors
/// A sentence naming what is wrong.
pub fn validate(slug: &str) -> Result<(), String> {
    let bad = || {
        Err(format!(
            "not a valid owner/name: {slug:?} (letters, digits, . _ - only, one slash, no ..)"
        ))
    };
    let Some((owner, name)) = slug.split_once('/') else {
        return bad();
    };
    let part_ok = |p: &str| {
        !p.is_empty()
            && !p.starts_with('.')
            && p.chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
    };
    if part_ok(owner) && part_ok(name) && !slug.contains("..") {
        Ok(())
    } else {
        bad()
    }
}

/// Checks a branch name that becomes a git argument and a ref: not empty, no leading `-` or `/`, only letters, digits, `.`, `_`,
/// `-`, `/`, no `..`, no `//`, no trailing `/` or `.`.
///
/// # Errors
/// A sentence naming what is wrong.
pub fn validate_branch(name: &str) -> Result<(), String> {
    let ok = !name.is_empty()
        && !name.starts_with(['-', '/'])
        && !name.ends_with(['/', '.'])
        && !name.contains("..")
        && !name.contains("//")
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '/'));
    if ok {
        Ok(())
    } else {
        Err(format!(
            "not a valid branch name: {name:?} (letters, digits, . _ - / only, no leading -, no ..)"
        ))
    }
}

/// The repo name part (`fs-store` of `FreeSynergy/fs-store`).
#[must_use]
pub fn name(slug: &str) -> &str {
    slug.rsplit('/').next().unwrap_or(slug)
}
