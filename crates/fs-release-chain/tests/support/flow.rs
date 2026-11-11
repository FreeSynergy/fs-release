//! Helpers of the chain tests: what GitHub would answer for a green release, and a short name for where a run stopped.

use fs_release_chain::Outcome;

use super::{run_json, World, CAT, SLUG};

/// Everything GitHub would answer for a fully green release of the current head.
pub fn all_green(w: &World, sha: &str) {
    w.runs(
        SLUG,
        &run_json(
            sha,
            &[
                (1, "check", "feat", "completed", "success"),
                (2, "release", "v0.0.9", "completed", "success"),
            ],
        ),
    );
    w.runs(
        CAT,
        &run_json("@MAIN@", &[(3, "sign", "main", "completed", "success")]),
    );
    w.release_draft();
}

/// `halt <step> <reason>`, `red <step>` or `done`.
pub fn stopped_at(o: &Outcome) -> String {
    match o {
        Outcome::Halt { step, reason, .. } => format!("halt {step} {reason}"),
        Outcome::Red { step, .. } => format!("red {step}"),
        Outcome::Done { .. } => "done".into(),
    }
}
