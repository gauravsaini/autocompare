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

#[test]
fn test_migrate_session_v0_embedded_options() {
    let v0_json = r#"{
        "schema_version": 0,
        "tabs": [
            {
                "name": "Embedded Options Tab",
                "left": "/src/a",
                "right": "/src/b",
                "session_type": "folder",
                "sync_mode": "mirror",
                "depth_limit": 5,
                "content_compare": true,
                "case_insensitive": true,
                "include": "*.rs",
                "exclude": "*.tmp",
                "quick_compare": "crc",
                "follow_symlinks": true,
                "archive_traversal": true,
                "is_locked": true
            }
        ]
    }"#;

    let doc = migrate(v0_json).expect("migrate embedded options tab");
    assert_eq!(doc.tabs.len(), 1);
    let tab = doc.tabs.first().expect("one tab");
    assert_eq!(tab.title, "Embedded Options Tab");
    assert_eq!(tab.left_path.as_deref(), Some("/src/a"));
    assert_eq!(tab.right_path.as_deref(), Some("/src/b"));
    assert_eq!(tab.sync_mode(), Some("mirror"));
    assert_eq!(tab.folder_depth_limit(), Some(5));
    assert!(tab.folder_content_compare());
    assert!(tab.folder_case_insensitive());
    assert_eq!(tab.folder_include(), Some("*.rs"));
    assert_eq!(tab.folder_exclude(), Some("*.tmp"));
    assert_eq!(tab.folder_quick_compare(), Some("crc"));
    assert!(tab.folder_follow_symlinks());
    assert!(tab.folder_archive_traversal());
    assert!(tab.locked);
}

#[test]
fn test_migrate_session_v0_path_pairs_array() {
    let v0_json = r#"[
        ["/pair/left1.txt", "/pair/right1.txt"],
        ["/pair/left2.rs", "/pair/right2.rs"]
    ]"#;

    let doc = migrate(v0_json).expect("migrate array of path pairs");
    assert_eq!(doc.tabs.len(), 2);
    let t0 = doc.tabs.first().expect("pair 0");
    assert_eq!(t0.left_path.as_deref(), Some("/pair/left1.txt"));
    assert_eq!(t0.right_path.as_deref(), Some("/pair/right1.txt"));

    let t1 = doc.tabs.get(1).expect("pair 1");
    assert_eq!(t1.left_path.as_deref(), Some("/pair/left2.rs"));
    assert_eq!(t1.right_path.as_deref(), Some("/pair/right2.rs"));
}

#[test]
fn test_migrate_session_v0_single_tab_root() {
    let v0_json = r#"{
        "left_path": "/single/left.txt",
        "right_path": "/single/right.txt",
        "session_type": "text",
        "name": "Single Tab Doc",
        "is_locked": true
    }"#;

    let doc = migrate(v0_json).expect("migrate single tab root object");
    assert_eq!(doc.tabs.len(), 1);
    let tab = doc.tabs.first().expect("single tab");
    assert_eq!(tab.left_path.as_deref(), Some("/single/left.txt"));
    assert_eq!(tab.right_path.as_deref(), Some("/single/right.txt"));
    assert_eq!(tab.title, "Single Tab Doc");
    assert!(tab.locked);
}

#[test]
fn test_migrate_settings_v0_aliases_and_locked() {
    let v0_json = r#"{
        "color_theme": "solarized_light",
        "keybindings": "vim",
        "syntax": "rust",
        "tab_width": 2,
        "autosave": true,
        "line_numbers": false,
        "read_only": true,
        "custom": {
            "wrap_lines": "true"
        }
    }"#;

    let settings = migrate_settings(v0_json).expect("migrate settings aliases");
    assert_eq!(settings.theme, "solarized_light");
    assert_eq!(settings.keymap, "vim");
    assert_eq!(settings.grammar_preset_name, "rust");
    assert_eq!(settings.tab_size, Some(2));
    assert!(settings.auto_save);
    assert!(!settings.show_line_numbers);
    assert!(settings.is_locked());
    assert_eq!(
        settings.extra.get("wrap_lines").map(String::as_str),
        Some("true")
    );
}

