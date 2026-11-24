//! Session, settings, and report-export IPC commands (Wave 1d leaf).
//!
//! # Ownership
//!
//! This module is owned by Wave 1d. Wave 1R owns `commands/mod.rs` and the
//! `lib.rs` handler registration, so this file is intentionally **not**
//! referenced from anywhere yet; that is fine and the crate still builds.
//!
//! # Wiring (Wave 1R handoff)
//!
//! 1. Add to `apps/desktop/src-tauri/Cargo.toml`:
//!    `autocompare-sessions = { path = "../../../crates/sessions" }`.
//!    (Report rendering stays in-leaf: [`report_export`] builds the
//!    `html`/`json`/`md` bodies directly so no `autocompare-reports`
//!    dependency is needed. If 1R prefers the shared renderers, swap the
//!    builders for `autocompare_reports::{render_html, render_text, ...}`
//!    over the same inputs -- DTO shapes are unchanged.)
//! 2. Create `commands/mod.rs` with `pub mod session;` (and
//!    `pub mod media;`).
//! 3. Register `session_list`, `session_load`, `session_save`,
//!    `settings_get`, `settings_set`, and `report_export` in the `lib.rs`
//!    `generate_handler![...]` table.
//!
//! # Engine swap note (Wave 1R / later wave)
//!
//! The DTOs below are serde passthrough shapes of
//! `autocompare_sessions::{SessionDocument, SessionTab, AppSettings}`:
//! identical JSON field names, defaults, and aliases. Once the dependency
//! above lands, replace the local mirrors with
//! `pub use autocompare_sessions::{AppSettings, SessionDocument,
//! SessionTab};` and delete the local definitions -- handler bodies and
//! `ipc/session.ts` are unaffected.
//!
//! # Storage rule (locked)
//!
//! Everything persists under the system temp directory
//! (`<tmp>/autocompare-desktop/`, see [`app_data_dir`]) -- sessions in
//! `sessions/*.json`, settings in `settings.json`. Never under HOME
//! dotfiles. Report exports are likewise confined to the temp directory
//! (see [`confined_tmp_path`]). Writes are atomic (sibling `.tmp` file +
//! rename). Errors travel back as [`String`]; nothing here panics.

#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// Current session schema version (mirrors the engine constant).
pub const CURRENT_SESSION_SCHEMA_VERSION: u32 = 1;

/// Current settings schema version (mirrors the engine constant).
pub const CURRENT_SETTINGS_SCHEMA_VERSION: u32 = 1;

/// Counter for minted session ids (process-unique, never reused).
static NEXT_SESSION_ID: AtomicU64 = AtomicU64::new(1);

/// Default tab type when the descriptor omits it (mirrors the engine).
fn default_tab_type() -> String {
    "text".to_string()
}

/// Default theme (mirrors the engine default).
fn default_theme() -> String {
    "dark".to_string()
}

/// Default keymap (mirrors the engine default).
fn default_keymap() -> String {
    "default".to_string()
}

/// Default grammar preset name (mirrors the engine default).
fn default_grammar_preset() -> String {
    "default".to_string()
}

/// Default grammar presets: a single `"default"` entry. The engine ships
/// the full builtin suite; the full list arrives with the engine swap.
fn default_grammar_presets() -> Vec<GrammarPreset> {
    vec![GrammarPreset {
        name: "default".to_string(),
        extensions: Vec::new(),
        line_comment_prefix: None,
        block_comment_start: None,
        block_comment_end: None,
        ignore_case: false,
    }]
}

