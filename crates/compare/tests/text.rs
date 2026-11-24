//! Tests for text loading and text file comparison. Script-content
//! assertions are deliberately structural only: `autocompare-diff` is
//! still a stub, so these tests must not depend on diff outcomes.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use autocompare_compare::{Error, TextOptions, compare_text_files, load_lines};
use common::{cleanup, temp_root, write, write_bytes};

#[test]
fn load_lines_missing_file_is_missing() {
    let base = temp_root();
    let err = load_lines(&base.join("no-such.txt")).expect_err("missing file must fail");
    assert!(matches!(err, Error::Missing { .. }));
    assert!(err.to_string().contains("no-such.txt"));
    cleanup(&base);
}

#[test]
fn load_lines_splits_lines_and_strips_cr() {
    let base = temp_root();
    let path = base.join("crlf.txt");
    write(&path, "one\r\ntwo\r\nthree\r");
    let lines = load_lines(&path).expect("readable file must load");
    assert_eq!(lines, vec!["one", "two", "three"]);
    cleanup(&base);
}

#[test]
fn load_lines_trailing_newline_has_no_phantom_empty_line() {
    let base = temp_root();
    let path = base.join("nl.txt");
    write(&path, "a\nb\n");
    let lines = load_lines(&path).expect("readable file must load");
    assert_eq!(lines, vec!["a", "b"]);
    cleanup(&base);
}

#[test]
fn load_lines_without_trailing_newline_keeps_last_line() {
    let base = temp_root();
    let path = base.join("no-nl.txt");
    write(&path, "a\nb");
    let lines = load_lines(&path).expect("readable file must load");
    assert_eq!(lines, vec!["a", "b"]);
    cleanup(&base);
}

#[test]
fn load_lines_empty_file_is_zero_lines() {
    let base = temp_root();
    let path = base.join("empty.txt");
    write(&path, "");
    let lines = load_lines(&path).expect("readable file must load");
    assert!(lines.is_empty());
    cleanup(&base);
}

#[test]
fn load_lines_lone_newline_is_one_empty_line() {
    let base = temp_root();
    let path = base.join("blank.txt");
    write(&path, "\n");
    let lines = load_lines(&path).expect("readable file must load");
    assert_eq!(lines, vec![""]);
    cleanup(&base);
}

#[test]
fn load_lines_invalid_utf8_decodes_leniently() {
    let base = temp_root();
    let path = base.join("bad.bin");
    write_bytes(&path, b"ok\xff\xfeline\n");
    let lines = load_lines(&path).expect("invalid UTF-8 must not fail");
    assert_eq!(lines, vec!["ok\u{FFFD}\u{FFFD}line"]);
    cleanup(&base);
}

#[test]
fn load_lines_directory_is_io_error() {
    let base = temp_root();
    let dir = base.join("adir");
    std::fs::create_dir_all(&dir).expect("failed to create the dir fixture");
    let err = load_lines(&dir).expect_err("a directory must fail to load as text");
    assert!(matches!(err, Error::Io { .. }));
    assert!(err.to_string().contains("adir"));
    cleanup(&base);
}

#[test]
fn compare_text_files_missing_error_mentions_the_path() {
    let base = temp_root();
    let left = base.join("left.txt");
    write(&left, "a\n");
    let err = compare_text_files(
        &left,
        &base.join("right-missing.txt"),
        &TextOptions::default(),
    )
    .err()
    .expect("missing right file must fail");
    assert!(matches!(err, Error::Missing { .. }));
    assert!(err.to_string().contains("right-missing.txt"));
    cleanup(&base);
}

#[test]
fn compare_text_files_display_lines_keep_original_case() {
    let base = temp_root();
    let left = base.join("left.txt");
    let right = base.join("right.txt");
    write(&left, "Hello   World\nSecond line\n");
    write(&right, "hello world\n  second  line \n");
    let opts = TextOptions {
        ignore_case: true,
        ignore_space_change: true,
        ..Default::default()
    };
    let cmp = compare_text_files(&left, &right, &opts).expect("readable files must compare");
    // The returned lines are the originals; keys only drive the script.
    assert_eq!(cmp.left_lines, vec!["Hello   World", "Second line"]);
    assert_eq!(cmp.right_lines, vec!["hello world", "  second  line "]);
    // Structural only: the stub reports an empty script, a real diff
    // over equal keys is exact.
    assert!(cmp.script.exact || cmp.script.ops.is_empty());
    cleanup(&base);
}

#[test]
fn compare_text_files_identical_files_report_display_lines() {
    let base = temp_root();
    let left = base.join("left.txt");
    let right = base.join("right.txt");
    write(&left, "alpha\nbeta gamma\n");
    write(&right, "alpha\nbeta gamma\n");
    let cmp = compare_text_files(&left, &right, &TextOptions::default())
        .expect("readable files must compare");
    assert_eq!(cmp.left_lines, vec!["alpha", "beta gamma"]);
    assert_eq!(cmp.right_lines, cmp.left_lines);
    assert!(cmp.script.exact || cmp.script.ops.is_empty());
    cleanup(&base);
}

#[test]
fn compare_text_files_different_files_do_not_panic() {
    let base = temp_root();
    let left = base.join("left.txt");
    let right = base.join("right.txt");
    write(&left, "a\nb\nc\n");
    write(&right, "a\nx\nc\n");
    let cmp = compare_text_files(&left, &right, &TextOptions::default())
        .expect("readable files must compare");
    assert_eq!(cmp.left_lines, vec!["a", "b", "c"]);
    assert_eq!(cmp.right_lines, vec!["a", "x", "c"]);
    assert!(cmp.script.exact || cmp.script.ops.is_empty());
    cleanup(&base);
}
