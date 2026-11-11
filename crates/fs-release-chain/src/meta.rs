//! Pattern: Value object (risk level, reach and the fixed facts of one command; valid on construction, compared by value).

use serde::Deserialize;

/// How dangerous a command is. 1 to 5 are the scale for the own machine (look, open, start, install, system-wide);
/// `Publish` ("P") does not fit that scale because it affects other people, so it is its own level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Risk {
    View,
    Open,
    Start,
    Install,
    System,
    Publish,
}

/// Who can see the effect of a command. The second axis next to [`Risk`]; a run may only touch up to the reach it was
/// given (`--reach`), so a chain started during a push pause cannot publish by accident.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Reach {
    /// Only this machine (worktree, local files, read-only queries).
    Local,
    /// Shared privately (a branch on the remote, CI runs of a not yet released state).
    Shared,
    /// Everybody (main, tags, releases, catalog).
    Public,
}

impl Reach {
    /// Word used on the command line and in the chain file.
    #[must_use]
    pub fn word(self) -> &'static str {
        match self {
            Reach::Local => "local",
            Reach::Shared => "shared",
            Reach::Public => "public",
        }
    }

    /// Reads the command line word.
    #[must_use]
    pub fn parse(word: &str) -> Option<Reach> {
        [Reach::Local, Reach::Shared, Reach::Public]
            .into_iter()
            .find(|r| r.word() == word)
    }
}

impl Risk {
    /// Word used in the chain file.
    #[must_use]
    pub fn word(self) -> &'static str {
        match self {
            Risk::View => "view",
            Risk::Open => "open",
            Risk::Start => "start",
            Risk::Install => "install",
            Risk::System => "system",
            Risk::Publish => "publish",
        }
    }
}

/// The fixed facts of one command, the same ones the chain file repeats (a test keeps them equal).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Meta {
    /// Command name (`gates`, `push`, ...).
    pub name: &'static str,
    pub risk: Risk,
    pub reach: Reach,
    /// One whole sentence with named placeholders, shown before a run.
    pub preview: &'static str,
    /// What undoes the command, or `none`.
    pub undo: &'static str,
}
