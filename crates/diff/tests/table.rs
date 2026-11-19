//! Tests for delimited table parsing and cell/row comparison. The
//! parser is total, so every hostile input here asserts the exact
//! lenient outcome documented on `parse_delimited`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use autocompare_diff::{
    DiffScript, Op, Table, TableCompareOptions, cells_equal_with_tolerance,
    detect_key_columns, diff_cells, diff_cells_with_options, diff_rows,
    diff_table_by_key, diff_table_by_key_with_options, parse_delimited, sort_table_rows,
};

/// Shorthand for expected cells: `cells(&["a", "b"])`.
fn cells(spec: &[&str]) -> Vec<String> {
    spec.iter().map(|c| c.to_string()).collect()
}

/// Replays a row script over both sides, asserting full consumption.
fn replay_rows(
    a: &[Vec<String>],
    b: &[Vec<String>],
    script: &DiffScript,
) -> (Vec<Vec<String>>, Vec<Vec<String>>) {
    let mut ra: Vec<Vec<String>> = Vec::new();
    let mut rb: Vec<Vec<String>> = Vec::new();
    let (mut ai, mut bi) = (0usize, 0usize);
    for op in &script.ops {
        match op {
            Op::Equal { len } => {
                for _ in 0..*len {
                    ra.push(a.get(ai).cloned().unwrap_or_default());
                    rb.push(b.get(bi).cloned().unwrap_or_default());
                    ai += 1;
                    bi += 1;
                }
            }
            Op::Delete { len } => {
                for _ in 0..*len {
                    ra.push(a.get(ai).cloned().unwrap_or_default());
                    ai += 1;
                }
            }
            Op::Insert { len } => {
                for _ in 0..*len {
                    rb.push(b.get(bi).cloned().unwrap_or_default());
                    bi += 1;
                }
            }
        }
    }
    assert_eq!(ai, a.len(), "script must consume all rows of the left side");
    assert_eq!(
        bi,
        b.len(),
        "script must consume all rows of the right side"
    );
    (ra, rb)
}

// ---------------------------------------------------------------------
// parse_delimited: the happy path
// ---------------------------------------------------------------------

#[test]
fn simple_csv_header_and_one_row() {
    let t = parse_delimited("a,b\n1,2\n", ',', true);
    assert_eq!(t.header, cells(&["a", "b"]));
    assert_eq!(t.rows, vec![cells(&["1", "2"])]);
}

#[test]
fn quoted_fields_keep_delimiter_and_embedded_newline() {
    let text = "\"x,y\",z\n\"line\nbreak\",2\n";
    let t = parse_delimited(text, ',', false);
    assert_eq!(
        t.rows,
        vec![cells(&["x,y", "z"]), cells(&["line\nbreak", "2"])]
    );
}

#[test]
fn escaped_quotes_double_to_one() {
    let t = parse_delimited("\"say \"\"hi\"\"\",n\n", ',', false);
    assert_eq!(t.rows, vec![cells(&["say \"hi\"", "n"])]);
}

// ---------------------------------------------------------------------
// parse_delimited: row breaks, endings and empties
// ---------------------------------------------------------------------

#[test]
fn crlf_everywhere_parses_like_lf() {
    let t = parse_delimited("h1,h2\r\nv1,v2\r\n", ',', true);
    assert_eq!(t.header, cells(&["h1", "h2"]));
    assert_eq!(t.rows, vec![cells(&["v1", "v2"])]);
}

#[test]
fn trailing_newline_creates_no_phantom_row() {
    assert_eq!(
        parse_delimited("a,b\n", ',', false).rows,
        vec![cells(&["a", "b"])]
    );
    assert_eq!(
        parse_delimited("a,b", ',', false).rows,
        vec![cells(&["a", "b"])]
    );
    assert_eq!(
        parse_delimited("a,b\r\n", ',', false).rows,
        vec![cells(&["a", "b"])]
    );
}

#[test]
fn bare_cr_is_ordinary_data() {
    let t = parse_delimited("a\rb,c\n", ',', false);
    assert_eq!(t.rows, vec![cells(&["a\rb", "c"])]);
    // A trailing bare \r is data too.
    let t = parse_delimited("a\r", ',', false);
    assert_eq!(t.rows, vec![cells(&["a\r"])]);
}

#[test]
fn crlf_inside_quotes_is_data() {
    let t = parse_delimited("a,\"x\r\ny\"\n", ',', false);
    assert_eq!(t.rows, vec![cells(&["a", "x\r\ny"])]);
}

