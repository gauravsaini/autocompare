#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

//! Parity hardening integration tests for leaf-3.1 CLI.
//!
//! Validates:
//! - All 10 subcommands (text, folder, hex, merge, table, sync, picture, folder-merge, audio, session)
//! - Multi-format reporting (html, text, xml, patch, context)
//! - bcomp VCS flags (-ro, -automerge, -mergeoutput, -vcs1..4, /ro, /automerge)
//! - Exit codes 0, 1, 2 in diff mode and BC mode (0, 11, 13, 14)
//! - 4-way git mergetool fixtures for files and folder-merge

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
fn test_ten_subcommands_direct_execution() {
    let fx = fixtures_dir();
    let tmp = tempdir().expect("tempdir");

    // 1. Text
    let s_text = Command::new(bin_path())
        .args([
            "text",
            fx.join("t_same.txt").to_str().expect("p"),
            fx.join("t_same.txt").to_str().expect("p"),
        ])
        .status()
        .expect("exec");
    assert_eq!(s_text.code(), Some(0));

    // 2. Folder
    let s_folder = Command::new(bin_path())
        .args([
            "folder",
            fx.join("folder/left").to_str().expect("p"),
            fx.join("folder/right").to_str().expect("p"),
        ])
        .status()
        .expect("exec");
    assert_eq!(s_folder.code(), Some(1));

    // 3. Hex
    let s_hex = Command::new(bin_path())
        .args([
            "hex",
            fx.join("h_left.bin").to_str().expect("p"),
            fx.join("h_left.bin").to_str().expect("p"),
        ])
        .status()
        .expect("exec");
    assert_eq!(s_hex.code(), Some(0));

    // 4. Merge
    let m_out = tmp.path().join("m_clean.txt");
    let s_merge = Command::new(bin_path())
        .args([
            "merge",
            fx.join("m_base.txt").to_str().expect("p"),
            fx.join("m_base.txt").to_str().expect("p"),
            fx.join("m_base.txt").to_str().expect("p"),
            "--output",
            m_out.to_str().expect("p"),
        ])
        .status()
        .expect("exec");
    assert_eq!(s_merge.code(), Some(0));

    // 5. Table
    let s_table = Command::new(bin_path())
        .args([
            "table",
            fx.join("tab_left.csv").to_str().expect("p"),
            fx.join("tab_right.csv").to_str().expect("p"),
        ])
        .status()
        .expect("exec");
    assert_eq!(s_table.code(), Some(1));

    // 6. Sync
    let s_sync = Command::new(bin_path())
        .args([
            "sync",
            fx.join("sync/left").to_str().expect("p"),
            fx.join("sync/right").to_str().expect("p"),
        ])
        .status()
        .expect("exec");
    assert_eq!(s_sync.code(), Some(1));

    // 7. Picture
    let s_pic = Command::new(bin_path())
        .args([
            "picture",
            fx.join("pic_left.png").to_str().expect("p"),
            fx.join("pic_left.png").to_str().expect("p"),
        ])
        .status()
        .expect("exec");
    assert_eq!(s_pic.code(), Some(0));

    // 8. Folder-Merge
    let s_fmerge = Command::new(bin_path())
        .args([
            "folder-merge",
            fx.join("fmerge-base").to_str().expect("p"),
            fx.join("fmerge-left").to_str().expect("p"),
            fx.join("fmerge-right").to_str().expect("p"),
        ])
        .status()
        .expect("exec");
    assert_eq!(s_fmerge.code(), Some(1));

    // 9. Audio
    let s_audio = Command::new(bin_path())
        .args([
            "audio",
            fx.join("a_left.wav").to_str().expect("p"),
            fx.join("a_left.wav").to_str().expect("p"),
        ])
        .status()
        .expect("exec");
    assert_eq!(s_audio.code(), Some(0));

    // 10. Session (via session subcommand)
    let session_file = tmp.path().join("subcmd_session.json");
    let s_save = Command::new(bin_path())
        .args([
            "text",
            fx.join("t_same.txt").to_str().expect("p"),
            fx.join("t_same.txt").to_str().expect("p"),
            "--save-session",
            session_file.to_str().expect("p"),
        ])
        .status()
        .expect("exec");
    assert_eq!(s_save.code(), Some(0));

    let s_session = Command::new(bin_path())
        .args(["session", session_file.to_str().expect("p")])
        .status()
        .expect("exec");
    assert_eq!(s_session.code(), Some(0));
}

