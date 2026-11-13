//! Pattern: Factory (reads the chain file into typed steps and checks it against the commands in the program).

use std::fs;
use std::path::Path;

use serde::Deserialize;

use crate::meta::{Reach, Risk};
use crate::registry;

/// The chain file in the "FS profile" precursor: a fixed list of known commands, no loops, no expressions.
/// Mapping to Serverless Workflow: see DESIGN.md ("FS profile precursor").
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Chain {
    pub chain: String,
    pub version: String,
    /// Command catalog and its version, `fs/0.0.5`.
    pub catalog: String,
    pub steps: Vec<StepDecl>,
}

/// One line of the chain: the command and the facts that must equal the program's own.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct StepDecl {
    pub id: String,
    /// `fs.<command>`.
    pub call: String,
    pub risk: Risk,
    pub reach: Reach,
    /// Who acts: `machine`, `program` or `llm` (descriptive; the approval point is `llm`).
    pub actor: String,
}

impl Chain {
    /// Reads and checks a chain file.
    ///
    /// # Errors
    /// A sentence: the file is unreadable, malformed, or differs from the commands in the program (order, name, risk, reach).
    pub fn load(path: &Path) -> Result<Chain, String> {
        let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let chain: Chain =
            serde_norway::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        chain.verify()?;
        Ok(chain)
    }

    /// The command names in order.
    #[must_use]
    pub fn names(&self) -> Vec<String> {
        self.steps.iter().map(|s| s.id.clone()).collect()
    }

    /// The file must list exactly the commands the program has, in the same order, with the same risk and reach.
    ///
    /// # Errors
    /// A sentence naming the first difference.
    pub fn verify(&self) -> Result<(), String> {
        let code = registry::all();
        if self.steps.len() != code.len() {
            return Err(format!(
                "chain lists {} commands, the program has {}",
                self.steps.len(),
                code.len()
            ));
        }
        for (decl, step) in self.steps.iter().zip(code.iter()) {
            let m = step.meta();
            if decl.id != m.name || decl.call != format!("fs.{}", m.name) {
                return Err(format!(
                    "chain has {} ({}) where the program has {}",
                    decl.id, decl.call, m.name
                ));
            }
            if decl.risk != m.risk || decl.reach != m.reach {
                return Err(format!(
                    "{}: chain says risk {} reach {}, the program says risk {} reach {}",
                    m.name,
                    decl.risk.word(),
                    decl.reach.word(),
                    m.risk.word(),
                    m.reach.word()
                ));
            }
        }
        Ok(())
    }
}
