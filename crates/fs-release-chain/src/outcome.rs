//! Pattern: Value object (the three possible ends of a run and their one-line form).

use std::collections::BTreeMap;
use std::io;

/// Longest line the tool prints; an agent reads three of them at most.
const MAX_LINE: usize = 220;

/// Why a command stopped the chain. Both stop it at once; nothing after the step runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stop {
    /// Something has to be decided or fixed by somebody else (approval, reach, queue). Exit code 10.
    Halt { reason: String, needs: String },
    /// A check is red. The line is the first error line. Exit code 1.
    Red { line: String },
}

impl Stop {
    /// A halt with a reason code and what is needed to go on.
    #[must_use]
    pub fn halt(reason: &str, needs: impl Into<String>) -> Stop {
        Stop::Halt {
            reason: reason.to_string(),
            needs: needs.into(),
        }
    }

    /// A red check.
    #[must_use]
    pub fn red(line: impl Into<String>) -> Stop {
        Stop::Red { line: line.into() }
    }
}

impl From<io::Error> for Stop {
    fn from(e: io::Error) -> Stop {
        Stop::red(format!("io: {e}"))
    }
}

/// What a command reports when it did its job or found it already done: a short text and facts later commands need.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Note {
    pub text: String,
    pub data: BTreeMap<String, String>,
}

impl Note {
    /// A note with a text and no data.
    #[must_use]
    pub fn text(text: impl Into<String>) -> Note {
        Note {
            text: text.into(),
            data: BTreeMap::new(),
        }
    }

    /// Adds one fact.
    #[must_use]
    pub fn with(mut self, key: &str, value: impl Into<String>) -> Note {
        self.data.insert(key.to_string(), value.into());
        self
    }
}

/// What a command returns.
pub type StepResult = Result<Note, Stop>;

/// The end of a whole run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Done {
        summary: String,
    },
    Halt {
        step: String,
        reason: String,
        needs: String,
    },
    Red {
        step: String,
        line: String,
    },
}

impl Outcome {
    /// Process exit code: 0 done, 10 halt, 1 red.
    #[must_use]
    pub fn code(&self) -> i32 {
        match self {
            Outcome::Done { .. } => 0,
            Outcome::Halt { .. } => 10,
            Outcome::Red { .. } => 1,
        }
    }

    /// At most one line (`DONE ...`, `HALT ...`, `RED ...`), no line breaks, cut to a fixed length.
    #[must_use]
    pub fn line(&self) -> String {
        let text = match self {
            Outcome::Done { summary } => format!("DONE {summary}"),
            Outcome::Halt {
                step,
                reason,
                needs,
            } => format!("HALT {step} {reason} needs: {needs}"),
            Outcome::Red { step, line } => format!("RED {step} {line}"),
        };
        let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
        flat.chars().take(MAX_LINE).collect()
    }

    /// Builds the end of a run from the stop of the step `step`.
    #[must_use]
    pub fn from_stop(name: &str, stop: Stop) -> Outcome {
        match stop {
            Stop::Halt { reason, needs } => Outcome::Halt {
                step: name.to_string(),
                reason,
                needs,
            },
            Stop::Red { line } => Outcome::Red {
                step: name.to_string(),
                line,
            },
        }
    }
}