/// Compare-session tab descriptor. Serde passthrough shape of
/// `autocompare_sessions::SessionTab`: identical JSON field names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionTab {
    /// Unique tab identifier.
    pub id: String,
    /// Display title.
    pub title: String,
    /// Session kind (`"text"`, `"picture"`, `"audio"`, ...).
    #[serde(default = "default_tab_type")]
    pub session_type: String,
    /// Optional left path or URI.
    #[serde(default)]
    pub left_path: Option<String>,
    /// Optional right path or URI.
    #[serde(default)]
    pub right_path: Option<String>,
    /// Optional base/ancestor path for 3-way merges.
    #[serde(default)]
    pub base_path: Option<String>,
    /// Optional output path for merges.
    #[serde(default)]
    pub output_path: Option<String>,
    /// Whether the tab is read-only.
    #[serde(default)]
    pub read_only: bool,
    /// Whether the tab is locked against modification.
    #[serde(default)]
    pub locked: bool,
    /// Session-specific options.
    #[serde(default)]
    pub options: HashMap<String, String>,
}

/// Saved compare session. Serde passthrough shape of
/// `autocompare_sessions::SessionDocument`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionDocument {
    /// Schema version.
    pub schema_version: u32,
    /// Open tabs.
    pub tabs: Vec<SessionTab>,
    /// Zero-based index of the active tab.
    #[serde(default)]
    pub active_tab_index: usize,
    /// Optional user-assigned session name.
    #[serde(default)]
    pub name: Option<String>,
    /// Whether the session is locked against overwrite.
    #[serde(default)]
    pub locked: bool,
}

/// Grammar preset descriptor. Serde passthrough shape of
/// `autocompare_sessions::GrammarPreset`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrammarPreset {
    /// Preset name.
    pub name: String,
    /// File extensions without leading dot.
    #[serde(default)]
    pub extensions: Vec<String>,
    /// Single-line comment prefix.
    #[serde(default)]
    pub line_comment_prefix: Option<String>,
    /// Block comment start delimiter.
    #[serde(default)]
    pub block_comment_start: Option<String>,
    /// Block comment end delimiter.
    #[serde(default)]
    pub block_comment_end: Option<String>,
    /// Whether case is insignificant under this preset.
    #[serde(default)]
    pub ignore_case: bool,
}

/// Application settings. Serde passthrough shape of
/// `autocompare_sessions::AppSettings`: identical JSON field names,
/// defaults, and `grammar_preset`/`grammar` aliases.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppSettings {
    /// Schema version.
    pub schema_version: u32,
    /// Color theme name.
    #[serde(default = "default_theme")]
    pub theme: String,
    /// Keymap preset name.
    #[serde(default = "default_keymap")]
    pub keymap: String,
    /// Grammar preset name.
    #[serde(
        default = "default_grammar_preset",
        alias = "grammar_preset",
        alias = "grammar"
    )]
    pub grammar_preset_name: String,
    /// Configured grammar presets.
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
    /// Whether settings are locked against overwrite.
    #[serde(default)]
    pub locked: bool,
    /// Extra user-configured key-value preferences.
    #[serde(default)]
    pub extra: HashMap<String, String>,
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

/// One saved session for [`session_list`]. Mirrors `ipc/session.ts`
/// `SessionEntry`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionEntry {
    /// Session id (file stem under `sessions/`).
    pub id: String,
    /// User-assigned session name, if any.
    pub name: Option<String>,
    /// Number of tabs.
    pub tabs: u64,
    /// Whether the session is locked.
    pub locked: bool,
}

/// Report formats accepted by [`report_export`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReportFormat {
    Html,
    Json,
    Md,
}

impl ReportFormat {
    fn parse(raw: &str) -> Result<Self, String> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "html" | "htm" => Ok(Self::Html),
            "json" => Ok(Self::Json),
            "md" | "markdown" => Ok(Self::Md),
            _ => Err(format!(
                "unknown report format '{raw}'; expected html, json, or md"
            )),
        }
    }
}

/// Root of all desktop persistence: `<tmp>/autocompare-desktop`.
/// Never under HOME dotfiles.
fn app_data_dir() -> PathBuf {
    std::env::temp_dir().join("autocompare-desktop")
}

/// Directory holding `*.json` session documents.
fn sessions_dir() -> PathBuf {
    app_data_dir().join("sessions")
}

/// Path of the persisted settings document.
fn settings_path() -> PathBuf {
    app_data_dir().join("settings.json")
}

