//! CLI error definitions and exit code mappings.
//!
//! Provides typed error structures for autocompare CLI execution,
//! mapping all failure conditions to process exit code 2.

use std::fmt;
use std::process::ExitCode;

/// Errors arising during CLI execution and argument handling.
#[derive(Debug)]
pub enum CliError {
    /// File or directory I/O operation failed.
    Io {
        /// Subject filesystem path.
        path: String,
        /// Underlying system I/O error.
        source: std::io::Error,
    },
    /// Comparison engine error.
    Compare(autocompare_compare::Error),
    /// Session document persistence error.
    Session(autocompare_sessions::SessionError),
    /// Invalid argument or unsupported combination of flags.
    InvalidArg(String),
    /// Modification attempted in read-only mode.
    ReadOnly(String),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => write!(f, "I/O error at {path}: {source}"),
            Self::Compare(err) => write!(f, "{err}"),
            Self::Session(err) => write!(f, "session error: {err}"),
            Self::InvalidArg(msg) => write!(f, "invalid argument: {msg}"),
            Self::ReadOnly(msg) => write!(f, "read-only mode: {msg}"),
        }
    }
}

impl std::error::Error for CliError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Compare(err) => Some(err),
            Self::Session(err) => Some(err),
            _ => None,
        }
    }
}

impl From<autocompare_compare::Error> for CliError {
    fn from(err: autocompare_compare::Error) -> Self {
        Self::Compare(err)
    }
}

impl From<autocompare_sessions::SessionError> for CliError {
    fn from(err: autocompare_sessions::SessionError) -> Self {
        Self::Session(err)
    }
}

impl CliError {
    /// Maps any CLI error to process exit code 2.
    pub fn exit_code(&self) -> ExitCode {
        ExitCode::from(2)
    }
}
