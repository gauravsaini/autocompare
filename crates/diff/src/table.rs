//! Delimited (CSV-style) table parsing, key-column alignment, multi-sheet workbooks, and cell comparison.
//!
//! Pure layer 0: no I/O, no panics. Parsing is total — malformed
//! input is preserved as parsed, never an error. The parser walks
//! `char` boundaries only, so multibyte UTF-8, NUL bytes and
//! megabyte cells pass through untouched.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::iter::Peekable;
use std::mem;
use std::str::Chars;

use crate::engine;
use crate::{DiffScript, EDIT_BUDGET};

/// One parsed table.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Table {
    /// Header cells when the first row is treated as a header.
    pub header: Vec<String>,
    /// Data rows; rows keep their own width (uneven is preserved).
    pub rows: Vec<Vec<String>>,
}

impl Table {
    /// Returns a new table with rows sorted by the given columns.
    pub fn sorted(&self, columns: &[usize], ascending: bool) -> Self {
        let mut cloned = self.clone();
        sort_table_rows(&mut cloned, columns, ascending);
        cloned
    }
}

/// A named table representing one sheet of a multi-sheet spreadsheet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sheet {
    /// Sheet name (e.g. "Sheet1").
    pub name: String,
    /// Table contents of the sheet.
    pub table: Table,
}

/// A multi-sheet spreadsheet workbook.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Workbook {
    /// Sheets in workbook order.
    pub sheets: Vec<Sheet>,
}

impl Workbook {
    /// Looks up a sheet by name.
    pub fn sheet(&self, name: &str) -> Option<&Sheet> {
        self.sheets.iter().find(|s| s.name == name)
    }

    /// Looks up a mutable sheet by name.
    pub fn sheet_mut(&mut self, name: &str) -> Option<&mut Sheet> {
        self.sheets.iter_mut().find(|s| s.name == name)
    }
}

/// Status of a sheet comparison between two workbooks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SheetDiffStatus {
    /// Sheet exists on both sides and their rows are compared.
    Compared {
        row_diff: DiffScript,
        identical: bool,
    },
    /// Sheet exists only in the left workbook.
    LeftOnly,
    /// Sheet exists only in the right workbook.
    RightOnly,
}

/// Difference of one sheet between two workbooks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetDiff {
    pub name: String,
    pub status: SheetDiffStatus,
}

/// Comparison result across all sheets of two workbooks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkbookDiff {
    pub sheet_diffs: Vec<SheetDiff>,
    pub identical: bool,
}

/// Compares two workbooks by matching sheets on name.
pub fn diff_workbooks(a: &Workbook, b: &Workbook) -> WorkbookDiff {
    let mut sheet_diffs = Vec::new();
    let mut identical = true;

    let b_names: HashSet<&str> = b.sheets.iter().map(|s| s.name.as_str()).collect();
    let mut seen_b = HashSet::new();

    for sheet_a in &a.sheets {
        if let Some(sheet_b) = b.sheets.iter().find(|s| s.name == sheet_a.name) {
            seen_b.insert(sheet_b.name.as_str());
            let row_diff = diff_rows(&sheet_a.table.rows, &sheet_b.table.rows);
            let headers_same = sheet_a.table.header == sheet_b.table.header;
            let rows_same = sheet_a.table.rows == sheet_b.table.rows;
            let sheet_identical = headers_same && rows_same;
            if !sheet_identical {
                identical = false;
            }
            sheet_diffs.push(SheetDiff {
                name: sheet_a.name.clone(),
                status: SheetDiffStatus::Compared {
                    row_diff,
                    identical: sheet_identical,
                },
            });
        } else {
            identical = false;
            sheet_diffs.push(SheetDiff {
                name: sheet_a.name.clone(),
                status: SheetDiffStatus::LeftOnly,
            });
        }
    }

    for sheet_b in &b.sheets {
        if !seen_b.contains(sheet_b.name.as_str()) && !b_names.contains(sheet_b.name.as_str()) {
            identical = false;
            sheet_diffs.push(SheetDiff {
                name: sheet_b.name.clone(),
                status: SheetDiffStatus::RightOnly,
            });
        }
    }

    WorkbookDiff {
        sheet_diffs,
        identical,
    }
}

/// Parses delimiter-separated text, RFC-4180-lite: quoted fields with
/// doubled-quote escapes; delimiters and newlines inside quotes are
/// data; `\r\n` is tolerated; a trailing newline is not a row.
pub fn parse_delimited(text: &str, delimiter: char, first_row_is_header: bool) -> Table {
    let mut parser = Parser::new(delimiter);
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        parser.step(c, &mut chars);
    }
    parser.finish();
    let rows = parser.rows;
    let (header, data) = if first_row_is_header {
        let mut iter = rows.into_iter();
        (iter.next().unwrap_or_default(), iter.collect())
    } else {
        (Vec::new(), rows)
    };
    Table { header, rows: data }
}

/// Row-level diff over whole cell rows: the Myers engine over `Vec<String>` rows.
pub fn diff_rows(a: &[Vec<String>], b: &[Vec<String>]) -> DiffScript {
    let (ops, exact) = engine::diff_slices(a, b, Some(EDIT_BUDGET));
    DiffScript { ops, exact }
}