#[test]
fn empty_text_is_an_empty_table() {
    let t = parse_delimited("", ',', true);
    assert_eq!(t, Table::default());
}

#[test]
fn blank_line_only_is_zero_rows() {
    let t = parse_delimited("\n", ',', true);
    assert!(t.header.is_empty() && t.rows.is_empty());
    let t = parse_delimited("\r\n", ',', false);
    assert!(t.header.is_empty() && t.rows.is_empty());
}

#[test]
fn blank_line_between_content_is_an_empty_cell_row() {
    let t = parse_delimited("a\n\nb", ',', false);
    assert_eq!(t.rows, vec![cells(&["a"]), cells(&[""]), cells(&["b"])]);
}

#[test]
fn trailing_blank_lines_produce_no_rows() {
    let t = parse_delimited("a\n\n\n", ',', false);
    assert_eq!(t.rows, vec![cells(&["a"])]);
}

// ---------------------------------------------------------------------
// parse_delimited: hostile and lenient cases
// ---------------------------------------------------------------------

#[test]
fn unterminated_quote_ends_at_input_end() {
    let t = parse_delimited("a,\"open\n", ',', false);
    assert_eq!(t.rows, vec![cells(&["a", "open\n"])]);
}

#[test]
fn uneven_rows_keep_their_widths() {
    let t = parse_delimited("a,b,c\n1,2\n", ',', true);
    assert_eq!(t.header, cells(&["a", "b", "c"]));
    assert_eq!(t.rows, vec![cells(&["1", "2"])]);
}

#[test]
fn custom_delimiters_semicolon_and_tab() {
    let t = parse_delimited("a;b\n1;2\n", ';', true);
    assert_eq!(t.header, cells(&["a", "b"]));
    assert_eq!(t.rows, vec![cells(&["1", "2"])]);
    let t = parse_delimited("a\tb\n1\t2\n", '\t', true);
    assert_eq!(t.header, cells(&["a", "b"]));
    assert_eq!(t.rows, vec![cells(&["1", "2"])]);
}

#[test]
fn multibyte_cells_and_quoted_delimiter() {
    let t = parse_delimited("名前,絵文字\n日本,🇯🇵\n", ',', true);
    assert_eq!(t.header, cells(&["名前", "絵文字"]));
    assert_eq!(t.rows, vec![cells(&["日本", "🇯🇵"])]);
    // Delimiters and quotes stay structural even around multibyte,
    // including the lenient chars-after-closing-quote rule.
    let t = parse_delimited("\"🚀,x\",cé\n\"🌍\"y,z\n", ',', false);
    assert_eq!(t.rows, vec![cells(&["🚀,x", "cé"]), cells(&["🌍y", "z"])]);
}

#[test]
fn nul_bytes_are_preserved() {
    let t = parse_delimited("a\u{0}b,c\n", ',', false);
    assert_eq!(t.rows, vec![cells(&["a\u{0}b", "c"])]);
}

#[test]
fn megabyte_cell_survives() {
    let big = "x".repeat(1_000_000);
    let text = format!("{big},y\n{big},z\n");
    let t = parse_delimited(&text, ',', false);
    assert_eq!(t.rows.len(), 2);
    assert_eq!(t.rows.first().map(|r| r.len()), Some(2));
    assert_eq!(
        t.rows.first().and_then(|r| r.first().map(|c| c.len())),
        Some(1_000_000)
    );
}

#[test]
fn no_header_mode_keeps_header_empty() {
    let t = parse_delimited("a,b\n1,2\n", ',', false);
    assert!(t.header.is_empty());
    assert_eq!(t.rows, vec![cells(&["a", "b"]), cells(&["1", "2"])]);
}

#[test]
fn header_with_empty_cells_is_kept() {
    let t = parse_delimited(",\n1,2\n", ',', true);
    assert_eq!(t.header, cells(&["", ""]));
    assert_eq!(t.rows, vec![cells(&["1", "2"])]);
}

#[test]
fn mid_field_quote_is_ordinary_data() {
    let t = parse_delimited("ab\"c,d\n", ',', false);
    assert_eq!(t.rows, vec![cells(&["ab\"c", "d"])]);
}

#[test]
fn chars_after_closing_quote_stay_in_the_cell() {
    let t = parse_delimited("\"ab\"x,y\n", ',', false);
    assert_eq!(t.rows, vec![cells(&["abx", "y"])]);
}

