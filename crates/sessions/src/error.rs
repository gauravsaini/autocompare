use std::fmt;

/// Errors arising from session, workspace, or settings serialization,
/// deserialization, I/O, or schema version migration.
#[derive(Debug)]
pub enum SessionError {
    /// An I/O error occurred while reading or writing a file.
    Io(std::io::Error),
    /// A JSON serialization or deserialization error occurred.
    Json(serde_json::Error),
    /// An unsupported schema version was encountered (future version).
    UnsupportedVersion { found: u32, max_supported: u32 },
    /// The input format is corrupted, malformed, or has an invalid structure.
    CorruptInput(String),
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "I/O error: {err}"),
            Self::Json(err) => write!(f, "JSON error: {err}"),
            Self::UnsupportedVersion {
                found,
                max_supported,
            } => {
                write!(
                    f,
                    "Unsupported schema version {found}; maximum supported version is {max_supported}"
                )
            }
            Self::CorruptInput(msg) => write!(f, "Corrupted or invalid input: {msg}"),
        }
    }
}

impl std::error::Error for SessionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::Json(err) => Some(err),
            Self::UnsupportedVersion { .. } | Self::CorruptInput(_) => None,
        }
    }
}

impl From<std::io::Error> for SessionError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<serde_json::Error> for SessionError {
    fn from(err: serde_json::Error) -> Self {
        Self::Json(err)
    }
}