/// Indices of columns whose cells differ between two rows, compared
/// position by position; a cell missing on one side counts as
/// differing. Sorted, no duplicates.
pub fn diff_cells(a: &[String], b: &[String]) -> Vec<usize> {
    let len = a.len().max(b.len());
    (0..len).filter(|&i| a.get(i) != b.get(i)).collect()
}

/// Result of row comparison aligned by key columns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyRowDiff {
    /// Key matches and all other cells match.
    Unchanged { left_row: usize, right_row: usize },
    /// Key matches but other cells differ.
    Modified {
        left_row: usize,
        right_row: usize,
        differing_columns: Vec<usize>,
    },
    /// Row exists only in the left table.
    LeftOnly { left_row: usize },
    /// Row exists only in the right table.
    RightOnly { right_row: usize },
}

/// Detailed difference report when comparing two tables using primary key columns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableKeyDiff {
    /// Columns used as primary keys.
    pub key_columns: Vec<usize>,
    /// Alignment operations per key.
    pub row_diffs: Vec<KeyRowDiff>,
    /// Count of identical matching rows.
    pub matching_rows: usize,
    /// Count of rows with key match but differing data.
    pub modified_rows: usize,
    /// Count of left-only rows.
    pub left_only_rows: usize,
    /// Count of right-only rows.
    pub right_only_rows: usize,
}

/// Automatically detects primary key columns in a table.
///
/// A primary key column must have non-empty, unique values for every row.
/// Checks single columns first, then two-column pairs if no single column is unique.
pub fn detect_key_columns(table: &Table) -> Vec<usize> {
    if table.rows.is_empty() {
        return Vec::new();
    }

    let col_count = table.rows.iter().map(|r| r.len()).max().unwrap_or(0);
    if col_count == 0 {
        return Vec::new();
    }

    // Single column pass
    for col in 0..col_count {
        let mut seen = HashSet::new();
        let mut valid = true;
        for row in &table.rows {
            match row.get(col) {
                Some(val) if !val.trim().is_empty() => {
                    if !seen.insert(val.as_str()) {
                        valid = false;
                        break;
                    }
                }
                _ => {
                    valid = false;
                    break;
                }
            }
        }
        if valid && seen.len() == table.rows.len() {
            return vec![col];
        }
    }

    // Composite two-column pass
    if col_count >= 2 {
        for c1 in 0..col_count {
            for c2 in (c1 + 1)..col_count {
                let mut seen = HashSet::new();
                let mut valid = true;
                for row in &table.rows {
                    let v1 = row.get(c1).map(|s| s.as_str()).unwrap_or("");
                    let v2 = row.get(c2).map(|s| s.as_str()).unwrap_or("");
                    if v1.is_empty() && v2.is_empty() {
                        valid = false;
                        break;
                    }
                    if !seen.insert((v1, v2)) {
                        valid = false;
                        break;
                    }
                }
                if valid && seen.len() == table.rows.len() {
                    return vec![c1, c2];
                }
            }
        }
    }

    Vec::new()
}

