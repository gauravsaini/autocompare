#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

//! Subcommand integration tests covering table, sync, picture, folder-merge, and audio modes.
//!
//! Asserts exit codes and report generation across parity harness fixture files.

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
fn test_table_subcommand_exit_code_and_report() {
    let fx = fixtures_dir();
    let tmp = tempdir().expect("tempdir");

    // Hand-computed ground truth: tab_left has "2,bob,ops", tab_right has "2,bob,lead".
    // Exit code must be 1.
    let rep_file = tmp.path().join("table.html");
    let status = Command::new(bin_path())
        .args([
            "table",
            fx.join("tab_left.csv").to_str().expect("path"),
            fx.join("tab_right.csv").to_str().expect("path"),
            "--report",
            &format!("html:{}", rep_file.display()),
        ])
        .status()
        .expect("exec");

    assert_eq!(status.code(), Some(1));
    assert!(rep_file.exists());
    let html = std::fs::read_to_string(&rep_file).expect("read");
    assert!(html.contains("lead") || html.contains("ops"));
}

#[test]
fn test_sync_subcommand_exit_code_and_report() {
    let fx = fixtures_dir();
    let tmp = tempdir().expect("tempdir");

    // Hand-computed ground truth: sync/left has extra.txt which sync/right lacks.
    // In dry run, steps are planned, so exit code must be 1.
    let rep_file = tmp.path().join("sync.txt");
    let status = Command::new(bin_path())
        .args([
            "sync",
            fx.join("sync/left").to_str().expect("path"),
            fx.join("sync/right").to_str().expect("path"),
            "--report",
            &format!("text:{}", rep_file.display()),
        ])
        .status()
        .expect("exec");

    assert_eq!(status.code(), Some(1));
    assert!(rep_file.exists());
    let txt = std::fs::read_to_string(&rep_file).expect("read");
    assert!(txt.contains("extra.txt"));
}

#[test]
fn test_picture_subcommand_exit_code_and_report() {
    let fx = fixtures_dir();
    let tmp = tempdir().expect("tempdir");

    // Hand-computed ground truth: pic_left and pic_right differ. Exit code must be 1.
    let rep_file = tmp.path().join("pic.html");
    let status = Command::new(bin_path())
        .args([
            "picture",
            fx.join("pic_left.png").to_str().expect("path"),
            fx.join("pic_right.png").to_str().expect("path"),
            "--report",
            &format!("html:{}", rep_file.display()),
        ])
        .status()
        .expect("exec");

    assert_eq!(status.code(), Some(1));
    assert!(rep_file.exists());

    // Identical picture test
    let status_same = Command::new(bin_path())
        .args([
            "picture",
            fx.join("pic_left.png").to_str().expect("path"),
            fx.join("pic_left.png").to_str().expect("path"),
        ])
        .status()
        .expect("exec");
    assert_eq!(status_same.code(), Some(0));
}

#[test]
fn test_folder_merge_subcommand_exit_code_and_report() {
    let fx = fixtures_dir();
    let tmp = tempdir().expect("tempdir");

    // Hand-computed ground truth: fmerge fixtures have conflicting edits in doc.txt.
    // Exit code must be 1.
    let rep_file = tmp.path().join("fmerge.txt");
    let status = Command::new(bin_path())
        .args([
            "folder-merge",
            fx.join("fmerge-base").to_str().expect("path"),
            fx.join("fmerge-left").to_str().expect("path"),
            fx.join("fmerge-right").to_str().expect("path"),
            "--report",
            &format!("text:{}", rep_file.display()),
        ])
        .status()
        .expect("exec");

    assert_eq!(status.code(), Some(1));
    assert!(rep_file.exists());
}

#[test]
fn test_audio_subcommand_exit_code_and_report() {
    let fx = fixtures_dir();
    let tmp = tempdir().expect("tempdir");

    // Hand-computed ground truth: a_left.wav and a_right.wav differ. Exit code must be 1.
    let rep_file = tmp.path().join("audio.txt");
    let status = Command::new(bin_path())
        .args([
            "audio",
            fx.join("a_left.wav").to_str().expect("path"),
            fx.join("a_right.wav").to_str().expect("path"),
            "--report",
            &format!("text:{}", rep_file.display()),
        ])
        .status()
        .expect("exec");

    assert_eq!(status.code(), Some(1));
    assert!(rep_file.exists());

    // Identical audio test
    let status_same = Command::new(bin_path())
        .args([
            "audio",
            fx.join("a_left.wav").to_str().expect("path"),
            fx.join("a_left.wav").to_str().expect("path"),
        ])
        .status()
        .expect("exec");
    assert_eq!(status_same.code(), Some(0));
}
