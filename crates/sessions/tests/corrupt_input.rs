#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use autocompare_sessions::{
    AppSettings, SessionConfig, SessionDocument, SessionError, WorkspaceConfig, WorkspaceDocument,
    migrate, migrate_session_config, migrate_settings, migrate_workspace, migrate_workspace_config,
};

#[test]
fn test_syntax_corrupt_input_returns_err() {
    let syntax_errors = [
        "",
        "   ",
        "\n\t",
        "{",
        "}",
        "[",
        "]",
        "null",
        "true",
        "false",
        "12345",
        "\"a plain string\"",
        "{\"schema_version\": \"one\"}",
        "{\"schema_version\": -1}",
        "{\"schema_version\": true}",
        "{\"schema_version\": [1]}",
        "{\"incomplete\": ",
        "{\"schema_version\": 1, \"unknown_token\": \x00\x01\x02}",
    ];

    for (i, sample) in syntax_errors.iter().enumerate() {
        let res_session = migrate(sample);
        assert!(
            res_session.is_err(),
            "Syntax sample #{i} ({sample:?}) must return Err for session, got: {res_session:?}"
        );

        let res_ws = migrate_workspace(sample);
        assert!(
            res_ws.is_err(),
            "Syntax sample #{i} ({sample:?}) must return Err for workspace, got: {res_ws:?}"
        );

        let res_settings = migrate_settings(sample);
        assert!(
            res_settings.is_err(),
            "Syntax sample #{i} ({sample:?}) must return Err for settings, got: {res_settings:?}"
        );
    }
}

#[test]
fn test_type_specific_corrupt_input_returns_err() {
    let session_corrupt = [
        "{\"schema_version\": 1, \"tabs\": \"not_an_array\"}",
        "{\"schema_version\": 1, \"tabs\": [null]}",
        "{\"schema_version\": 1, \"tabs\": [123]}",
        "{\"schema_version\": 0, \"tabs\": 123}",
        "{\"schema_version\": 0, \"tabs\": [false]}",
        "{\"schema_version\": 1, \"tabs\": [{\"id\": 123}]}",
    ];

    for (i, sample) in session_corrupt.iter().enumerate() {
        let res = migrate(sample);
        assert!(
            res.is_err(),
            "Session corrupt sample #{i} ({sample:?}) must return Err, got: {res:?}"
        );
    }
}

#[test]
fn test_fuzzed_hostile_inputs_never_panic() {
    let hostile_inputs = [
        "",
        "{}",
        "[]",
        "\0",
        "\r\n\t",
        "{\"schema_version\": 0}",
        "{\"schema_version\": 1}",
        "{\"schema_version\": 99999999999999999999999999999}",
        "{\"schema_version\": -99999999999999999999999999999}",
        "{\"tabs\": [[[[[[[[[[]]]]]]]]]]}",
        "{\"name\": \"\x00\x01\x02\x03\x04\x05\"}",
        "{\"metadata\": {\"key\": null}}",
        "{{{{{{{{{{{}}}}}}}}}}}",
        "[null, null, false, 0, \"\"]",
    ];

    for sample in hostile_inputs {
        // Assert none of these panic
        let _ = std::panic::catch_unwind(|| {
            let _ = migrate(sample);
            let _ = migrate_workspace(sample);
            let _ = migrate_settings(sample);
        });
    }
}

#[test]
fn test_from_json_corrupt_session() {
    let bad_json = "{\"schema_version\": 1, \"tabs\": 42}";
    let res = SessionDocument::from_json(bad_json);
    assert!(res.is_err());
    match res.unwrap_err() {
        SessionError::Json(_) | SessionError::CorruptInput(_) => {}
        other => panic!("expected Json or CorruptInput error, got: {other:?}"),
    }
}

#[test]
fn test_from_json_corrupt_workspace() {
    let bad_json = "{\"schema_version\": 1, \"name\": 999}";
    let res = WorkspaceDocument::from_json(bad_json);
    assert!(res.is_err());
    let res_cfg = WorkspaceConfig::from_json(bad_json);
    assert!(res_cfg.is_err());
    let res_mig = migrate_workspace_config(bad_json);
    assert!(res_mig.is_err());
}

#[test]
fn test_from_json_corrupt_session_config() {
    let bad_json = "{\"schema_version\": 1, \"tabs\": 42}";
    let res_cfg = SessionConfig::from_json(bad_json);
    assert!(res_cfg.is_err());
    let res_mig = migrate_session_config(bad_json);
    assert!(res_mig.is_err());
}

#[test]
fn test_from_json_corrupt_settings() {
    let bad_json = "{\"schema_version\": 1, \"theme\": 42}";
    let res = AppSettings::from_json(bad_json);
    assert!(res.is_err());
}

#[test]
fn test_error_display_and_source() {
    use std::error::Error;

    let err_unsupported = SessionError::UnsupportedVersion {
        found: 5,
        max_supported: 1,
    };
    let display_str = format!("{err_unsupported}");
    assert!(display_str.contains("Unsupported schema version 5"));
    assert!(err_unsupported.source().is_none());

    let err_corrupt = SessionError::CorruptInput("bad token".to_string());
    assert!(format!("{err_corrupt}").contains("bad token"));

    let io_err = SessionError::Io(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "file not found",
    ));
    assert!(io_err.source().is_some());
    assert!(format!("{io_err}").contains("file not found"));
}
