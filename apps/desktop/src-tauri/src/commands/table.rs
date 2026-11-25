//! Table (delimited CSV) compare IPC surface for the desktop shell.
//!
//! Ownership: wave-1b. Wave-1R wires these handlers into the Tauri
//! invoke handler (`lib.rs`) and owns `commands/mod.rs`.
//!
//! # Paged-fetch rule
//!
//! No command serializes a whole table over IPC. [`table_compare`]
//! parses both files once (capped at [`TABLE_MAX_BYTES`); the parsed
//! rows live only in the server-side session store) and returns a
//! summary; [`table_get_page`] serves bounded cell windows (`limit`
//! clamped to [`MAX_PAGE_ROWS`], `col_limit` to [`MAX_PAGE_COLS`]);
//! [`table_find`] returns at most [`MAX_FIND_HITS`] hits.
//!
//! # Engine swap note (Wave-1R / later wave)
//!
//! This module parses CSV directly (RFC-4180-lite: quoted fields with
//! doubled-quote escapes, `\r\n` tolerated, first row treated as the
//! header, blank lines ignored) and aligns key mode with a
//! first-unique-column rule so it needs no new crate dependency (this
//! wave may not touch `Cargo.toml`). Once `autocompare-compare` is
//! added to the desktop crate, delegate the summary step to
//! `autocompare_compare::compare_table_files` (positional) and
//! `compare_table_files_by_key` (key mode, with `TableKeyOptions`
//! derived from the detected key columns) and map the results onto
//! [`TableSummary`]. DTO shapes stay unchanged, so `ipc/table.ts` is
//! unaffected.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{LazyLock, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use super::{read_file_meta, FileMetaDto};

/// Maximum table file size accepted for parsing (both sides).
pub const TABLE_MAX_BYTES: u64 = 8 * 1024 * 1024;

/// Maximum rows served by one [`table_get_page`] call (per side).
pub const MAX_PAGE_ROWS: u64 = 1000;

/// Maximum columns served by one [`table_get_page`] call.
pub const MAX_PAGE_COLS: u64 = 128;

/// Maximum hits returned by one [`table_find`] call (both sides).
pub const MAX_FIND_HITS: usize = 100;

/// Summary of one table comparison. Mirrors `ipc/table.ts` `TableSummary`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TableSummary {
    /// Opaque session id for follow-up `table_get_page` / `table_find` calls.
    pub session_id: String,
    /// Data-row count on the left (header excluded).
    pub left_rows: u64,
    /// Data-row count on the right (header excluded).
    pub right_rows: u64,
    /// Widest row width on the left.
    pub left_cols: u64,
    /// Widest row width on the right.
    pub right_cols: u64,
    /// True when headers and aligned rows are identical.
    pub equal: bool,
    /// Echoes the requested mode (see `key_columns` for fallback).
    pub by_key: bool,
    /// Total difference rows over the FULL tables (key mode:
    /// modified + left-only + right-only; positional: zipped
    /// full-row mismatches + length delta). Drives the wave-69
    /// status bar total.
    pub diff_rows: u64,
    /// Key columns used for alignment; empty when positional was used
    /// (either not requested, or no header / no unique column found).
    pub key_columns: Vec<u64>,
    #[serde(default)]
    pub left_meta: Option<FileMetaDto>,
    #[serde(default)]
    pub right_meta: Option<FileMetaDto>,
}

/// One bounded cell window over both tables.
/// Mirrors `ipc/table.ts` `TablePage`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TablePage {
    /// Session this page belongs to.
    pub session_id: String,
    /// Requested first data-row index (0-based, header excluded).
    pub start_row: u64,
    /// Effective row limit (requested `limit` clamped to [`MAX_PAGE_ROWS`]).
    pub limit: u64,
    /// Requested first column index (0-based).
    pub start_col: u64,
    /// Effective column limit (clamped to [`MAX_PAGE_COLS`]).
    pub col_limit: u64,
    /// Left header window.
    pub header_l: Vec<String>,
    /// Right header window.
    pub header_r: Vec<String>,
    /// Left data-row window.
    pub rows_l: Vec<Vec<String>>,
    /// Right data-row window.
    pub rows_r: Vec<Vec<String>>,
    /// Total left data rows.
    pub total_rows_l: u64,
    /// Total right data rows.
    pub total_rows_r: u64,
    /// Widest row across both sides.
    pub total_cols: u64,
    /// True when rows or columns remain beyond this window.
    pub truncated: bool,
}

/// One substring hit. Row/col are 0-based data-cell indexes on `side`.
/// Mirrors `ipc/table.ts` `TableHit`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TableHit {
    /// `"left"` or `"right"`.
    pub side: String,
    /// Data-row index (header excluded).
    pub row: u64,
    /// Column index.
    pub col: u64,
    /// Cell text.
    pub value: String,
}

/// Server-side session: parsed tables plus the effective key columns.
/// Parsed rows never cross IPC except through bounded pages/hits; the
/// summary itself is returned to the caller at compare time.
#[derive(Debug, Clone)]
struct TableSession {
    header_l: Vec<String>,
    header_r: Vec<String>,
    rows_l: Vec<Vec<String>>,
    rows_r: Vec<Vec<String>>,
}

static SESSIONS: LazyLock<Mutex<HashMap<String, TableSession>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

fn new_session_id(prefix: &str) -> String {
    let n = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let nanos: u128 = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(d) => d.as_nanos(),
        Err(_) => 0,
    };
    format!("{prefix}-{nanos:x}-{n}-{pid}", pid = std::process::id())
}

fn missing_msg(path: &Path) -> String {
    format!("path not found: {}", path.display())
}

fn io_msg(path: &Path, err: &std::io::Error) -> String {
    format!("reading {} failed: {err}", path.display())
}

fn open_msg(path: &Path, err: &std::io::Error) -> String {
    if err.kind() == std::io::ErrorKind::NotFound {
        missing_msg(path)
    } else {
        io_msg(path, err)
    }
}

fn store_msg() -> String {
    "table session store unavailable".to_string()
}

fn unknown_session_msg(id: &str) -> String {
    format!("unknown table session: {id}")
}