/// Compares two tables aligned by key columns.
///
/// Exact text equality on every cell. For numeric tolerance, case
/// folding or space trimming see [`diff_table_by_key_with_options`].
pub fn diff_table_by_key(a: &Table, b: &Table, key_columns: &[usize]) -> TableKeyDiff {
    if key_columns.is_empty() {
        // Fall back to sequential row diff
        let script = diff_rows(&a.rows, &b.rows);
        let mut row_diffs = Vec::new();
        let mut matching_rows = 0usize;
        let mut left_only_rows = 0usize;
        let mut right_only_rows = 0usize;
        let mut l_idx = 0usize;
        let mut r_idx = 0usize;

        for op in script.ops {
            match op {
                crate::Op::Equal { len } => {
                    for _ in 0..len {
                        row_diffs.push(KeyRowDiff::Unchanged {
                            left_row: l_idx,
                            right_row: r_idx,
                        });
                        l_idx = l_idx.saturating_add(1);
                        r_idx = r_idx.saturating_add(1);
                        matching_rows = matching_rows.saturating_add(1);
                    }
                }
                crate::Op::Delete { len } => {
                    for _ in 0..len {
                        row_diffs.push(KeyRowDiff::LeftOnly { left_row: l_idx });
                        l_idx = l_idx.saturating_add(1);
                        left_only_rows = left_only_rows.saturating_add(1);
                    }
                }
                crate::Op::Insert { len } => {
                    for _ in 0..len {
                        row_diffs.push(KeyRowDiff::RightOnly { right_row: r_idx });
                        r_idx = r_idx.saturating_add(1);
                        right_only_rows = right_only_rows.saturating_add(1);
                    }
                }
            }
        }
        return TableKeyDiff {
            key_columns: Vec::new(),
            row_diffs,
            matching_rows,
            modified_rows: 0,
            left_only_rows,
            right_only_rows,
        };
    }

    // Map right table rows by composite key
    let mut right_key_map: HashMap<Vec<String>, Vec<usize>> = HashMap::new();
    for (idx, row) in b.rows.iter().enumerate() {
        let key: Vec<String> = key_columns
            .iter()
            .map(|&c| row.get(c).cloned().unwrap_or_default())
            .collect();
        right_key_map.entry(key).or_default().push(idx);
    }

    let mut row_diffs = Vec::new();
    let mut matching_rows = 0usize;
    let mut modified_rows = 0usize;
    let mut left_only_rows = 0usize;
    let mut right_only_rows = 0usize;
    let mut used_right = HashSet::new();

    for (l_idx, row_a) in a.rows.iter().enumerate() {
        let key: Vec<String> = key_columns
            .iter()
            .map(|&c| row_a.get(c).cloned().unwrap_or_default())
            .collect();

        if let Some(r_indices) = right_key_map.get_mut(&key)
            && let Some(r_idx) = r_indices.iter().copied().find(|i| !used_right.contains(i))
        {
            used_right.insert(r_idx);
            let row_b = b.rows.get(r_idx).cloned().unwrap_or_default();
            let differing_columns = diff_cells(row_a, &row_b);
            if differing_columns.is_empty() {
                matching_rows = matching_rows.saturating_add(1);
                row_diffs.push(KeyRowDiff::Unchanged {
                    left_row: l_idx,
                    right_row: r_idx,
                });
            } else {
                modified_rows = modified_rows.saturating_add(1);
                row_diffs.push(KeyRowDiff::Modified {
                    left_row: l_idx,
                    right_row: r_idx,
                    differing_columns,
                });
            }
        } else {
            left_only_rows = left_only_rows.saturating_add(1);
            row_diffs.push(KeyRowDiff::LeftOnly { left_row: l_idx });
        }
    }

    // Remaining unused right rows
    for (r_idx, _) in b.rows.iter().enumerate() {
        if !used_right.contains(&r_idx) {
            right_only_rows = right_only_rows.saturating_add(1);
            row_diffs.push(KeyRowDiff::RightOnly { right_row: r_idx });
        }
    }

    TableKeyDiff {
        key_columns: key_columns.to_vec(),
        row_diffs,
        matching_rows,
        modified_rows,
        left_only_rows,
        right_only_rows,
    }
}

/// Options for tolerant table comparison (Beyond Compare parity:
/// key-column alignment plus numeric tolerance, case folding and
/// space trimming). All comparisons are pure and total.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TableCompareOptions {
    /// Columns used as the alignment key (empty = no key alignment).
    pub key_columns: Vec<usize>,
    /// When set to a finite value `>= 0`, two cells that both parse
    /// as numbers (after trimming surrounding whitespace) compare
    /// equal when their absolute difference is within the tolerance.
    /// `None`, negative, infinite or NaN tolerance means exact text.
    pub numeric_tolerance: Option<f64>,
    /// Fold ASCII/Unicode case before comparing text.
    pub ignore_case: bool,
    /// Trim surrounding whitespace before comparing text.
    pub trim_space: bool,
}

/// Compares two cells for equality under a numeric tolerance.
///
/// Exact match wins first. Otherwise, when `tolerance` is finite and
/// `>= 0` and both cells parse as finite `f64` (surrounding
/// whitespace ignored), they are equal when `|a - b| <= tolerance`.
/// Anything else (non-numeric text, NaN/infinite values, invalid
/// tolerance) is unequal unless the raw strings match.
///
/// Limitation: numbers go through `f64`, so integers beyond 2^53 may
/// conflate (e.g. `9007199254740993` vs `9007199254740992`).
pub fn cells_equal_with_tolerance(a: &str, b: &str, tolerance: Option<f64>) -> bool {
    if a == b {
        return true;
    }
    let tol = match tolerance {
        Some(t) if t.is_finite() && t >= 0.0 => t,
        _ => return false,
    };
    numeric_equal(a, b, tol)
}

/// Compares two cells under the full [`TableCompareOptions`].
///
/// Order: trimmed-or-raw exact match, then case-folded match when
/// `ignore_case`, then numeric tolerance. Numeric parsing always
/// ignores surrounding whitespace; text equality honours
/// `trim_space` (without it, `" 1"` and `"1"` differ as text but can
/// still match numerically when a tolerance is set).
pub fn cells_equal_with_options(a: &str, b: &str, opts: &TableCompareOptions) -> bool {
    let (left, right) = if opts.trim_space {
        (a.trim(), b.trim())
    } else {
        (a, b)
    };
    if left == right {
        return true;
    }
    if opts.ignore_case && left.to_lowercase() == right.to_lowercase() {
        return true;
    }
    let tol = match opts.numeric_tolerance {
        Some(t) if t.is_finite() && t >= 0.0 => t,
        _ => return false,
    };
    numeric_equal(a, b, tol)
}

