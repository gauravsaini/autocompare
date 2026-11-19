use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::error::SessionError;

/// The current schema version for WorkspaceDocument.
pub const CURRENT_WORKSPACE_SCHEMA_VERSION: u32 = 1;

/// Window layout details persisted within a workspace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowLayout {
    /// Window width in pixels.
    pub width: u32,
    /// Window height in pixels.
    pub height: u32,
    /// Whether the workspace window is maximized.
    #[serde(default)]
    pub is_maximized: bool,
}

impl Default for WindowLayout {
    fn default() -> Self {
        Self {
            width: 1280,
            height: 800,
            is_maximized: false,
        }
    }
}

/// A serialized workspace document holding multi-session configurations and window state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceDocument {
    /// Schema version for forward/backward compatibility.
    pub schema_version: u32,
    /// Workspace name.
    pub name: String,
    /// File paths or URIs to associated sessions.
    #[serde(default)]
    pub session_paths: Vec<String>,
    /// Path or ID of the currently active session.
    #[serde(default)]
    pub active_session: Option<String>,
    /// Optional window geometry / state.
    #[serde(default)]
    pub window_layout: Option<WindowLayout>,
    /// Recent paths or session history.
    #[serde(default)]
    pub recent_paths: Vec<String>,
    /// Arbitrary metadata or user tags.
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

fn default_workspace_version() -> u32 {
    CURRENT_WORKSPACE_SCHEMA_VERSION
}

impl Default for WorkspaceDocument {
    fn default() -> Self {
        Self {
            schema_version: default_workspace_version(),
            name: "Default Workspace".to_string(),
            session_paths: Vec::new(),
            active_session: None,
            window_layout: None,
            recent_paths: Vec::new(),
            metadata: HashMap::new(),
        }
    }
}

impl WorkspaceDocument {
    /// Creates a new workspace document with the given name at the current schema version.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            schema_version: default_workspace_version(),
            name: name.into(),
            session_paths: Vec::new(),
            active_session: None,
            window_layout: None,
            recent_paths: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Serializes the workspace document to a compact JSON string.
    pub fn to_json(&self) -> Result<String, SessionError> {
        serde_json::to_string(self).map_err(SessionError::Json)
    }

    /// Serializes the workspace document to a pretty-printed JSON string.
    pub fn to_json_pretty(&self) -> Result<String, SessionError> {
        serde_json::to_string_pretty(self).map_err(SessionError::Json)
    }

    /// Deserializes and automatically migrates any supported JSON version to the current version.
    pub fn from_json(json_str: &str) -> Result<Self, SessionError> {
        migrate_workspace(json_str)
    }

    /// Saves the workspace document as pretty-printed JSON to the specified file path.
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), SessionError> {
        let json = self.to_json_pretty()?;
        std::fs::write(path, json).map_err(SessionError::Io)
    }

    /// Loads a workspace document from a file path, automatically migrating if needed.
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, SessionError> {
        let content = std::fs::read_to_string(path).map_err(SessionError::Io)?;
        Self::from_json(&content)
    }

    /// Migrates an older JSON representation to the current schema version.
    pub fn migrate(old_json: &str) -> Result<Self, SessionError> {
        migrate_workspace(old_json)
    }
}