#[test]
fn lone_quoted_empty_field_is_one_empty_cell_row() {
    let t = parse_delimited("\"\"", ',', false);
    assert_eq!(t.rows, vec![cells(&[""])]);
    let t = parse_delimited("\"", ',', false);
    assert_eq!(t.rows, vec![cells(&[""])]);
}

// ---------------------------------------------------------------------
// diff_cells
// ---------------------------------------------------------------------

#[test]
fn diff_cells_identical_rows_are_empty() {
    assert_eq!(
        diff_cells(&cells(&["a", "b"]), &cells(&["a", "b"])),
        Vec::<usize>::new()
    );
}

#[test]
fn diff_cells_flags_the_changed_index() {
    let d = diff_cells(&cells(&["a", "b", "c"]), &cells(&["a", "x", "c"]));
    assert_eq!(d, vec![1]);
}

#[test]
fn diff_cells_flags_missing_tail_cells() {
    let d = diff_cells(&cells(&["a"]), &cells(&["a", "b", "c"]));
    assert_eq!(d, vec![1, 2]);
    let d = diff_cells(&cells(&["a", "b"]), &cells(&["a"]));
    assert_eq!(d, vec![1]);
}

#[test]
fn diff_cells_both_empty_is_empty() {
    assert_eq!(diff_cells(&Vec::new(), &Vec::new()), Vec::<usize>::new());
}

// ---------------------------------------------------------------------
// diff_rows via the public API
// ---------------------------------------------------------------------

#[test]
fn diff_rows_identical_tables_are_all_equal() {
    let a = vec![cells(&["1"]), cells(&["2"])];
    let script = diff_rows(&a, &a);
    assert!(script.exact);
    assert_eq!(script.ops, vec![Op::Equal { len: 2 }]);
}

#[test]
fn diff_rows_empty_tables_are_empty_script() {
    let a: Vec<Vec<String>> = Vec::new();
    let script = diff_rows(&a, &a);
    assert!(script.exact);
    assert!(script.ops.is_empty());
}

#[test]
fn diff_rows_inserted_row_is_one_insert() {
    let a = vec![cells(&["1"]), cells(&["2"])];
    let b = vec![cells(&["1"]), cells(&["9"]), cells(&["2"])];
    let script = diff_rows(&a, &b);
    assert!(script.exact);
    assert_eq!(
        script.ops,
        vec![
            Op::Equal { len: 1 },
            Op::Insert { len: 1 },
            Op::Equal { len: 1 }
        ]
    );
}

#[test]
fn diff_rows_reordered_rows_replay_and_stay_minimal() {
    let a = vec![cells(&["1"]), cells(&["2"]), cells(&["3"])];
    let b = vec![cells(&["2"]), cells(&["1"]), cells(&["3"])];
    let script = diff_rows(&a, &b);
    assert!(script.exact);
    let edits: usize = script
        .ops
        .iter()
        .map(|op| match op {
            Op::Equal { .. } => 0,
            Op::Delete { len } | Op::Insert { len } => *len,
        })
        .sum();
    assert_eq!(edits, 2, "one reorder is one delete plus one insert");
    let (ra, rb) = replay_rows(&a, &b, &script);
    assert_eq!(ra, a);
    assert_eq!(rb, b);
}

#[test]
fn parse_then_diff_rows_across_tables() {
    let a = parse_delimited("h\nx\n", ',', false).rows;
    let b = parse_delimited("h\nx\ny\n", ',', false).rows;
    let script = diff_rows(&a, &b);
    assert!(script.exact);
    assert_eq!(
        script.ops,
        vec![Op::Equal { len: 2 }, Op::Insert { len: 1 }]
    );
}

// ---------------------------------------------------------------------
// Key-column alignment (Beyond Compare table parity)
// ---------------------------------------------------------------------

#[test]
fn key_alignment_matches_rows_despite_reorder() {
    let a = parse_delimited("id,name\n1,alice\n2,bob\n3,carol\n", ',', true);
    let b = parse_delimited("id,name\n3,carol\n1,alice\n2,bobby\n", ',', true);
    let diff = diff_table_by_key(&a, &b, &[0]);
    assert_eq!(diff.key_columns, vec![0]);
    assert_eq!(diff.matching_rows, 2);
    assert_eq!(diff.modified_rows, 1);
    assert_eq!(diff.left_only_rows, 0);
    assert_eq!(diff.right_only_rows, 0);
    // Total rows accounted for on the left side.
    let accounted = diff.matching_rows + diff.modified_rows + diff.left_only_rows;
    assert_eq!(accounted, a.rows.len());
}

