use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::error::SessionError;

/// The current schema version for AppSettings.
pub const CURRENT_SETTINGS_SCHEMA_VERSION: u32 = 1;

/// Syntax grammar preset defining comment syntax, file associations, and case sensitivity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrammarPreset {
    /// Name of the grammar preset (e.g. "rust", "python", "c_cpp", "html_xml", "sql", "default").
    pub name: String,
    /// File extensions associated with this grammar preset (without leading dot).
    #[serde(default)]
    pub extensions: Vec<String>,
    /// Single-line comment prefix (e.g. "//", "#", "--").
    #[serde(default)]
    pub line_comment_prefix: Option<String>,
    /// Multi-line block comment start delimiter (e.g. "/*", "<!--").
    #[serde(default)]
    pub block_comment_start: Option<String>,
    /// Multi-line block comment end delimiter (e.g. "*/", "-->").
    #[serde(default)]
    pub block_comment_end: Option<String>,
    /// Whether case differences are considered unimportant under this grammar.
    #[serde(default)]
    pub ignore_case: bool,
}

impl GrammarPreset {
    /// Creates a new custom grammar preset with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            extensions: Vec::new(),
            line_comment_prefix: None,
            block_comment_start: None,
            block_comment_end: None,
            ignore_case: false,
        }
    }

    /// Builder method to associate file extensions.
    pub fn with_extensions(mut self, extensions: Vec<String>) -> Self {
        self.extensions = extensions;
        self
    }

    /// Builder method to set single-line comment prefix.
    pub fn with_line_comment(mut self, prefix: impl Into<String>) -> Self {
        self.line_comment_prefix = Some(prefix.into());
        self
    }

    /// Builder method to set multi-line block comment delimiters.
    pub fn with_block_comment(mut self, start: impl Into<String>, end: impl Into<String>) -> Self {
        self.block_comment_start = Some(start.into());
        self.block_comment_end = Some(end.into());
        self
    }

    /// Builder method to set case insensitivity.
    pub fn with_ignore_case(mut self, ignore: bool) -> Self {
        self.ignore_case = ignore;
        self
    }

    /// Default generic grammar preset with no special comment syntax.
    pub fn default_preset() -> Self {
        Self {
            name: "default".to_string(),
            extensions: Vec::new(),
            line_comment_prefix: None,
            block_comment_start: None,
            block_comment_end: None,
            ignore_case: false,
        }
    }

    /// C/C++/Java/Rust/JavaScript/Go style comments (`//` and `/* ... */`).
    pub fn c_style(name: impl Into<String>, extensions: Vec<String>) -> Self {
        Self {
            name: name.into(),
            extensions,
            line_comment_prefix: Some("//".to_string()),
            block_comment_start: Some("/*".to_string()),
            block_comment_end: Some("*/".to_string()),
            ignore_case: false,
        }
    }

    /// Python/Shell/Ruby/YAML style line comments (`#`).
    pub fn hash_style(name: impl Into<String>, extensions: Vec<String>) -> Self {
        Self {
            name: name.into(),
            extensions,
            line_comment_prefix: Some("#".to_string()),
            block_comment_start: None,
            block_comment_end: None,
            ignore_case: false,
        }
    }

    /// SQL style comments (`--` and `/* ... */`).
    pub fn sql_style(name: impl Into<String>, extensions: Vec<String>) -> Self {
        Self {
            name: name.into(),
            extensions,
            line_comment_prefix: Some("--".to_string()),
            block_comment_start: Some("/*".to_string()),
            block_comment_end: Some("*/".to_string()),
            ignore_case: true,
        }
    }

    /// HTML / XML style comments (`<!-- ... -->`).
    pub fn xml_style(name: impl Into<String>, extensions: Vec<String>) -> Self {
        Self {
            name: name.into(),
            extensions,
            line_comment_prefix: None,
            block_comment_start: Some("<!--".to_string()),
            block_comment_end: Some("-->".to_string()),
            ignore_case: false,
        }
    }

    /// Checks whether this grammar preset handles the given file extension.
    pub fn matches_extension(&self, ext: &str) -> bool {
        let needle = ext.trim_start_matches('.').to_ascii_lowercase();
        self.extensions
            .iter()
            .any(|e| e.to_ascii_lowercase() == needle)
    }

    /// Checks whether this grammar preset handles the file at the given path.
    pub fn matches_path<P: AsRef<Path>>(&self, path: P) -> bool {
        path.as_ref()
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| self.matches_extension(ext))
    }

    /// Returns true if the given line (trimmed of leading whitespace) begins with a line comment prefix.
    pub fn is_comment_line(&self, line: &str) -> bool {
        if let Some(ref prefix) = self.line_comment_prefix {
            line.trim_start().starts_with(prefix)
        } else {
            false
        }
    }

    /// Strips any trailing line comment from a line of text, if this preset defines a line comment prefix.
    pub fn strip_line_comment<'a>(&self, line: &'a str) -> &'a str {
        if let Some(ref prefix) = self.line_comment_prefix {
            line.split(prefix.as_str()).next().unwrap_or(line)
        } else {
            line
        }
    }

    /// Standard suite of built-in grammar presets for common languages.
    pub fn builtin_presets() -> Vec<Self> {
        vec![
            Self::default_preset(),
            Self::c_style(
                "c_cpp",
                vec![
                    "c".to_string(),
                    "cpp".to_string(),
                    "cc".to_string(),
                    "h".to_string(),
                    "hpp".to_string(),
                    "cxx".to_string(),
                    "hxx".to_string(),
                ],
            ),
            Self::c_style("rust", vec!["rs".to_string()]),
            Self::c_style(
                "javascript",
                vec![
                    "js".to_string(),
                    "jsx".to_string(),
                    "ts".to_string(),
                    "tsx".to_string(),
                    "mjs".to_string(),
                    "cjs".to_string(),
                ],
            ),
            Self::c_style("java", vec!["java".to_string()]),
            Self::c_style("csharp", vec!["cs".to_string()]),
            Self::c_style("go", vec!["go".to_string()]),
            Self::c_style("swift", vec!["swift".to_string()]),
            Self::c_style("kotlin", vec!["kt".to_string(), "kts".to_string()]),
            Self::c_style("php", vec!["php".to_string(), "phtml".to_string()]),
            Self::hash_style(
                "python",
                vec!["py".to_string(), "pyw".to_string(), "pyi".to_string()],
            ),
            Self::hash_style(
                "shell",
                vec![
                    "sh".to_string(),
                    "bash".to_string(),
                    "zsh".to_string(),
                    "fish".to_string(),
                ],
            ),
            Self::hash_style("ruby", vec!["rb".to_string()]),
            Self::hash_style("yaml", vec!["yaml".to_string(), "yml".to_string()]),
            Self::hash_style("toml", vec!["toml".to_string()]),
            Self::sql_style("sql", vec!["sql".to_string()]),
            Self::xml_style(
                "html_xml",
                vec![
                    "html".to_string(),
                    "htm".to_string(),
                    "xml".to_string(),
                    "svg".to_string(),
                    "xhtml".to_string(),
                ],
            ),
            Self::c_style(
                "css",
                vec!["css".to_string(), "scss".to_string(), "less".to_string()],
            ),
            Self::default_preset().with_extensions(vec!["json".to_string()]),
            Self::hash_style("markdown", vec!["md".to_string(), "markdown".to_string()]),
            Self::default_preset().with_extensions(vec!["diff".to_string(), "patch".to_string()]),
        ]
    }
}

