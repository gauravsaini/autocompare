#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

//! Integration tests for Beyond Compare (bcomp) CLI compatibility flags.
//!
//! Validates -ro, -automerge, -mergeoutput, -vcs1..4, and shorthand invocations.

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
fn test_bcomp_direct_invocation_two_files() {
    let fx = fixtures_dir();

    // Invoking without subcommand: 2 files should run text comparison.
    let status = Command::new(bin_path())
        .args([
            fx.join("t_left.txt").to_str().expect("path"),
            fx.join("t_right.txt").to_str().expect("path"),
        ])
        .status()
        .expect("exec");
    assert_eq!(status.code(), Some(1));

    let status_same = Command::new(bin_path())
        .args([
            fx.join("t_same.txt").to_str().expect("path"),
            fx.join("t_same.txt").to_str().expect("path"),
        ])
        .status()
        .expect("exec");
    assert_eq!(status_same.code(), Some(0));
}

#[test]
fn test_bcomp_direct_invocation_two_folders() {
    let fx = fixtures_dir();

    // Invoking without subcommand: 2 directories should run folder comparison.
    let status = Command::new(bin_path())
        .args([
            fx.join("folder/left").to_str().expect("path"),
            fx.join("folder/right").to_str().expect("path"),
        ])
        .status()
        .expect("exec");
    assert_eq!(status.code(), Some(1));
}

#[test]
fn test_bcomp_ro_read_only_flags() {
    let fx = fixtures_dir();
    let tmp = tempdir().expect("tempdir");

    // -ro with compare is allowed and exits normally.
    let status = Command::new(bin_path())
        .args([
            "-ro",
            fx.join("t_left.txt").to_str().expect("path"),
            fx.join("t_right.txt").to_str().expect("path"),
        ])
        .status()
        .expect("exec");
    assert_eq!(status.code(), Some(1));

    // -ro with sync --apply must fail with exit code 2 (read-only error).
    let status_ro_sync = Command::new(bin_path())
        .args([
            "sync",
            fx.join("sync/left").to_str().expect("path"),
            fx.join("sync/right").to_str().expect("path"),
            "--apply",
            "-ro",
        ])
        .status()
        .expect("exec");
    assert_eq!(status_ro_sync.code(), Some(2));

    // -ro with merge output write must fail with exit code 2.
    let out = tmp.path().join("merged.txt");
    let status_ro_merge = Command::new(bin_path())
        .args([
            "merge",
            fx.join("m_base.txt").to_str().expect("path"),
            fx.join("m_left.txt").to_str().expect("path"),
            fx.join("m_right.txt").to_str().expect("path"),
            "-mergeoutput",
            out.to_str().expect("path"),
            "-ro",
        ])
        .status()
        .expect("exec");
    assert_eq!(status_ro_merge.code(), Some(2));
}

#[test]
fn test_bcomp_automerge_and_mergeoutput() {
    let fx = fixtures_dir();
    let tmp = tempdir().expect("tempdir");

    // 3 files without subcommand defaults to merge.
    // Conflicted merge with -automerge exits 1.
    let out_file = tmp.path().join("automerge_out.txt");
    let status_conflict = Command::new(bin_path())
        .args([
            fx.join("m_base.txt").to_str().expect("path"),
            fx.join("m_left.txt").to_str().expect("path"),
            fx.join("m_right.txt").to_str().expect("path"),
            &format!("-mergeoutput={}", out_file.display()),
            "-automerge",
        ])
        .status()
        .expect("exec");

    assert_eq!(status_conflict.code(), Some(1));
    assert!(out_file.exists());
    let content = std::fs::read_to_string(&out_file).expect("read");
    assert!(content.contains("<<<<<<<"));

    // Clean merge with -automerge exits 0.
    let clean_out = tmp.path().join("clean_out.txt");
    let status_clean = Command::new(bin_path())
        .args([
            fx.join("m_base.txt").to_str().expect("path"),
            fx.join("m_base.txt").to_str().expect("path"),
            fx.join("m_base.txt").to_str().expect("path"),
            &format!("-mergeoutput={}", clean_out.display()),
            "-automerge",
        ])
        .status()
        .expect("exec");
    assert_eq!(status_clean.code(), Some(0));
    assert!(clean_out.exists());
}

#[test]
fn test_bcomp_vcs_alias_flags() {
    let fx = fixtures_dir();
    let tmp = tempdir().expect("tempdir");

    // -vcs1 and -vcs2 compare
    let status_2way = Command::new(bin_path())
        .args([
            &format!("-vcs1={}", fx.join("t_left.txt").display()),
            &format!("-vcs2={}", fx.join("t_right.txt").display()),
        ])
        .status()
        .expect("exec");
    assert_eq!(status_2way.code(), Some(1));

    // -vcs1..3 merge with -mergeoutput
    let out_file = tmp.path().join("vcs_merge.txt");
    let status_3way = Command::new(bin_path())
        .args([
            &format!("-vcs1={}", fx.join("t_same.txt").display()),
            &format!("-vcs2={}", fx.join("t_same.txt").display()),
            &format!("-vcs3={}", fx.join("t_same.txt").display()),
            &format!("-mergeoutput={}", out_file.display()),
        ])
        .status()
        .expect("exec");
    assert_eq!(status_3way.code(), Some(0));
    assert!(out_file.exists());
}