fn get_session(id: &str) -> Result<TableSession, String> {
    match SESSIONS.lock() {
        Ok(guard) => match guard.get(id) {
            Some(session) => Ok(session.clone()),
            None => Err(unknown_session_msg(id)),
        },
        Err(_) => Err(store_msg()),
    }
}

fn put_session(id: &str, session: TableSession) -> Result<(), String> {
    match SESSIONS.lock() {
        Ok(mut guard) => {
            guard.insert(id.to_string(), session);
            Ok(())
        }
        Err(_) => Err(store_msg()),
    }
}

/// Pushes the pending field/row; lone-empty rows (blank lines and the
/// phantom row behind a trailing newline) are ignored.
fn end_row(field: &mut String, row: &mut Vec<String>, rows: &mut Vec<Vec<String>>) {
    row.push(std::mem::take(field));
    let lone_empty = match row.len() {
        1 => match row.first() {
            Some(cell) => cell.is_empty(),
            None => false,
        },
        _ => false,
    };
    if lone_empty {
        row.clear();
    } else {
        rows.push(std::mem::take(row));
    }
}

/// Parses one delimited file: rows of cells with `delim` separators,
/// RFC-4180-lite quoting (doubled `""` escapes, delimiters and
/// newlines inside quotes are data) and `\r\n` tolerance.
fn parse_csv(text: &str, delim: char) -> Vec<Vec<String>> {
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut field = String::new();
    let mut in_quotes = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if in_quotes {
            if c == '"' {
                match chars.peek() {
                    Some('"') => {
                        chars.next();
                        field.push('"');
                    }
                    _ => {
                        in_quotes = false;
                    }
                }
            } else {
                field.push(c);
            }
        } else if c == '"' && field.is_empty() {
            in_quotes = true;
        } else if c == delim {
            row.push(std::mem::take(&mut field));
        } else if c == '\n' {
            end_row(&mut field, &mut row, &mut rows);
        } else if c == '\r' {
            if chars.peek() == Some(&'\n') {
                chars.next();
            }
            end_row(&mut field, &mut row, &mut rows);
        } else {
            field.push(c);
        }
    }
    if !field.is_empty() || !row.is_empty() {
        end_row(&mut field, &mut row, &mut rows);
    }
    rows
}

/// Reads, size-caps and parses one table file into `(header, rows)`.
/// The first row is the header (empty when the file has no rows).
/// `delim` is the resolved field separator (`,` when the caller sends
/// no `delimiter`).
fn load_table(path: &Path, delim: char) -> Result<(Vec<String>, Vec<Vec<String>>), String> {
    let size = match std::fs::metadata(path) {
        Ok(meta) => meta.len(),
        Err(err) => return Err(open_msg(path, &err)),
    };
    if size > TABLE_MAX_BYTES {
        return Err(format!(
            "{}: table too large for desktop compare (8 MiB cap)",
            path.display()
        ));
    }
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(err) => return Err(open_msg(path, &err)),
    };
    let text = String::from_utf8_lossy(bytes.as_slice());
    let mut rows = parse_csv(text.as_ref(), delim);
    if rows.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }
    let header = rows.remove(0);
    Ok((header, rows))
}

fn width(rows: &[Vec<String>]) -> usize {
    let mut w = 0usize;
    for row in rows {
        w = w.max(row.len());
    }
    w
}

/// Finds the first column whose cells are all non-empty and unique.
/// Mirrors the single-column pass of the engine's key detection.
fn detect_key_column(rows: &[Vec<String>]) -> Option<usize> {
    let cols = width(rows);
    for col in 0..cols {
        let mut seen: HashSet<&str> = HashSet::new();
        let mut ok = true;
        for row in rows {
            match row.get(col) {
                Some(cell) if !cell.trim().is_empty() => {
                    if !seen.insert(cell.as_str()) {
                        ok = false;
                        break;
                    }
                }
                _ => {
                    ok = false;
                    break;
                }
            }
        }
        if ok && seen.len() == rows.len() {
            return Some(col);
        }
    }
    None
}

/// Resolves the optional `delimiter` wire param to one separator
/// char. Omitted or empty means `,` (legacy path byte-identical); a
/// single char wins; anything longer is an honest error (prevents
/// silent mis-splits).
fn resolve_delimiter(raw: Option<String>) -> Result<char, String> {
    match raw {
        None => Ok(','),
        Some(s) if s.is_empty() => Ok(','),
        Some(s) => match s.chars().count() {
            1 => Ok(s.chars().next().unwrap_or(',')),
            _ => Err("delimiter must be a single character".to_string()),
        },
    }
}

/// Resolves the optional numeric tolerance from its three accepted
/// spellings (`numericTolerance` / `numeric_tolerance` / `epsilon`,
/// first `Some` wins). Omitted means exact numeric equality; a set
/// value must be finite and `>= 0` (mirrors the engine
/// `cells_equal_with_tolerance` contract).
fn resolve_numeric_tolerance(
    numeric_tolerance_camel: Option<f64>,
    numeric_tolerance_snake: Option<f64>,
    epsilon: Option<f64>,
) -> Result<Option<f64>, String> {
    let picked = numeric_tolerance_camel
        .or(numeric_tolerance_snake)
        .or(epsilon);
    match picked {
        None => Ok(None),
        Some(v) if v.is_finite() && v >= 0.0 => Ok(Some(v)),
        Some(_) => Err("numeric tolerance must be a finite number >= 0".to_string()),
    }
}

/// Parses both cells as finite `f64` (surrounding whitespace ignored)
/// and reports `|a - b| <= tol`. Non-numeric text, NaN/infinite
/// values and invalid tolerances never match (mirrors the engine
/// `cells_equal_with_tolerance` numeric half).
fn numeric_cells_equal(a: &str, b: &str, tol: f64) -> bool {
    if !(tol.is_finite() && tol >= 0.0) {
        return false;
    }
    let (Ok(x), Ok(y)) = (a.trim().parse::<f64>(), b.trim().parse::<f64>()) else {
        return false;
    };
    if !x.is_finite() || !y.is_finite() {
        return false;
    }
    (x - y).abs() <= tol
}

