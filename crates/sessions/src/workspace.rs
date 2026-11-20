use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::error::SessionError;

/// The current schema version for WorkspaceDocument.
pub const CURRENT_WORKSPACE_SCHEMA_VERSION: u32 = 1;

/// Alias for [`WorkspaceDocument`] representing a versioned workspace configuration.
pub type WorkspaceConfig = WorkspaceDocument;

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
    /// Whether this workspace is locked against accidental modification or overwrite.
    #[serde(default)]
    pub locked: bool,
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
            locked: false,
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
            locked: false,
        }
    }

    /// Builder method to set session paths.
    pub fn with_session_paths(mut self, paths: Vec<String>) -> Self {
        self.session_paths = paths;
        self
    }

    /// Builder method to set active session.
    pub fn with_active_session(mut self, active: impl Into<String>) -> Self {
        self.active_session = Some(active.into());
        self
    }

    /// Builder method to set window layout.
    pub fn with_window_layout(mut self, layout: WindowLayout) -> Self {
        self.window_layout = Some(layout);
        self
    }

    /// Builder method to set recent paths.
    pub fn with_recent_paths(mut self, paths: Vec<String>) -> Self {
        self.recent_paths = paths;
        self
    }

    /// Builder method to set metadata.
    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = metadata;
        self
    }

    /// Builder method to set whether the workspace is locked.
    pub fn with_locked(mut self, locked: bool) -> Self {
        self.locked = locked;
        self
    }

    /// Returns true if the workspace document is locked.
    pub fn is_locked(&self) -> bool {
        self.locked
    }

    /// Sets whether the workspace document is locked.
    pub fn set_locked(&mut self, locked: bool) {
        self.locked = locked;
    }

    /// Adds a session path to the workspace.
    pub fn add_session_path(&mut self, path: impl Into<String>) {
        self.session_paths.push(path.into());
    }

    /// Adds a path to the recent paths list.
    pub fn add_recent_path(&mut self, path: impl Into<String>) {
        self.recent_paths.push(path.into());
    }

    /// Sets a key-value entry in metadata.
    pub fn set_metadata(&mut self, key: impl Into<String>, value: impl Into<String>) {
        let _ = self.metadata.insert(key.into(), value.into());
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
    ///
    /// If the workspace is locked (`self.locked == true`), returns an I/O PermissionDenied error.
    /// To bypass the lock check, use [`save_to_file_force`].
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), SessionError> {
        if self.locked {
            return Err(SessionError::Io(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "Cannot overwrite locked workspace document; unlock or use save_to_file_force",
            )));
        }
        self.save_to_file_force(path)
    }

    /// Saves the workspace document unconditionally, bypassing any lock check.
    pub fn save_to_file_force<P: AsRef<Path>>(&self, path: P) -> Result<(), SessionError> {
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

/// Migrates an arbitrary JSON string into a current [`WorkspaceConfig`].
pub fn migrate_workspace_config(old_json: &str) -> Result<WorkspaceConfig, SessionError> {
    migrate_workspace(old_json)
}

/// Migrates an arbitrary JSON string into a current [`WorkspaceDocument`].
///
/// Supports:
/// - Version 0 (unversioned JSON, `schema_version: 0`, legacy field mappings, or bare session arrays)
/// - Version 1 (current schema version)
///
/// Errors on:
/// - Future schema versions (`schema_version > CURRENT_WORKSPACE_SCHEMA_VERSION`)
/// - Malformed or corrupted JSON
/// - Invalid data structures (e.g. non-object and non-array root)
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
                migrate_v0_object(map)
            }
        }
        serde_json::Value::Array(items) => migrate_v0_array(&items),
        _ => Err(SessionError::CorruptInput(
            "Root JSON must be an object or array for WorkspaceDocument".to_string(),
        )),
    }
}

fn migrate_v0_array(items: &[serde_json::Value]) -> Result<WorkspaceDocument, SessionError> {
    let mut session_paths = Vec::new();
    for item in items {
        match item {
            serde_json::Value::String(s) => session_paths.push(s.clone()),
            serde_json::Value::Object(obj) => {
                let path_opt = obj
                    .get("path")
                    .or_else(|| obj.get("file"))
                    .or_else(|| obj.get("uri"))
                    .or_else(|| obj.get("session_path"))
                    .and_then(|v| v.as_str());
                if let Some(p) = path_opt {
                    session_paths.push(p.to_string());
                }
            }
            _ => {}
        }
    }

    Ok(WorkspaceDocument {
        schema_version: CURRENT_WORKSPACE_SCHEMA_VERSION,
        name: "Default Workspace".to_string(),
        session_paths,
        active_session: None,
        window_layout: None,
        recent_paths: Vec::new(),
        metadata: HashMap::new(),
        locked: false,
    })
}

