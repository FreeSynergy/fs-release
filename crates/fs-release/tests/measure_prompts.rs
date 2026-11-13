//! Pattern: none (test). `scripts/measure-prompts.py` on sample data: prompt size and cost per kind of run, with counter-tests
//! (a missing source, an aborted run without a figure, a `-fix` name).

mod support;
use std::fs;
use std::path::Path;

use support::{python, workflows::root};

const COSTS: &str =
    "| Datum | Aufgabe | Modell | USD | Tokens | Ergebnis |\n|---|---|---|---|---|---|\n\
| 2026-11-12 | wiki-008 | sonnet | 0.20 | 1000 | success: a |\n\
| 2026-11-12 | wiki-009 | sonnet | 0.40 | 3000 | success: b |\n\
| 2026-11-12 | wiki-fix | sonnet | 0.50 | 500 | success: c |\n\
| 2026-11-12 | rev-wiki-008 | opus | 1.00 | 2000 | success: d |\n\
| 2026-11-12 | dossier-wiki | haiku | 0.10 | 400 | success: e |\n\
| 2026-11-12 | wiki-010 | sonnet | 0.0000 | 0 | Fehler rc=1 (kein JSON) |\n";

fn sample() -> (tempfile::TempDir, tempfile::TempDir) {
    let prompts = tempfile::tempdir().unwrap();
    for (name, size) in [
        ("build-wiki-008.txt", 1000),
        ("build-wiki-009.txt", 3000),
        ("build-wiki-fix.txt", 500),
        ("rev-wiki-008.txt", 2000),
        ("dossier-wiki.txt", 700),
        ("research-x.txt", 9999),
    ] {
        fs::write(prompts.path().join(name), "x".repeat(size)).unwrap();
    }
    let costs = tempfile::tempdir().unwrap();
    fs::write(costs.path().join("kosten.md"), COSTS).unwrap();
    (prompts, costs)
}

fn run(prompts: &Path, costs: &Path) -> (i32, String) {
    python(
        &root().join("scripts/measure-prompts.py"),
        &[
            Path::new("--prompts"),
            prompts,
            Path::new("--costs"),
            &costs.join("kosten.md"),
        ],
    )
}

#[test]
fn the_table_has_mean_size_and_cost_per_kind() {
    let (prompts, costs) = sample();
    let (code, out) = run(prompts.path(), costs.path());
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains("| build | 2 | 2000 | 2 | 0.3000 | 2000 |"),
        "{out}"
    );
    assert!(
        out.contains("| fix | 1 | 500 | 1 | 0.5000 | 500 |"),
        "{out}"
    );
    assert!(
        out.contains("| rev | 1 | 2000 | 1 | 1.0000 | 2000 |"),
        "{out}"
    );
    assert!(
        out.contains("| dossier | 1 | 700 | 1 | 0.1000 | 400 |"),
        "{out}"
    );
}

#[test]
fn counter_aborted_run_and_foreign_prompt_do_not_count() {
    let (prompts, costs) = sample();
    let (_, out) = run(prompts.path(), costs.path());
    assert!(!out.contains("9999"), "research-x.txt is not a kind: {out}");
    assert!(
        !out.contains("| build | 2 | 2000 | 3 |"),
        "the run without a figure was counted: {out}"
    );
}

/// Twelve build runs on 2026-11-01..12 (USD = day, tokens = day * 100) and two review runs, around the day 2026-11-07.
fn runs_by_day() -> (tempfile::TempDir, tempfile::TempDir) {
    let (prompts, costs) = sample();
    let mut text = String::from("| Datum | Aufgabe | Modell | USD | Tokens | Ergebnis |\n|---|---|---|---|---|---|\n");
    text.extend((1..=12).map(|day| {
        format!(
            "| 2026-11-{day:02} | store-{day} | sonnet | {day}.00 | {} | success |\n",
            day * 100
        )
    }));
    text += "| 2026-11-03 | rev-store-3 | opus | 2.00 | 1000 | success |\n| 2026-11-09 | rev-store-9 | opus | 1.00 | 400 | success |\n";
    fs::write(costs.path().join("kosten.md"), text).unwrap();
    (prompts, costs)
}

fn run_since(prompts: &Path, costs: &Path, since: &str) -> (i32, String) {
    python(
        &root().join("scripts/measure-prompts.py"),
        &[
            Path::new("--prompts"),
            prompts,
            Path::new("--costs"),
            &costs.join("kosten.md"),
            Path::new("--since"),
            Path::new(since),
        ],
    )
}

#[test]
fn before_and_after_compare_five_runs_of_cost_and_tokens() {
    let (prompts, costs) = runs_by_day();
    let (code, out) = run_since(prompts.path(), costs.path(), "2026-11-07");
    assert_eq!(code, 0, "{out}");
    // the last five before the 7th are the days 2..6, the first five from it on are the days 7..11 (the 12th is outside)
    assert!(out.contains("| build | 5 | 4.0000 | 400 | 5 | 9.0000 | 900 |"), "{out}");
    assert!(out.contains("| rev | 1 | 2.0000 | 1000 | 1 | 1.0000 | 400 |"), "{out}");
}

#[test]
fn counter_a_window_without_enough_runs_is_shown_and_a_bad_date_is_red() {
    let (prompts, costs) = runs_by_day();
    let (_, out) = run_since(prompts.path(), costs.path(), "2026-11-11");
    assert!(out.contains("| build | 5 | 8.0000 | 800 | 2 | 11.5000 | 1150 |"), "{out}");
    assert!(out.contains("fewer than 5 runs"), "{out}");
    let (_, plain) = run(prompts.path(), costs.path());
    assert!(!plain.contains("runs before"), "no comparison without --since: {plain}");
    let (code, out) = run_since(prompts.path(), costs.path(), "yesterday");
    assert_eq!(code, 1, "{out}");
}

#[test]
fn counter_a_missing_source_is_red() {
    let (prompts, costs) = sample();
    let (code, out) = run(&prompts.path().join("none"), costs.path());
    assert_eq!(code, 1);
    assert!(out.contains("missing"), "{out}");
    let (code, _) = run(prompts.path(), &costs.path().join("none"));
    assert_eq!(code, 1);
}