/// Normalises one cell for tolerant comparison: trims and collapses
/// whitespace runs when `ignore_space`, case-folds when `ignore_case`.
/// Displayed cells are untouched; only equality goes through here.
fn norm_cell(cell: &str, ignore_case: bool, ignore_space: bool) -> String {
    let mut s = if ignore_space {
        cell.split_whitespace().collect::<Vec<&str>>().join(" ")
    } else {
        cell.to_owned()
    };
    if ignore_case {
        s = s.to_lowercase();
    }
    s
}

fn cells_equal(
    a: &str,
    b: &str,
    ignore_case: bool,
    ignore_space: bool,
    numeric_tol: Option<f64>,
) -> bool {
    // Exact win first, then the numeric-tolerance half (engine order:
    // exact, case-fold via flags below, numeric). A set tolerance lets
    // "1.001" equal "1.002" at 0.01 without touching text equality.
    if a == b {
        return true;
    }
    if let Some(tol) = numeric_tol
        && numeric_cells_equal(a, b, tol)
    {
        return true;
    }
    norm_cell(a, ignore_case, ignore_space) == norm_cell(b, ignore_case, ignore_space)
}

fn rows_equal(
    a: &[String],
    b: &[String],
    ignore_case: bool,
    ignore_space: bool,
    numeric_tol: Option<f64>,
) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b.iter())
            .all(|(x, y)| cells_equal(x, y, ignore_case, ignore_space, numeric_tol))
}

/// Aligns rows by key column and counts `(matching, modified,
/// left_only, right_only)`. Rows with empty/missing keys count as
/// one-sided; duplicate right keys attach to their first row.
/// Key lookup and row equality honour `ignore_case` / `ignore_space`.
fn align_counts(
    left: &[Vec<String>],
    right: &[Vec<String>],
    key: usize,
    ignore_case: bool,
    ignore_space: bool,
    numeric_tol: Option<f64>,
) -> (u64, u64, u64, u64) {
    let mut right_index: HashMap<String, usize> = HashMap::new();
    let mut right_no_key: u64 = 0;
    for (i, row) in right.iter().enumerate() {
        match row.get(key) {
            Some(cell) if !cell.trim().is_empty() => {
                let norm = norm_cell(cell, ignore_case, ignore_space);
                if !right_index.contains_key(&norm) {
                    right_index.insert(norm, i);
                }
            }
            _ => {
                right_no_key = right_no_key.saturating_add(1);
            }
        }
    }
    let mut used: HashSet<usize> = HashSet::new();
    let mut matching: u64 = 0;
    let mut modified: u64 = 0;
    let mut left_only: u64 = 0;
    for row in left {
        let key_text: Option<String> = match row.get(key) {
            Some(cell) if !cell.trim().is_empty() => {
                Some(norm_cell(cell, ignore_case, ignore_space))
            }
            _ => None,
        };
        match key_text {
            Some(k) => match right_index.get(&k) {
                Some(pos) => {
                    used.insert(*pos);
                    let same = match right.get(*pos) {
                        Some(rrow) => rows_equal(rrow, row, ignore_case, ignore_space, numeric_tol),
                        None => false,
                    };
                    if same {
                        matching = matching.saturating_add(1);
                    } else {
                        modified = modified.saturating_add(1);
                    }
                }
                None => {
                    left_only = left_only.saturating_add(1);
                }
            },
            None => {
                left_only = left_only.saturating_add(1);
            }
        }
    }
    let mut right_only = right_no_key;
    for (i, _) in right.iter().enumerate() {
        if !used.contains(&i) {
            right_only = right_only.saturating_add(1);
        }
    }
    (matching, modified, left_only, right_only)
}

fn as_usize_saturating(v: u64) -> usize {
    match usize::try_from(v) {
        Ok(n) => n,
        Err(_) => usize::MAX,
    }
}

/// Cuts one bounded cell window out of parsed rows.
fn page_rows(
    rows: &[Vec<String>],
    start_row: u64,
    limit: u64,
    start_col: u64,
    col_limit: u64,
) -> Vec<Vec<String>> {
    rows.iter()
        .skip(as_usize_saturating(start_row))
        .take(as_usize_saturating(limit))
        .map(|row| {
            row.iter()
                .skip(as_usize_saturating(start_col))
                .take(as_usize_saturating(col_limit))
                .cloned()
                .collect()
        })
        .collect()
}

fn page_cells(cells: &[String], start_col: u64, col_limit: u64) -> Vec<String> {
    cells
        .iter()
        .skip(as_usize_saturating(start_col))
        .take(as_usize_saturating(col_limit))
        .cloned()
        .collect()
}