#[test]
fn test_migrate_workspace_v0_geometry_alias() {
    let v0_json = r#"{
        "label": "Geometry WS",
        "paths": ["/sess/1.json"],
        "geometry": {
            "width": 2560,
            "height": 1600,
            "maximized": true
        },
        "read_only": true
    }"#;

    let ws = migrate_workspace(v0_json).expect("migrate geometry workspace");
    assert_eq!(ws.name, "Geometry WS");
    assert_eq!(ws.session_paths, vec!["/sess/1.json"]);
    let geom = ws.window_layout.as_ref().expect("geometry parsed");
    assert_eq!(geom.width, 2560);
    assert_eq!(geom.height, 1600);
    assert!(geom.is_maximized);
    assert!(ws.is_locked());
}

#[test]
fn test_migrate_session_v0_3way_merge_with_aliases() {
    let v0_json = r#"{
        "schema_version": 0,
        "tabs": [
            {
                "id": "tab-merge-v0",
                "title": "Legacy 3-Way Merge",
                "session_type": "merge",
                "ancestor": "/old/base.rs",
                "lhs": "/old/branch.rs",
                "rhs": "/old/main.rs",
                "result": "/old/merged.rs",
                "conflict_strategy": "take_left",
                "auto_merge": true,
                "is_locked": true
            }
        ]
    }"#;

    let doc = migrate(v0_json).expect("migrate 3-way merge v0 aliases");
    assert_eq!(doc.schema_version, CURRENT_SESSION_SCHEMA_VERSION);
    assert_eq!(doc.tabs.len(), 1);

    let t0 = doc.tabs.first().expect("tab 0");
    assert_eq!(t0.id, "tab-merge-v0");
    assert_eq!(t0.title, "Legacy 3-Way Merge");
    assert_eq!(t0.base_path.as_deref(), Some("/old/base.rs"));
    assert_eq!(t0.left_path.as_deref(), Some("/old/branch.rs"));
    assert_eq!(t0.right_path.as_deref(), Some("/old/main.rs"));
    assert_eq!(t0.output_path.as_deref(), Some("/old/merged.rs"));
    assert!(t0.locked);

    let merge_opts = t0.merge_options();
    assert_eq!(merge_opts.conflict_strategy.as_deref(), Some("take_left"));
    assert!(merge_opts.auto_merge);
}

#[test]
fn test_migrate_session_v0_4way_path_array() {
    let v0_json = r#"[
        ["/base.rs", "/left.rs", "/right.rs", "/out.rs"]
    ]"#;

    let doc = migrate(v0_json).expect("migrate 4-tuple path array");
    assert_eq!(doc.schema_version, CURRENT_SESSION_SCHEMA_VERSION);
    assert_eq!(doc.tabs.len(), 1);

    let t0 = doc.tabs.first().expect("tab 0");
    assert_eq!(t0.session_type, "merge");
    assert_eq!(t0.base_path.as_deref(), Some("/base.rs"));
    assert_eq!(t0.left_path.as_deref(), Some("/left.rs"));
    assert_eq!(t0.right_path.as_deref(), Some("/right.rs"));
    assert_eq!(t0.output_path.as_deref(), Some("/out.rs"));
}

#[test]
fn test_migrate_workspace_v0_null_and_empty_fields() {
    let v0_json = r#"{
        "name": "Empty WS",
        "sessions": null,
        "active": null,
        "window": null,
        "recent": null,
        "meta": null
    }"#;

    let ws = migrate_workspace(v0_json).expect("migrate workspace with null fields");
    assert_eq!(ws.schema_version, CURRENT_WORKSPACE_SCHEMA_VERSION);
    assert_eq!(ws.name, "Empty WS");
    assert!(ws.session_paths.is_empty());
    assert!(ws.active_session.is_none());
    assert!(ws.window_layout.is_none());
    assert!(ws.recent_paths.is_empty());
    assert!(ws.metadata.is_empty());
}
