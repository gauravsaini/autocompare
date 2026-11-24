//! Comparison outcome and exit code mapping.
//!
//! Maps comparison and merge outcomes to process exit codes:
//! - diff(1) mode: 0 for identical/clean, 1 for differences/conflicts, 2 for trouble.
//! - Beyond Compare (bcomp) mode: 0 for success/clean, 11 for binary differences,
//!   13 for rules-based differences, 14 for merge conflicts.

use std::process::ExitCode;

/// Exit code convention mode.
#[derive(clap::ValueEnum, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExitCodeMode {
    /// Standard diff(1) exit codes: 0 = same, 1 = different / conflicts, 2 = error.
    #[default]
    Diff,
    /// Beyond Compare exit codes: 0 = success, 11 = binary diff, 13 = rules diff, 14 = conflicts.
    #[value(name = "bc", alias = "beyond-compare", alias = "bcomp")]
    BeyondCompare,
}

/// Result of a comparison or merge operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Inputs are identical, or merge completed cleanly without conflicts (exit code 0).
    Same,
    /// Inputs differ, or merge encountered unresolved conflicts (exit code 1 in diff mode).
    Different,
    /// Beyond Compare: Success / identical / clean merge (exit code 0).
    BcSuccess,
    /// Beyond Compare /quickcompare: Binary identical (exit code 1).
    BcBinarySame,
    /// Beyond Compare /quickcompare: Rules-based identical (exit code 2).
    BcRulesSame,
    /// Beyond Compare: Binary differences (exit code 11).
    BcBinaryDiff,
    /// Beyond Compare: Similar / unimportant differences (exit code 12).
    BcSimilar,
    /// Beyond Compare: Rules-based differences (exit code 13).
    BcRulesDiff,
    /// Beyond Compare: Conflicts detected (exit code 14).
    BcConflicts,
    /// Beyond Compare: Conflicts detected, merge output not written (exit code 101).
    BcConflictsNotWritten,
    /// Custom exit code.
    Custom(u8),
}

impl Outcome {
    /// Returns the integer exit code.
    pub fn code(&self) -> u8 {
        match self {
            Self::Same | Self::BcSuccess => 0,
            Self::Different | Self::BcBinarySame => 1,
            Self::BcRulesSame => 2,
            Self::BcBinaryDiff => 11,
            Self::BcSimilar => 12,
            Self::BcRulesDiff => 13,
            Self::BcConflicts => 14,
            Self::BcConflictsNotWritten => 101,
            Self::Custom(c) => *c,
        }
    }

    /// Converts comparison outcome to process exit code.
    pub fn exit_code(&self) -> ExitCode {
        ExitCode::from(self.code())
    }

    /// Whether this outcome represents identical inputs or conflict-free merge.
    pub fn is_same(&self) -> bool {
        matches!(
            self,
            Self::Same | Self::BcSuccess | Self::BcBinarySame | Self::BcRulesSame
        )
    }

    /// Whether this outcome represents differences or conflicts.
    pub fn is_diff(&self) -> bool {
        !self.is_same()
    }

    /// Creates an outcome for rules-based / text diff according to the selected mode.
    pub fn for_diff(diff: bool, mode: ExitCodeMode) -> Self {
        match mode {
            ExitCodeMode::Diff => {
                if diff {
                    Self::Different
                } else {
                    Self::Same
                }
            }
            ExitCodeMode::BeyondCompare => {
                if diff {
                    Self::BcRulesDiff
                } else {
                    Self::BcSuccess
                }
            }
        }
    }

    /// Creates an outcome for binary diff according to the selected mode.
    pub fn for_binary(diff: bool, mode: ExitCodeMode) -> Self {
        match mode {
            ExitCodeMode::Diff => {
                if diff {
                    Self::Different
                } else {
                    Self::Same
                }
            }
            ExitCodeMode::BeyondCompare => {
                if diff {
                    Self::BcBinaryDiff
                } else {
                    Self::BcSuccess
                }
            }
        }
    }

    /// Creates an outcome for three-way merge according to the selected mode.
    pub fn for_merge(conflicts: bool, mode: ExitCodeMode) -> Self {
        match mode {
            ExitCodeMode::Diff => {
                if conflicts {
                    Self::Different
                } else {
                    Self::Same
                }
            }
            ExitCodeMode::BeyondCompare => {
                if conflicts {
                    Self::BcConflicts
                } else {
                    Self::BcSuccess
                }
            }
        }
    }

    /// Adapts an existing outcome to the target exit code mode.
    pub fn to_mode(self, mode: ExitCodeMode) -> Self {
        match (self, mode) {
            (Self::Same, ExitCodeMode::BeyondCompare) => Self::BcSuccess,
            (Self::Different, ExitCodeMode::BeyondCompare) => Self::BcRulesDiff,
            (other, ExitCodeMode::Diff) if other.is_same() => Self::Same,
            (other, ExitCodeMode::Diff) if other.is_diff() => Self::Different,
            (current, _) => current,
        }
    }
}
