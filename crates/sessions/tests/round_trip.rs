#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use autocompare_sessions::settings::GrammarPreset;
use autocompare_sessions::{
    AppSettings, SessionConfig, SessionDocument, SessionTab, WindowLayout, WorkspaceConfig,
    WorkspaceDocument, CURRENT_SESSION_SCHEMA_VERSION, CURRENT_SETTINGS_SCHEMA_VERSION,
    CURRENT_WORKSPACE_SCHEMA_VERSION,
};
use std::collections::HashMap;

#[test]
fn test_session_document_round_trip() {
    let mut tab1 = SessionTab::new("tab-1", "Compare Cargo.toml")
        .with_paths("/left/Cargo.toml", "/right/Cargo.toml")
        .with_session_type("text")
        .with_locked(false);
    tab1.read_only = true;
    tab1.options
        .insert("ignore_whitespace".to_string(), "true".to_string());

    let tab2 = SessionTab::new("tab-2", "Compare Src Tree")
        .with_paths("/left/src", "/right/src")
        .with_session_type("folder")
        .with_locked(true);

    let mut doc = SessionDocument::with_tabs(vec![tab1, tab2]);
    doc.active_tab_index = 1;
    doc.name = Some("Workspace Session Alpha".to_string());
    doc.set_locked(false);

    // JSON compact round-trip
    let json_compact = doc.to_json().expect("serialize compact");
    let loaded_compact = SessionDocument::from_json(&json_compact).expect("deserialize compact");
    assert_eq!(doc, loaded_compact);
    assert_eq!(
        loaded_compact.schema_version,
        CURRENT_SESSION_SCHEMA_VERSION
    );
    assert!(!loaded_compact.is_locked());
    assert!(loaded_compact.tabs[1].locked);

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
fn test_locked_session_round_trip_and_guard() {
    let tab = SessionTab::new("tab-locked", "Protected Session")
        .with_paths("/src/a.rs", "/src/b.rs")
        .with_locked(true);
    let doc = SessionDocument::with_tabs(vec![tab]).with_locked(true);
    assert!(doc.is_locked());

    let json = doc.to_json_pretty().expect("serialize locked session");
    let loaded = SessionDocument::from_json(&json).expect("deserialize locked session");
    assert_eq!(doc, loaded);
    assert!(loaded.is_locked());
    assert!(loaded.tabs[0].locked);

    // Guard test: save_to_file fails when locked
    let temp_dir = std::env::temp_dir();
    let file_path = temp_dir.join(format!("test_locked_sess_{}.json", std::process::id()));
    assert!(doc.save_to_file(&file_path).is_err());
    // Force save succeeds
    assert!(doc.save_to_file_force(&file_path).is_ok());

    let loaded_from_disk =
        SessionDocument::load_from_file(&file_path).expect("load locked from file");
    assert!(loaded_from_disk.is_locked());
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
    doc.set_locked(false);

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
fn test_locked_workspace_round_trip_and_guard() {
    let doc = WorkspaceDocument::new("Locked Parity Workspace")
        .with_session_paths(vec!["/locked/s1.json".to_string()])
        .with_locked(true);
    assert!(doc.is_locked());

    let json = doc.to_json_pretty().expect("serialize locked workspace");
    let loaded = WorkspaceDocument::from_json(&json).expect("deserialize locked workspace");
    assert_eq!(doc, loaded);
    assert!(loaded.is_locked());

    let temp_dir = std::env::temp_dir();
    let file_path = temp_dir.join(format!("test_locked_ws_{}.json", std::process::id()));
    assert!(doc.save_to_file(&file_path).is_err());
    assert!(doc.save_to_file_force(&file_path).is_ok());

    let loaded_disk = WorkspaceDocument::load_from_file(&file_path).expect("load from file");
    assert!(loaded_disk.is_locked());
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
        .with_session_type("folder")
        .with_locked(true);
    let config = SessionConfig::with_tabs(vec![tab]).with_locked(true);
    let json = config.to_json().expect("serialize SessionConfig");
    let loaded = SessionConfig::from_json(&json).expect("deserialize SessionConfig");
    assert_eq!(config, loaded);
    assert!(loaded.is_locked());
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

#[test]
fn test_app_settings_custom_grammar_presets_round_trip() {
    let custom_preset = GrammarPreset::new("kotlin")
        .with_extensions(vec!["kt".to_string(), "kts".to_string()])
        .with_line_comment("//")
        .with_block_comment("/*", "*/")
        .with_ignore_case(false);

    let mut settings = AppSettings::new();
    settings.add_grammar_preset(custom_preset);
    assert_eq!(
        settings
            .find_preset_for_extension("kt")
            .map(|p| p.name.as_str()),
        Some("kotlin")
    );

    let json = settings.to_json_pretty().expect("serialize settings");
    let loaded = AppSettings::from_json(&json).expect("deserialize settings");
    assert_eq!(settings, loaded);
    assert_eq!(
        loaded
            .find_preset_for_extension(".kts")
            .map(|p| p.name.as_str()),
        Some("kotlin")
    );
}

#[test]
fn test_locked_settings_round_trip_and_guard() {
    let mut settings = AppSettings::with_presets("dracula", "emacs", "rust");
    settings.set_locked(true);
    assert!(settings.is_locked());

    let json = settings
        .to_json_pretty()
        .expect("serialize locked settings");
    let loaded = AppSettings::from_json(&json).expect("deserialize locked settings");
    assert_eq!(settings, loaded);
    assert!(loaded.is_locked());

    let temp_dir = std::env::temp_dir();
    let file_path = temp_dir.join(format!("test_locked_settings_{}.json", std::process::id()));
    assert!(settings.save_to_file(&file_path).is_err());
    assert!(settings.save_to_file_force(&file_path).is_ok());

    let loaded_disk = AppSettings::load_from_file(&file_path).expect("load from file");
    assert!(loaded_disk.is_locked());
    let _ = std::fs::remove_file(file_path);
}

#[test]
fn test_session_tab_options_depth_round_trip() {
    use autocompare_sessions::{
        AudioCompareOptions, FolderCompareOptions, HexCompareOptions, PictureCompareOptions,
        SyncCompareOptions, TableCompareOptions, TextCompareOptions,
    };

    let mut tab_text = SessionTab::new_text("t-text", "Text Compare", "/a.txt", "/b.txt")
        .with_grammar_preset("rust");
    tab_text.set_text_options(&TextCompareOptions {
        ignore_case: true,
        ignore_whitespace: true,
        ignore_space_change: true,
        ignore_blank_lines: true,
        ignore_line_endings: true,
        ignore_comments: true,
        grammar_preset: Some("rust".to_string()),
        encoding_left: Some("utf-8".to_string()),
        encoding_right: Some("utf-8".to_string()),
    });

    let mut tab_folder = SessionTab::new_folder("t-folder", "Folder Compare", "/dir1", "/dir2");
    tab_folder.set_folder_options(&FolderCompareOptions {
        content_compare: true,
        depth_limit: Some(8),
        case_insensitive: true,
        include: Some("*.rs".to_string()),
        exclude: Some("*.bak".to_string()),
        quick_compare: Some("crc".to_string()),
        follow_symlinks: true,
        archive_traversal: true,
    });

    let mut tab_sync = SessionTab::new_sync("t-sync", "Sync Folders", "/src", "/dst");
    tab_sync.set_sync_options(&SyncCompareOptions {
        mode: Some("mirror".to_string()),
        direction: Some("left_to_right".to_string()),
        prune: true,
        delete_empty_folders: true,
    });

    let mut tab_table = SessionTab::new_table("t-tbl", "Table Compare", "/d1.csv", "/d2.csv");
    tab_table.set_table_options(&TableCompareOptions {
        delimiter: Some('\t'),
        header: false,
        key_column: Some(0),
    });

    let mut tab_pic = SessionTab::new_picture("t-pic", "Pic Compare", "/p1.png", "/p2.png");
    tab_pic.set_picture_options(&PictureCompareOptions {
        heatmap_path: Some("/tmp/heat.png".to_string()),
        tolerance: Some(15),
        ignore_alpha: true,
    });

    let mut tab_hex = SessionTab::new_hex("t-hex", "Hex Compare", "/b1.bin", "/b2.bin");
    tab_hex.set_hex_options(&HexCompareOptions {
        bytes_per_row: Some(32),
        group_size: Some(4),
    });

    let mut tab_audio = SessionTab::new_audio("t-aud", "Audio Compare", "/a1.wav", "/a2.wav");
    tab_audio.set_audio_options(&AudioCompareOptions {
        sample_rate: Some(44100),
        tolerance: Some(0.01),
    });

    let doc = SessionDocument::with_tabs(vec![
        tab_text, tab_folder, tab_sync, tab_table, tab_pic, tab_hex, tab_audio,
    ])
    .with_name("Comprehensive Multi-Format Session");

    let json = doc.to_json_pretty().expect("serialize comprehensive doc");
    let loaded = SessionDocument::from_json(&json).expect("deserialize comprehensive doc");
    assert_eq!(doc, loaded);
    assert_eq!(loaded.tab_count(), 7);

    // Verify roundtripped option depths
    let t0 = loaded.tabs.first().expect("tab 0");
    let text_opts = t0.text_options();
    assert!(text_opts.ignore_case);
    assert!(text_opts.ignore_comments);
    assert_eq!(text_opts.grammar_preset.as_deref(), Some("rust"));

    let t1 = loaded.tabs.get(1).expect("tab 1");
    let folder_opts = t1.folder_options();
    assert!(folder_opts.content_compare);
    assert_eq!(folder_opts.depth_limit, Some(8));
    assert!(folder_opts.follow_symlinks);
    assert_eq!(folder_opts.include.as_deref(), Some("*.rs"));

    let t2 = loaded.tabs.get(2).expect("tab 2");
    let sync_opts = t2.sync_options();
    assert_eq!(sync_opts.mode.as_deref(), Some("mirror"));
    assert!(sync_opts.prune);

    let t3 = loaded.tabs.get(3).expect("tab 3");
    let table_opts = t3.table_options();
    assert_eq!(table_opts.delimiter, Some('\t'));
    assert!(!table_opts.header);

    let t4 = loaded.tabs.get(4).expect("tab 4");
    let pic_opts = t4.picture_options();
    assert_eq!(pic_opts.heatmap_path.as_deref(), Some("/tmp/heat.png"));
    assert_eq!(pic_opts.tolerance, Some(15));

    let t5 = loaded.tabs.get(5).expect("tab 5");
    let hex_opts = t5.hex_options();
    assert_eq!(hex_opts.bytes_per_row, Some(32));

    let t6 = loaded.tabs.get(6).expect("tab 6");
    let aud_opts = t6.audio_options();
    assert_eq!(aud_opts.sample_rate, Some(44100));
}

#[test]
fn test_session_tab_3way_merge_round_trip() {
    use autocompare_sessions::MergeCompareOptions;

    let mut tab_text_merge = SessionTab::new_merge(
        "tab-m1",
        "Merge Logic Feature",
        "/src/base.rs",
        "/src/feature.rs",
        "/src/main.rs",
        "/src/output.rs",
    )
    .with_locked(true);

    tab_text_merge.set_merge_options(&MergeCompareOptions {
        conflict_strategy: Some("take_left".to_string()),
        auto_merge: true,
        alignment: Some("patience".to_string()),
    });

    let tab_folder_merge = SessionTab::new_folder_merge(
        "tab-fm2",
        "Merge Folder Trees",
        "/repo/base",
        "/repo/left",
        "/repo/right",
        "/repo/out",
    );

    let doc = SessionDocument::with_tabs(vec![tab_text_merge, tab_folder_merge])
        .with_name("Active 3-Way Merge Session")
        .with_locked(false);

    let json = doc.to_json_pretty().expect("serialize merge session doc");
    let loaded = SessionDocument::from_json(&json).expect("deserialize merge session doc");
    assert_eq!(doc, loaded);
    assert_eq!(loaded.tab_count(), 2);

    let t0 = loaded.tabs.first().expect("tab 0");
    assert_eq!(t0.session_type, "merge");
    assert_eq!(t0.base_path.as_deref(), Some("/src/base.rs"));
    assert_eq!(t0.left_path.as_deref(), Some("/src/feature.rs"));
    assert_eq!(t0.right_path.as_deref(), Some("/src/main.rs"));
    assert_eq!(t0.output_path.as_deref(), Some("/src/output.rs"));
    assert!(t0.locked);

    let merge_opts = t0.merge_options();
    assert_eq!(merge_opts.conflict_strategy.as_deref(), Some("take_left"));
    assert!(merge_opts.auto_merge);
    assert_eq!(merge_opts.alignment.as_deref(), Some("patience"));

    let t1 = loaded.tabs.get(1).expect("tab 1");
    assert_eq!(t1.session_type, "folder_merge");
    assert_eq!(t1.base_path.as_deref(), Some("/repo/base"));
    assert_eq!(t1.output_path.as_deref(), Some("/repo/out"));
}

#[test]
fn test_edge_fixture_merge_3way_round_trip() {
    let content = include_str!(
        "../../../checks/agyx-sess-h2/round-1/comparison/edge-merge-3way-session.json"
    );
    let doc = SessionDocument::from_json(content).expect("parse edge merge fixture");

    assert_eq!(doc.schema_version, 1);
    assert_eq!(doc.name.as_deref(), Some("Enterprise 3-Way Merge Session"));
    assert!(doc.is_locked());
    assert_eq!(doc.tab_count(), 2);

    let t0 = doc.tabs.first().expect("tab 0");
    assert_eq!(t0.id, "tab-merge-text-1");
    assert_eq!(t0.session_type, "merge");
    assert_eq!(t0.base_path.as_deref(), Some("/repo/ancestor_common.rs"));
    assert_eq!(t0.left_path.as_deref(), Some("/repo/branch_feature.rs"));
    assert_eq!(t0.right_path.as_deref(), Some("/repo/main.rs"));
    assert_eq!(t0.output_path.as_deref(), Some("/repo/merged_result.rs"));
    assert!(t0.locked);

    let opts = t0.merge_options();
    assert!(opts.auto_merge);
    assert_eq!(opts.conflict_strategy.as_deref(), Some("manual"));
    assert_eq!(opts.alignment.as_deref(), Some("patience"));

    let t1 = doc.tabs.get(1).expect("tab 1");
    assert_eq!(t1.id, "tab-merge-folder-2");
    assert_eq!(t1.session_type, "folder_merge");
    assert_eq!(t1.base_path.as_deref(), Some("/build/release_base"));
    assert_eq!(t1.output_path.as_deref(), Some("/build/release_output"));
    assert!(!t1.locked);

    // Round-trip serialize & deserialize
    let json_pretty = doc.to_json_pretty().expect("serialize pretty");
    let doc2 = SessionDocument::from_json(&json_pretty).expect("deserialize roundtrip");
    assert_eq!(doc, doc2);
}