/// Compares two CSV files and opens a table session.
///
/// `by_key` requests key-aligned comparison: when both files have
/// headers and the left table has a unique non-empty column, rows
/// align by that column; otherwise the summary falls back to
/// positional comparison and `key_columns` is empty. Returns row/col
/// counts, the equality flag and the effective key columns -- never
/// table contents; page those with [`table_get_page`].
///
/// Wave-66M: `key_columns` carries user-designated key columns (Beyond
/// Compare parity). A non-empty list wins when EVERY index is in-range
/// for BOTH tables; otherwise (empty list, or any index out of range)
/// the call falls back to the pre-66M auto-detect path byte-identically.
///
/// Wave-163c: optional `ignoreCase`/`ignore_case` and
/// `ignoreSpace`/`ignore_space` tolerate case and whitespace in cell
/// comparison (positional and key mode alike). Either spelling wins;
/// omitted flags mean byte-identical cells. Old callers sending only
/// `{left, right, by_key, key_columns}` are unaffected.
///
/// Wave-C2c: optional `delimiter` (single-char string, omitted/empty
/// = `,`), plus optional numeric tolerance under three spellings
/// (`numericTolerance` / `numeric_tolerance` / `epsilon`, first `Some`
/// wins, must be finite `>= 0`). The tolerance lets near-equal numbers
/// (`1.001` vs `1.002` at `0.01`) compare equal in positional and key
/// mode alike (engine `cells_equal_with_tolerance` parity); text
/// equality is untouched.
#[tauri::command]
#[allow(non_snake_case)]
pub fn table_compare(
    left: String,
    right: String,
    by_key: bool,
    key_columns: Vec<u64>,
    ignoreCase: Option<bool>,
    ignore_case: Option<bool>,
    ignoreSpace: Option<bool>,
    ignore_space: Option<bool>,
    delimiter: Option<String>,
    numericTolerance: Option<f64>,
    numeric_tolerance: Option<f64>,
    epsilon: Option<f64>,
) -> Result<TableSummary, String> {
    let ignore_cs = ignoreCase.or(ignore_case).unwrap_or(false);
    let ignore_ws = ignoreSpace.or(ignore_space).unwrap_or(false);
    let delim = resolve_delimiter(delimiter)?;
    let num_tol = resolve_numeric_tolerance(numericTolerance, numeric_tolerance, epsilon)?;
    let left_path = Path::new(left.as_str());
    let right_path = Path::new(right.as_str());
    let (header_l, rows_l) = load_table(left_path, delim)?;
    let (header_r, rows_r) = load_table(right_path, delim)?;

    let left_w = width(rows_l.as_slice());
    let right_w = width(rows_r.as_slice());
    let mut keys: Vec<usize> = Vec::new();
    // Wave-66M explicit override: every requested index must address a
    // real data column of BOTH tables; anything else falls through to
    // the auto-detect below, unchanged.
    let explicit: Vec<usize> = key_columns
        .iter()
        .map(|k| as_usize_saturating(*k))
        .collect();
    if !explicit.is_empty() && explicit.iter().all(|c| *c < left_w && *c < right_w) {
        keys = explicit;
    } else if by_key
        && !header_l.is_empty()
        && !header_r.is_empty()
        && let Some(col) = detect_key_column(rows_l.as_slice())
        && col < right_w.max(1)
    {
        keys.push(col);
    }
    let (equal, diff_rows) = if keys.is_empty() {
        // Positional (requested or fallback): zipped full-row
        // mismatches plus the length delta. Cell equality honours the
        // wave-163c ignore flags plus the wave-C2c numeric tolerance.
        let mut mismatches: u64 = 0;
        for (row_l, row_r) in rows_l.iter().zip(rows_r.iter()) {
            if !rows_equal(row_l, row_r, ignore_cs, ignore_ws, num_tol) {
                mismatches = mismatches.saturating_add(1);
            }
        }
        let delta = rows_l.len().abs_diff(rows_r.len()) as u64;
        let diff_rows = mismatches.saturating_add(delta);
        // Wave-C2c: numeric tolerance applies to body cells only;
        // headers are names (exact-or-flagged), matching key mode.
        let equal = rows_equal(&header_l, &header_r, ignore_cs, ignore_ws, None)
            && rows_l.len() == rows_r.len()
            && diff_rows == 0;
        (equal, diff_rows)
    } else {
        let key = match keys.first() {
            Some(k) => *k,
            None => 0,
        };
        let (_, modified, left_only, right_only) =
            align_counts(rows_l.as_slice(), rows_r.as_slice(), key, ignore_cs, ignore_ws, num_tol);
        let diff_rows = modified
            .saturating_add(left_only)
            .saturating_add(right_only);
        // Wave-C2c: headers stay exact-or-flagged (no numeric tolerance
        // on header names); body equality carries the tolerance.
        let equal = rows_equal(&header_l, &header_r, ignore_cs, ignore_ws, None)
            && modified == 0
            && left_only == 0
            && right_only == 0;
        (equal, diff_rows)
    };

    let summary = TableSummary {
        session_id: new_session_id("table"),
        left_rows: rows_l.len() as u64,
        right_rows: rows_r.len() as u64,
        left_cols: width(rows_l.as_slice()) as u64,
        right_cols: width(rows_r.as_slice()) as u64,
        equal,
        by_key,
        diff_rows,
        key_columns: keys.iter().map(|k| *k as u64).collect(),
        left_meta: read_file_meta(&left),
        right_meta: read_file_meta(&right),
    };
    put_session(
        summary.session_id.as_str(),
        TableSession {
            header_l,
            header_r,
            rows_l,
            rows_r,
        },
    )?;
    Ok(summary)
}

/// Serves one bounded cell window over both session tables.
///
/// `limit` is clamped to [`MAX_PAGE_ROWS`], `col_limit` to
/// [`MAX_PAGE_COLS`]. Starts past the end yield empty row lists with
/// `truncated == false`.
#[tauri::command]
pub fn table_get_page(
    session: String,
    start_row: u64,
    limit: u64,
    start_col: u64,
    col_limit: u64,
) -> Result<TablePage, String> {
    let snapshot = get_session(session.as_str())?;
    let eff_limit = limit.min(MAX_PAGE_ROWS);
    let eff_cols = col_limit.min(MAX_PAGE_COLS);
    let rows_l = page_rows(
        snapshot.rows_l.as_slice(),
        start_row,
        eff_limit,
        start_col,
        eff_cols,
    );
    let rows_r = page_rows(
        snapshot.rows_r.as_slice(),
        start_row,
        eff_limit,
        start_col,
        eff_cols,
    );
    let total_rows_l = snapshot.rows_l.len() as u64;
    let total_rows_r = snapshot.rows_r.len() as u64;
    let total_cols = (width(snapshot.rows_l.as_slice()))
        .max(width(snapshot.rows_r.as_slice()))
        .max(snapshot.header_l.len())
        .max(snapshot.header_r.len()) as u64;
    let rows_done_l = start_row.saturating_add(rows_l.len() as u64) >= total_rows_l;
    let rows_done_r = start_row.saturating_add(rows_r.len() as u64) >= total_rows_r;
    let cols_done = start_col.saturating_add(eff_cols) >= total_cols;
    Ok(TablePage {
        session_id: session,
        start_row,
        limit: eff_limit,
        start_col,
        col_limit: eff_cols,
        header_l: page_cells(snapshot.header_l.as_slice(), start_col, eff_cols),
        header_r: page_cells(snapshot.header_r.as_slice(), start_col, eff_cols),
        rows_l,
        rows_r,
        total_rows_l,
        total_rows_r,
        total_cols,
        truncated: !rows_done_l || !rows_done_r || !cols_done,
    })
}

