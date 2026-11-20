//! Session, workspace, and application settings persistence layer for AutoCompare.
//!
//! Provides schema-versioned serialization, deserialization, and safe migrations
//! for:
//! - [`SessionDocument`] / [`SessionConfig`]: Multi-tab compare sessions with paths and options.
//! - [`WorkspaceDocument`] / [`WorkspaceConfig`]: Multi-session workspace definitions and window layout state.
//! - [`AppSettings`]: Global preferences including theme, keymap, and grammar preset.

#![forbid(unsafe_code)]

pub mod error;
pub mod options;
pub mod session;
pub mod settings;
pub mod workspace;

pub use error::SessionError;
pub use options::{
    AudioCompareOptions, FolderCompareOptions, HexCompareOptions, MergeCompareOptions,
    PictureCompareOptions, SyncCompareOptions, TableCompareOptions, TextCompareOptions,
};
pub use session::{
    migrate, migrate_session_config, SessionConfig, SessionDocument, SessionTab,
    CURRENT_SESSION_SCHEMA_VERSION,
};
pub use settings::{
    default_grammar_presets, migrate_settings, AppSettings, GrammarPreset,
    CURRENT_SETTINGS_SCHEMA_VERSION,
};
pub use workspace::{
    migrate_workspace, migrate_workspace_config, WindowLayout, WorkspaceConfig, WorkspaceDocument,
    CURRENT_WORKSPACE_SCHEMA_VERSION,
};