/// Session ids double as file stems, so they are restricted to a safe
/// charset: no separators, no parent traversal.
fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id != "."
        && id != ".."
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
}

/// Mint a process-unique session id.
fn mint_session_id() -> String {
    let counter = NEXT_SESSION_ID.fetch_add(1, Ordering::Relaxed);
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("sess-{millis}-{counter}")
}

/// Atomic write (sibling `.tmp` file + rename) so a crash can never
/// leave a torn document behind.
fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut tmp: std::ffi::OsString = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp_path = PathBuf::from(tmp);
    if let Err(source) = std::fs::write(&tmp_path, bytes) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(format!("writing {} failed: {source}", path.display()));
    }
    if let Err(source) = std::fs::rename(&tmp_path, path) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(format!("writing {} failed: {source}", path.display()));
    }
    Ok(())
}

/// Resolve `raw` to a path confined to the system temp directory:
/// relative paths resolve under it, absolute paths must already be
/// inside it. Anything else is an error.
fn confined_tmp_path(raw: &str) -> Result<PathBuf, String> {
    if raw.is_empty() {
        return Err("out_path must not be empty".to_string());
    }
    let base = std::env::temp_dir();
    let candidate = Path::new(raw);
    let joined = if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        base.join(candidate)
    };
    if joined.starts_with(&base) {
        Ok(joined)
    } else {
        Err("out_path must stay under the system temp directory".to_string())
    }
}

/// Escape HTML special characters (`&`, `<`, `>`, `"`, `'`).
fn escape_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            other => out.push(other),
        }
    }
    out
}

/// Lists saved sessions (newest... in directory order). Unreadable files
/// are skipped; a missing store is an empty list, never an error.
#[tauri::command]
pub fn session_list() -> Vec<SessionEntry> {
    let mut entries = Vec::new();
    let dir = match std::fs::read_dir(sessions_dir()) {
        Ok(dir) => dir,
        Err(_) => return entries,
    };
    for entry in dir.flatten() {
        let path = entry.path();
        let is_json = path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("json"));
        if !is_json {
            continue;
        }
        let id = match path.file_stem().and_then(|stem| stem.to_str()) {
            Some(id) => id.to_string(),
            None => continue,
        };
        if !valid_id(&id) {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(doc) = serde_json::from_str::<SessionDocument>(&content) else {
            continue;
        };
        entries.push(SessionEntry {
            id,
            name: doc.name.clone(),
            tabs: doc.tabs.len() as u64,
            locked: doc.locked,
        });
    }
    entries.sort_by(|a, b| a.id.cmp(&b.id));
    entries
}

/// Loads a saved session by id. Dangling `active_tab_index` values are
/// clamped to 0, mirroring the engine loader.
#[tauri::command]
pub fn session_load(id: String) -> Result<SessionDocument, String> {
    if !valid_id(&id) {
        return Err(format!("unknown session id '{id}'"));
    }
    let path = sessions_dir().join(format!("{id}.json"));
    let content = match std::fs::read_to_string(&path) {
        Ok(content) => content,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
            return Err(format!("session '{id}' not found"));
        }
        Err(source) => return Err(format!("reading session '{id}' failed: {source}")),
    };
    let mut doc: SessionDocument = serde_json::from_str(&content)
        .map_err(|source| format!("session '{id}' is corrupt: {source}"))?;
    if doc.schema_version > CURRENT_SESSION_SCHEMA_VERSION {
        return Err(format!(
            "session '{id}' uses unsupported schema version {}",
            doc.schema_version
        ));
    }
    if doc.active_tab_index >= doc.tabs.len() {
        doc.active_tab_index = 0;
    }
    Ok(doc)
}

/// Validates a session descriptor for saving.
fn validate_descriptor(doc: &SessionDocument) -> Result<(), String> {
    if doc.schema_version > CURRENT_SESSION_SCHEMA_VERSION {
        return Err(format!(
            "unsupported session schema version {}",
            doc.schema_version
        ));
    }
    if !doc.tabs.is_empty() && doc.active_tab_index >= doc.tabs.len() {
        return Err("active_tab_index is out of range for the tab list".to_string());
    }
    for tab in &doc.tabs {
        if tab.id.is_empty() {
            return Err("session tab id must not be empty".to_string());
        }
    }
    Ok(())
}

