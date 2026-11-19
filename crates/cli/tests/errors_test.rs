#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

//! Error handling and exit code 2 verification tests.
//!
//! Asserts that invalid arguments, nonexistent paths, and syntax errors
//! reliably exit with status code 2.

use std::process::Command;

fn bin_path() -> &'static str {
    env!("CARGO_BIN_EXE_autocompare")
}

#[test]
fn test_missing_file_exits_with_code_2() {
    let status = Command::new(bin_path())
        .args(["text", "nonexistent_left.txt", "nonexistent_right.txt"])
        .status()
        .expect("exec");
    assert_eq!(status.code(), Some(2));
}

#[test]
fn test_invalid_flag_exits_with_code_2() {
    let status = Command::new(bin_path())
        .args(["text", "--completely-invalid-flag"])
        .status()
        .expect("exec");
    assert_eq!(status.code(), Some(2));
}

#[test]
fn test_empty_report_spec_exits_with_code_2() {
    let status = Command::new(bin_path())
        .args(["text", "a", "b", "--report", ""])
        .status()
        .expect("exec");
    assert_eq!(status.code(), Some(2));
}

#[test]
fn test_no_arguments_exits_with_code_2() {
    let status = Command::new(bin_path()).status().expect("exec");
    assert_eq!(status.code(), Some(2));
}

#[test]
fn test_help_flag_exits_with_code_0() {
    let status = Command::new(bin_path())
        .arg("--help")
        .status()
        .expect("exec");
    assert_eq!(status.code(), Some(0));
}
