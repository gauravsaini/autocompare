#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use autocompare_sessions::{
    CURRENT_SESSION_SCHEMA_VERSION, CURRENT_SETTINGS_SCHEMA_VERSION,
    CURRENT_WORKSPACE_SCHEMA_VERSION, SessionError, migrate, migrate_session_config,
    migrate_settings, migrate_workspace, migrate_workspace_config,
};

#[test]
fn test_migrate_session_v0_unversioned() {
    let v0_json = r#"{
        "tabs": [
            {
                "id": "tab-legacy-1",
                "title": "Legacy Diff",
                "session_type": "text",
                "left_path": "/old/left.txt",
                "right_path": "/old/right.txt",
                "read_only": true,
                "locked": true
            }
        ],
        "active_tab_index": 0,
        "name": "Old Session",
        "locked": true
    }"#;

    let doc = migrate(v0_json).expect("v0 unversioned migration should succeed");
    assert_eq!(doc.schema_version, CURRENT_SESSION_SCHEMA_VERSION);
    assert_eq!(doc.tabs.len(), 1);
    assert_eq!(doc.tabs[0].id, "tab-legacy-1");
    assert_eq!(doc.tabs[0].title, "Legacy Diff");
    assert_eq!(doc.tabs[0].left_path.as_deref(), Some("/old/left.txt"));
    assert_eq!(doc.tabs[0].right_path.as_deref(), Some("/old/right.txt"));
    assert!(doc.tabs[0].read_only);
    assert!(doc.tabs[0].locked);
    assert!(doc.locked);
    assert_eq!(doc.name.as_deref(), Some("Old Session"));
}

#[test]
fn test_migrate_session_v0_explicit_zero_and_legacy_fields() {
    let v0_json = r#"{
        "schema_version": 0,
        "is_locked": true,
        "sessions": [
            {
                "name": "Legacy Tab",
                "left": "/legacy/left.txt",
                "right": "/legacy/right.txt",
                "type": "table",
                "is_locked": true
            }
        ]
    }"#;

    let doc = migrate(v0_json).expect("v0 explicit schema_version 0 should succeed");
    assert_eq!(doc.schema_version, CURRENT_SESSION_SCHEMA_VERSION);
    assert_eq!(doc.tabs.len(), 1);
    assert_eq!(doc.tabs[0].title, "Legacy Tab");
    assert_eq!(doc.tabs[0].left_path.as_deref(), Some("/legacy/left.txt"));
    assert_eq!(doc.tabs[0].right_path.as_deref(), Some("/legacy/right.txt"));
    assert_eq!(doc.tabs[0].session_type, "table");
    assert!(doc.locked);
    assert!(doc.tabs[0].locked);
}

#[test]
fn test_migrate_session_v0_bare_array() {
    let v0_json = r#"[
        {
            "id": "tab-1",
            "title": "Tab One",
            "left_path": "/path1"
        },
        {
            "id": "tab-2",
            "title": "Tab Two",
            "right_path": "/path2"
        }
    ]"#;

    let doc = migrate(v0_json).expect("v0 bare array should migrate to SessionDocument");
    assert_eq!(doc.schema_version, CURRENT_SESSION_SCHEMA_VERSION);
    assert_eq!(doc.tabs.len(), 2);
    assert_eq!(doc.tabs[0].title, "Tab One");
    assert_eq!(doc.tabs[1].title, "Tab Two");
    assert!(!doc.locked);
}

#[test]
fn test_migrate_session_v0_array_of_string_paths() {
    let v0_json = r#"[
        "/file/a.txt",
        "/file/b.txt"
    ]"#;

    let doc = migrate(v0_json).expect("v0 array of string paths should succeed");
    assert_eq!(doc.schema_version, CURRENT_SESSION_SCHEMA_VERSION);
    assert_eq!(doc.tabs.len(), 2);
    assert_eq!(doc.tabs[0].left_path.as_deref(), Some("/file/a.txt"));
    assert_eq!(doc.tabs[1].left_path.as_deref(), Some("/file/b.txt"));
    assert!(!doc.locked);
}