/// Saves a session descriptor under a minted id and returns the id.
/// Locked descriptors are refused unless the lock is cleared first.
#[tauri::command]
pub fn session_save(descriptor: SessionDocument) -> Result<String, String> {
    validate_descriptor(&descriptor)?;
    if descriptor.locked {
        return Err("session is locked against overwrite".to_string());
    }
    let dir = sessions_dir();
    std::fs::create_dir_all(&dir)
        .map_err(|source| format!("creating {} failed: {source}", dir.display()))?;
    let id = mint_session_id();
    let mut stored = descriptor;
    stored.schema_version = CURRENT_SESSION_SCHEMA_VERSION;
    let json = serde_json::to_string_pretty(&stored)
        .map_err(|source| format!("encoding session failed: {source}"))?;
    atomic_write(&dir.join(format!("{id}.json")), json.as_bytes())?;
    Ok(id)
}

/// Loads persisted settings, or defaults when nothing is stored yet.
/// A corrupt store is an error, never a silent default.
#[tauri::command]
pub fn settings_get() -> Result<AppSettings, String> {
    let path = settings_path();
    let content = match std::fs::read_to_string(&path) {
        Ok(content) => content,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
            return Ok(AppSettings::default());
        }
        Err(source) => {
            return Err(format!("reading settings failed: {source}"));
        }
    };
    let settings: AppSettings = serde_json::from_str(&content)
        .map_err(|source| format!("settings are corrupt: {source}"))?;
    if settings.schema_version > CURRENT_SETTINGS_SCHEMA_VERSION {
        return Err(format!(
            "unsupported settings schema version {}",
            settings.schema_version
        ));
    }
    Ok(settings)
}

/// Applies a JSON settings patch onto the stored settings. Unknown keys
/// are rejected; only documented value types are accepted.
#[tauri::command]
pub fn settings_set(patch: serde_json::Value) -> Result<AppSettings, String> {
    let fields = match patch {
        serde_json::Value::Object(fields) => fields,
        _ => return Err("settings patch must be a JSON object".to_string()),
    };
    let current = settings_get()?;
    if current.locked {
        return Err("settings are locked against modification".to_string());
    }
    let mut next = current;
    for (key, value) in &fields {
        apply_setting(&mut next, key, value)?;
    }
    let json = serde_json::to_string_pretty(&next)
        .map_err(|source| format!("encoding settings failed: {source}"))?;
    let path = settings_path();
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)
            .map_err(|source| format!("creating {} failed: {source}", parent.display()))?;
    }
    atomic_write(&path, json.as_bytes())?;
    Ok(next)
}

