//! Error types for VFS operations.

#![forbid(unsafe_code)]

use std::fmt;
use std::io;

use crate::path::VfsPath;

/// Errors that can occur during VFS operations.
#[derive(Debug)]
pub enum VfsError {
    /// Target path was not found.
    NotFound(VfsPath),
    /// Permission denied for operation on target path.
    PermissionDenied(VfsPath),
    /// Target path already exists.
    AlreadyExists(VfsPath),
    /// Filesystem or archive is read-only.
    ReadOnly,
    /// Expected a directory, but target path is not a directory.
    NotADirectory(VfsPath),
    /// Expected a file, but target path is not a file.
    NotAFile(VfsPath),
    /// Virtual path is malformed or invalid.
    InvalidPath(String),
    /// Symlink cycle or loop was detected during traversal.
    SymlinkCycle(VfsPath),
    /// Symlink or path traversal escaped the root directory boundary.
    TraversalEscape(VfsPath),
    /// File size exceeded the requested maximum byte budget.
    FileTooLarge {
        /// Path to the oversized file.
        path: VfsPath,
        /// Maximum allowed bytes.
        max_bytes: u64,
        /// Actual file size observed.
        actual: u64,
    },
    /// Archive payload or metadata is corrupted.
    CorruptedArchive(String),
    /// Archive or file format is unsupported.
    UnsupportedFormat(String),
    /// Snapshot schema version is incompatible with current version.
    SchemaVersionMismatch {
        /// Expected schema version.
        expected: u32,
        /// Observed schema version in payload.
        found: u32,
    },
    /// Underlying I/O error.
    Io(io::Error),
}

impl fmt::Display for VfsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(path) => write!(f, "path not found: {path}"),
            Self::PermissionDenied(path) => write!(f, "permission denied: {path}"),
            Self::AlreadyExists(path) => write!(f, "path already exists: {path}"),
            Self::ReadOnly => write!(f, "filesystem is read-only"),
            Self::NotADirectory(path) => write!(f, "not a directory: {path}"),
            Self::NotAFile(path) => write!(f, "not a file: {path}"),
            Self::InvalidPath(msg) => write!(f, "invalid path: {msg}"),
            Self::SymlinkCycle(path) => write!(f, "symlink cycle detected at: {path}"),
            Self::TraversalEscape(path) => {
                write!(f, "traversal escaping root directory detected at: {path}")
            }
            Self::FileTooLarge {
                path,
                max_bytes,
                actual,
            } => {
                write!(
                    f,
                    "file too large at {path}: size {actual} bytes exceeds limit of {max_bytes} bytes"
                )
            }
            Self::CorruptedArchive(msg) => write!(f, "corrupted archive: {msg}"),
            Self::UnsupportedFormat(msg) => write!(f, "unsupported format: {msg}"),
            Self::SchemaVersionMismatch { expected, found } => {
                write!(
                    f,
                    "schema version mismatch: expected version {expected}, found {found}"
                )
            }
            Self::Io(err) => write!(f, "I/O error: {err}"),
        }
    }
}

impl std::error::Error for VfsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for VfsError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<serde_json::Error> for VfsError {
    fn from(err: serde_json::Error) -> Self {
        Self::CorruptedArchive(err.to_string())
    }
}

impl From<zip::result::ZipError> for VfsError {
    fn from(err: zip::result::ZipError) -> Self {
        match err {
            zip::result::ZipError::Io(io_err) => Self::Io(io_err),
            zip::result::ZipError::FileNotFound => Self::NotFound(VfsPath::root()),
            zip::result::ZipError::InvalidArchive(msg) => Self::CorruptedArchive(msg.to_string()),
            zip::result::ZipError::UnsupportedArchive(msg) => {
                Self::UnsupportedFormat(msg.to_string())
            }
            other => Self::CorruptedArchive(other.to_string()),
        }
    }
}
