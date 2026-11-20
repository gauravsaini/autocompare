use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::error::SessionError;

/// The current schema version for SessionDocument.
pub const CURRENT_SESSION_SCHEMA_VERSION: u32 = 1;

/// Alias for [`SessionDocument`] representing a versioned session configuration.
pub type SessionConfig = SessionDocument;

/// Represents a single tab within a compare session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionTab {
    /// Unique identifier for the tab.
    pub id: String,
    /// Display title for the tab.
    pub title: String,
    /// Type of compare session (e.g., "text", "folder", "table", "hex", "picture").
    #[serde(default = "default_tab_type")]
    pub session_type: String,
    /// Optional left path or URI.
    #[serde(default)]
    pub left_path: Option<String>,
    /// Optional right path or URI.
    #[serde(default)]
    pub right_path: Option<String>,
    /// Whether this tab is marked read-only.
    #[serde(default)]
    pub read_only: bool,
    /// Whether this tab is locked against modification.
    #[serde(default)]
    pub locked: bool,
    /// Additional session-specific options.
    #[serde(default)]
    pub options: HashMap<String, String>,
}

fn default_tab_type() -> String {
    "text".to_string()
}

impl SessionTab {
    /// Creates a new tab with default settings.
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            session_type: default_tab_type(),
            left_path: None,
            right_path: None,
            read_only: false,
            locked: false,
            options: HashMap::new(),
        }
    }

    /// Builder method to set left and right paths.
    pub fn with_paths(mut self, left: impl Into<String>, right: impl Into<String>) -> Self {
        self.left_path = Some(left.into());
        self.right_path = Some(right.into());
        self
    }

    /// Builder method to set the session type.
    pub fn with_session_type(mut self, session_type: impl Into<String>) -> Self {
        self.session_type = session_type.into();
        self
    }

    /// Builder method to set whether this tab is locked.
    pub fn with_locked(mut self, locked: bool) -> Self {
        self.locked = locked;
        self
    }
}

/// A serialized session document containing one or more tabs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionDocument {
    /// Schema version for forward/backward compatibility.
    pub schema_version: u32,
    /// Open tabs in this session.
    pub tabs: Vec<SessionTab>,
    /// Zero-based index of the active tab.
    #[serde(default)]
    pub active_tab_index: usize,
    /// Optional user-assigned session name.
    #[serde(default)]
    pub name: Option<String>,
    /// Whether this session is locked against accidental modification or overwrite.
    #[serde(default)]
    pub locked: bool,
}

impl Default for SessionDocument {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SESSION_SCHEMA_VERSION,
            tabs: Vec::new(),
            active_tab_index: 0,
            name: None,
            locked: false,
        }
    }
}

impl SessionDocument {
    /// Creates a new empty session document at the current schema version.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a session document containing the specified tabs.
    pub fn with_tabs(tabs: Vec<SessionTab>) -> Self {
        Self {
            schema_version: CURRENT_SESSION_SCHEMA_VERSION,
            tabs,
            active_tab_index: 0,
            name: None,
            locked: false,
        }
    }

    /// Returns true if the session document is locked against modification.
    pub fn is_locked(&self) -> bool {
        self.locked
    }

    /// Sets whether the session document is locked.
    pub fn set_locked(&mut self, locked: bool) {
        self.locked = locked;
    }

    /// Builder method to set whether the session is locked.
    pub fn with_locked(mut self, locked: bool) -> Self {
        self.locked = locked;
        self
    }

    /// Serializes the session document to a compact JSON string.
    pub fn to_json(&self) -> Result<String, SessionError> {
        serde_json::to_string(self).map_err(SessionError::Json)
    }

    /// Serializes the session document to a pretty-printed JSON string.
    pub fn to_json_pretty(&self) -> Result<String, SessionError> {
        serde_json::to_string_pretty(self).map_err(SessionError::Json)
    }

    /// Deserializes and automatically migrates any supported JSON version to the current version.
    pub fn from_json(json_str: &str) -> Result<Self, SessionError> {
        migrate(json_str)
    }