/// Applies one validated patch field. Unknown keys and mistyped values
/// are errors.
fn apply_setting(
    settings: &mut AppSettings,
    key: &str,
    value: &serde_json::Value,
) -> Result<(), String> {
    let type_error = |expected: &str| format!("settings key '{key}' must be {expected}");
    match key {
        "theme" => {
            let text = value.as_str().ok_or_else(|| type_error("a string"))?;
            settings.theme = text.to_string();
        }
        "keymap" => {
            let text = value.as_str().ok_or_else(|| type_error("a string"))?;
            settings.keymap = text.to_string();
        }
        "grammar_preset_name" | "grammar_preset" | "grammar" => {
            let text = value.as_str().ok_or_else(|| type_error("a string"))?;
            settings.grammar_preset_name = text.to_string();
        }
        "font_family" => {
            if value.is_null() {
                settings.font_family = None;
            } else {
                let text = value
                    .as_str()
                    .ok_or_else(|| type_error("a string or null"))?;
                settings.font_family = Some(text.to_string());
            }
        }
        "font_size" => {
            if value.is_null() {
                settings.font_size = None;
            } else {
                let num = value
                    .as_u64()
                    .ok_or_else(|| type_error("a number or null"))?;
                settings.font_size =
                    Some(u32::try_from(num).map_err(|_| type_error("a number or null"))?);
            }
        }
        "tab_size" => {
            if value.is_null() {
                settings.tab_size = None;
            } else {
                let num = value
                    .as_u64()
                    .ok_or_else(|| type_error("a number or null"))?;
                settings.tab_size =
                    Some(u32::try_from(num).map_err(|_| type_error("a number or null"))?);
            }
        }
        "auto_save" => {
            settings.auto_save = value.as_bool().ok_or_else(|| type_error("a boolean"))?;
        }
        "show_line_numbers" => {
            settings.show_line_numbers = value.as_bool().ok_or_else(|| type_error("a boolean"))?;
        }
        "locked" => {
            settings.locked = value.as_bool().ok_or_else(|| type_error("a boolean"))?;
        }
        "extra" => {
            let fields = value.as_object().ok_or_else(|| type_error("an object"))?;
            let mut extra = HashMap::new();
            for (k, v) in fields {
                if let Some(text) = v.as_str() {
                    extra.insert(k.clone(), text.to_string());
                } else if let Some(num) = v.as_i64() {
                    extra.insert(k.clone(), num.to_string());
                } else if let Some(num) = v.as_u64() {
                    extra.insert(k.clone(), num.to_string());
                } else if let Some(flag) = v.as_bool() {
                    extra.insert(k.clone(), flag.to_string());
                } else {
                    return Err(format!("settings key 'extra.{k}' must be a scalar"));
                }
            }
            settings.extra = extra;
        }
        "schema_version" => {
            let version = value.as_u64().ok_or_else(|| type_error("a number"))?;
            if version != u64::from(CURRENT_SETTINGS_SCHEMA_VERSION) {
                return Err(format!("unsupported settings schema version {version}"));
            }
        }
        _ => {
            return Err(format!("unknown settings key: '{key}'"));
        }
    }
    Ok(())
}

/// Renders a session descriptor to `html`, `json`, or `md` and writes it
/// to `out_path` (confined to the temp directory). Returns the written
/// path. The file is verified non-empty after the write.
#[tauri::command]
pub fn report_export(
    session: SessionDocument,
    format: String,
    out_path: String,
) -> Result<String, String> {
    validate_descriptor(&session)?;
    let rendered = match ReportFormat::parse(&format)? {
        ReportFormat::Html => render_html_report(&session),
        ReportFormat::Json => serde_json::to_string_pretty(&session)
            .map_err(|source| format!("encoding report failed: {source}"))?,
        ReportFormat::Md => render_markdown_report(&session),
    };
    if rendered.is_empty() {
        return Err("rendered report is empty".to_string());
    }
    let path = confined_tmp_path(&out_path)?;
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)
            .map_err(|source| format!("creating {} failed: {source}", parent.display()))?;
    }
    atomic_write(&path, rendered.as_bytes())?;
    let non_empty = std::fs::metadata(&path)
        .map(|meta| meta.len() > 0)
        .unwrap_or(false);
    if !non_empty {
        return Err(format!("report at {} is empty", path.display()));
    }
    Ok(path.to_string_lossy().to_string())
}

/// Markdown report body for a session descriptor.
fn render_markdown_report(session: &SessionDocument) -> String {
    let title = session.name.as_deref().unwrap_or("Session report");
    let mut out = String::new();
    out.push_str("# ");
    out.push_str(title);
    out.push_str("\n\n");
    out.push_str(&format!("Tabs: {}\n\n", session.tabs.len()));
    for (index, tab) in session.tabs.iter().enumerate() {
        out.push_str(&format!(
            "## {}. {} (`{}`)\n\n",
            index + 1,
            tab.title,
            tab.session_type
        ));
        if let Some(left) = tab.left_path.as_deref() {
            out.push_str(&format!("- Left: `{left}`\n"));
        }
        if let Some(right) = tab.right_path.as_deref() {
            out.push_str(&format!("- Right: `{right}`\n"));
        }
        if let Some(base) = tab.base_path.as_deref() {
            out.push_str(&format!("- Base: `{base}`\n"));
        }
        out.push('\n');
    }
    out
}

