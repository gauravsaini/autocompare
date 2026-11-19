//! Tests for byte-level file comparison.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use autocompare_compare::{
    Error, HexAlignment, compare_binary_files, compare_binary_files_aligned,
};
use common::{cleanup, temp_root, write_bytes};
use std::path::PathBuf;

/// A `len`-byte vector of 'x' with 'z' at each offset in `diffs`.
fn corpus(len: usize, diffs: &[usize]) -> Vec<u8> {
    (0..len)
        .map(|i| if diffs.contains(&i) { b'z' } else { b'x' })
        .collect()
}

#[test]
fn compare_binary_identical_files() {
    let base = temp_root();
    // Larger than one 64 KiB chunk to cross chunk boundaries.
    let data = corpus(100_000, &[]);
    let left = base.join("l.bin");
    let right = base.join("r.bin");
    write_bytes(&left, &data);
    write_bytes(&right, &data);
    let cmp = compare_binary_files(&left, &right, 4).expect("identical files must compare");
    assert_eq!(cmp.first_diff, None);
    assert!(cmp.hunks.is_empty());
    assert!(!cmp.truncated);
    assert_eq!(cmp.left_size, 100_000);
    assert_eq!(cmp.right_size, 100_000);
    cleanup(&base);
}

#[test]
fn compare_binary_single_difference_hunks_64_bytes() {
    let base = temp_root();
    let left = corpus(200, &[]);
    let right = corpus(200, &[10]);
    let left_path = base.join("l.bin");
    let right_path = base.join("r.bin");
    write_bytes(&left_path, &left);
    write_bytes(&right_path, &right);
    let cmp = compare_binary_files(&left_path, &right_path, 4).expect("files must compare");
    assert_eq!(cmp.first_diff, Some(10));
    assert_eq!(cmp.hunks.len(), 1);
    let hunk = cmp.hunks.first().expect("exactly one hunk");
    assert_eq!(hunk.offset, 10);
    let want_left: Vec<u8> = left.iter().skip(10).take(64).copied().collect();
    let want_right: Vec<u8> = right.iter().skip(10).take(64).copied().collect();
    assert_eq!(hunk.left, want_left);
    assert_eq!(hunk.right, want_right);
    assert!(!cmp.truncated, "no second difference exists");
    cleanup(&base);
}

#[test]
fn compare_binary_hunk_at_end_is_clipped_to_file() {
    let base = temp_root();
    let left = corpus(30, &[]);
    let right = corpus(30, &[25]);
    let left_path = base.join("l.bin");
    let right_path = base.join("r.bin");
    write_bytes(&left_path, &left);
    write_bytes(&right_path, &right);
    let cmp = compare_binary_files(&left_path, &right_path, 4).expect("files must compare");
    assert_eq!(cmp.first_diff, Some(25));
    let hunk = cmp.hunks.first().expect("exactly one hunk");
    assert_eq!(hunk.offset, 25);
    assert_eq!(hunk.left.len(), 5, "only the bytes left in the file");
    assert_eq!(hunk.right.len(), 5);
    assert!(!cmp.truncated);
    cleanup(&base);
}

#[test]
fn compare_binary_hunks_cap_truncates() {
    let base = temp_root();
    let left = corpus(400, &[]);
    let right = corpus(400, &[0, 100, 200, 300]);
    let left_path = base.join("l.bin");
    let right_path = base.join("r.bin");
    write_bytes(&left_path, &left);
    write_bytes(&right_path, &right);
    let cmp = compare_binary_files(&left_path, &right_path, 2).expect("files must compare");
    assert_eq!(cmp.hunks.len(), 2);
    assert_eq!(cmp.hunks.first().map(|h| h.offset), Some(0));
    assert_eq!(cmp.hunks.get(1).map(|h| h.offset), Some(100));
    assert!(cmp.truncated, "a third difference exists beyond the cap");
    // With capacity for all differences nothing is withheld.
    let full = compare_binary_files(&left_path, &right_path, 4).expect("files must compare");
    assert_eq!(full.hunks.len(), 4);
    assert!(!full.truncated);
    cleanup(&base);
}