/// Indices of columns whose cells differ between two rows under
/// [`TableCompareOptions`]. A cell missing on one side is compared as
/// `""`. Sorted, no duplicates.
pub fn diff_cells_with_options(
    a: &[String],
    b: &[String],
    opts: &TableCompareOptions,
) -> Vec<usize> {
    let len = a.len().max(b.len());
    let mut out = Vec::new();
    for i in 0..len {
        let left = a.get(i).map(String::as_str).unwrap_or("");
        let right = b.get(i).map(String::as_str).unwrap_or("");
        if !cells_equal_with_options(left, right, opts) {
            out.push(i);
        }
    }
    out
}

/// Compares two tables aligned by the key columns in `opts`, with
/// tolerant cell comparison.
///
/// Key matching normalises keys with `trim_space`/`ignore_case` but
/// never applies the numeric tolerance (keys `"1"` and `"1.0"` are
/// different keys). Data cells compare with
/// [`cells_equal_with_options`].
///
/// When `key_columns` is empty the rows compare positionally
/// (row `i` vs row `i`, tails are one-sided); unlike the Myers
/// fallback in [`diff_table_by_key`] this is positional rather than
/// minimal-edit, which keeps tolerance semantics predictable.
pub fn diff_table_by_key_with_options(
    a: &Table,
    b: &Table,
    opts: &TableCompareOptions,
) -> TableKeyDiff {
    if opts.numeric_tolerance.is_none() && !opts.ignore_case && !opts.trim_space {
        return diff_table_by_key(a, b, &opts.key_columns);
    }
    if opts.key_columns.is_empty() {
        return diff_tables_positional(a, b, opts);
    }

    let mut right_key_map: HashMap<Vec<String>, Vec<usize>> = HashMap::new();
    for (idx, row) in b.rows.iter().enumerate() {
        let key = normalised_key(row, &opts.key_columns, opts);
        right_key_map.entry(key).or_default().push(idx);
    }

    let mut row_diffs = Vec::new();
    let mut matching_rows = 0usize;
    let mut modified_rows = 0usize;
    let mut left_only_rows = 0usize;
    let mut right_only_rows = 0usize;
    let mut used_right = HashSet::new();

    for (l_idx, row_a) in a.rows.iter().enumerate() {
        let key = normalised_key(row_a, &opts.key_columns, opts);
        let hit = right_key_map
            .get_mut(&key)
            .and_then(|indices| {
                indices
                    .iter()
                    .copied()
                    .find(|i| !used_right.contains(i))
            });
        if let Some(r_idx) = hit {
            used_right.insert(r_idx);
            let row_b = b.rows.get(r_idx).cloned().unwrap_or_default();
            let differing_columns = diff_cells_with_options(row_a, &row_b, opts);
            // Key columns normalised equal but raw text may differ in
            // case/space: those columns still count as differing so the
            // report stays honest about the raw bytes.
            if differing_columns.is_empty() {
                matching_rows = matching_rows.saturating_add(1);
                row_diffs.push(KeyRowDiff::Unchanged {
                    left_row: l_idx,
                    right_row: r_idx,
                });
            } else {
                modified_rows = modified_rows.saturating_add(1);
                row_diffs.push(KeyRowDiff::Modified {
                    left_row: l_idx,
                    right_row: r_idx,
                    differing_columns,
                });
            }
        } else {
            left_only_rows = left_only_rows.saturating_add(1);
            row_diffs.push(KeyRowDiff::LeftOnly { left_row: l_idx });
        }
    }

    for (r_idx, _) in b.rows.iter().enumerate() {
        if !used_right.contains(&r_idx) {
            right_only_rows = right_only_rows.saturating_add(1);
            row_diffs.push(KeyRowDiff::RightOnly { right_row: r_idx });
        }
    }

    TableKeyDiff {
        key_columns: opts.key_columns.clone(),
        row_diffs,
        matching_rows,
        modified_rows,
        left_only_rows,
        right_only_rows,
    }
}

/// Numeric equality of two cells: both must parse as finite `f64`
/// after trimming, with `|a - b| <= tolerance`.
fn numeric_equal(a: &str, b: &str, tolerance: f64) -> bool {
    let left: f64 = match a.trim().parse() {
        Ok(v) => v,
        Err(_) => return false,
    };
    let right: f64 = match b.trim().parse() {
        Ok(v) => v,
        Err(_) => return false,
    };
    if !left.is_finite() || !right.is_finite() {
        return false;
    }
    (left - right).abs() <= tolerance
}

/// Key projection of one row under the key columns, normalised for
/// matching with `trim_space`/`ignore_case`.
fn normalised_key(
    row: &[String],
    key_columns: &[usize],
    opts: &TableCompareOptions,
) -> Vec<String> {
    key_columns
        .iter()
        .map(|&c| {
            let raw = row.get(c).map(String::as_str).unwrap_or("");
            let trimmed = if opts.trim_space { raw.trim() } else { raw };
            if opts.ignore_case {
                trimmed.to_lowercase()
            } else {
                trimmed.to_string()
            }
        })
        .collect()
}