/// Wave-66G: numeric-aware find — substring win, else exact f64 equality (mirrors the exact-equality half of cells_equal_with_tolerance with no tolerance: find carries none).
fn cell_matches_find(query: &str, cell: &str) -> bool {
    if cell.contains(query) {
        return true;
    }
    let q = query.trim();
    let c = cell.trim();
    if q.is_empty() || c.is_empty() {
        return false;
    }
    match (q.parse::<f64>(), c.parse::<f64>()) {
        (Ok(qn), Ok(cn)) if qn.is_finite() && cn.is_finite() => qn == cn,
        _ => false,
    }
}

/// Searches both session tables (data cells only; substring or exact
/// numeric equality) and
/// returns at most [`MAX_FIND_HITS`] hits in row-major order, left
/// table first. Empty queries are an error.
#[tauri::command]
pub fn table_find(session: String, query: String) -> Result<Vec<TableHit>, String> {
    if query.is_empty() {
        return Err("empty table query".to_string());
    }
    let snapshot = get_session(session.as_str())?;
    let mut hits: Vec<TableHit> = Vec::new();
    let sides: [(&str, &Vec<Vec<String>>); 2] =
        [("left", &snapshot.rows_l), ("right", &snapshot.rows_r)];
    for (side, rows) in sides {
        for (r, row) in rows.iter().enumerate() {
            for (c, cell) in row.iter().enumerate() {
                if cell_matches_find(query.as_str(), cell.as_str()) {
                    if hits.len() >= MAX_FIND_HITS {
                        return Ok(hits);
                    }
                    hits.push(TableHit {
                        side: side.to_string(),
                        row: r as u64,
                        col: c as u64,
                        value: cell.clone(),
                    });
                }
            }
        }
    }
    Ok(hits)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_path(tag: &str) -> PathBuf {
        let n = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "wave1b-table-{tag}-{pid}-{n}.csv",
            pid = std::process::id()
        ))
    }

    fn write_text(path: &Path, text: &str) -> bool {
        std::fs::write(path, text).is_ok()
    }

    fn remove_quiet(path: &Path) {
        let _ = std::fs::remove_file(path);
    }

    fn open_session(
        left: &str,
        right: &str,
        by_key: bool,
        key_columns: Vec<u64>,
    ) -> Result<TableSummary, String> {
        open_session_with_flags(left, right, by_key, key_columns, None, None)
    }

    fn open_session_with_flags(
        left: &str,
        right: &str,
        by_key: bool,
        key_columns: Vec<u64>,
        ignore_case: Option<bool>,
        ignore_space: Option<bool>,
    ) -> Result<TableSummary, String> {
        let l = temp_path("l");
        let r = temp_path("r");
        if !write_text(&l, left) || !write_text(&r, right) {
            remove_quiet(&l);
            remove_quiet(&r);
            return Err("test fixture write failed".to_string());
        }
        let res = table_compare(
            l.display().to_string(),
            r.display().to_string(),
            by_key,
            key_columns,
            ignore_case,
            None,
            ignore_space,
            None,
            None,
            None,
            None,
            None,
        );
        remove_quiet(&l);
        remove_quiet(&r);
        res
    }

    #[test]
    fn identical_tables_compare_equal() {
        let csv = "id,name\n1,amy\n2,bo\n";
        let res = open_session(csv, csv, false, Vec::new());
        assert!(res.is_ok());
        let summary = match res {
            Ok(s) => s,
            Err(_) => return,
        };
        assert!(summary.equal);
        assert_eq!(summary.left_rows, 2);
        assert_eq!(summary.right_rows, 2);
        assert_eq!(summary.left_cols, 2);
        assert_eq!(summary.key_columns, Vec::<u64>::new());
        assert!(!summary.session_id.is_empty());
    }

    #[test]
    fn differing_cell_is_not_equal() {
        let res = open_session("id,name\n1,amy\n", "id,name\n1,bo\n", false, Vec::new());
        assert!(res.is_ok());
        let summary = match res {
            Ok(s) => s,
            Err(_) => return,
        };
        assert!(!summary.equal);
        assert_eq!(summary.left_rows, 1);
    }

    #[test]
    fn ignore_case_flag_tolerates_cell_case_positional() {
        let left = "id,name\n1,Amy\n";
        let right = "id,name\n1,amy\n";
        let strict = open_session_with_flags(left, right, false, Vec::new(), None, None);
        let strict_summary = match strict {
            Ok(s) => s,
            Err(_) => return,
        };
        assert!(!strict_summary.equal);
        let folded = open_session_with_flags(left, right, false, Vec::new(), Some(true), None);
        let summary = match folded {
            Ok(s) => s,
            Err(_) => return,
        };
        assert!(summary.equal);
        assert_eq!(summary.diff_rows, 0);
    }

    #[test]
    fn ignore_space_flag_tolerates_cell_whitespace_positional() {
        let left = "id,name\n1,  amy   jones \n";
        let right = "id,name\n1,amy jones\n";
        let strict = open_session_with_flags(left, right, false, Vec::new(), None, None);
        let strict_summary = match strict {
            Ok(s) => s,
            Err(_) => return,
        };
        assert!(!strict_summary.equal);
        let trimmed = open_session_with_flags(left, right, false, Vec::new(), None, Some(true));
        let summary = match trimmed {
            Ok(s) => s,
            Err(_) => return,
        };
        assert!(summary.equal);
        assert_eq!(summary.diff_rows, 0);
    }

    #[test]
    fn ignore_flags_apply_in_key_mode() {
        let left = "id,name\nA1,Amy\n";
        let right = "id,name\na1,amy\n";
        let strict = open_session_with_flags(left, right, true, vec![0], None, None);
        let strict_summary = match strict {
            Ok(s) => s,
            Err(_) => return,
        };
        assert!(!strict_summary.equal);
        // Case-folded keys align and case-folded cells match.
        let folded = open_session_with_flags(left, right, true, vec![0], Some(true), None);
        let summary = match folded {
            Ok(s) => s,
            Err(_) => return,
        };
        assert!(summary.equal);
        assert_eq!(summary.diff_rows, 0);
    }

    #[test]
    fn empty_files_have_zero_counts() {
        let res = open_session("", "", false, Vec::new());
        assert!(res.is_ok());
        let summary = match res {
            Ok(s) => s,
            Err(_) => return,
        };
        assert!(summary.equal);
        assert_eq!(summary.left_rows, 0);
        assert_eq!(summary.right_rows, 0);
        assert_eq!(summary.left_cols, 0);
    }

    #[test]
    fn missing_file_errors() {
        let res = table_compare(
            "/tmp/wave1b-definitely-missing-left.csv".to_string(),
            "/tmp/wave1b-definitely-missing-right.csv".to_string(),
            false,
            Vec::new(),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        );
        assert!(res.is_err());
        let msg = match res {
            Ok(_) => return,
            Err(e) => e,
        };
        assert!(msg.contains("not found"));
    }

    #[test]
    fn page_limits_are_clamped() {
        let csv = "id,name\n1,amy\n2,bo\n";
        let res = open_session(csv, csv, false, Vec::new());
        assert!(res.is_ok());
        let session_id = match res {
            Ok(s) => s.session_id,
            Err(_) => return,
        };
        let page = table_get_page(session_id, 0, u64::MAX, 0, u64::MAX);
        assert!(page.is_ok());
        let page = match page {
            Ok(p) => p,
            Err(_) => return,
        };
        assert!(page.limit <= MAX_PAGE_ROWS);
        assert!(page.col_limit <= MAX_PAGE_COLS);
        assert_eq!(page.rows_l.len(), 2);
        assert_eq!(page.total_rows_l, 2);
        assert_eq!(page.header_l, vec!["id".to_string(), "name".to_string()]);
        assert!(!page.truncated);
    }

    #[test]
    fn page_start_past_end_is_empty() {
        let csv = "id,name\n1,amy\n";
        let res = open_session(csv, csv, false, Vec::new());
        assert!(res.is_ok());
        let session_id = match res {
            Ok(s) => s.session_id,
            Err(_) => return,
        };
        let page = table_get_page(session_id, 100, 10, 50, 10);
        assert!(page.is_ok());
        let page = match page {
            Ok(p) => p,
            Err(_) => return,
        };
        assert!(page.rows_l.is_empty());
        assert!(page.rows_r.is_empty());
        assert!(!page.truncated);
    }

    #[test]
    fn page_unknown_session_errors() {
        let res = table_get_page("table-no-such-session".to_string(), 0, 10, 0, 10);
        assert!(res.is_err());
        let msg = match res {
            Ok(_) => return,
            Err(e) => e,
        };
        assert!(msg.contains("unknown table session"));
    }

    #[test]
    fn key_mode_detects_first_unique_column() {
        // Same rows, shuffled: positional would differ, key mode is equal.
        let left = "id,name\n1,amy\n2,bo\n";
        let right = "id,name\n2,bo\n1,amy\n";
        let res = open_session(left, right, true, Vec::new());
        assert!(res.is_ok());
        let summary = match res {
            Ok(s) => s,
            Err(_) => return,
        };
        assert!(summary.equal);
        assert!(summary.by_key);
        assert_eq!(summary.key_columns, vec![0u64]);
    }

    #[test]
    fn key_mode_reports_modified_rows_as_unequal() {
        let left = "id,name\n1,amy\n2,bo\n";
        let right = "id,name\n1,amy\n2,bea\n";
        let res = open_session(left, right, true, Vec::new());
        assert!(res.is_ok());
        let summary = match res {
            Ok(s) => s,
            Err(_) => return,
        };
        assert!(!summary.equal);
        assert_eq!(summary.key_columns, vec![0u64]);
    }

    #[test]
    fn key_mode_without_unique_key_falls_back() {
        // Duplicate key column: no unique key exists even though
        // headers are present, so key_columns stays empty even
        // though by_key was requested.
        let csv = "id\n1\n1\n";
        let res = open_session(csv, csv, true, Vec::new());
        assert!(res.is_ok());
        let summary = match res {
            Ok(s) => s,
            Err(_) => return,
        };
        assert!(summary.by_key);
        assert!(summary.key_columns.is_empty());
        assert!(summary.equal);
    }

    #[test]
    fn explicit_key_column_is_honored() {
        // Wave-66M: auto-detect would pick column 0 ("id", first unique
        // column); the explicit [1] ("name") designation wins instead and
        // is echoed back. Keyed by name, the shuffled rows still match.
        let left = "id,name\n2,bo\n1,amy\n";
        let right = "id,name\n1,amy\n2,bo\n";
        let res = open_session(left, right, true, vec![1u64]);
        assert!(res.is_ok());
        let summary = match res {
            Ok(s) => s,
            Err(_) => return,
        };
        assert!(summary.equal);
        assert_eq!(summary.key_columns, vec![1u64]);
    }

    #[test]
    fn out_of_range_explicit_key_falls_back_to_auto_detect() {
        // Wave-66M: index 99 addresses no column of either 2-column
        // table, so the request falls back to the legacy auto-detect
        // byte-identically (column 0, rows key-equal).
        let left = "id,name\n2,bo\n1,amy\n";
        let right = "id,name\n1,amy\n2,bo\n";
        let res = open_session(left, right, true, vec![99u64]);
        assert!(res.is_ok());
        let summary = match res {
            Ok(s) => s,
            Err(_) => return,
        };
        assert!(summary.by_key);
        assert_eq!(summary.key_columns, vec![0u64]);
        assert!(summary.equal);
    }

    #[test]
    fn empty_explicit_key_matches_legacy_behavior() {
        // Wave-66M: an empty list is the pre-66M call -- auto-detect
        // under by_key, positional otherwise.
        let left = "id,name\n2,bo\n1,amy\n";
        let right = "id,name\n1,amy\n2,bo\n";
        let keyed = open_session(left, right, true, Vec::new());
        assert!(keyed.is_ok());
        let keyed = match keyed {
            Ok(s) => s,
            Err(_) => return,
        };
        assert_eq!(keyed.key_columns, vec![0u64]);
        assert!(keyed.equal);
        let positional = open_session(left, right, false, Vec::new());
        assert!(positional.is_ok());
        let positional = match positional {
            Ok(s) => s,
            Err(_) => return,
        };
        assert!(positional.key_columns.is_empty());
        assert!(!positional.equal);
    }

    #[test]
    fn find_returns_bounded_hits() {
        let csv = "id,name\n1,amy\n2,bo\n";
        let res = open_session(csv, csv, false, Vec::new());
        assert!(res.is_ok());
        let session_id = match res {
            Ok(s) => s.session_id,
            Err(_) => return,
        };
        let hits = table_find(session_id, "amy".to_string());
        assert!(hits.is_ok());
        let hits = match hits {
            Ok(h) => h,
            Err(_) => return,
        };
        // One hit per side (left table first).
        assert_eq!(hits.len(), 2);
        let first = match hits.first() {
            Some(h) => h,
            None => return,
        };
        assert_eq!(first.side, "left".to_string());
        assert_eq!(first.row, 0);
        assert_eq!(first.col, 1);
        assert_eq!(first.value, "amy".to_string());
    }

    #[test]
    fn find_rejects_empty_query() {
        let csv = "id,name\n1,amy\n";
        let res = open_session(csv, csv, false, Vec::new());
        assert!(res.is_ok());
        let session_id = match res {
            Ok(s) => s.session_id,
            Err(_) => return,
        };
        let hits = table_find(session_id, String::new());
        assert!(hits.is_err());
    }

    #[test]
    fn find_numeric_equality_hits_string_different_cell() {
        let csv = "id,val\n1,42.0\n2,1000\n";
        let res = open_session(csv, csv, false, Vec::new());
        assert!(res.is_ok());
        let session_id = match res {
            Ok(s) => s.session_id,
            Err(_) => return,
        };
        // "42.00" is not a substring of "42.0" but parses to the same f64.
        let hits = table_find(session_id.clone(), "42.00".to_string());
        assert!(hits.is_ok());
        let hits = match hits {
            Ok(h) => h,
            Err(_) => return,
        };
        assert_eq!(hits.len(), 2);
        let first = match hits.first() {
            Some(h) => h,
            None => return,
        };
        assert_eq!(first.side, "left".to_string());
        assert_eq!((first.row, first.col), (0, 1));
        assert_eq!(first.value, "42.0".to_string());
    }

    #[test]
    fn find_comma_query_does_not_numeric_match() {
        let csv = "id,val\n1,42.0\n2,1000\n";
        let res = open_session(csv, csv, false, Vec::new());
        assert!(res.is_ok());
        let session_id = match res {
            Ok(s) => s.session_id,
            Err(_) => return,
        };
        // "1,000" comma-parses to NaN/fails, so it falls back to substring
        // only and never hits "1000".
        let hits = table_find(session_id, "1,000".to_string());
        assert!(hits.is_ok());
        let hits = match hits {
            Ok(h) => h,
            Err(_) => return,
        };
        assert!(hits.is_empty());
    }

    #[test]
    fn diff_rows_key_mode_counts_modified_plus_orphans() {
        // Keyed by `id`: one modified row (id 2: bo -> bea), one
        // left-only orphan (id 3), zero right-only orphans.
        let left = "id,name\n1,amy\n2,bo\n3,cy\n";
        let right = "id,name\n1,amy\n2,bea\n";
        let res = open_session(left, right, true, Vec::new());
        assert!(res.is_ok());
        let summary = match res {
            Ok(s) => s,
            Err(_) => return,
        };
        assert!(!summary.equal);
        assert_eq!(summary.key_columns, vec![0u64]);
        assert_eq!(summary.diff_rows, 2);
    }

    #[test]
    fn diff_rows_positional_counts_mismatch_plus_length_delta() {
        // Positional: row 2 differs (2 vs 9) plus one extra left
        // row (length delta 1) for a total of 2.
        let left = "id\n1\n2\n3\n";
        let right = "id\n1\n9\n";
        let res = open_session(left, right, false, Vec::new());
        assert!(res.is_ok());
        let summary = match res {
            Ok(s) => s,
            Err(_) => return,
        };
        assert!(!summary.equal);
        assert!(summary.key_columns.is_empty());
        assert_eq!(summary.diff_rows, 2);
    }

    #[test]
    fn summary_dto_round_trip() {
        let summary = TableSummary {
            session_id: "table-abc".to_string(),
            left_rows: 2,
            right_rows: 3,
            left_cols: 2,
            right_cols: 2,
            equal: false,
            by_key: true,
            diff_rows: 2,
            key_columns: vec![0],
            left_meta: Some(FileMetaDto {
                bytes: 42,
                mtime_unix: Some(1700000000),
            }),
            right_meta: None,
        };
        let json_result = serde_json::to_string(&summary);
        assert!(json_result.is_ok());
        let json = match json_result {
            Ok(j) => j,
            Err(_) => return,
        };
        assert!(json.contains("sessionId"));
        assert!(json.contains("leftRows"));
        assert!(json.contains("keyColumns"));
        assert!(json.contains("diffRows"));
        assert!(json.contains("\"leftMeta\":{"));
        assert!(json.contains("\"bytes\":42"));
        assert!(json.contains("\"mtimeUnix\":1700000000"));
        assert!(json.contains("\"rightMeta\":null"));
        let back: Result<TableSummary, _> = serde_json::from_str(json.as_str());
        assert!(back.is_ok());
        assert_eq!(back.ok(), Some(summary));
    }

    #[test]
    fn summary_dto_missing_meta_deserializes_to_none() {
        let json = r#"{"sessionId":"s1","leftRows":1,"rightRows":1,"leftCols":1,"rightCols":1,"equal":true,"byKey":false,"diffRows":0,"keyColumns":[]}"#;
        let summary: TableSummary = serde_json::from_str(json).expect("deserialize without meta");
        assert_eq!(summary.left_meta, None);
        assert_eq!(summary.right_meta, None);
    }

    #[test]
    fn table_compare_populates_file_meta_for_fixtures_and_null_for_missing() {
        use std::io::Write;
        let dir = std::env::temp_dir();
        let left = dir.join("wave85-meta-left.csv");
        let right = dir.join("wave85-meta-right.csv");
        let left_content = "id,val\n1,a\n";
        let right_content = "id,val\n1,b\n";
        {
            let mut f1 = std::fs::File::create(&left).expect("create left fixture");
            f1.write_all(left_content.as_bytes()).expect("write left");
            let mut f2 = std::fs::File::create(&right).expect("create right fixture");
            f2.write_all(right_content.as_bytes()).expect("write right");
        }
        let ls = left.display().to_string();
        let rs = right.display().to_string();
        let res = table_compare(ls.clone(), rs.clone(), false, Vec::new(), None, None, None, None, None, None, None, None)
            .expect("table_compare must succeed");

        let left_meta = res.left_meta.expect("left_meta should be populated");
        assert_eq!(left_meta.bytes, left_content.len() as u64);
        assert!(left_meta.mtime_unix.is_some());
        assert!(left_meta.mtime_unix.unwrap() > 0);

        let right_meta = res.right_meta.expect("right_meta should be populated");
        assert_eq!(right_meta.bytes, right_content.len() as u64);
        assert!(right_meta.mtime_unix.is_some());
        assert!(right_meta.mtime_unix.unwrap() > 0);

        // Missing path yields None / null meta without failing
        let missing_meta = read_file_meta("/definitely/not/a/real/path/wave85-missing.csv");
        assert_eq!(missing_meta, None);

        // Verify JSON wire spelling matches contract
        let json = serde_json::to_string(&res).expect("serialize table_compare result");
        assert!(json.contains("\"leftMeta\":{"));
        assert!(json.contains("\"rightMeta\":{"));
        assert!(json.contains("\"bytes\":"));
        assert!(json.contains("\"mtimeUnix\":"));

        let _ = std::fs::remove_file(&left);
        let _ = std::fs::remove_file(&right);
    }

    #[test]
    fn hit_dto_round_trip() {
        let hit = TableHit {
            side: "right".to_string(),
            row: 4,
            col: 1,
            value: "bo".to_string(),
        };
        let json_result = serde_json::to_string(&hit);
        assert!(json_result.is_ok());
        let json = match json_result {
            Ok(j) => j,
            Err(_) => return,
        };
        let back: Result<TableHit, _> = serde_json::from_str(json.as_str());
        assert!(back.is_ok());
        assert_eq!(back.ok(), Some(hit));
    }

    /// Wave-C2c helper: full 12-arg compare over temp fixtures.
    fn open_session_with_pic_opts(
        left: &str,
        right: &str,
        delimiter: Option<String>,
        numeric_tolerance: Option<f64>,
        epsilon: Option<f64>,
    ) -> Result<TableSummary, String> {
        let l = temp_path("picopt-l");
        let r = temp_path("picopt-r");
        if !write_text(&l, left) || !write_text(&r, right) {
            remove_quiet(&l);
            remove_quiet(&r);
            return Err("test fixture write failed".to_string());
        }
        let res = table_compare(
            l.display().to_string(),
            r.display().to_string(),
            false,
            Vec::new(),
            None,
            None,
            None,
            None,
            delimiter,
            numeric_tolerance,
            None,
            epsilon,
        );
        remove_quiet(&l);
        remove_quiet(&r);
        res
    }

    #[test]
    fn delimiter_semicolon_splits_columns() {
        // The delimiter applies to BOTH files (one format per compare).
        // A `;` file under the default parse is a single wide column;
        // under `;` it splits into two — same content, honest col count.
        let semi = "id;name\n1;amy\n";
        let strict = open_session_with_pic_opts(semi, semi, None, None, None);
        let strict_summary = match strict {
            Ok(s) => s,
            Err(_) => return,
        };
        assert!(strict_summary.equal);
        assert_eq!(strict_summary.left_cols, 1);
        let split = open_session_with_pic_opts(semi, semi, Some(";".to_string()), None, None);
        let summary = match split {
            Ok(s) => s,
            Err(_) => return,
        };
        assert!(summary.equal);
        assert_eq!(summary.left_cols, 2);
        assert_eq!(summary.diff_rows, 0);
        // Cross-delimiter files genuinely differ (`,` side stays one col).
        let mixed = open_session_with_pic_opts(semi, "id,name\n1,amy\n", Some(";".to_string()), None, None);
        let mixed_summary = match mixed {
            Ok(s) => s,
            Err(_) => return,
        };
        assert!(!mixed_summary.equal);
    }

    #[test]
    fn delimiter_multi_char_is_rejected() {
        let res = open_session_with_pic_opts("a\n1\n", "a\n1\n", Some(";;".to_string()), None, None);
        assert!(res.is_err());
        let msg = match res {
            Ok(_) => return,
            Err(e) => e,
        };
        assert!(msg.contains("single character"));
    }

    #[test]
    fn numeric_tolerance_equalizes_near_numbers() {
        let left = "id,val\n1,1.001\n";
        let right = "id,val\n1,1.002\n";
        let strict = open_session_with_pic_opts(left, right, None, None, None);
        let strict_summary = match strict {
            Ok(s) => s,
            Err(_) => return,
        };
        assert!(!strict_summary.equal);
        let relaxed = open_session_with_pic_opts(left, right, None, Some(0.01), None);
        let summary = match relaxed {
            Ok(s) => s,
            Err(_) => return,
        };
        assert!(summary.equal);
        assert_eq!(summary.diff_rows, 0);
        // `epsilon` spelling wins identically.
        let via_epsilon = open_session_with_pic_opts(left, right, None, None, Some(0.01));
        let eps_summary = match via_epsilon {
            Ok(s) => s,
            Err(_) => return,
        };
        assert!(eps_summary.equal);
    }

    #[test]
    fn numeric_tolerance_outside_band_stays_unequal() {
        let left = "id,val\n1,1.001\n";
        let right = "id,val\n1,1.02\n";
        let res = open_session_with_pic_opts(left, right, None, Some(0.01), None);
        let summary = match res {
            Ok(s) => s,
            Err(_) => return,
        };
        assert!(!summary.equal);
    }

    #[test]
    fn numeric_tolerance_rejects_bad_values() {
        for bad in [f64::NAN, f64::INFINITY, -0.5] {
            let res = open_session_with_pic_opts("a\n1\n", "a\n1\n", None, Some(bad), None);
            assert!(res.is_err(), "tolerance {bad} must fail");
        }
    }

    #[test]
    fn legacy_omitted_pic_opts_stay_byte_identical() {
        // No delimiter / no tolerance = the pre-C2c path exactly.
        let csv = "id,val\n1,1.5\n";
        let res = open_session_with_pic_opts(csv, csv, None, None, None);
        let summary = match res {
            Ok(s) => s,
            Err(_) => return,
        };
        assert!(summary.equal);
        assert_eq!((summary.left_cols, summary.right_cols), (2, 2));
    }
}
