#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use autocompare_sessions::{
    CURRENT_SESSION_SCHEMA_VERSION, CURRENT_SETTINGS_SCHEMA_VERSION,
    CURRENT_WORKSPACE_SCHEMA_VERSION, SessionError, migrate, migrate_settings, migrate_workspace,
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
                "read_only": true
            }
        ],
        "active_tab_index": 0,
        "name": "Old Session"
    }"#;

    let doc = migrate(v0_json).expect("v0 unversioned migration should succeed");
    assert_eq!(doc.schema_version, CURRENT_SESSION_SCHEMA_VERSION);
    assert_eq!(doc.tabs.len(), 1);
    assert_eq!(doc.tabs[0].id, "tab-legacy-1");
    assert_eq!(doc.tabs[0].title, "Legacy Diff");
    assert_eq!(doc.tabs[0].left_path.as_deref(), Some("/old/left.txt"));
    assert_eq!(doc.tabs[0].right_path.as_deref(), Some("/old/right.txt"));
    assert!(doc.tabs[0].read_only);
    assert_eq!(doc.name.as_deref(), Some("Old Session"));
}

#[test]
fn test_migrate_session_v0_explicit_zero_and_legacy_fields() {
    let v0_json = r#"{
        "schema_version": 0,
        "sessions": [
            {
                "name": "Legacy Tab",
                "left": "/legacy/left.txt",
                "right": "/legacy/right.txt",
                "type": "table"
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
                "options": {}
            }
        ],
        "active_tab_index": 0,
        "name": "V1 Doc"
    }"#;

    let doc = migrate(v1_json).expect("v1 should pass through cleanly");
    assert_eq!(doc.schema_version, 1);
    assert_eq!(doc.tabs.len(), 1);
    assert_eq!(doc.tabs[0].title, "Current V1");
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
        "sessions": ["/path/s1.json", "/path/s2.json"]
    }"#;

    let ws = migrate_workspace(v0_json).expect("v0 workspace migration should succeed");
    assert_eq!(ws.schema_version, CURRENT_WORKSPACE_SCHEMA_VERSION);
    assert_eq!(ws.name, "Legacy Workspace");
    assert_eq!(ws.session_paths.len(), 2);

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