/// Positional tolerant comparison used when no key columns are set:
/// row `i` vs row `i`, length tails are one-sided.
fn diff_tables_positional(a: &Table, b: &Table, opts: &TableCompareOptions) -> TableKeyDiff {
    let mut row_diffs = Vec::new();
    let mut matching_rows = 0usize;
    let mut modified_rows = 0usize;
    let mut left_only_rows = 0usize;
    let mut right_only_rows = 0usize;
    let common = a.rows.len().min(b.rows.len());
    for i in 0..common {
        let row_a = a.rows.get(i).cloned().unwrap_or_default();
        let row_b = b.rows.get(i).cloned().unwrap_or_default();
        let differing_columns = diff_cells_with_options(&row_a, &row_b, opts);
        if differing_columns.is_empty() {
            matching_rows = matching_rows.saturating_add(1);
            row_diffs.push(KeyRowDiff::Unchanged {
                left_row: i,
                right_row: i,
            });
        } else {
            modified_rows = modified_rows.saturating_add(1);
            row_diffs.push(KeyRowDiff::Modified {
                left_row: i,
                right_row: i,
                differing_columns,
            });
        }
    }
    for l_idx in common..a.rows.len() {
        left_only_rows = left_only_rows.saturating_add(1);
        row_diffs.push(KeyRowDiff::LeftOnly { left_row: l_idx });
    }
    for r_idx in common..b.rows.len() {
        right_only_rows = right_only_rows.saturating_add(1);
        row_diffs.push(KeyRowDiff::RightOnly { right_row: r_idx });
    }
    TableKeyDiff {
        key_columns: Vec::new(),
        row_diffs,
        matching_rows,
        modified_rows,
        left_only_rows,
        right_only_rows,
    }
}

/// Sorts the data rows of a table in-place across specified columns.
/// Numerical values are compared as numbers (integer/float); otherwise compared lexicographically.
pub fn sort_table_rows(table: &mut Table, columns: &[usize], ascending: bool) {
    if columns.is_empty() || table.rows.len() <= 1 {
        return;
    }

    table.rows.sort_by(|row_a, row_b| {
        for &col in columns {
            let val_a = row_a.get(col).map(|s| s.as_str()).unwrap_or("");
            let val_b = row_b.get(col).map(|s| s.as_str()).unwrap_or("");

            let ord = compare_cells_natural(val_a, val_b);
            if ord != Ordering::Equal {
                return if ascending { ord } else { ord.reverse() };
            }
        }
        Ordering::Equal
    });
}

fn compare_cells_natural(a: &str, b: &str) -> Ordering {
    // Try integer parsing
    if let (Ok(ia), Ok(ib)) = (a.trim().parse::<i64>(), b.trim().parse::<i64>()) {
        return ia.cmp(&ib);
    }
    // Try float parsing
    if let (Ok(fa), Ok(fb)) = (a.trim().parse::<f64>(), b.trim().parse::<f64>())
        && let Some(ord) = fa.partial_cmp(&fb)
    {
        return ord;
    }
    // Fall back to lexicographic
    a.cmp(b)
}

/// Decodes an Excel (.xlsx) file from raw bytes into a `Workbook`.
///
/// Parses ZIP container and worksheet XML. If the file is not a valid
/// PKZIP spreadsheet or corrupted, returns `None` safely without panic.
#[allow(dead_code)]
pub fn decode_xlsx(bytes: &[u8]) -> Option<Workbook> {
    if bytes.len() < 30 || bytes.get(0..4) != Some(b"PK\x03\x04") {
        return None;
    }

    let entries = read_zip_entries(bytes)?;
    let shared_strings = if let Some(ss_data) = entries.get("xl/sharedStrings.xml") {
        parse_shared_strings(ss_data)
    } else {
        Vec::new()
    };

    let mut sheets = Vec::new();

    // Look for worksheets sheet1.xml, sheet2.xml, etc.
    let mut sheet_idx = 1usize;
    while let Some(sheet_data) = entries.get(&format!("xl/worksheets/sheet{sheet_idx}.xml")) {
        let sheet_name = format!("Sheet{sheet_idx}");
        let table = parse_sheet_xml(sheet_data, &shared_strings);
        sheets.push(Sheet {
            name: sheet_name,
            table,
        });
        sheet_idx = sheet_idx.saturating_add(1);
    }

    if sheets.is_empty() {
        // Search any worksheet in xl/worksheets/
        for (name, sheet_data) in &entries {
            if name.starts_with("xl/worksheets/") && name.ends_with(".xml") {
                let s_name = name
                    .strip_prefix("xl/worksheets/")
                    .unwrap_or(name)
                    .strip_suffix(".xml")
                    .unwrap_or(name)
                    .to_string();
                let table = parse_sheet_xml(sheet_data, &shared_strings);
                sheets.push(Sheet {
                    name: s_name,
                    table,
                });
            }
        }
    }

    if sheets.is_empty() {
        None
    } else {
        Some(Workbook { sheets })
    }
}