#[test]
fn test_migrate_session_current_v1() {
    let v1_json = r#"{
        "schema_version": 1,
        "tabs": [
            {
                "id": "tab-v1",
                "title": "Current V1",
                "session_type": "text",
                "left_path": null,
                "right_path": null,
                "read_only": false,
                "locked": true,
                "options": {}
            }
        ],
        "active_tab_index": 0,
        "name": "V1 Doc",
        "locked": true
    }"#;

    let doc = migrate(v1_json).expect("v1 should pass through cleanly");
    assert_eq!(doc.schema_version, 1);
    assert_eq!(doc.tabs.len(), 1);
    assert_eq!(doc.tabs[0].title, "Current V1");
    assert!(doc.locked);
    assert!(doc.tabs[0].locked);
}

#[test]
fn test_migrate_session_future_version_error() {
    let future_json = r#"{
        "schema_version": 2,
        "tabs": []
    }"#;

    match migrate(future_json) {
        Err(SessionError::UnsupportedVersion {
            found,
            max_supported,
        }) => {
            assert_eq!(found, 2);
            assert_eq!(max_supported, CURRENT_SESSION_SCHEMA_VERSION);
        }
        other => panic!("expected UnsupportedVersion, got {other:?}"),
    }

    let future_huge = r#"{
        "schema_version": 9999,
        "tabs": []
    }"#;

    match migrate(future_huge) {
        Err(SessionError::UnsupportedVersion {
            found,
            max_supported,
        }) => {
            assert_eq!(found, 9999);
            assert_eq!(max_supported, CURRENT_SESSION_SCHEMA_VERSION);
        }
        other => panic!("expected UnsupportedVersion, got {other:?}"),
    }
}

#[test]
fn test_migrate_workspace_v0_and_future() {
    let v0_json = r#"{
        "name": "Legacy Workspace",
        "sessions": ["/path/s1.json", "/path/s2.json"],
        "locked": true
    }"#;

    let ws = migrate_workspace(v0_json).expect("v0 workspace migration should succeed");
    assert_eq!(ws.schema_version, CURRENT_WORKSPACE_SCHEMA_VERSION);
    assert_eq!(ws.name, "Legacy Workspace");
    assert_eq!(ws.session_paths.len(), 2);
    assert!(ws.locked);

    let future_json = r#"{
        "schema_version": 3,
        "name": "Future Workspace"
    }"#;
    match migrate_workspace(future_json) {
        Err(SessionError::UnsupportedVersion {
            found,
            max_supported,
        }) => {
            assert_eq!(found, 3);
            assert_eq!(max_supported, CURRENT_WORKSPACE_SCHEMA_VERSION);
        }
        other => panic!("expected UnsupportedVersion, got {other:?}"),
    }
}

#[test]
fn test_migrate_workspace_bare_array_of_paths_and_objects() {
    let array_json = r#"[
        "/workspace/sess1.json",
        {"path": "/workspace/sess2.json"},
        {"file": "/workspace/sess3.json"},
        {"uri": "file:///workspace/sess4.json"},
        {"session_path": "/workspace/sess5.json"}
    ]"#;

    let ws = migrate_workspace(array_json).expect("bare array should migrate to WorkspaceDocument");
    assert_eq!(ws.schema_version, CURRENT_WORKSPACE_SCHEMA_VERSION);
    assert_eq!(ws.name, "Default Workspace");
    assert_eq!(
        ws.session_paths,
        vec![
            "/workspace/sess1.json",
            "/workspace/sess2.json",
            "/workspace/sess3.json",
            "file:///workspace/sess4.json",
            "/workspace/sess5.json"
        ]
    );
    assert!(!ws.locked);

    let config = migrate_workspace_config(array_json).expect("migrate_workspace_config bare array");
    assert_eq!(config.session_paths.len(), 5);
}

