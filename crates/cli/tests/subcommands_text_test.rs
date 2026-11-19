#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

//! Subcommand integration tests covering text, folder, hex, and merge modes.
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
fn test_text_subcommand_exit_codes_and_reports() {
    let fx = fixtures_dir();
    let tmp = tempdir().expect("tempdir");

    // Hand-computed ground truth: t_left and t_right differ at line 2 ("beta" vs "BETA").
    // Exit code must be 1 (different).
    let html_report = tmp.path().join("report.html");
    let status = Command::new(bin_path())
        .args([
            "text",
            fx.join("t_left.txt").to_str().expect("path"),
            fx.join("t_right.txt").to_str().expect("path"),
            "--report",
            &format!("html:{}", html_report.display()),
        ])
        .status()
        .expect("exec");

    assert_eq!(status.code(), Some(1));
    assert!(html_report.exists());
    let html = std::fs::read_to_string(&html_report).expect("read");
    assert!(html.contains("<!DOCTYPE html>"));
    assert!(html.contains("alpha"));
    assert!(html.contains("beta"));
    assert!(html.contains("BETA"));

    // Patch report test
    let patch_report = tmp.path().join("diff.patch");
    let status_patch = Command::new(bin_path())
        .args([
            "text",
            fx.join("t_left.txt").to_str().expect("path"),
            fx.join("t_right.txt").to_str().expect("path"),
            "--report",
            &format!("patch:{}", patch_report.display()),
        ])
        .status()
        .expect("exec");
    assert_eq!(status_patch.code(), Some(1));
    assert!(patch_report.exists());
    let patch = std::fs::read_to_string(&patch_report).expect("read");
    assert!(patch.contains("@@ -1,3 +1,3 @@"));

    // Hand-computed ground truth: t_same vs t_same are identical (3 identical lines).
    // Exit code must be 0 (same).
    let txt_report = tmp.path().join("same.txt");
    let status_same = Command::new(bin_path())
        .args([
            "text",
            fx.join("t_same.txt").to_str().expect("path"),
            fx.join("t_same.txt").to_str().expect("path"),
            "--report",
            &format!("text:{}", txt_report.display()),
        ])
        .status()
        .expect("exec");

    assert_eq!(status_same.code(), Some(0));
    assert!(txt_report.exists());
    let txt = std::fs::read_to_string(&txt_report).expect("read");
    assert!(txt.contains("DIFF REPORT:"));

    // Case-insensitive test: "beta" == "BETA" when ignore_case is active.
    let status_icase = Command::new(bin_path())
        .args([
            "text",
            fx.join("t_left.txt").to_str().expect("path"),
            fx.join("t_right.txt").to_str().expect("path"),
            "--ignore-case",
        ])
        .status()
        .expect("exec");
    assert_eq!(status_icase.code(), Some(0));
}

#[test]
fn test_folder_subcommand_exit_code_and_report() {
    let fx = fixtures_dir();
    let tmp = tempdir().expect("tempdir");

    // Hand-computed ground truth: right folder has extra.txt which left lacks.
    // Exit code must be 1.
    let rep_file = tmp.path().join("folder.txt");
    let status = Command::new(bin_path())
        .args([
            "folder",
            fx.join("folder/left").to_str().expect("path"),
            fx.join("folder/right").to_str().expect("path"),
            "--report",
            &format!("text:{}", rep_file.display()),
        ])
        .status()
        .expect("exec");

    assert_eq!(status.code(), Some(1));
    assert!(rep_file.exists());
    let content = std::fs::read_to_string(&rep_file).expect("read");
    assert!(content.contains("extra.txt"));
}

#[test]
fn test_hex_subcommand_exit_code_and_report() {
    let fx = fixtures_dir();
    let tmp = tempdir().expect("tempdir");

    // Hand-computed ground truth: "hello world" vs "hallo world" differ at offset 1 ('e' vs 'a').
    // Exit code must be 1.
    let rep_file = tmp.path().join("hex.txt");
    let status = Command::new(bin_path())
        .args([
            "hex",
            fx.join("h_left.bin").to_str().expect("path"),
            fx.join("h_right.bin").to_str().expect("path"),
            "--report",
            &format!("text:{}", rep_file.display()),
        ])
        .status()
        .expect("exec");

    assert_eq!(status.code(), Some(1));
    assert!(rep_file.exists());
    let content = std::fs::read_to_string(&rep_file).expect("read");
    assert!(content.contains("offset 0x00000001"));

    // Identical hex test
    let status_same = Command::new(bin_path())
        .args([
            "hex",
            fx.join("h_left.bin").to_str().expect("path"),
            fx.join("h_left.bin").to_str().expect("path"),
        ])
        .status()
        .expect("exec");
    assert_eq!(status_same.code(), Some(0));
}

#[test]
fn test_merge_subcommand_exit_code_and_report() {
    let fx = fixtures_dir();
    let tmp = tempdir().expect("tempdir");

    // Hand-computed ground truth: m_left added "left", m_right added "right" after "base".
    // Causes a merge conflict. Exit code must be 1.
    let rep_file = tmp.path().join("merge.xml");
    let out_file = tmp.path().join("merged.txt");
    let status = Command::new(bin_path())
        .args([
            "merge",
            fx.join("m_base.txt").to_str().expect("path"),
            fx.join("m_left.txt").to_str().expect("path"),
            fx.join("m_right.txt").to_str().expect("path"),
            "--output",
            out_file.to_str().expect("path"),
            "--report",
            &format!("xml:{}", rep_file.display()),
        ])
        .status()
        .expect("exec");

    assert_eq!(status.code(), Some(1));
    assert!(rep_file.exists());
    assert!(out_file.exists());
    let xml = std::fs::read_to_string(&rep_file).expect("read");
    assert!(xml.contains("<?xml"));
    assert!(xml.contains("diff-report"));

    // Clean merge test: base vs base vs base is clean. Exit code must be 0.
    let status_clean = Command::new(bin_path())
        .args([
            "merge",
            fx.join("m_base.txt").to_str().expect("path"),
            fx.join("m_base.txt").to_str().expect("path"),
            fx.join("m_base.txt").to_str().expect("path"),
        ])
        .status()
        .expect("exec");
    assert_eq!(status_clean.code(), Some(0));
}
