use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::error::SessionError;

/// The current schema version for AppSettings.
pub const CURRENT_SETTINGS_SCHEMA_VERSION: u32 = 1;

/// Application configuration including theme, keybindings, and grammar presets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppSettings {
    /// Schema version for forward/backward compatibility.
    pub schema_version: u32,
    /// Color theme name (e.g. "dark", "light", "high_contrast").
    #[serde(default = "default_theme")]
    pub theme: String,
    /// Keymap preset name (e.g. "default", "vim", "vscode", "emacs").
    #[serde(default = "default_keymap")]
    pub keymap: String,
    /// Syntax highlighting / grammar preset name (e.g. "default", "standard", "extended").
    #[serde(
        default = "default_grammar_preset",
        alias = "grammar_preset",
        alias = "grammar"
    )]
    pub grammar_preset_name: String,
    /// Optional editor font family.
    #[serde(default)]
    pub font_family: Option<String>,
    /// Optional editor font size in points.
    #[serde(default)]
    pub font_size: Option<u32>,
    /// Optional tab indentation width.
    #[serde(default)]
    pub tab_size: Option<u32>,
    /// Whether auto-save is enabled.
    #[serde(default)]
    pub auto_save: bool,
    /// Whether line numbers are displayed.
    #[serde(default)]
    pub show_line_numbers: bool,
    /// Extra user-configured key-value preferences.
    #[serde(default)]
    pub extra: HashMap<String, String>,
}

fn default_theme() -> String {
    "dark".to_string()
}

fn default_keymap() -> String {
    "default".to_string()
}

fn default_grammar_preset() -> String {
    "default".to_string()
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SETTINGS_SCHEMA_VERSION,
            theme: default_theme(),
            keymap: default_keymap(),
            grammar_preset_name: default_grammar_preset(),
            font_family: None,
            font_size: Some(13),
            tab_size: Some(4),
            auto_save: false,
            show_line_numbers: true,
            extra: HashMap::new(),
        }
    }
}

impl AppSettings {
    /// Creates a new AppSettings instance with default values at the current schema version.
    pub fn new() -> Self {
        Self::default()
    }

    /// Builder method to customize theme, keymap, and grammar preset name.
    pub fn with_presets(
        theme: impl Into<String>,
        keymap: impl Into<String>,
        grammar_preset: impl Into<String>,
    ) -> Self {
        Self {
            schema_version: CURRENT_SETTINGS_SCHEMA_VERSION,
            theme: theme.into(),
            keymap: keymap.into(),
            grammar_preset_name: grammar_preset.into(),
            ..Default::default()
        }
    }

    /// Serializes the settings to a compact JSON string.
    pub fn to_json(&self) -> Result<String, SessionError> {
        serde_json::to_string(self).map_err(SessionError::Json)
    }

    /// Serializes the settings to a pretty-printed JSON string.
    pub fn to_json_pretty(&self) -> Result<String, SessionError> {
        serde_json::to_string_pretty(self).map_err(SessionError::Json)
    }

    /// Deserializes and automatically migrates any supported JSON version to current.
    pub fn from_json(json_str: &str) -> Result<Self, SessionError> {
        migrate_settings(json_str)
    }

    /// Saves the settings document as pretty-printed JSON to the specified file path.
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), SessionError> {
        let json = self.to_json_pretty()?;
        std::fs::write(path, json).map_err(SessionError::Io)
    }

    /// Loads settings from a file path, automatically migrating if needed.
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, SessionError> {
        let content = std::fs::read_to_string(path).map_err(SessionError::Io)?;
        Self::from_json(&content)
    }

    /// Migrates an older JSON representation to the current schema version.
    pub fn migrate(old_json: &str) -> Result<Self, SessionError> {
        migrate_settings(old_json)
    }
}

/// Migrates an arbitrary JSON string into a current [`AppSettings`].
pub fn migrate_settings(old_json: &str) -> Result<AppSettings, SessionError> {
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
                                max_supported: CURRENT_SETTINGS_SCHEMA_VERSION,
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

            if version > CURRENT_SETTINGS_SCHEMA_VERSION {
                return Err(SessionError::UnsupportedVersion {
                    found: version,
                    max_supported: CURRENT_SETTINGS_SCHEMA_VERSION,
                });
            }

            if version == CURRENT_SETTINGS_SCHEMA_VERSION {
                let settings: AppSettings = serde_json::from_value(root)?;
                Ok(settings)
            } else {
                // v0 migration
                let theme = map
                    .get("theme")
                    .and_then(|v| v.as_str())
                    .unwrap_or("dark")
                    .to_string();

                let keymap = map
                    .get("keymap")
                    .or_else(|| map.get("keys"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("default")
                    .to_string();

                let grammar_preset_name = map
                    .get("grammar_preset_name")
                    .or_else(|| map.get("grammar_preset"))
                    .or_else(|| map.get("grammar"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("default")
                    .to_string();

                let font_family = map
                    .get("font_family")
                    .and_then(|v| v.as_str())
                    .map(ToString::to_string);

                let font_size = map
                    .get("font_size")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32);

                let tab_size = map
                    .get("tab_size")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32);

                let auto_save = map
                    .get("auto_save")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                let show_line_numbers = map
                    .get("show_line_numbers")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);

                let mut extra = HashMap::new();
                if let Some(serde_json::Value::Object(extra_map)) = map.get("extra") {
                    for (k, v) in extra_map {
                        if let Some(s) = v.as_str() {
                            extra.insert(k.clone(), s.to_string());
                        }
                    }
                }

                Ok(AppSettings {
                    schema_version: CURRENT_SETTINGS_SCHEMA_VERSION,
                    theme,
                    keymap,
                    grammar_preset_name,
                    font_family,
                    font_size,
                    tab_size,
                    auto_save,
                    show_line_numbers,
                    extra,
                })
            }
        }
        _ => Err(SessionError::CorruptInput(
            "Root JSON must be an object for AppSettings".to_string(),
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
    fn test_settings_builder_and_roundtrip_unit() {
        let s = AppSettings::with_presets("light", "default", "python");
        assert_eq!(s.schema_version, CURRENT_SETTINGS_SCHEMA_VERSION);
        assert_eq!(s.theme, "light");
        assert_eq!(s.keymap, "default");
        assert_eq!(s.grammar_preset_name, "python");
        let json = s.to_json().expect("to_json");
        let parsed = AppSettings::from_json(&json).expect("from_json");
        assert_eq!(s, parsed);
    }

    #[test]
    fn test_settings_v0_migration_unit() {
        let v0 = r#"{"theme": "dracula", "grammar": "markdown"}"#;
        let s = migrate_settings(v0).expect("migrate");
        assert_eq!(s.schema_version, CURRENT_SETTINGS_SCHEMA_VERSION);
        assert_eq!(s.theme, "dracula");
        assert_eq!(s.grammar_preset_name, "markdown");
    }

    #[test]
    fn test_settings_future_version_unit() {
        let v9 = r#"{"schema_version": 9, "theme": "dracula"}"#;
        assert!(migrate_settings(v9).is_err());
    }
}
