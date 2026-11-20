//! Delimited table comparison over the filesystem.
//!
//! Files are read whole (quoted fields may span lines), decoded
//! leniently (invalid bytes become replacement characters) and parsed
//! by `autocompare_diff::parse_delimited` with the caller's delimiter
//! and header choice. The row script comes from the same engine used
//! for lines; cell-level highlighting stays the CLI's job, driven by
//! the public `diff_cells`.

use std::path::Path;

use autocompare_diff::{
    Table, TableCompareOptions, TableKeyDiff, Workbook, WorkbookDiff, diff_rows,
    diff_table_by_key_with_options, diff_workbooks, parse_delimited, sort_table_rows,
};

use crate::{Error, Result, TableComparison, TableOptions};

/// Reads and parses one delimited file: bytes in, lenient UTF-8
/// decode, total parse. `NotFound` becomes [`Error::Missing`],
/// anything else becomes [`Error::Io`].
fn load_table(path: &Path, opts: &TableOptions) -> Result<Table> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(Error::Missing {
                path: path.display().to_string(),
            });
        }
        Err(e) => {
            return Err(Error::Io {
                path: path.display().to_string(),
                source: e,
            });
        }
    };
    let text = String::from_utf8_lossy(&bytes);
    Ok(parse_delimited(&text, opts.delimiter, opts.header))
}

/// Compares two delimited files. See the crate documentation.
pub(crate) fn compare_table_files(
    left: &Path,
    right: &Path,
    opts: &TableOptions,
) -> Result<TableComparison> {
    let left_table = load_table(left, opts)?;
    let right_table = load_table(right, opts)?;
    let script = diff_rows(&left_table.rows, &right_table.rows);
    Ok(TableComparison {
        header_l: left_table.header,
        header_r: right_table.header,
        rows_l: left_table.rows,
        rows_r: right_table.rows,
        script,
    })
}

/// Options for key-aligned tolerant table file comparison.
///
/// `TableOptions` stays the plain sequential shape used by the CLI and
/// GUI; these options layer Beyond Compare parity (key alignment plus
/// numeric tolerance, case folding and space trimming) on top.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TableKeyOptions {
    /// Columns used as the alignment key (empty = positional compare).
    pub key_columns: Vec<usize>,
    /// Numeric tolerance for cell comparison (see
    /// [`autocompare_diff::cells_equal_with_tolerance`]).
    pub numeric_tolerance: Option<f64>,
    /// Fold case before comparing text.
    pub ignore_case: bool,
    /// Trim surrounding whitespace before comparing text.
    pub trim_space: bool,
}

/// The result of comparing two delimited files aligned by key columns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableKeyFileComparison {
    /// Header cells of the left table (empty when no header).
    pub header_l: Vec<String>,
    /// Header cells of the right table.
    pub header_r: Vec<String>,
    /// Key-aligned row differences with tolerant cell comparison.
    pub diff: TableKeyDiff,
}

/// Compares two delimited files aligned by key columns with tolerant
/// cell comparison. Files are read, decoded and parsed exactly like
/// [`compare_table_files`]; only the row alignment and cell equality
/// follow `key_opts` (see
/// [`autocompare_diff::diff_table_by_key_with_options`]).
pub(crate) fn compare_table_files_by_key(
    left: &Path,
    right: &Path,
    opts: &TableOptions,
    key_opts: &TableKeyOptions,
) -> Result<TableKeyFileComparison> {
    let left_table = load_table(left, opts)?;
    let right_table = load_table(right, opts)?;
    let cmp_opts = TableCompareOptions {
        key_columns: key_opts.key_columns.clone(),
        numeric_tolerance: key_opts.numeric_tolerance,
        ignore_case: key_opts.ignore_case,
        trim_space: key_opts.trim_space,
    };
    let diff = diff_table_by_key_with_options(&left_table, &right_table, &cmp_opts);
    Ok(TableKeyFileComparison {
        header_l: left_table.header,
        header_r: right_table.header,
        diff,
    })
}

/// Detects table delimiter from file extension or content (supports CSV and TSV).
#[allow(dead_code)]
pub fn detect_table_delimiter(path: &Path, content: &str) -> char {
    if path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|s| s.eq_ignore_ascii_case("tsv") || s.eq_ignore_ascii_case("tab"))
        .unwrap_or(false)
    {
        return '\t';
    }
    if content.contains('\t') && !content.contains(',') {
        '\t'
    } else {
        ','
    }
}

/// Reads and parses a tab-separated values (TSV) file.
#[allow(dead_code)]
pub fn load_table_tsv(path: &Path, header: bool) -> Result<Table> {
    let opts = TableOptions {
        delimiter: '\t',
        header,
    };
    load_table(path, &opts)
}

/// Compares two tab-separated values (TSV) files.
#[allow(dead_code)]
pub fn compare_table_files_tsv(left: &Path, right: &Path, header: bool) -> Result<TableComparison> {
    let opts = TableOptions {
        delimiter: '\t',
        header,
    };
    compare_table_files(left, right, &opts)
}

/// Compares two delimited files aligned by key columns after sorting their data
/// rows across `sort_columns` (sort-before-align parity).
#[allow(dead_code)]
pub fn compare_table_files_by_key_sorted(
    left: &Path,
    right: &Path,
    opts: &TableOptions,
    key_opts: &TableKeyOptions,
    sort_columns: &[usize],
    ascending: bool,
) -> Result<TableKeyFileComparison> {
    let mut left_table = load_table(left, opts)?;
    let mut right_table = load_table(right, opts)?;
    sort_table_rows(&mut left_table, sort_columns, ascending);
    sort_table_rows(&mut right_table, sort_columns, ascending);
    let cmp_opts = TableCompareOptions {
        key_columns: key_opts.key_columns.clone(),
        numeric_tolerance: key_opts.numeric_tolerance,
        ignore_case: key_opts.ignore_case,
        trim_space: key_opts.trim_space,
    };
    let diff = diff_table_by_key_with_options(&left_table, &right_table, &cmp_opts);
    Ok(TableKeyFileComparison {
        header_l: left_table.header,
        header_r: right_table.header,
        diff,
    })
}

/// Loads a multi-sheet Excel (.xlsx) workbook from the filesystem.
#[allow(dead_code)]
pub fn load_workbook(path: &Path) -> Result<Workbook> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(Error::Missing {
                path: path.display().to_string(),
            });
        }
        Err(e) => {
            return Err(Error::Io {
                path: path.display().to_string(),
                source: e,
            });
        }
    };
    Workbook::from_xlsx(&bytes).ok_or_else(|| Error::Unsupported {
        path: path.display().to_string(),
        what: "not a decodable .xlsx workbook",
    })
}

/// The result of comparing two multi-sheet Excel workbooks.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub struct WorkbookComparison {
    /// Multi-sheet difference across all sheets.
    pub diff: WorkbookDiff,
}

/// Compares two multi-sheet Excel (.xlsx) workbooks sheet by sheet.
#[allow(dead_code)]
pub fn compare_workbook_files(left: &Path, right: &Path) -> Result<WorkbookComparison> {
    let wb_l = load_workbook(left)?;
    let wb_r = load_workbook(right)?;
    let diff = diff_workbooks(&wb_l, &wb_r);
    Ok(WorkbookComparison { diff })
}