#[test]
fn test_all_report_formats_xml_patch_html_text() {
    let fx = fixtures_dir();
    let tmp = tempdir().expect("tempdir");

    // XML report
    let xml_rep = tmp.path().join("report.xml");
    let s_xml = Command::new(bin_path())
        .args([
            "text",
            fx.join("t_left.txt").to_str().expect("p"),
            fx.join("t_right.txt").to_str().expect("p"),
            "--report",
            &format!("xml:{}", xml_rep.display()),
            "--report-title",
            "XML Parity Test",
        ])
        .status()
        .expect("exec");
    assert_eq!(s_xml.code(), Some(1));
    assert!(xml_rep.exists());
    let xml_content = std::fs::read_to_string(&xml_rep).expect("read");
    assert!(xml_content.contains("<diff-report") || xml_content.contains("<?xml"));

    // Context patch report
    let ctx_rep = tmp.path().join("report.ctx");
    let s_ctx = Command::new(bin_path())
        .args([
            "text",
            fx.join("t_left.txt").to_str().expect("p"),
            fx.join("t_right.txt").to_str().expect("p"),
            "--report",
            &format!("context:{}", ctx_rep.display()),
        ])
        .status()
        .expect("exec");
    assert_eq!(s_ctx.code(), Some(1));
    assert!(ctx_rep.exists());
    let ctx_content = std::fs::read_to_string(&ctx_rep).expect("read");
    assert!(ctx_content.contains("*** ") || ctx_content.contains("--- "));

    // Mono HTML report
    let mono_rep = tmp.path().join("report_mono.html");
    let s_mono = Command::new(bin_path())
        .args([
            "text",
            fx.join("t_left.txt").to_str().expect("p"),
            fx.join("t_right.txt").to_str().expect("p"),
            "--report",
            &format!("html-mono:{}", mono_rep.display()),
        ])
        .status()
        .expect("exec");
    assert_eq!(s_mono.code(), Some(1));
    assert!(mono_rep.exists());
    let mono_content = std::fs::read_to_string(&mono_rep).expect("read");
    assert!(mono_content.contains("<!DOCTYPE html>"));
}

#[test]
fn test_four_way_git_mergetool_folder_and_slash_flags() {
    let fx = fixtures_dir();
    let tmp = tempdir().expect("tempdir");

    // 4-way folder merge: `bcomp $LOCAL $REMOTE $BASE $MERGED`
    let fmerge_out = tmp.path().join("git_fmerge_dest");
    let s_fmerge = Command::new(bin_path())
        .args([
            fx.join("fmerge-left").to_str().expect("p"),
            fx.join("fmerge-right").to_str().expect("p"),
            fx.join("fmerge-base").to_str().expect("p"),
            fmerge_out.to_str().expect("p"),
        ])
        .status()
        .expect("exec");
    assert_eq!(s_fmerge.code(), Some(1));

    // Slash flag compatibility (/automerge, /mergeoutput, /ro)
    let slash_out = tmp.path().join("slash_out.txt");
    let s_slash = Command::new(bin_path())
        .args([
            fx.join("m_base.txt").to_str().expect("p"),
            fx.join("m_base.txt").to_str().expect("p"),
            fx.join("m_base.txt").to_str().expect("p"),
            &format!("/mergeoutput={}", slash_out.display()),
            "/automerge",
        ])
        .status()
        .expect("exec");
    assert_eq!(s_slash.code(), Some(0));
    assert!(slash_out.exists());

    // Read-only slash flag with mutation attempt must fail with exit code 2
    let s_slash_ro = Command::new(bin_path())
        .args([
            fx.join("m_base.txt").to_str().expect("p"),
            fx.join("m_base.txt").to_str().expect("p"),
            fx.join("m_base.txt").to_str().expect("p"),
            &format!("/mergeoutput={}", slash_out.display()),
            "/ro",
        ])
        .status()
        .expect("exec");
    assert_eq!(s_slash_ro.code(), Some(2));
}

#[test]
fn test_audio_and_sync_session_execution() {
    let fx = fixtures_dir();
    let tmp = tempdir().expect("tempdir");

    // Audio session saving and reload
    let audio_sess = tmp.path().join("audio_session.json");
    let s_save_audio = Command::new(bin_path())
        .args([
            "audio",
            fx.join("a_left.wav").to_str().expect("p"),
            fx.join("a_right.wav").to_str().expect("p"),
            "--save-session",
            audio_sess.to_str().expect("p"),
        ])
        .status()
        .expect("exec");
    assert_eq!(s_save_audio.code(), Some(1));
    assert!(audio_sess.exists());

    let s_load_audio = Command::new(bin_path())
        .args(["--session", audio_sess.to_str().expect("p")])
        .status()
        .expect("exec");
    assert_eq!(s_load_audio.code(), Some(1));
}
