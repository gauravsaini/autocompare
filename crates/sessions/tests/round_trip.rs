#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use autocompare_sessions::{
    AppSettings, CURRENT_SESSION_SCHEMA_VERSION, CURRENT_SETTINGS_SCHEMA_VERSION,
    CURRENT_WORKSPACE_SCHEMA_VERSION, SessionConfig, SessionDocument, SessionTab, WindowLayout,
    WorkspaceConfig, WorkspaceDocument,
};
use std::collections::HashMap;

#[test]
fn test_session_document_round_trip() {
    let mut tab1 = SessionTab::new("tab-1", "Compare Cargo.toml")
        .with_paths("/left/Cargo.toml", "/right/Cargo.toml")
        .with_session_type("text");
    tab1.read_only = true;
    tab1.options
        .insert("ignore_whitespace".to_string(), "true".to_string());

    let tab2 = SessionTab::new("tab-2", "Compare Src Tree")
        .with_paths("/left/src", "/right/src")
        .with_session_type("folder");

    let mut doc = SessionDocument::with_tabs(vec![tab1, tab2]);
    doc.active_tab_index = 1;
    doc.name = Some("Workspace Session Alpha".to_string());

    // JSON compact round-trip
    let json_compact = doc.to_json().expect("serialize compact");
    let loaded_compact = SessionDocument::from_json(&json_compact).expect("deserialize compact");
    assert_eq!(doc, loaded_compact);
    assert_eq!(
        loaded_compact.schema_version,
        CURRENT_SESSION_SCHEMA_VERSION
    );

    // JSON pretty round-trip
    let json_pretty = doc.to_json_pretty().expect("serialize pretty");
    let loaded_pretty = SessionDocument::from_json(&json_pretty).expect("deserialize pretty");
    assert_eq!(doc, loaded_pretty);

    // File I/O round-trip
    let temp_dir = std::env::temp_dir();
    let file_path = temp_dir.join(format!("test_session_{}.json", std::process::id()));
    doc.save_to_file(&file_path).expect("save to file");
    let loaded_file = SessionDocument::load_from_file(&file_path).expect("load from file");
    assert_eq!(doc, loaded_file);
    let _ = std::fs::remove_file(file_path);
}

#[test]
fn test_workspace_document_round_trip() {
    let mut metadata = HashMap::new();
    metadata.insert("author".to_string(), "engineer".to_string());
    metadata.insert("environment".to_string(), "test".to_string());

    let mut doc = WorkspaceDocument::new("Main Project Workspace");
    doc.session_paths = vec![
        "/sessions/session1.json".to_string(),
        "/sessions/session2.json".to_string(),
    ];
    doc.active_session = Some("/sessions/session1.json".to_string());
    doc.window_layout = Some(WindowLayout {
        width: 1920,
        height: 1080,
        is_maximized: true,
    });
    doc.recent_paths = vec!["/recent/a".to_string(), "/recent/b".to_string()];
    doc.metadata = metadata;

    // JSON compact round-trip
    let json_compact = doc.to_json().expect("serialize compact");
    let loaded_compact = WorkspaceDocument::from_json(&json_compact).expect("deserialize compact");
    assert_eq!(doc, loaded_compact);
    assert_eq!(
        loaded_compact.schema_version,
        CURRENT_WORKSPACE_SCHEMA_VERSION
    );

    // JSON pretty round-trip
    let json_pretty = doc.to_json_pretty().expect("serialize pretty");
    let loaded_pretty = WorkspaceDocument::from_json(&json_pretty).expect("deserialize pretty");
    assert_eq!(doc, loaded_pretty);

    // File I/O round-trip
    let temp_dir = std::env::temp_dir();
    let file_path = temp_dir.join(format!("test_workspace_{}.json", std::process::id()));
    doc.save_to_file(&file_path).expect("save to file");
    let loaded_file = WorkspaceDocument::load_from_file(&file_path).expect("load from file");
    assert_eq!(doc, loaded_file);
    let _ = std::fs::remove_file(file_path);
}