/// Uncompressed or basic ZIP entries map: filename -> slice of uncompressed bytes.
fn read_zip_entries(bytes: &[u8]) -> Option<HashMap<String, Vec<u8>>> {
    let mut entries = HashMap::new();
    let mut pos = 0usize;

    while pos.saturating_add(30) <= bytes.len() {
        if bytes.get(pos..pos + 4) != Some(b"PK\x03\x04") {
            break;
        }

        let compression = u16::from_le_bytes(bytes.get(pos + 8..pos + 10)?.try_into().ok()?);
        let comp_size =
            u32::from_le_bytes(bytes.get(pos + 18..pos + 22)?.try_into().ok()?) as usize;
        let fn_len = u16::from_le_bytes(bytes.get(pos + 26..pos + 28)?.try_into().ok()?) as usize;
        let extra_len =
            u16::from_le_bytes(bytes.get(pos + 28..pos + 30)?.try_into().ok()?) as usize;

        let fn_start = pos.checked_add(30)?;
        let fn_end = fn_start.checked_add(fn_len)?;
        let fn_bytes = bytes.get(fn_start..fn_end)?;
        let filename = String::from_utf8_lossy(fn_bytes).to_string();

        let data_start = fn_end.checked_add(extra_len)?;
        let data_end = data_start.checked_add(comp_size)?;
        if data_end > bytes.len() {
            break;
        }

        if compression == 0 {
            // Stored (no compression)
            let file_data = bytes.get(data_start..data_end)?.to_vec();
            entries.insert(filename, file_data);
        }

        pos = data_end;
    }

    Some(entries)
}

fn parse_shared_strings(data: &[u8]) -> Vec<String> {
    let text = String::from_utf8_lossy(data);
    let mut strings = Vec::new();
    let mut rest = text.as_ref();

    while let Some(start) = rest.find("<t>") {
        let after = match rest.get(start + 3..) {
            Some(a) => a,
            None => break,
        };
        if let Some(end) = after.find("</t>") {
            let val = after.get(..end).unwrap_or("");
            strings.push(unescape_xml(val));
            rest = after.get(end + 4..).unwrap_or("");
        } else {
            break;
        }
    }

    strings
}

fn parse_sheet_xml(data: &[u8], shared_strings: &[String]) -> Table {
    let text = String::from_utf8_lossy(data);
    let mut rows: Vec<Vec<String>> = Vec::new();

    let mut row_rest = text.as_ref();
    while let Some(r_start) = row_rest.find("<row") {
        let after_row_start = match row_rest.get(r_start..) {
            Some(a) => a,
            None => break,
        };
        let r_end = match after_row_start.find("</row>") {
            Some(e) => e,
            None => break,
        };
        let row_content = after_row_start.get(..r_end).unwrap_or("");
        let mut row_cells: Vec<(usize, String)> = Vec::new();

        let mut cell_rest = row_content;
        while let Some(c_start) = cell_rest.find("<c") {
            let after_c = match cell_rest.get(c_start..) {
                Some(a) => a,
                None => break,
            };
            let c_end = match after_c.find("</c>") {
                Some(e) => e + 4,
                None => match after_c.find("/>") {
                    Some(e) => e + 2,
                    None => break,
                },
            };
            let cell_chunk = after_c.get(..c_end).unwrap_or("");

            // Parse column coordinate from r="A1"
            let col_idx = parse_cell_col_index(cell_chunk);
            let is_shared = cell_chunk.contains("t=\"s\"");

            let val = if let Some(v_start) = cell_chunk.find("<v>")
                && let Some(v_end) = cell_chunk.find("</v>")
            {
                let raw_val = cell_chunk.get(v_start + 3..v_end).unwrap_or("");
                if is_shared {
                    if let Ok(idx) = raw_val.trim().parse::<usize>() {
                        shared_strings.get(idx).cloned().unwrap_or_default()
                    } else {
                        raw_val.to_string()
                    }
                } else {
                    raw_val.to_string()
                }
            } else if let Some(t_start) = cell_chunk.find("<t>")
                && let Some(t_end) = cell_chunk.find("</t>")
            {
                cell_chunk.get(t_start + 3..t_end).unwrap_or("").to_string()
            } else {
                String::new()
            };

            row_cells.push((col_idx, val));
            cell_rest = after_c.get(c_end..).unwrap_or("");
        }

        if !row_cells.is_empty() {
            let max_col = row_cells.iter().map(|(c, _)| *c).max().unwrap_or(0);
            let mut full_row = vec![String::new(); max_col.saturating_add(1)];
            for (col, val) in row_cells {
                if let Some(slot) = full_row.get_mut(col) {
                    *slot = val;
                }
            }
            rows.push(full_row);
        }

        row_rest = after_row_start.get(r_end + 6..).unwrap_or("");
    }

    Table {
        header: Vec::new(),
        rows,
    }
}

fn parse_cell_col_index(cell_xml: &str) -> usize {
    if let Some(r_pos) = cell_xml.find("r=\"") {
        let after_r = cell_xml.get(r_pos + 3..).unwrap_or("");
        if let Some(quote_pos) = after_r.find('"') {
            let coord = after_r.get(..quote_pos).unwrap_or("");
            let letters: String = coord.chars().filter(|c| c.is_ascii_alphabetic()).collect();
            let mut col = 0usize;
            for ch in letters.chars() {
                let val = (ch.to_ascii_uppercase() as usize)
                    .saturating_sub('A' as usize)
                    .saturating_add(1);
                col = col.saturating_mul(26).saturating_add(val);
            }
            return col.saturating_sub(1);
        }
    }
    0
}