#[test]
fn compare_binary_hunks_do_not_overlap() {
    let base = temp_root();
    let left = corpus(400, &[]);
    // The diffs at 5 and 30 both fall inside the first hunk's 64-byte
    // span; the scan resumes at 5 + 64 and the next hunk is at 70.
    let right = corpus(400, &[5, 30, 70]);
    let left_path = base.join("l.bin");
    let right_path = base.join("r.bin");
    write_bytes(&left_path, &left);
    write_bytes(&right_path, &right);
    let cmp = compare_binary_files(&left_path, &right_path, 4).expect("files must compare");
    assert_eq!(cmp.first_diff, Some(5));
    let offsets: Vec<u64> = cmp.hunks.iter().map(|h| h.offset).collect();
    assert_eq!(offsets, vec![5, 70]);
    assert!(!cmp.truncated);
    cleanup(&base);
}

#[test]
fn compare_binary_zero_max_hunks_still_finds_first_diff() {
    let base = temp_root();
    let left = corpus(100, &[]);
    let right = corpus(100, &[50]);
    let left_path = base.join("l.bin");
    let right_path = base.join("r.bin");
    write_bytes(&left_path, &left);
    write_bytes(&right_path, &right);
    let cmp = compare_binary_files(&left_path, &right_path, 0).expect("files must compare");
    assert_eq!(cmp.first_diff, Some(50));
    assert!(cmp.hunks.is_empty());
    assert!(
        cmp.truncated,
        "a difference exists but no hunks were allowed"
    );
    cleanup(&base);
}

#[test]
fn compare_binary_prefix_file_reports_shorter_length() {
    let base = temp_root();
    let left_path = base.join("l.bin");
    let right_path = base.join("r.bin");
    write_bytes(&left_path, b"abcdef");
    write_bytes(&right_path, b"abcdefgh");
    let cmp = compare_binary_files(&left_path, &right_path, 3).expect("files must compare");
    assert_eq!(cmp.first_diff, Some(6));
    assert!(cmp.hunks.is_empty(), "a pure prefix produces no hunks");
    assert_eq!(cmp.left_size, 6);
    assert_eq!(cmp.right_size, 8);
    assert!(!cmp.truncated);
    // Documented edge: with max_hunks 0 any difference — including a
    // length-only one — is marked truncated.
    let capped = compare_binary_files(&left_path, &right_path, 0).expect("files must compare");
    assert_eq!(capped.first_diff, Some(6));
    assert!(capped.hunks.is_empty());
    assert!(capped.truncated);
    cleanup(&base);
}

#[test]
fn compare_binary_directory_is_io_error() {
    let base = temp_root();
    let dir = base.join("adir");
    std::fs::create_dir_all(&dir).expect("failed to create the dir fixture");
    let file: PathBuf = base.join("f.bin");
    write_bytes(&file, b"data");
    let err = compare_binary_files(&dir, &file, 4).expect_err("a directory must fail to compare");
    assert!(matches!(err, Error::Io { .. }));
    assert!(err.to_string().contains("adir"));
    cleanup(&base);
}

#[test]
fn compare_binary_missing_file_is_missing_error() {
    let base = temp_root();
    let file = base.join("exists.bin");
    write_bytes(&file, b"x");
    let err =
        compare_binary_files(&base.join("nope.bin"), &file, 4).expect_err("missing file must fail");
    assert!(matches!(err, Error::Missing { .. }));
    assert!(err.to_string().contains("nope.bin"));
    cleanup(&base);
}

#[test]
fn aligned_none_matches_positional_scan() {
    let base = temp_root();
    let left_path = base.join("l.bin");
    let right_path = base.join("r.bin");
    write_bytes(&left_path, b"abcdef");
    write_bytes(&right_path, b"abcXef");
    let cmp = compare_binary_files_aligned(&left_path, &right_path, 4, HexAlignment::None)
        .expect("files must compare");
    assert_eq!(cmp.first_diff, Some(3));
    assert_eq!(cmp.hunks.len(), 1);
    assert!(!cmp.truncated);
    cleanup(&base);
}

