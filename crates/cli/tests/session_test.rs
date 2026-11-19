#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

//! Integration tests for session saving and execution.
//!
//! Validates persisting comparison configurations to session JSON files
//! and reloading them via the CLI.

use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::tempdir;

fn bin_path() -> &'static str {
    env!("CARGO_BIN_EXE_autocompare")
}

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/parity-harness/fixtures")
}

#[test]
fn test_session_save_and_reload() {
    let fx = fixtures_dir();
    let tmp = tempdir().expect("tempdir");
    let session_file = tmp.path().join("saved_session.json");

    // 1. Run comparison and save session
    let status_save = Command::new(bin_path())
        .args([
            "text",
            fx.join("t_left.txt").to_str().expect("path"),
            fx.join("t_right.txt").to_str().expect("path"),
            "--save-session",
            session_file.to_str().expect("path"),
        ])
        .status()
        .expect("exec");

    assert_eq!(status_save.code(), Some(1));
    assert!(session_file.exists());

    let json_text = std::fs::read_to_string(&session_file).expect("read");
    assert!(json_text.contains("\"session_type\": \"text\""));
    assert!(json_text.contains("t_left.txt"));
    assert!(json_text.contains("t_right.txt"));

    // 2. Reload and execute saved session
    let status_load = Command::new(bin_path())
        .args(["--session", session_file.to_str().expect("path")])
        .status()
        .expect("exec");

    assert_eq!(status_load.code(), Some(1));
}
