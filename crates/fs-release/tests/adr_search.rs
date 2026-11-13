//! Pattern: none (test). `scripts/adr-search` on a throw-away git repo that stands in for fs-documentation: hits as `ADR <number>`
//! and `todo` lines, case-insensitive, only the published ref and only the two folders; counter-tests for no hit, a bad repo, no term.

mod support;
use std::fs;
use std::path::Path;

use support::workflows::root;
use support::{git, python};

fn docs() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    git(d, &["init", "--quiet", "-b", "main"]);
    for (path, text) in [
        (
            "conventions/decisions/0057-texte.md",
            "# 0057\nTexte laufen ueber Words, Format MF2.\n",
        ),
        (
            "conventions/decisions/0058-stil.md",
            "# 0058\nStil in Tokens.\n",
        ),
        ("todo/liste.md", "- Fluent pruefen\n"),
        (
            "README.md",
            "MF2 steht auch hier, aber ausserhalb der Ordner.\n",
        ),
    ] {
        fs::create_dir_all(d.join(path).parent().unwrap()).unwrap();
        fs::write(d.join(path), text).unwrap();
    }
    git(d, &["add", "-A"]);
    git(d, &["commit", "--quiet", "-m", "docs"]);
    git(d, &["update-ref", "refs/remotes/origin/main", "HEAD"]);
    fs::write(
        d.join("conventions/decisions/0058-stil.md"),
        "Fluent nur im Arbeitsverzeichnis\n",
    )
    .unwrap();
    dir
}

fn search(repo: &Path, terms: &[&str]) -> (i32, String) {
    let mut args = vec![Path::new("--repo"), repo];
    args.extend(terms.iter().map(Path::new));
    python(&root().join("scripts/adr-search"), &args)
}

#[test]
fn a_hit_names_the_adr_number_file_and_line() {
    let dir = docs();
    let (code, out) = search(dir.path(), &["mf2"]);
    assert_eq!(code, 0, "{out}");
    assert_eq!(
        out,
        "ADR 0057 conventions/decisions/0057-texte.md:2: Texte laufen ueber Words, Format MF2."
    );
}

#[test]
fn the_todo_list_counts_and_any_term_matches() {
    let dir = docs();
    let (_, out) = search(dir.path(), &["fluent", "tokens"]);
    assert!(
        out.contains("todo todo/liste.md:1: - Fluent pruefen"),
        "{out}"
    );
    assert!(
        out.contains("ADR 0058 conventions/decisions/0058-stil.md:2: Stil in Tokens."),
        "{out}"
    );
}

#[test]
fn counter_working_copy_and_other_folders_are_not_searched() {
    let dir = docs();
    let (_, out) = search(dir.path(), &["Arbeitsverzeichnis", "ausserhalb"]);
    assert_eq!(out, "no hits");
}

#[test]
fn counter_a_bad_repo_a_bad_ref_and_no_term_are_not_green() {
    let (code, out) = search(Path::new("/nonexistent/fs-documentation"), &["x"]);
    assert_eq!(code, 1, "{out}");
    let dir = docs();
    let args = [
        Path::new("--repo"),
        dir.path(),
        Path::new("--ref"),
        Path::new("origin/none"),
        Path::new("x"),
    ];
    assert_eq!(python(&root().join("scripts/adr-search"), &args).0, 1);
    assert_eq!(search(dir.path(), &[]).0, 2);
}