#[test]
fn test_workspace_config_round_trip() {
    let mut config = WorkspaceConfig::new("BC5 Parity Workspace")
        .with_session_paths(vec![
            "/sessions/compare1.json".to_string(),
            "/sessions/compare2.json".to_string(),
        ])
        .with_active_session("/sessions/compare1.json")
        .with_window_layout(WindowLayout {
            width: 2560,
            height: 1440,
            is_maximized: false,
        })
        .with_recent_paths(vec!["/recent/repo".to_string()]);

    config.set_metadata("workspace_type", "beyond_compare_parity");
    config.add_session_path("/sessions/compare3.json");
    config.add_recent_path("/recent/scratch");

    // JSON compact roundtrip
    let compact_json = config.to_json().expect("serialize compact WorkspaceConfig");
    let parsed_compact =
        WorkspaceConfig::from_json(&compact_json).expect("deserialize compact WorkspaceConfig");
    assert_eq!(config, parsed_compact);
    assert_eq!(
        parsed_compact.schema_version,
        CURRENT_WORKSPACE_SCHEMA_VERSION
    );
    assert_eq!(parsed_compact.session_paths.len(), 3);
    assert_eq!(parsed_compact.recent_paths.len(), 2);

    // JSON pretty roundtrip
    let pretty_json = config
        .to_json_pretty()
        .expect("serialize pretty WorkspaceConfig");
    let parsed_pretty =
        WorkspaceConfig::from_json(&pretty_json).expect("deserialize pretty WorkspaceConfig");
    assert_eq!(config, parsed_pretty);

    // File I/O roundtrip
    let temp_dir = std::env::temp_dir();
    let file_path = temp_dir.join(format!("test_ws_config_{}.json", std::process::id()));
    config
        .save_to_file(&file_path)
        .expect("save WorkspaceConfig to file");
    let loaded_config =
        WorkspaceConfig::load_from_file(&file_path).expect("load WorkspaceConfig from file");
    assert_eq!(config, loaded_config);
    let _ = std::fs::remove_file(file_path);
}

#[test]
fn test_session_config_round_trip() {
    let tab = SessionTab::new("tab-cfg", "Config Tab")
        .with_paths("/left", "/right")
        .with_session_type("folder");
    let config = SessionConfig::with_tabs(vec![tab]);
    let json = config.to_json().expect("serialize SessionConfig");
    let loaded = SessionConfig::from_json(&json).expect("deserialize SessionConfig");
    assert_eq!(config, loaded);
}

#[test]
fn test_app_settings_round_trip() {
    let mut extra = HashMap::new();
    extra.insert("diff_algorithm".to_string(), "myers".to_string());

    let mut settings = AppSettings::with_presets("nord_dark", "vim", "rust_enhanced");
    settings.font_family = Some("JetBrains Mono".to_string());
    settings.font_size = Some(14);
    settings.tab_size = Some(2);
    settings.auto_save = true;
    settings.show_line_numbers = true;
    settings.extra = extra;

    // JSON compact round-trip
    let json_compact = settings.to_json().expect("serialize compact");
    let loaded_compact = AppSettings::from_json(&json_compact).expect("deserialize compact");
    assert_eq!(settings, loaded_compact);
    assert_eq!(
        loaded_compact.schema_version,
        CURRENT_SETTINGS_SCHEMA_VERSION
    );
    assert_eq!(loaded_compact.theme, "nord_dark");
    assert_eq!(loaded_compact.keymap, "vim");
    assert_eq!(loaded_compact.grammar_preset_name, "rust_enhanced");

    // JSON pretty round-trip
    let json_pretty = settings.to_json_pretty().expect("serialize pretty");
    let loaded_pretty = AppSettings::from_json(&json_pretty).expect("deserialize pretty");
    assert_eq!(settings, loaded_pretty);

    // File I/O round-trip
    let temp_dir = std::env::temp_dir();
    let file_path = temp_dir.join(format!("test_settings_{}.json", std::process::id()));
    settings.save_to_file(&file_path).expect("save to file");
    let loaded_file = AppSettings::load_from_file(&file_path).expect("load from file");
    assert_eq!(settings, loaded_file);
    let _ = std::fs::remove_file(file_path);
}