fn unescape_xml(val: &str) -> String {
    val.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}

/// Where the parser stands inside the current field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FieldState {
    Start,
    Unquoted,
    Quoted,
}

/// One-pass, character-level state machine over raw text.
struct Parser {
    delimiter: char,
    state: FieldState,
    rows: Vec<Vec<String>>,
    row: Vec<String>,
    field: String,
    row_started: bool,
    pending_blanks: usize,
}

impl Parser {
    fn new(delimiter: char) -> Self {
        Self {
            delimiter,
            state: FieldState::Start,
            rows: Vec::new(),
            row: Vec::new(),
            field: String::new(),
            row_started: false,
            pending_blanks: 0,
        }
    }

    fn step(&mut self, c: char, chars: &mut Peekable<Chars<'_>>) {
        if self.state == FieldState::Quoted {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    self.field.push('"');
                } else {
                    self.state = FieldState::Unquoted;
                }
            } else {
                self.field.push(c);
            }
            return;
        }
        if c == '\n' {
            self.end_row();
        } else if c == '\r' && chars.peek() == Some(&'\n') {
            chars.next();
            self.end_row();
        } else if c == self.delimiter {
            self.end_field();
        } else if c == '"' && self.state == FieldState::Start {
            self.start_row();
            self.state = FieldState::Quoted;
        } else {
            self.start_row();
            self.field.push(c);
            self.state = FieldState::Unquoted;
        }
    }

    fn start_row(&mut self) {
        for _ in 0..self.pending_blanks {
            self.rows.push(vec![String::new()]);
        }
        self.pending_blanks = 0;
        self.row_started = true;
    }

    fn end_field(&mut self) {
        self.start_row();
        self.row.push(mem::take(&mut self.field));
        self.state = FieldState::Start;
    }

    fn end_row(&mut self) {
        if self.row_started {
            self.flush_row();
        } else {
            self.pending_blanks = self.pending_blanks.saturating_add(1);
        }
        self.state = FieldState::Start;
    }

    fn finish(&mut self) {
        if self.row_started {
            self.flush_row();
        }
        self.pending_blanks = 0;
    }

    fn flush_row(&mut self) {
        self.row.push(mem::take(&mut self.field));
        self.rows.push(mem::take(&mut self.row));
        self.row_started = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_key_columns() {
        let table = Table {
            header: vec!["ID".into(), "Name".into(), "City".into()],
            rows: vec![
                vec!["101".into(), "Alice".into(), "Paris".into()],
                vec!["102".into(), "Bob".into(), "London".into()],
                vec!["103".into(), "Charlie".into(), "Paris".into()],
            ],
        };
        let keys = detect_key_columns(&table);
        // Column 0 ("ID") has all unique values!
        assert_eq!(keys, vec![0]);
    }

    #[test]
    fn test_diff_table_by_key() {
        let t1 = Table {
            header: vec!["ID".into(), "Status".into()],
            rows: vec![
                vec!["1".into(), "Active".into()],
                vec!["2".into(), "Pending".into()],
                vec!["3".into(), "Active".into()],
            ],
        };
        let t2 = Table {
            header: vec!["ID".into(), "Status".into()],
            rows: vec![
                vec!["1".into(), "Active".into()],
                vec!["2".into(), "Closed".into()], // Modified!
                vec!["4".into(), "New".into()],    // RightOnly!
            ],
        };

        let diff = diff_table_by_key(&t1, &t2, &[0]);
        assert_eq!(diff.matching_rows, 1);
        assert_eq!(diff.modified_rows, 1);
        assert_eq!(diff.left_only_rows, 1); // ID 3
        assert_eq!(diff.right_only_rows, 1); // ID 4
    }

    #[test]
    fn test_sort_table_rows() {
        let mut table = Table {
            header: vec!["Age".to_string(), "Name".to_string()],
            rows: vec![
                vec!["30".to_string(), "Bob".to_string()],
                vec!["5".to_string(), "Alice".to_string()],
                vec!["100".to_string(), "Charlie".to_string()],
            ],
        };
        // Natural numeric sort on column 0
        sort_table_rows(&mut table, &[0], true);
        assert_eq!(
            table.rows,
            vec![
                vec!["5".to_string(), "Alice".to_string()],
                vec!["30".to_string(), "Bob".to_string()],
                vec!["100".to_string(), "Charlie".to_string()],
            ]
        );
    }

    #[test]
    fn test_workbook_diff() {        let wb1 = Workbook {
            sheets: vec![Sheet {
                name: "Sheet1".into(),
                table: Table {
                    header: vec!["A".into()],
                    rows: vec![vec!["1".into()]],
                },
            }],
        };
        let wb2 = Workbook {
            sheets: vec![Sheet {
                name: "Sheet1".into(),
                table: Table {
                    header: vec!["A".into()],
                    rows: vec![vec!["2".into()]],
                },
            }],
        };
        let diff = diff_workbooks(&wb1, &wb2);
        assert!(!diff.identical);
        assert_eq!(diff.sheet_diffs.len(), 1);
    }

    #[test]
    fn test_cells_equal_with_tolerance_exact_and_numeric() {
        assert!(cells_equal_with_tolerance("abc", "abc", None));
        assert!(!cells_equal_with_tolerance("abc", "abd", None));
        // Tolerance only helps numbers.
        assert!(!cells_equal_with_tolerance("abc", "abd", Some(100.0)));
        // Within and outside tolerance.
        assert!(cells_equal_with_tolerance("1.001", "1.002", Some(0.01)));
        assert!(!cells_equal_with_tolerance("1.001", "1.02", Some(0.01)));
        // Zero tolerance still unifies text spellings of the same number.
        assert!(cells_equal_with_tolerance("1.0", "1", Some(0.0)));
        // Surrounding spaces never block numeric comparison.
        assert!(cells_equal_with_tolerance("  2.5 ", "2.5", Some(0.0)));
        // One side non-numeric stays different.
        assert!(!cells_equal_with_tolerance("2.5", "n/a", Some(1.0)));
        // NaN / infinite values never match numerically (spellings
        // that differ as text stay different).
        assert!(!cells_equal_with_tolerance("NaN", "nan", Some(1.0)));
        assert!(!cells_equal_with_tolerance("inf", "Infinity", Some(1.0)));
        // Invalid tolerances fall back to exact text.
        assert!(!cells_equal_with_tolerance("1.0", "1", Some(f64::NAN)));
        assert!(!cells_equal_with_tolerance("1.0", "1", Some(f64::INFINITY)));
        assert!(!cells_equal_with_tolerance("1.0", "1", Some(-0.5)));
        assert!(cells_equal_with_tolerance("same", "same", Some(f64::NAN)));
    }

    #[test]
    fn test_diff_cells_with_options_tolerance_and_flags() {
        let opts = TableCompareOptions {
            key_columns: Vec::new(),
            numeric_tolerance: Some(0.1),
            ignore_case: true,
            trim_space: true,
        };
        let a = vec!["ID".to_string(), "9.99".to_string(), "  Ready ".to_string()];
        let b = vec!["id".to_string(), "10.05".to_string(), "ready".to_string()];
        assert!(diff_cells_with_options(&a, &b, &opts).is_empty());

        let strict = TableCompareOptions::default();
        assert_eq!(diff_cells_with_options(&a, &b, &strict), vec![0, 1, 2]);

        // Missing tail cells still count as differing.
        let short = vec!["ID".to_string()];
        assert_eq!(diff_cells_with_options(&short, &b, &opts), vec![1, 2]);
    }

    #[test]
    fn test_key_diff_with_options_tolerance() {
        let mk = |rows: Vec<Vec<&str>>| Table {
            header: vec!["ID".into(), "Price".into()],
            rows: rows
                .into_iter()
                .map(|r| r.into_iter().map(|c| c.to_string()).collect())
                .collect(),
        };
        let t1 = mk(vec![vec!["1", "9.99"], vec!["2", "5.00"]]);
        let t2 = mk(vec![vec!["1", "10.04"], vec!["2", "7.50"]]);
        let opts = TableCompareOptions {
            key_columns: vec![0],
            numeric_tolerance: Some(0.1),
            ignore_case: false,
            trim_space: false,
        };
        let diff = diff_table_by_key_with_options(&t1, &t2, &opts);
        assert_eq!(diff.matching_rows, 1);
        assert_eq!(diff.modified_rows, 1);
        assert_eq!(diff.left_only_rows, 0);
        assert_eq!(diff.right_only_rows, 0);

        // Exact path delegates: same tables compare fully unchanged.
        let exact = diff_table_by_key_with_options(
            &t1,
            &t1,
            &TableCompareOptions {
                key_columns: vec![0],
                ..TableCompareOptions::default()
            },
        );
        assert_eq!(exact.matching_rows, 2);
        assert_eq!(exact.modified_rows, 0);
    }

    #[test]
    fn test_key_diff_with_options_positional_fallback() {
        let mk = |rows: Vec<Vec<&str>>| Table {
            header: Vec::new(),
            rows: rows
                .into_iter()
                .map(|r| r.into_iter().map(|c| c.to_string()).collect())
                .collect(),
        };
        let t1 = mk(vec![vec!["a", "1.00"], vec!["b", "2.00"], vec!["gone", "3.00"]]);
        let t2 = mk(vec![vec!["a", "1.05"], vec!["b", "9.99"]]);
        let opts = TableCompareOptions {
            key_columns: Vec::new(),
            numeric_tolerance: Some(0.1),
            ignore_case: false,
            trim_space: false,
        };
        let diff = diff_table_by_key_with_options(&t1, &t2, &opts);
        assert_eq!(diff.matching_rows, 1);
        assert_eq!(diff.modified_rows, 1);
        assert_eq!(diff.left_only_rows, 1);
        assert_eq!(diff.right_only_rows, 0);
    }
}