fn migrate_v0_object(
    map: &serde_json::Map<String, serde_json::Value>,
) -> Result<WorkspaceDocument, SessionError> {
    let name = map
        .get("name")
        .or_else(|| map.get("title"))
        .or_else(|| map.get("workspace_name"))
        .or_else(|| map.get("label"))
        .and_then(|v| v.as_str())
        .unwrap_or("Default Workspace")
        .to_string();

    let mut session_paths = Vec::new();
    let raw_sessions = map
        .get("session_paths")
        .or_else(|| map.get("sessions"))
        .or_else(|| map.get("paths"))
        .or_else(|| map.get("open_sessions"))
        .or_else(|| map.get("session_list"))
        .or_else(|| map.get("items"));

    if let Some(serde_json::Value::Array(paths)) = raw_sessions {
        for p in paths {
            match p {
                serde_json::Value::String(s) => session_paths.push(s.clone()),
                serde_json::Value::Object(obj) => {
                    let path_opt = obj
                        .get("path")
                        .or_else(|| obj.get("file"))
                        .or_else(|| obj.get("uri"))
                        .or_else(|| obj.get("session_path"))
                        .and_then(|v| v.as_str());
                    if let Some(path_str) = path_opt {
                        session_paths.push(path_str.to_string());
                    }
                }
                _ => {}
            }
        }
    }

    let active_session = map
        .get("active_session")
        .or_else(|| map.get("active"))
        .or_else(|| map.get("current_session"))
        .or_else(|| map.get("active_tab"))
        .or_else(|| map.get("selected_session"))
        .and_then(|v| v.as_str())
        .map(ToString::to_string);

    let window_layout = {
        let raw_layout = map
            .get("window_layout")
            .or_else(|| map.get("layout"))
            .or_else(|| map.get("window"));

        if let Some(serde_json::Value::Object(win)) = raw_layout {
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
                .or_else(|| win.get("maximized"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            Some(WindowLayout {
                width,
                height,
                is_maximized,
            })
        } else {
            None
        }
    };

    let mut recent_paths = Vec::new();
    let raw_recents = map
        .get("recent_paths")
        .or_else(|| map.get("recent"))
        .or_else(|| map.get("recents"))
        .or_else(|| map.get("history"));

    if let Some(serde_json::Value::Array(recents)) = raw_recents {
        for r in recents {
            match r {
                serde_json::Value::String(s) => recent_paths.push(s.clone()),
                serde_json::Value::Object(obj) => {
                    let path_opt = obj
                        .get("path")
                        .or_else(|| obj.get("file"))
                        .or_else(|| obj.get("uri"))
                        .and_then(|v| v.as_str());
                    if let Some(path_str) = path_opt {
                        recent_paths.push(path_str.to_string());
                    }
                }
                _ => {}
            }
        }
    }

    let mut metadata = HashMap::new();
    let raw_meta = map
        .get("metadata")
        .or_else(|| map.get("meta"))
        .or_else(|| map.get("tags"))
        .or_else(|| map.get("properties"));

    if let Some(serde_json::Value::Object(meta)) = raw_meta {
        for (k, v) in meta {
            if let Some(s) = v.as_str() {
                let _ = metadata.insert(k.clone(), s.to_string());
            } else if let Some(n) = v.as_i64() {
                let _ = metadata.insert(k.clone(), n.to_string());
            } else if let Some(b) = v.as_bool() {
                let _ = metadata.insert(k.clone(), b.to_string());
            }
        }
    }

    let locked = map
        .get("locked")
        .or_else(|| map.get("is_locked"))
        .or_else(|| map.get("read_only_workspace"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    Ok(WorkspaceDocument {
        schema_version: CURRENT_WORKSPACE_SCHEMA_VERSION,
        name,
        session_paths,
        active_session,
        window_layout,
        recent_paths,
        metadata,
        locked,
    })
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
        let ws = WorkspaceDocument::new("Test WS")
            .with_session_paths(vec!["/session1.json".to_string()])
            .with_active_session("/session1.json")
            .with_window_layout(WindowLayout {
                width: 1440,
                height: 900,
                is_maximized: true,
            })
            .with_recent_paths(vec!["/recent.txt".to_string()])
            .with_locked(true);
        assert_eq!(ws.schema_version, CURRENT_WORKSPACE_SCHEMA_VERSION);
        assert_eq!(ws.name, "Test WS");
        assert_eq!(ws.session_paths.len(), 1);
        assert_eq!(ws.active_session.as_deref(), Some("/session1.json"));
        assert!(ws.window_layout.as_ref().unwrap().is_maximized);
        assert!(ws.is_locked());

        let json = ws.to_json().expect("to_json");
        let parsed = WorkspaceDocument::from_json(&json).expect("from_json");
        assert_eq!(ws, parsed);
        assert!(parsed.is_locked());
    }

    #[test]
    fn test_workspace_config_alias_and_helpers_unit() {
        let mut cfg = WorkspaceConfig::new("Config WS");
        cfg.add_session_path("/session/a.json");
        cfg.add_recent_path("/path/recent.rs");
        cfg.set_metadata("theme_override", "dark");
        cfg.set_locked(true);
        assert!(cfg.is_locked());

        let json_pretty = cfg.to_json_pretty().expect("to_json_pretty");
        let loaded = migrate_workspace_config(&json_pretty).expect("migrate_workspace_config");
        assert_eq!(loaded.name, "Config WS");
        assert_eq!(loaded.session_paths, vec!["/session/a.json"]);
        assert_eq!(loaded.recent_paths, vec!["/path/recent.rs"]);
        assert!(loaded.is_locked());
        assert_eq!(
            loaded.metadata.get("theme_override").map(String::as_str),
            Some("dark")
        );
    }

    #[test]
    fn test_workspace_v0_migration_unit() {
        let v0 = r#"{"name": "Old", "sessions": ["/path/s"], "locked": true}"#;
        let ws = migrate_workspace(v0).expect("migrate");
        assert_eq!(ws.schema_version, CURRENT_WORKSPACE_SCHEMA_VERSION);
        assert_eq!(ws.name, "Old");
        assert_eq!(ws.session_paths.len(), 1);
        assert!(ws.locked);
    }

    #[test]
    fn test_workspace_v0_legacy_aliases_unit() {
        let v0 = r#"{
            "title": "Legacy Project",
            "open_sessions": [
                "/raw/path.json",
                {"path": "/obj/path.json"}
            ],
            "current_session": "/raw/path.json",
            "layout": {
                "width": 1600,
                "height": 1000,
                "maximized": true
            },
            "history": ["/hist1", {"path": "/hist2"}],
            "tags": {"project": "alpha", "level": 42},
            "is_locked": true
        }"#;

        let ws = migrate_workspace(v0).expect("migrate legacy aliases");
        assert_eq!(ws.name, "Legacy Project");
        assert_eq!(ws.session_paths, vec!["/raw/path.json", "/obj/path.json"]);
        assert_eq!(ws.active_session.as_deref(), Some("/raw/path.json"));
        let layout = ws.window_layout.expect("window layout exists");
        assert_eq!(layout.width, 1600);
        assert_eq!(layout.height, 1000);
        assert!(layout.is_maximized);
        assert_eq!(ws.recent_paths, vec!["/hist1", "/hist2"]);
        assert!(ws.locked);
        assert_eq!(
            ws.metadata.get("project").map(String::as_str),
            Some("alpha")
        );
        assert_eq!(ws.metadata.get("level").map(String::as_str), Some("42"));
    }

    #[test]
    fn test_workspace_v0_bare_array_unit() {
        let array_json = r#"[
            "/workspace/session1.json",
            {"path": "/workspace/session2.json"},
            {"file": "/workspace/session3.json"}
        ]"#;

        let ws = migrate_workspace(array_json).expect("migrate bare array");
        assert_eq!(ws.schema_version, CURRENT_WORKSPACE_SCHEMA_VERSION);
        assert_eq!(ws.name, "Default Workspace");
        assert_eq!(
            ws.session_paths,
            vec![
                "/workspace/session1.json",
                "/workspace/session2.json",
                "/workspace/session3.json"
            ]
        );
        assert!(!ws.locked);
    }

    #[test]
    fn test_workspace_future_version_unit() {
        let v2 = r#"{"schema_version": 2, "name": "Future"}"#;
        assert!(migrate_workspace(v2).is_err());
    }

    #[test]
    fn test_locked_workspace_save_guard() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join(format!("test_ws_locked_guard_{}.json", std::process::id()));
        let ws = WorkspaceDocument::new("Locked Workspace").with_locked(true);
        assert!(ws.save_to_file(&path).is_err());
        assert!(ws.save_to_file_force(&path).is_ok());
        let _ = std::fs::remove_file(path);
    }
}
