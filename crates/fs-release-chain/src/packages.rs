//! Pattern: none (one pure function: sets the `tag` of one package in the text of the catalog's `packages.toml`).

/// Result of an edit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edit {
    /// Catalog id of the package (`fs-store`).
    pub id: String,
    /// The new text of the file.
    pub text: String,
    /// False when the tag was there already.
    pub changed: bool,
}

fn value(line: &str, key: &str) -> Option<String> {
    let rest = line
        .trim_start()
        .strip_prefix(key)?
        .trim_start()
        .strip_prefix('=')?;
    Some(rest.trim().trim_matches('"').to_string())
}

/// Sets `tag = "<tag>"` in the `[[package]]` table whose `repo` is `repo`; comments and all other lines stay.
///
/// # Errors
/// A sentence when no package has this repo or the package has no `tag` line.
pub fn set_tag(text: &str, repo: &str, tag: &str) -> Result<Edit, String> {
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    let starts: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, l)| l.trim() == "[[package]]")
        .map(|(i, _)| i)
        .collect();
    for (n, &from) in starts.iter().enumerate() {
        let to = starts.get(n + 1).copied().unwrap_or(lines.len());
        let find = |key: &str| (from..to).find_map(|i| value(&lines[i], key).map(|v| (i, v)));
        if find("repo").is_none_or(|(_, v)| v != repo) {
            continue;
        }
        let id = find("id").map_or_else(|| repo.to_string(), |(_, v)| v);
        let Some((at, old)) = find("tag") else {
            return Err(format!(
                "the package of {repo} has no tag line in packages.toml"
            ));
        };
        let changed = old != tag;
        if changed {
            lines[at] = format!("tag = \"{tag}\"");
        }
        let mut out = lines.join("\n");
        if text.ends_with('\n') {
            out.push('\n');
        }
        return Ok(Edit {
            id,
            text: out,
            changed,
        });
    }
    Err(format!("no package with repo \"{repo}\" in packages.toml"))
}

#[cfg(test)]
mod tests {
    use super::set_tag;

    const FILE: &str = "# head\n[[package]]\nid = \"a\"\nrepo = \"O/a\"\ntag = \"v0.0.1\"\n\n[[package]]\nid = \"b\"\nrepo = \"O/b\"\ntag = \"v0.0.7\" \nkind = \"x\"\n";

    #[test]
    fn sets_only_the_matching_package() {
        let e = set_tag(FILE, "O/b", "v0.0.9").unwrap();
        assert!(e.changed && e.id == "b");
        assert!(
            e.text.contains("tag = \"v0.0.1\"")
                && e.text.contains("tag = \"v0.0.9\"")
                && e.text.ends_with("kind = \"x\"\n")
        );
    }

    #[test]
    fn same_tag_changes_nothing_and_unknown_repo_is_an_error() {
        assert!(!set_tag(FILE, "O/a", "v0.0.1").unwrap().changed);
        assert!(set_tag(FILE, "O/zzz", "v1").is_err());
    }
}