/// Migrates an arbitrary JSON string into a current [`WorkspaceDocument`].
pub fn migrate_workspace(old_json: &str) -> Result<WorkspaceDocument, SessionError> {
    if old_json.trim().is_empty() {
        return Err(SessionError::CorruptInput("Empty JSON input".to_string()));
    }

    let root: serde_json::Value = serde_json::from_str(old_json)?;

    match root {
        serde_json::Value::Object(ref map) => {
            let version_opt = map.get("schema_version");
            let version = match version_opt {
                None => 0,
                Some(serde_json::Value::Null) => 0,
                Some(serde_json::Value::Number(num)) => match num.as_u64() {
                    Some(v) => {
                        if v > u32::MAX as u64 {
                            return Err(SessionError::UnsupportedVersion {
                                found: u32::MAX,
                                max_supported: CURRENT_WORKSPACE_SCHEMA_VERSION,
                            });
                        }
                        v as u32
                    }
                    None => {
                        return Err(SessionError::CorruptInput(
                            "schema_version must be an unsigned integer".to_string(),
                        ));
                    }
                },
                Some(_) => {
                    return Err(SessionError::CorruptInput(
                        "schema_version must be a numeric integer".to_string(),
                    ));
                }
            };

            if version > CURRENT_WORKSPACE_SCHEMA_VERSION {
                return Err(SessionError::UnsupportedVersion {
                    found: version,
                    max_supported: CURRENT_WORKSPACE_SCHEMA_VERSION,
                });
            }

            if version == CURRENT_WORKSPACE_SCHEMA_VERSION {
                let doc: WorkspaceDocument = serde_json::from_value(root)?;
                Ok(doc)
            } else {
                // v0 migration
                let name = map
                    .get("name")
                    .or_else(|| map.get("title"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("Default Workspace")
                    .to_string();

                let mut session_paths = Vec::new();
                if let Some(serde_json::Value::Array(paths)) =
                    map.get("session_paths").or_else(|| map.get("sessions"))
                {
                    for p in paths {
                        if let Some(s) = p.as_str() {
                            session_paths.push(s.to_string());
                        }
                    }
                }

                let active_session = map
                    .get("active_session")
                    .and_then(|v| v.as_str())
                    .map(ToString::to_string);

                let window_layout =
                    if let Some(serde_json::Value::Object(win)) = map.get("window_layout") {
                        let width = win
                            .get("width")
                            .and_then(|v| v.as_u64())
                            .map(|v| v as u32)
                            .unwrap_or(1280);
                        let height = win
                            .get("height")
                            .and_then(|v| v.as_u64())
                            .map(|v| v as u32)
                            .unwrap_or(800);
                        let is_maximized = win
                            .get("is_maximized")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);
                        Some(WindowLayout {
                            width,
                            height,
                            is_maximized,
                        })
                    } else {
                        None
                    };

                let mut recent_paths = Vec::new();
                if let Some(serde_json::Value::Array(recents)) = map.get("recent_paths") {
                    for r in recents {
                        if let Some(s) = r.as_str() {
                            recent_paths.push(s.to_string());
                        }
                    }
                }

                let mut metadata = HashMap::new();
                if let Some(serde_json::Value::Object(meta)) = map.get("metadata") {
                    for (k, v) in meta {
                        if let Some(s) = v.as_str() {
                            metadata.insert(k.clone(), s.to_string());
                        }
                    }
                }

                Ok(WorkspaceDocument {
                    schema_version: CURRENT_WORKSPACE_SCHEMA_VERSION,
                    name,
                    session_paths,
                    active_session,
                    window_layout,
                    recent_paths,
                    metadata,
                })
            }
        }
        _ => Err(SessionError::CorruptInput(
            "Root JSON must be an object for WorkspaceDocument".to_string(),
        )),
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_builder_and_roundtrip_unit() {
        let ws = WorkspaceDocument::new("Test WS");
        assert_eq!(ws.schema_version, CURRENT_WORKSPACE_SCHEMA_VERSION);
        assert_eq!(ws.name, "Test WS");
        let json = ws.to_json().expect("to_json");
        let parsed = WorkspaceDocument::from_json(&json).expect("from_json");
        assert_eq!(ws, parsed);
    }

    #[test]
    fn test_workspace_v0_migration_unit() {
        let v0 = r#"{"name": "Old", "sessions": ["/path/s"]}"#;
        let ws = migrate_workspace(v0).expect("migrate");
        assert_eq!(ws.schema_version, CURRENT_WORKSPACE_SCHEMA_VERSION);
        assert_eq!(ws.name, "Old");
        assert_eq!(ws.session_paths.len(), 1);
    }

    #[test]
    fn test_workspace_future_version_unit() {
        let v2 = r#"{"schema_version": 2, "name": "Future"}"#;
        assert!(migrate_workspace(v2).is_err());
    }
}
