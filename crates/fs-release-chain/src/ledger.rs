//! Pattern: Repository over a JSON-lines file (the run log: what ran on which commit with which result).

use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::outcome::Stop;

/// One finished or stopped command.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Entry {
    pub step: String,
    /// Commit of the branch head the command ran for; every fact in the log is bound to it.
    pub sha: String,
    /// `ok`, `red` or `halt`.
    pub result: String,
    pub detail: String,
    /// Facts later commands need (catalog commit, rerun marker).
    #[serde(default)]
    pub data: BTreeMap<String, String>,
}

/// Append-only log of one release (repo plus version).
#[derive(Debug, Clone)]
pub struct Ledger {
    path: PathBuf,
}

impl Ledger {
    /// The log file for `version` in `dir`.
    #[must_use]
    pub fn new(dir: &Path, version: &str) -> Ledger {
        Ledger {
            path: dir.join(format!("chain-{version}.jsonl")),
        }
    }

    /// Where the log is.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// All entries, oldest first; a missing file is an empty log, a damaged line is skipped.
    #[must_use]
    pub fn entries(&self) -> Vec<Entry> {
        let text = fs::read_to_string(&self.path).unwrap_or_default();
        text.lines()
            .filter_map(|l| serde_json::from_str(l).ok())
            .collect()
    }

    /// Adds an entry (the directory is created on the way).
    ///
    /// # Errors
    /// [`Stop::Red`] when the log cannot be written: a chain that cannot remember must not go on.
    pub fn append(&self, e: &Entry) -> Result<(), Stop> {
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir)?;
        }
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        let line = serde_json::to_string(e).map_err(|x| Stop::red(format!("ledger: {x}")))?;
        writeln!(f, "{line}")?;
        Ok(())
    }

    /// The newest entry of `step` for `sha` with result `ok`.
    #[must_use]
    pub fn last_ok(&self, step: &str, sha: &str) -> Option<Entry> {
        self.entries()
            .into_iter()
            .rev()
            .find(|e| e.step == step && e.sha == sha && e.result == "ok")
    }

    /// True when `step` already left a marker `key` for `sha` (used for "retry once").
    #[must_use]
    pub fn has_marker(&self, step: &str, sha: &str, key: &str) -> bool {
        self.entries()
            .iter()
            .any(|e| e.step == step && e.sha == sha && e.data.contains_key(key))
    }
}