    /// Saves the session document as pretty-printed JSON to the specified file path.
    ///
    /// If the session is locked (`self.locked == true`), returns an I/O PermissionDenied error.
    /// To bypass the lock check, use [`save_to_file_force`].
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), SessionError> {
        if self.locked {
            return Err(SessionError::Io(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "Cannot overwrite locked session document; unlock or use save_to_file_force",
            )));
        }
        self.save_to_file_force(path)
    }

    /// Saves the session document unconditionally, bypassing any lock check.
    pub fn save_to_file_force<P: AsRef<Path>>(&self, path: P) -> Result<(), SessionError> {
        let json = self.to_json_pretty()?;
        std::fs::write(path, json).map_err(SessionError::Io)
    }

    /// Loads a session document from a file path, automatically migrating if needed.
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, SessionError> {
        let content = std::fs::read_to_string(path).map_err(SessionError::Io)?;
        Self::from_json(&content)
    }

    /// Migrates an older JSON representation (such as v0) to the current schema version.
    pub fn migrate(old_json: &str) -> Result<Self, SessionError> {
        migrate(old_json)
    }
}

/// Migrates an arbitrary JSON string into a current [`SessionDocument`].
///
/// Supports:
/// - Version 0 (unversioned JSON, `schema_version: 0`, legacy field mappings, or bare tab arrays)
/// - Version 1 (current schema version)
///
/// Errors on:
/// - Future schema versions (`schema_version > CURRENT_SESSION_SCHEMA_VERSION`)
/// - Malformed or corrupted JSON
/// - Invalid data structures (e.g. non-object root where object is expected)
///
/// This function never panics.
pub fn migrate(old_json: &str) -> Result<SessionDocument, SessionError> {
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
                                max_supported: CURRENT_SESSION_SCHEMA_VERSION,
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

            if version > CURRENT_SESSION_SCHEMA_VERSION {
                return Err(SessionError::UnsupportedVersion {
                    found: version,
                    max_supported: CURRENT_SESSION_SCHEMA_VERSION,
                });
            }

            if version == CURRENT_SESSION_SCHEMA_VERSION {
                let doc: SessionDocument = serde_json::from_value(root)?;
                Ok(doc)
            } else {
                // v0 migration path
                migrate_v0_object(map)
            }
        }
        serde_json::Value::Array(items) => {
            // v0 array-of-tabs migration path
            migrate_v0_array(&items)
        }
        _ => Err(SessionError::CorruptInput(
            "Root JSON must be an object or array".to_string(),
        )),
    }
}

