//! Session, workspace, and application settings persistence layer for AutoCompare.
//!
//! Provides schema-versioned serialization, deserialization, and safe migrations
//! for:
//! - [`SessionDocument`]: Multi-tab compare sessions with paths and options.
//! - [`WorkspaceDocument`]: Multi-session workspace definitions and window layout state.
//! - [`AppSettings`]: Global preferences including theme, keymap, and grammar preset.

#![forbid(unsafe_code)]

pub mod error;
pub mod session;
pub mod settings;
pub mod workspace;

pub use error::SessionError;
pub use session::{CURRENT_SESSION_SCHEMA_VERSION, SessionDocument, SessionTab, migrate};
pub use settings::{AppSettings, CURRENT_SETTINGS_SCHEMA_VERSION, migrate_settings};
pub use workspace::{
    CURRENT_WORKSPACE_SCHEMA_VERSION, WindowLayout, WorkspaceDocument, migrate_workspace,
};
