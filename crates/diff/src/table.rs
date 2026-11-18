//! Delimited (CSV-style) table parsing and cell comparison.
//!
//! Pure layer 0: no I/O, no panics. Parsing is total — malformed
//! input is preserved as parsed, never an error.

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

/// Parses delimiter-separated text, RFC-4180-lite: quoted fields with
/// doubled-quote escapes; delimiters and newlines inside quotes are
/// data; `\r\n` is tolerated; a trailing newline is not a row. Never
/// errors: uneven or unterminated input is preserved as parsed (an
/// unterminated quoted field ends at end of input).
pub fn parse_delimited(text: &str, delimiter: char, first_row_is_header: bool) -> Table {
    // STUB: replaced by the real implementation in crates/diff.
    let _ = (text, delimiter, first_row_is_header);
    Table::default()
}

/// Row-level diff over whole cell rows: the same Myers engine used
/// for lines, over `Vec<String>` rows.
pub fn diff_rows(a: &[Vec<String>], b: &[Vec<String>]) -> DiffScript {
    let (ops, exact) = engine::diff_slices(a, b, Some(EDIT_BUDGET));
    DiffScript { ops, exact }
}

/// Indices of columns whose cells differ between two rows, compared
/// position by position; a cell missing on one side counts as
/// differing. Sorted, no duplicates.
pub fn diff_cells(a: &[String], b: &[String]) -> Vec<usize> {
    // STUB: replaced by the real implementation in crates/diff.
    let _ = (a, b);
    Vec::new()
}
