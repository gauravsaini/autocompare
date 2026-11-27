//! Session, workspace, and application settings persistence layer for AutoCompare.
//!
//! Provides schema-versioned serialization, deserialization, and safe migrations
//! for:
//! - [`SessionDocument`] / [`SessionConfig`]: Multi-tab compare sessions with paths and options.
//! - [`WorkspaceDocument`] / [`WorkspaceConfig`]: Multi-session workspace definitions and window layout state.
//! - [`AppSettings`]: Global preferences including theme, keymap, and grammar preset.

#![forbid(unsafe_code)]

pub mod error;
pub mod event_log;
pub mod options;
pub mod session;
pub mod settings;
pub mod workspace;

pub use error::SessionError;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// Writes `bytes` to `path` atomically via a sibling `.tmp` file + rename,
/// so a crash can never leave a torn half-written document behind.
pub(crate) fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), SessionError> {
    let mut tmp: OsString = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp_path = PathBuf::from(tmp);
    if let Err(e) = std::fs::write(&tmp_path, bytes) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(SessionError::Io(e));
    }
    if let Err(e) = std::fs::rename(&tmp_path, path) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(SessionError::Io(e));
    }
    Ok(())
}
pub use event_log::{SessionEvent, SessionEventKind, SessionEventLog, SessionReplayState};
pub use options::{
    AudioCompareOptions, FolderCompareOptions, HexCompareOptions, MergeCompareOptions,
    PictureCompareOptions, SyncCompareOptions, TableCompareOptions, TextCompareOptions,
};
pub use session::{
    CURRENT_SESSION_SCHEMA_VERSION, SessionConfig, SessionDocument, SessionTab, migrate,
    migrate_session_config,
};
pub use settings::{
    AppSettings, CURRENT_SETTINGS_SCHEMA_VERSION, GrammarPreset, default_grammar_presets,
    migrate_settings,
};
pub use workspace::{
    CURRENT_WORKSPACE_SCHEMA_VERSION, WindowLayout, WorkspaceConfig, WorkspaceDocument,
    migrate_workspace, migrate_workspace_config,
};