#[test]
fn test_migrate_workspace_legacy_aliases_and_config() {
    let legacy_json = r#"{
        "workspace_name": "Beyond Compare Legacy",
        "open_sessions": [
            "/path/s1.json",
            {"path": "/path/s2.json"}
        ],
        "active": "/path/s1.json",
        "window": {
            "width": 1920,
            "height": 1080,
            "is_maximized": true
        },
        "recents": ["/recent1", {"path": "/recent2"}],
        "meta": {
            "sync_mode": "mirror",
            "count": 5,
            "enabled": true
        },
        "read_only_workspace": true
    }"#;

    let ws = migrate_workspace_config(legacy_json).expect("legacy aliases should migrate cleanly");
    assert_eq!(ws.schema_version, CURRENT_WORKSPACE_SCHEMA_VERSION);
    assert_eq!(ws.name, "Beyond Compare Legacy");
    assert_eq!(ws.session_paths, vec!["/path/s1.json", "/path/s2.json"]);
    assert_eq!(ws.active_session.as_deref(), Some("/path/s1.json"));
    let win = ws.window_layout.expect("window layout parsed");
    assert_eq!(win.width, 1920);
    assert_eq!(win.height, 1080);
    assert!(win.is_maximized);
    assert_eq!(ws.recent_paths, vec!["/recent1", "/recent2"]);
    assert!(ws.locked);
    assert_eq!(
        ws.metadata.get("sync_mode").map(String::as_str),
        Some("mirror")
    );
    assert_eq!(ws.metadata.get("count").map(String::as_str), Some("5"));
    assert_eq!(ws.metadata.get("enabled").map(String::as_str), Some("true"));
}

#[test]
fn test_migrate_session_config_helper() {
    let v0_json = r#"[
        {"id": "t1", "title": "Tab 1"}
    ]"#;
    let config = migrate_session_config(v0_json).expect("migrate_session_config");
    assert_eq!(config.tabs.len(), 1);
    assert_eq!(config.tabs[0].title, "Tab 1");
}

#[test]
fn test_migrate_settings_v0_and_aliases_and_future() {
    let v0_json = r#"{
        "theme": "solarized_dark",
        "keys": "emacs",
        "grammar_preset": "c_cpp"
    }"#;

    let settings = migrate_settings(v0_json).expect("v0 settings migration should succeed");
    assert_eq!(settings.schema_version, CURRENT_SETTINGS_SCHEMA_VERSION);
    assert_eq!(settings.theme, "solarized_dark");
    assert_eq!(settings.keymap, "emacs");
    assert_eq!(settings.grammar_preset_name, "c_cpp");
    assert!(!settings.grammar_presets.is_empty());

    let future_json = r#"{
        "schema_version": 42,
        "theme": "future_theme"
    }"#;
    match migrate_settings(future_json) {
        Err(SessionError::UnsupportedVersion {
            found,
            max_supported,
        }) => {
            assert_eq!(found, 42);
            assert_eq!(max_supported, CURRENT_SETTINGS_SCHEMA_VERSION);
        }
        other => panic!("expected UnsupportedVersion, got {other:?}"),
    }
}

#[test]
fn test_migrate_settings_v0_custom_grammar_presets() {
    let v0_json = r##"{
        "theme": "monokai",
        "grammar_preset_name": "ruby",
        "grammar_presets": [
            {
                "name": "elixir",
                "extensions": ["ex", "exs"],
                "line_comment_prefix": "#",
                "ignore_case": false
            }
        ]
    }"##;

    let settings = migrate_settings(v0_json).expect("v0 settings with presets");
    assert_eq!(settings.theme, "monokai");
    assert_eq!(settings.grammar_preset_name, "ruby");
    assert_eq!(settings.grammar_presets.len(), 1);
    assert_eq!(
        settings
            .find_preset_for_extension("ex")
            .map(|p| p.name.as_str()),
        Some("elixir")
    );
}