fn migrate_v0_object(
    map: &serde_json::Map<String, serde_json::Value>,
) -> Result<SessionDocument, SessionError> {
    let raw_tabs = map
        .get("tabs")
        .or_else(|| map.get("sessions"))
        .or_else(|| map.get("items"));

    let tabs = match raw_tabs {
        Some(serde_json::Value::Array(items)) => parse_v0_tabs(items)?,
        Some(_) => {
            return Err(SessionError::CorruptInput(
                "v0 tabs field must be a JSON array".to_string(),
            ));
        }
        None => Vec::new(),
    };

    let active_tab_index = map
        .get("active_tab_index")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize)
        .unwrap_or(0);

    let name = map
        .get("name")
        .or_else(|| map.get("title"))
        .and_then(|v| v.as_str())
        .map(ToString::to_string);

    let locked = map
        .get("locked")
        .or_else(|| map.get("is_locked"))
        .or_else(|| map.get("read_only_session"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    Ok(SessionDocument {
        schema_version: CURRENT_SESSION_SCHEMA_VERSION,
        tabs,
        active_tab_index,
        name,
        locked,
    })
}

fn migrate_v0_array(items: &[serde_json::Value]) -> Result<SessionDocument, SessionError> {
    let tabs = parse_v0_tabs(items)?;
    Ok(SessionDocument {
        schema_version: CURRENT_SESSION_SCHEMA_VERSION,
        tabs,
        active_tab_index: 0,
        name: None,
        locked: false,
    })
}

fn parse_v0_tabs(items: &[serde_json::Value]) -> Result<Vec<SessionTab>, SessionError> {
    let mut tabs = Vec::with_capacity(items.len());

    for (idx, item) in items.iter().enumerate() {
        match item {
            serde_json::Value::Object(tab_map) => {
                let id = tab_map
                    .get("id")
                    .or_else(|| tab_map.get("uuid"))
                    .and_then(|v| v.as_str())
                    .map(ToString::to_string)
                    .unwrap_or_else(|| format!("tab-{}", idx + 1));

                let title = tab_map
                    .get("title")
                    .or_else(|| tab_map.get("name"))
                    .and_then(|v| v.as_str())
                    .map(ToString::to_string)
                    .unwrap_or_else(|| "Untitled".to_string());

                let session_type = tab_map
                    .get("session_type")
                    .or_else(|| tab_map.get("type"))
                    .or_else(|| tab_map.get("kind"))
                    .and_then(|v| v.as_str())
                    .map(ToString::to_string)
                    .unwrap_or_else(default_tab_type);

                let left_path = tab_map
                    .get("left_path")
                    .or_else(|| tab_map.get("left"))
                    .or_else(|| tab_map.get("path_left"))
                    .and_then(|v| v.as_str())
                    .map(ToString::to_string);

                let right_path = tab_map
                    .get("right_path")
                    .or_else(|| tab_map.get("right"))
                    .or_else(|| tab_map.get("path_right"))
                    .and_then(|v| v.as_str())
                    .map(ToString::to_string);

                let read_only = tab_map
                    .get("read_only")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                let locked = tab_map
                    .get("locked")
                    .or_else(|| tab_map.get("is_locked"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                let mut options = HashMap::new();
                if let Some(serde_json::Value::Object(opts_map)) = tab_map.get("options") {
                    for (k, v) in opts_map {
                        if let Some(s) = v.as_str() {
                            options.insert(k.clone(), s.to_string());
                        }
                    }
                }

                tabs.push(SessionTab {
                    id,
                    title,
                    session_type,
                    left_path,
                    right_path,
                    read_only,
                    locked,
                    options,
                });
            }
            serde_json::Value::String(path_str) => {
                tabs.push(SessionTab {
                    id: format!("tab-{}", idx + 1),
                    title: path_str.clone(),
                    session_type: default_tab_type(),
                    left_path: Some(path_str.clone()),
                    right_path: None,
                    read_only: false,
                    locked: false,
                    options: HashMap::new(),
                });
            }
            _ => {
                return Err(SessionError::CorruptInput(
                    "Tab entry must be an object or string".to_string(),
                ));
            }
        }
    }

    Ok(tabs)
}

/// Migrates an arbitrary JSON string into a current [`SessionConfig`].
pub fn migrate_session_config(old_json: &str) -> Result<SessionConfig, SessionError> {
    migrate(old_json)
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
    fn test_session_tab_builder() {
        let tab = SessionTab::new("tab-1", "My Tab")
            .with_paths("/a", "/b")
            .with_session_type("hex")
            .with_locked(true);
        assert_eq!(tab.id, "tab-1");
        assert_eq!(tab.title, "My Tab");
        assert_eq!(tab.session_type, "hex");
        assert_eq!(tab.left_path.as_deref(), Some("/a"));
        assert_eq!(tab.right_path.as_deref(), Some("/b"));
        assert!(!tab.read_only);
        assert!(tab.locked);
    }

    #[test]
    fn test_session_document_roundtrip_unit() {
        let tab = SessionTab::new("t1", "Tab 1").with_paths("/left", "/right");
        let doc = SessionDocument::with_tabs(vec![tab]).with_locked(true);
        assert!(doc.is_locked());
        let json = doc.to_json().expect("to_json");
        let parsed = SessionDocument::from_json(&json).expect("from_json");
        assert_eq!(doc, parsed);
        assert!(parsed.is_locked());
    }

    #[test]
    fn test_v0_migration_unit() {
        let v0 = r#"{"tabs": [{"id": "t", "title": "T", "locked": true}], "locked": true}"#;
        let doc = migrate(v0).expect("migrate");
        assert_eq!(doc.schema_version, CURRENT_SESSION_SCHEMA_VERSION);
        assert_eq!(doc.tabs.len(), 1);
        assert!(doc.locked);
        assert!(doc.tabs[0].locked);
    }

    #[test]
    fn test_future_version_unit() {
        let v2 = r#"{"schema_version": 2, "tabs": []}"#;
        assert!(migrate(v2).is_err());
    }

    #[test]
    fn test_corrupt_unit() {
        assert!(migrate("not json").is_err());
    }

    #[test]
    fn test_locked_save_guard() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join(format!("test_locked_guard_{}.json", std::process::id()));
        let mut doc = SessionDocument::new();
        doc.set_locked(true);
        assert!(doc.save_to_file(&path).is_err());
        assert!(doc.save_to_file_force(&path).is_ok());
        let _ = std::fs::remove_file(path);
    }
}
