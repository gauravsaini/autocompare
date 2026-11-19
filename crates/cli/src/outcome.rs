//! Comparison outcome and exit code mapping.
//!
//! Maps comparison and merge outcomes to standard process exit codes:
//! 0 for identical/clean and 1 for differences/conflicts.

use std::process::ExitCode;

/// Result of a comparison or merge operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Inputs are identical, or merge completed cleanly without conflicts.
    Same,
    /// Inputs differ, or merge encountered unresolved conflicts.
    Different,
}

impl Outcome {
    /// Converts comparison outcome to process exit code.
    pub fn exit_code(&self) -> ExitCode {
        match self {
            Self::Same => ExitCode::SUCCESS,
            Self::Different => ExitCode::from(1),
        }
    }
}