#[test]
fn key_alignment_reports_one_sided_rows() {
    let a = parse_delimited("id,v\n1,a\n2,b\n", ',', true);
    let b = parse_delimited("id,v\n2,b\n3,c\n", ',', true);
    let diff = diff_table_by_key(&a, &b, &[0]);
    assert_eq!(diff.matching_rows, 1);
    assert_eq!(diff.left_only_rows, 1);
    assert_eq!(diff.right_only_rows, 1);
}

#[test]
fn detect_key_columns_finds_unique_and_composite() {
    let t = parse_delimited("id,city\n1,paris\n2,paris\n", ',', true);
    assert_eq!(detect_key_columns(&t), vec![0]);
    // No single column is unique, but the pair is.
    let t = parse_delimited("a,b\n1,x\n1,y\n2,x\n", ',', false);
    assert_eq!(detect_key_columns(&t), vec![0, 1]);
    // Nothing unique: empty.
    let t = parse_delimited("a,b\n1,x\n1,x\n", ',', false);
    assert!(detect_key_columns(&t).is_empty());
}

// ---------------------------------------------------------------------
// Numeric tolerance (Beyond Compare table parity)
// ---------------------------------------------------------------------

#[test]
fn tolerance_matrix() {
    assert!(cells_equal_with_tolerance("1.001", "1.002", Some(0.01)));
    assert!(!cells_equal_with_tolerance("1.001", "1.02", Some(0.01)));
    assert!(cells_equal_with_tolerance("1.0", "1", Some(0.0)));
    assert!(!cells_equal_with_tolerance("1.0", "2", Some(0.0)));
    assert!(!cells_equal_with_tolerance("abc", "abd", Some(99.0)));
    assert!(!cells_equal_with_tolerance("NaN", "nan", Some(1.0)));
}

#[test]
fn tolerance_key_diff_counts_match() {
    let a = parse_delimited("id,price\n1,9.99\n2,5.00\n", ',', true);
    let b = parse_delimited("id,price\n1,10.04\n2,7.50\n", ',', true);
    let tolerant = TableCompareOptions {
        key_columns: vec![0],
        numeric_tolerance: Some(0.1),
        ignore_case: false,
        trim_space: false,
    };
    let diff = diff_table_by_key_with_options(&a, &b, &tolerant);
    assert_eq!(diff.matching_rows, 1, "9.99 vs 10.04 is within 0.1");
    assert_eq!(diff.modified_rows, 1, "5.00 vs 7.50 is outside 0.1");

    let strict = TableCompareOptions {
        key_columns: vec![0],
        ..TableCompareOptions::default()
    };
    let diff = diff_table_by_key_with_options(&a, &b, &strict);
    assert_eq!(diff.matching_rows, 0);
    assert_eq!(diff.modified_rows, 2);
}

#[test]
fn tolerant_options_ignore_case_and_space() {
    let a = parse_delimited("id,name\n1,  Ready \n", ',', true);
    let b = parse_delimited("id,name\n1,ready\n", ',', true);
    let folding = TableCompareOptions {
        key_columns: vec![0],
        numeric_tolerance: None,
        ignore_case: true,
        trim_space: true,
    };
    let diff = diff_table_by_key_with_options(&a, &b, &folding);
    assert_eq!(diff.matching_rows, 1);
    assert_eq!(diff.modified_rows, 0);

    let strict = TableCompareOptions {
        key_columns: vec![0],
        ..TableCompareOptions::default()
    };
    let diff = diff_table_by_key_with_options(&a, &b, &strict);
    assert_eq!(diff.modified_rows, 1);
}

#[test]
fn sorted_and_diff_cells_with_options_agree() {
    let mut t = parse_delimited("age,name\n30,bob\n5,alice\n", ',', true);
    sort_table_rows(&mut t, &[0], true);
    let first = t.rows.first().expect("sorted rows exist");
    assert_eq!(first.first().map(String::as_str), Some("5"));

    let opts = TableCompareOptions {
        key_columns: Vec::new(),
        numeric_tolerance: Some(0.5),
        ignore_case: false,
        trim_space: false,
    };
    let d = diff_cells_with_options(
        &cells(&["5", "x"]),
        &cells(&["5.2", "x"]),
        &opts,
    );
    assert!(d.is_empty(), "5 vs 5.2 is within 0.5");
}
