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

#[test]
fn test_bcomp_four_way_git_mergetool() {
    let fx = fixtures_dir();
    let tmp = tempdir().expect("tempdir");

    // Git mergetool invokes: bcomp $LOCAL $REMOTE $BASE $MERGED
    // 1. Conflict case: local differs, remote differs from base
    let out_conflict = tmp.path().join("git_merged_conflict.txt");
    let status = Command::new(bin_path())
        .args([
            fx.join("m_left.txt").to_str().expect("path"),
            fx.join("m_right.txt").to_str().expect("path"),
            fx.join("m_base.txt").to_str().expect("path"),
            out_conflict.to_str().expect("path"),
        ])
        .status()
        .expect("exec");
    assert_eq!(status.code(), Some(1));
    assert!(out_conflict.exists());
    let content = std::fs::read_to_string(&out_conflict).expect("read");
    assert!(content.contains("<<<<<<<"));

    // 2. Clean case: all sides match
    let out_clean = tmp.path().join("git_merged_clean.txt");
    let status_clean = Command::new(bin_path())
        .args([
            fx.join("m_base.txt").to_str().expect("path"),
            fx.join("m_base.txt").to_str().expect("path"),
            fx.join("m_base.txt").to_str().expect("path"),
            out_clean.to_str().expect("path"),
        ])
        .status()
        .expect("exec");
    assert_eq!(status_clean.code(), Some(0));
    assert!(out_clean.exists());
}

#[test]
fn test_bcomp_exit_codes_bc_mode() {
    let fx = fixtures_dir();
    let tmp = tempdir().expect("tempdir");

    // 1. Text difference with -bcexitcode -> 13 (rules-based difference)
    let status_text_diff = Command::new(bin_path())
        .args([
            "-bcexitcode",
            "text",
            fx.join("t_left.txt").to_str().expect("path"),
            fx.join("t_right.txt").to_str().expect("path"),
        ])
        .status()
        .expect("exec");
    assert_eq!(status_text_diff.code(), Some(13));

    // 2. Text identical with -bcexitcode -> 0 (success)
    let status_text_same = Command::new(bin_path())
        .args([
            "-bcexitcode",
            "text",
            fx.join("t_same.txt").to_str().expect("path"),
            fx.join("t_same.txt").to_str().expect("path"),
        ])
        .status()
        .expect("exec");
    assert_eq!(status_text_same.code(), Some(0));

    // 3. Hex/Binary difference with -bcexitcode -> 11 (binary difference)
    let status_hex_diff = Command::new(bin_path())
        .args([
            "-bcexitcode",
            "hex",
            fx.join("h_left.bin").to_str().expect("path"),
            fx.join("h_right.bin").to_str().expect("path"),
        ])
        .status()
        .expect("exec");
    assert_eq!(status_hex_diff.code(), Some(11));

    // 4. Hex/Binary identical with -bcexitcode -> 0 (success)
    let status_hex_same = Command::new(bin_path())
        .args([
            "-bcexitcode",
            "hex",
            fx.join("h_left.bin").to_str().expect("path"),
            fx.join("h_left.bin").to_str().expect("path"),
        ])
        .status()
        .expect("exec");
    assert_eq!(status_hex_same.code(), Some(0));

    // 5. Merge with conflicts with -bcexitcode -> 14 (conflicts)
    let out_conflict = tmp.path().join("bc_conflict_out.txt");
    let status_merge_conflict = Command::new(bin_path())
        .args([
            "-bcexitcode",
            "merge",
            fx.join("m_base.txt").to_str().expect("path"),
            fx.join("m_left.txt").to_str().expect("path"),
            fx.join("m_right.txt").to_str().expect("path"),
            "-mergeoutput",
            out_conflict.to_str().expect("path"),
        ])
        .status()
        .expect("exec");
    assert_eq!(status_merge_conflict.code(), Some(14));

    // 6. Merge without conflicts with -bcexitcode -> 0 (clean)
    let out_clean = tmp.path().join("bc_clean_out.txt");
    let status_merge_clean = Command::new(bin_path())
        .args([
            "-bcexitcode",
            "merge",
            fx.join("m_base.txt").to_str().expect("path"),
            fx.join("m_base.txt").to_str().expect("path"),
            fx.join("m_base.txt").to_str().expect("path"),
            "-mergeoutput",
            out_clean.to_str().expect("path"),
        ])
        .status()
        .expect("exec");
    assert_eq!(status_merge_clean.code(), Some(0));
}

#[test]
fn test_bcomp_vcs_titles_and_at_session_syntax() {
    let fx = fixtures_dir();
    let tmp = tempdir().expect("tempdir");

    // Title flags -title1..4 and --report-title
    let report_out = tmp.path().join("report.html");
    let status = Command::new(bin_path())
        .args([
            "-title1=Local_Branch",
            "-title2=Remote_Branch",
            "--report-title=Custom Comparison",
            "--report",
            &format!("html:{}", report_out.display()),
            "text",
            fx.join("t_left.txt").to_str().expect("path"),
            fx.join("t_right.txt").to_str().expect("path"),
        ])
        .status()
        .expect("exec");
    assert_eq!(status.code(), Some(1));
    assert!(report_out.exists());
    let rep_content = std::fs::read_to_string(&report_out).expect("read");
    assert!(rep_content.contains("Custom Comparison"));

    // Save session then invoke via @<path> syntax
    let session_file = tmp.path().join("at_session.json");
    let status_save = Command::new(bin_path())
        .args([
            "text",
            fx.join("t_left.txt").to_str().expect("path"),
            fx.join("t_right.txt").to_str().expect("path"),
            "-savesession",
            session_file.to_str().expect("path"),
        ])
        .status()
        .expect("exec");
    assert_eq!(status_save.code(), Some(1));
    assert!(session_file.exists());

    let status_at = Command::new(bin_path())
        .arg(format!("@{}", session_file.display()))
        .status()
        .expect("exec");
    assert_eq!(status_at.code(), Some(1));
}