#[test]
fn aligned_complete_finds_insertion_as_gap() {
    let base = temp_root();
    let left_path = base.join("l.bin");
    let right_path = base.join("r.bin");
    write_bytes(&left_path, b"AAAA12345678BBBB");
    write_bytes(&right_path, b"AAAA____12345678BBBB");
    // Complete alignment resynchronises: the 4 inserted bytes are a
    // right-only gap hunk, not a wall of positional differences.
    let cmp = compare_binary_files_aligned(&left_path, &right_path, 4, HexAlignment::Complete)
        .expect("files must compare");
    assert_eq!(cmp.first_diff, Some(4));
    assert!(
        cmp.hunks
            .iter()
            .any(|h| h.left.is_empty() && h.right == b"____".to_vec() && h.offset == 4),
        "inserted bytes must surface as one right-only hunk, got {:?}",
        cmp.hunks
    );
    assert!(!cmp.truncated);
    // None alignment sees the same pair as shifted positional noise
    // plus a tail gap (the right file is 4 bytes longer).
    let flat = compare_binary_files_aligned(&left_path, &right_path, 4, HexAlignment::None)
        .expect("files must compare");
    assert_eq!(flat.first_diff, Some(4));
    assert!(
        flat.hunks
            .iter()
            .any(|h| !h.left.is_empty() && !h.right.is_empty() && h.offset == 4),
        "None mode must show a positional Different hunk at 4, got {:?}",
        flat.hunks
    );
    assert!(
        flat.hunks
            .iter()
            .filter(|h| h.left.is_empty())
            .all(|h| h.offset == 16),
        "None mode gaps are tail-only, got {:?}",
        flat.hunks
    );
    cleanup(&base);
}

#[test]
fn aligned_fast_resyncs_on_anchor() {
    let base = temp_root();
    let left_path = base.join("l.bin");
    let right_path = base.join("r.bin");
    write_bytes(&left_path, b"AAAA12345678BBBB");
    write_bytes(&right_path, b"AAAA____12345678BBBB");
    let cmp = compare_binary_files_aligned(&left_path, &right_path, 8, HexAlignment::Fast)
        .expect("files must compare");
    assert_eq!(cmp.first_diff, Some(4));
    assert!(
        cmp.hunks.iter().any(|h| h.left.is_empty()),
        "Fast mode must resync into a right-only gap, got {:?}",
        cmp.hunks
    );
    cleanup(&base);
}

#[test]
fn aligned_identical_and_truncation() {
    let base = temp_root();
    let left_path = base.join("l.bin");
    let right_path = base.join("r.bin");
    let data = corpus(300, &[]);
    write_bytes(&left_path, &data);
    write_bytes(&right_path, &data);
    for mode in [
        HexAlignment::None,
        HexAlignment::Fast,
        HexAlignment::Complete,
    ] {
        let cmp = compare_binary_files_aligned(&left_path, &right_path, 4, mode)
            .expect("identical files must compare");
        assert_eq!(cmp.first_diff, None);
        assert!(cmp.hunks.is_empty());
        assert!(!cmp.truncated);
    }
    // Zero hunk budget with a difference is truncated in every mode.
    write_bytes(&right_path, &corpus(300, &[50]));
    for mode in [
        HexAlignment::None,
        HexAlignment::Fast,
        HexAlignment::Complete,
    ] {
        let cmp = compare_binary_files_aligned(&left_path, &right_path, 0, mode)
            .expect("files must compare");
        assert_eq!(cmp.first_diff, Some(50));
        assert!(cmp.hunks.is_empty());
        assert!(cmp.truncated);
    }
    cleanup(&base);
}

#[test]
fn aligned_missing_file_is_missing_error() {
    let base = temp_root();
    let file = base.join("exists.bin");
    write_bytes(&file, b"x");
    let err =
        compare_binary_files_aligned(&base.join("nope.bin"), &file, 4, HexAlignment::Complete)
            .expect_err("missing file must fail");
    assert!(matches!(err, Error::Missing { .. }));
    cleanup(&base);
}
