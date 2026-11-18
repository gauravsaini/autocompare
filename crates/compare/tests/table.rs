//! Tests for delimited table file comparison: error mapping, the
//! round-trip of parsed headers and rows, delimiters, and lenient
//! decoding of invalid bytes.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use autocompare_compare::{Error, TableOptions, compare_table_files};
use autocompare_diff::Op;
use common::{cleanup, temp_root, write, write_bytes};

#[test]
fn missing_file_is_missing_error() {
    let base = temp_root();
    let left = base.join("left.csv");
    write(&left, "a\n");
    // Right side missing.
    let err = compare_table_files(&left, &base.join("no-right.csv"), &TableOptions::default())
        .err()
        .expect("missing right file must fail");
    assert!(matches!(err, Error::Missing { .. }));
    assert!(err.to_string().contains("no-right.csv"));
    // Left side missing.
    let err = compare_table_files(&base.join("no-left.csv"), &left, &TableOptions::default())
        .err()
        .expect("missing left file must fail");
    assert!(matches!(err, Error::Missing { .. }));
    assert!(err.to_string().contains("no-left.csv"));
    cleanup(&base);
}

#[test]
fn identical_files_compare_equal_with_equal_only_script() {
    let base = temp_root();
    let text = "name,score\nalice,9\nbob,7\n";
    let left = base.join("left.csv");
    let right = base.join("right.csv");
    write(&left, text);
    write(&right, text);
    let cmp = compare_table_files(&left, &right, &TableOptions::default())
        .expect("readable files must compare");
    assert_eq!(cmp.header_l, vec!["name", "score"]);
    assert_eq!(cmp.header_r, cmp.header_l);
    assert_eq!(cmp.rows_l, vec![vec!["alice", "9"], vec!["bob", "7"]]);
    assert_eq!(cmp.rows_r, cmp.rows_l);
    assert!(cmp.script.exact);
    assert!(
        cmp.script
            .ops
            .iter()
            .all(|op| matches!(op, Op::Equal { .. })),
        "identical rows must produce no Delete or Insert ops"
    );
    cleanup(&base);
}

#[test]
fn quoted_multiline_field_round_trips() {
    let base = temp_root();
    let text = "name,note\nalice,\"line one\nline two\"\n";
    let left = base.join("left.csv");
    let right = base.join("right.csv");
    write(&left, text);
    write(&right, text);
    let cmp = compare_table_files(&left, &right, &TableOptions::default())
        .expect("readable files must compare");
    assert_eq!(cmp.rows_l, vec![vec!["alice", "line one\nline two"]]);
    assert_eq!(cmp.rows_r, cmp.rows_l);
    assert!(cmp.script.exact);
    cleanup(&base);
}

#[test]
fn different_rows_produce_delete_or_insert() {
    let base = temp_root();
    let left = base.join("left.csv");
    let right = base.join("right.csv");
    write(&left, "h\na\nb\n");
    write(&right, "h\na\nx\n");
    let cmp = compare_table_files(&left, &right, &TableOptions::default())
        .expect("readable files must compare");
    assert_eq!(cmp.rows_l, vec![vec!["a"], vec!["b"]]);
    assert_eq!(cmp.rows_r, vec![vec!["a"], vec!["x"]]);
    assert!(
        cmp.script
            .ops
            .iter()
            .any(|op| matches!(op, Op::Delete { .. } | Op::Insert { .. })),
        "a changed row must show up as Delete or Insert"
    );
    cleanup(&base);
}

#[test]
fn differing_headers_are_exposed_separately() {
    let base = temp_root();
    let left = base.join("left.csv");
    let right = base.join("right.csv");
    write(&left, "a,b\n1,2\n");
    write(&right, "a,B\n1,2\n");
    let cmp = compare_table_files(&left, &right, &TableOptions::default())
        .expect("readable files must compare");
    assert_eq!(cmp.header_l, vec!["a", "b"]);
    assert_eq!(cmp.header_r, vec!["a", "B"]);
    // Data rows match, so the row script stays Equal-only.
    assert!(
        cmp.script
            .ops
            .iter()
            .all(|op| matches!(op, Op::Equal { .. })),
        "headers take no part in the row script"
    );
    cleanup(&base);
}

#[test]
fn custom_delimiter_via_options() {
    let base = temp_root();
    let left = base.join("left.csv");
    let right = base.join("right.csv");
    write(&left, "x;y\n1;2\n");
    write(&right, "x;y\n1;2\n");
    let opts = TableOptions {
        delimiter: ';',
        header: true,
    };
    let cmp = compare_table_files(&left, &right, &opts).expect("readable files must compare");
    assert_eq!(cmp.header_l, vec!["x", "y"]);
    assert_eq!(cmp.rows_l, vec![vec!["1", "2"]]);
    // With tabs the comma is no longer structural.
    write(&left, "a,b\tc\n");
    write(&right, "a,b\tc\n");
    let opts = TableOptions {
        delimiter: '\t',
        header: false,
    };
    let cmp = compare_table_files(&left, &right, &opts).expect("readable files must compare");
    assert!(cmp.header_l.is_empty());
    assert_eq!(cmp.rows_l, vec![vec!["a,b", "c"]]);
    cleanup(&base);
}

#[test]
fn invalid_utf8_decodes_leniently_into_cells() {
    let base = temp_root();
    let left = base.join("left.csv");
    let right = base.join("right.csv");
    write_bytes(&left, b"h1,h2\n\xff\xfe,x\n");
    write_bytes(&right, b"h1,h2\n\xff\xfe,x\n");
    let cmp = compare_table_files(&left, &right, &TableOptions::default())
        .expect("invalid UTF-8 must not fail");
    assert_eq!(cmp.header_l, vec!["h1", "h2"]);
    let row = cmp.rows_l.first().expect("one data row");
    let cell = row.first().expect("first cell exists");
    assert!(
        cell.contains('\u{FFFD}'),
        "lossy decode must surface a replacement char, got {cell:?}"
    );
    assert_eq!(cell.chars().count(), 2, "\\xff\\xfe each become one U+FFFD");
    assert_eq!(cmp.rows_r, cmp.rows_l);
    assert!(cmp.script.exact);
    cleanup(&base);
}

#[test]
fn directory_input_is_io_error() {
    let base = temp_root();
    let dir = base.join("adir");
    std::fs::create_dir_all(&dir).expect("failed to create the dir fixture");
    let file = base.join("f.csv");
    write(&file, "a\n");
    let err = compare_table_files(&dir, &file, &TableOptions::default())
        .err()
        .expect("a directory must fail to compare");
    assert!(matches!(err, Error::Io { .. }));
    assert!(err.to_string().contains("adir"));
    cleanup(&base);
}
