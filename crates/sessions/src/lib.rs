//! Session, workspace, and application settings persistence layer for AutoCompare.
//!
//! Provides schema-versioned serialization, deserialization, and safe migrations
//! for:
//! - [`SessionDocument`] / [`SessionConfig`]: Multi-tab compare sessions with paths and options.
//! - [`WorkspaceDocument`] / [`WorkspaceConfig`]: Multi-session workspace definitions and window layout state.
//! - [`AppSettings`]: Global preferences including theme, keymap, and grammar preset.

#![forbid(unsafe_code)]

pub mod error;
pub mod session;
pub mod settings;
pub mod workspace;

pub use error::SessionError;
pub use session::{
    CURRENT_SESSION_SCHEMA_VERSION, SessionConfig, SessionDocument, SessionTab, migrate,
    migrate_session_config,
};
pub use settings::{AppSettings, CURRENT_SETTINGS_SCHEMA_VERSION, migrate_settings};
pub use workspace::{
    CURRENT_WORKSPACE_SCHEMA_VERSION, WindowLayout, WorkspaceConfig, WorkspaceDocument,
    migrate_workspace, migrate_workspace_config,
};