/// HTML report body for a session descriptor. All interpolated text is
/// escaped.
fn render_html_report(session: &SessionDocument) -> String {
    let title = session.name.as_deref().unwrap_or("Session report");
    let mut out = String::new();
    out.push_str("<!DOCTYPE html>\n<html>\n<head>\n<meta charset=\"utf-8\">\n<title>");
    out.push_str(&escape_html(title));
    out.push_str("</title>\n</head>\n<body>\n<h1>");
    out.push_str(&escape_html(title));
    out.push_str("</h1>\n<ul>\n");
    for tab in &session.tabs {
        out.push_str("<li><strong>");
        out.push_str(&escape_html(&tab.title));
        out.push_str("</strong> (");
        out.push_str(&escape_html(&tab.session_type));
        out.push(')');
        if let Some(left) = tab.left_path.as_deref() {
            out.push_str(" left=");
            out.push_str(&escape_html(left));
        }
        if let Some(right) = tab.right_path.as_deref() {
            out.push_str(" right=");
            out.push_str(&escape_html(right));
        }
        out.push_str("</li>\n");
    }
    out.push_str("</ul>\n</body>\n</html>\n");
    out
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

    fn sample_tab(id: &str) -> SessionTab {
        SessionTab {
            id: id.to_string(),
            title: format!("Tab {id}"),
            session_type: "text".to_string(),
            left_path: Some("/tmp/left.txt".to_string()),
            right_path: Some("/tmp/right.txt".to_string()),
            base_path: None,
            output_path: None,
            read_only: false,
            locked: false,
            options: HashMap::new(),
        }
    }

    fn sample_doc() -> SessionDocument {
        SessionDocument {
            schema_version: CURRENT_SESSION_SCHEMA_VERSION,
            tabs: vec![sample_tab("tab-1"), sample_tab("tab-2")],
            active_tab_index: 1,
            name: Some("wave-1d <report> & check".to_string()),
            locked: false,
        }
    }

    fn unique_path(name: &str) -> PathBuf {
        let n = NEXT_SESSION_ID.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!("wave1d-session-tests-{n}-{name}"))
    }

    #[test]
    fn descriptor_round_trip() {
        let doc = sample_doc();
        let json = serde_json::to_string(&doc).expect("serialize");
        assert!(json.contains("session_type"));
        assert!(json.contains("active_tab_index"));
        let back: SessionDocument = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(doc, back);
    }

    #[test]
    fn save_load_list_round_trip() {
        let doc = sample_doc();
        let id = session_save(doc.clone()).expect("save");
        assert!(valid_id(&id));
        let loaded = session_load(id.clone()).expect("load");
        assert_eq!(loaded.tabs, doc.tabs);
        assert_eq!(loaded.name, doc.name);
        let entries = session_list();
        assert!(entries.iter().any(|e| e.id == id && e.tabs == 2));
    }

    #[test]
    fn load_missing_session_errors() {
        let err = session_load("sess-does-not-exist-1d".to_string()).expect_err("must fail");
        assert!(err.contains("not found"), "unexpected: {err}");
    }

    #[test]
    fn load_rejects_bad_ids() {
        for bad in ["", "../evil", "a/b", "a\\b", "..", "."] {
            assert!(session_load(bad.to_string()).is_err(), "id: {bad}");
        }
    }

    #[test]
    fn save_rejects_dangling_active_index() {
        let mut doc = sample_doc();
        doc.active_tab_index = 99;
        let err = session_save(doc).expect_err("must fail");
        assert!(err.contains("active_tab_index"), "unexpected: {err}");
    }

    #[test]
    fn save_rejects_locked_descriptor() {
        let mut doc = sample_doc();
        doc.locked = true;
        let err = session_save(doc).expect_err("must fail");
        assert!(err.contains("locked"), "unexpected: {err}");
    }

    #[test]
    fn save_rejects_future_schema() {
        let mut doc = sample_doc();
        doc.schema_version = CURRENT_SESSION_SCHEMA_VERSION + 1;
        assert!(session_save(doc).is_err());
    }

    #[test]
    fn settings_round_trip() {
        let settings = settings_get().expect("get");
        let json = serde_json::to_string(&settings).expect("serialize");
        let back: AppSettings = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(settings.theme, back.theme);
        assert_eq!(settings.keymap, back.keymap);
    }

    #[test]
    fn settings_rejects_unknown_keys() {
        let patch = serde_json::json!({"no_such_key": true, "theme": "light"});
        let err = settings_set(patch).expect_err("unknown key must fail");
        assert!(err.contains("unknown settings key"), "unexpected: {err}");
    }

    #[test]
    fn settings_rejects_mistyped_values() {
        let patch = serde_json::json!({"theme": 42});
        assert!(settings_set(patch).is_err());
        let patch = serde_json::json!({"auto_save": "yes"});
        assert!(settings_set(patch).is_err());
        let patch = serde_json::json!({"extra": [1, 2]});
        assert!(settings_set(patch).is_err());
        let patch = serde_json::json!("just a string");
        assert!(settings_set(patch).is_err());
    }

    #[test]
    fn settings_set_applies_and_persists() {
        let patch = serde_json::json!({
            "theme": "wave1d-test-theme",
            "tab_size": 2,
            "extra": {"suite": "1d"},
        });
        let updated = settings_set(patch).expect("set");
        assert_eq!(updated.theme, "wave1d-test-theme");
        assert_eq!(updated.tab_size, Some(2));
        assert_eq!(updated.extra.get("suite").map(String::as_str), Some("1d"));
        let reloaded = settings_get().expect("get");
        assert_eq!(reloaded.theme, "wave1d-test-theme");

        // Restore defaults so later waves see a clean store.
        let restore = serde_json::json!({
            "theme": "dark",
            "tab_size": 4,
            "extra": {},
        });
        let restored = settings_set(restore).expect("restore");
        assert_eq!(restored.theme, "dark");
    }

    #[test]
    fn storage_lives_under_tmp_app_data() {
        let base = std::env::temp_dir();
        assert!(app_data_dir().starts_with(&base));
        assert!(sessions_dir().starts_with(&base));
        assert!(settings_path().starts_with(&base));
        assert!(!app_data_dir().to_string_lossy().contains(".autocompare"));
    }

    #[test]
    fn report_export_writes_non_empty_tmp_files() {
        let doc = sample_doc();
        for format in ["html", "json", "md"] {
            let out = unique_path(&format!("report-1d.{format}"));
            let written = report_export(
                doc.clone(),
                format.to_string(),
                out.to_string_lossy().to_string(),
            )
            .expect("export");
            let content = std::fs::read_to_string(&written).expect("read");
            assert!(!content.is_empty(), "format: {format}");
            if format == "html" {
                assert!(content.contains("&lt;report&gt;"));
                assert!(!content.contains("<report>"));
            }
            if format == "md" {
                assert!(content.contains("# wave-1d <report> & check"));
            }
            if format == "json" {
                let parsed: SessionDocument = serde_json::from_str(&content).expect("parse");
                assert_eq!(parsed.tabs.len(), 2);
            }
            let _ = std::fs::remove_file(&written);
        }
    }

    #[test]
    fn report_export_rejects_unknown_format_and_escaping_paths() {
        let doc = sample_doc();
        let out = unique_path("report-1d.txt");
        let err = report_export(
            doc.clone(),
            "pdf".to_string(),
            out.to_string_lossy().to_string(),
        )
        .expect_err("unknown format must fail");
        assert!(err.contains("unknown report format"), "unexpected: {err}");
        let err = report_export(doc, "html".to_string(), "/etc/wave1d-evil.html".to_string())
            .expect_err("escaping tmp must fail");
        assert!(err.contains("temp directory"), "unexpected: {err}");
    }
}