/// Returns the default list of grammar presets.
pub fn default_grammar_presets() -> Vec<GrammarPreset> {
    GrammarPreset::builtin_presets()
}

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
    /// Configured syntax grammar presets for Beyond Compare file format parity.
    #[serde(default = "default_grammar_presets")]
    pub grammar_presets: Vec<GrammarPreset>,
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
    /// Whether this settings document is locked against accidental modification or overwrite.
    #[serde(default)]
    pub locked: bool,
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
            grammar_presets: default_grammar_presets(),
            font_family: None,
            font_size: Some(13),
            tab_size: Some(4),
            auto_save: false,
            show_line_numbers: true,
            locked: false,
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
            grammar_presets: default_grammar_presets(),
            ..Default::default()
        }
    }

    /// Look up a grammar preset by name.
    pub fn grammar_preset(&self, name: &str) -> Option<&GrammarPreset> {
        self.grammar_presets.iter().find(|p| p.name == name)
    }

    /// Look up a grammar preset matching a file extension (case-insensitive).
    pub fn find_preset_for_extension(&self, ext: &str) -> Option<&GrammarPreset> {
        self.grammar_presets
            .iter()
            .find(|p| p.matches_extension(ext))
    }

    /// Look up a grammar preset matching a file path.
    pub fn find_preset_for_path<P: AsRef<Path>>(&self, path: P) -> Option<&GrammarPreset> {
        self.grammar_presets.iter().find(|p| p.matches_path(&path))
    }

    /// Adds or updates a grammar preset.
    pub fn add_grammar_preset(&mut self, preset: GrammarPreset) {
        if let Some(pos) = self
            .grammar_presets
            .iter()
            .position(|p| p.name == preset.name)
        {
            if let Some(slot) = self.grammar_presets.get_mut(pos) {
                *slot = preset;
            }
        } else {
            self.grammar_presets.push(preset);
        }
    }

    /// Builder method to override the grammar presets list.
    pub fn with_grammar_presets(mut self, presets: Vec<GrammarPreset>) -> Self {
        self.grammar_presets = presets;
        self
    }

    /// Returns true if the settings document is locked.
    pub fn is_locked(&self) -> bool {
        self.locked
    }

    /// Sets whether the settings document is locked.
    pub fn set_locked(&mut self, locked: bool) {
        self.locked = locked;
    }

    /// Builder method to set whether the settings document is locked.
    pub fn with_locked(mut self, locked: bool) -> Self {
        self.locked = locked;
        self
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
    ///
    /// If the settings document is locked (`self.locked == true`), returns an I/O
    /// PermissionDenied error. To bypass the lock check, use [`save_to_file_force`].
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), SessionError> {
        if self.locked {
            return Err(SessionError::Io(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "Cannot overwrite locked app settings document; unlock or use save_to_file_force",
            )));
        }
        self.save_to_file_force(path)
    }

    /// Saves the settings document unconditionally, bypassing any lock check.
    pub fn save_to_file_force<P: AsRef<Path>>(&self, path: P) -> Result<(), SessionError> {
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
                    .or_else(|| map.get("color_theme"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("dark")
                    .to_string();

                let keymap = map
                    .get("keymap")
                    .or_else(|| map.get("keys"))
                    .or_else(|| map.get("keybindings"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("default")
                    .to_string();

                let grammar_preset_name = map
                    .get("grammar_preset_name")
                    .or_else(|| map.get("grammar_preset"))
                    .or_else(|| map.get("grammar"))
                    .or_else(|| map.get("syntax"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("default")
                    .to_string();

                let grammar_presets = if let Some(serde_json::Value::Array(arr)) =
                    map.get("grammar_presets").or_else(|| map.get("presets"))
                {
                    let mut presets = Vec::new();
                    for item in arr {
                        if let Ok(preset) = serde_json::from_value::<GrammarPreset>(item.clone()) {
                            presets.push(preset);
                        }
                    }
                    if presets.is_empty() {
                        default_grammar_presets()
                    } else {
                        presets
                    }
                } else {
                    default_grammar_presets()
                };

                let font_family = map
                    .get("font_family")
                    .or_else(|| map.get("font"))
                    .and_then(|v| v.as_str())
                    .map(ToString::to_string);

                let font_size = map
                    .get("font_size")
                    .or_else(|| map.get("size"))
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32);

                let tab_size = map
                    .get("tab_size")
                    .or_else(|| map.get("tab_width"))
                    .or_else(|| map.get("indent_size"))
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32);

                let auto_save = map
                    .get("auto_save")
                    .or_else(|| map.get("autosave"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                let show_line_numbers = map
                    .get("show_line_numbers")
                    .or_else(|| map.get("line_numbers"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);

                let locked = map
                    .get("locked")
                    .or_else(|| map.get("is_locked"))
                    .or_else(|| map.get("read_only"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                let mut extra = HashMap::new();
                let raw_extra = map
                    .get("extra")
                    .or_else(|| map.get("custom"))
                    .or_else(|| map.get("preferences"));

                if let Some(serde_json::Value::Object(extra_map)) = raw_extra {
                    for (k, v) in extra_map {
                        if let Some(s) = v.as_str() {
                            extra.insert(k.clone(), s.to_string());
                        } else if let Some(n) = v.as_i64() {
                            extra.insert(k.clone(), n.to_string());
                        } else if let Some(b) = v.as_bool() {
                            extra.insert(k.clone(), b.to_string());
                        }
                    }
                }

                Ok(AppSettings {
                    schema_version: CURRENT_SETTINGS_SCHEMA_VERSION,
                    theme,
                    keymap,
                    grammar_preset_name,
                    grammar_presets,
                    font_family,
                    font_size,
                    tab_size,
                    auto_save,
                    show_line_numbers,
                    locked,
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
        let s = AppSettings::with_presets("light", "default", "python").with_locked(true);
        assert_eq!(s.schema_version, CURRENT_SETTINGS_SCHEMA_VERSION);
        assert_eq!(s.theme, "light");
        assert_eq!(s.keymap, "default");
        assert_eq!(s.grammar_preset_name, "python");
        assert!(s.is_locked());
        assert!(!s.grammar_presets.is_empty());
        let json = s.to_json().expect("to_json");
        let parsed = AppSettings::from_json(&json).expect("from_json");
        assert_eq!(s, parsed);
        assert!(parsed.is_locked());
    }

    #[test]
    fn test_grammar_preset_extension_lookup() {
        let s = AppSettings::new();
        let rs_preset = s.find_preset_for_extension("rs").expect("find rs preset");
        assert_eq!(rs_preset.name, "rust");
        assert_eq!(rs_preset.line_comment_prefix.as_deref(), Some("//"));
        assert!(rs_preset.matches_path("src/lib.rs"));

        let py_preset = s.find_preset_for_extension(".py").expect("find py preset");
        assert_eq!(py_preset.name, "python");
        assert_eq!(py_preset.line_comment_prefix.as_deref(), Some("#"));
        assert!(py_preset.matches_path(Path::new("scripts/run.py")));

        assert!(s.find_preset_for_extension("unknown_xyz").is_none());
        assert!(s.find_preset_for_path("data.unknown_xyz").is_none());
    }

    #[test]
    fn test_grammar_preset_comment_helpers() {
        let rust_preset = GrammarPreset::c_style("rust", vec!["rs".to_string()]);
        assert!(rust_preset.is_comment_line("   // this is a rust comment"));
        assert!(!rust_preset.is_comment_line("let x = 42; // inline comment"));
        assert_eq!(
            rust_preset.strip_line_comment("let x = 42; // inline"),
            "let x = 42; "
        );
        assert_eq!(
            rust_preset.strip_line_comment("no comments here"),
            "no comments here"
        );
    }

    #[test]
    fn test_custom_grammar_preset() {
        let mut s = AppSettings::new();
        let custom = GrammarPreset::new("lua")
            .with_extensions(vec!["lua".to_string()])
            .with_line_comment("--")
            .with_block_comment("--[[", "]]");
        s.add_grammar_preset(custom);

        let lua_preset = s.find_preset_for_extension("lua").expect("find lua");
        assert_eq!(lua_preset.name, "lua");
        assert_eq!(lua_preset.line_comment_prefix.as_deref(), Some("--"));
        assert_eq!(lua_preset.block_comment_start.as_deref(), Some("--[["));
    }

    #[test]
    fn test_settings_v0_migration_unit() {
        let v0 = r#"{"theme": "dracula", "grammar": "markdown", "is_locked": true}"#;
        let s = migrate_settings(v0).expect("migrate");
        assert_eq!(s.schema_version, CURRENT_SETTINGS_SCHEMA_VERSION);
        assert_eq!(s.theme, "dracula");
        assert_eq!(s.grammar_preset_name, "markdown");
        assert!(s.is_locked());
        assert!(!s.grammar_presets.is_empty());
    }

    #[test]
    fn test_settings_future_version_unit() {
        let v9 = r#"{"schema_version": 9, "theme": "dracula"}"#;
        assert!(migrate_settings(v9).is_err());
    }

    #[test]
    fn test_settings_locked_guard() {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join(format!("test_settings_locked_{}.json", std::process::id()));
        let mut settings = AppSettings::new();
        settings.set_locked(true);

        assert!(settings.save_to_file(&file_path).is_err());
        assert!(settings.save_to_file_force(&file_path).is_ok());

        let loaded = AppSettings::load_from_file(&file_path).expect("load locked settings");
        assert!(loaded.is_locked());
        let _ = std::fs::remove_file(file_path);
    }
}
