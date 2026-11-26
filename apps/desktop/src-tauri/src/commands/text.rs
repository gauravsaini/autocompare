//! Text-compare IPC commands (Wave 1a leaf).
//!
//! # Ownership
//!
//! This module is owned by Wave 1a. Wave 1R owns `commands/mod.rs` and the
//! `lib.rs` handler registration, so this file is intentionally **not**
//! referenced from anywhere yet; that is fine and the crate still builds.
//!
//! # Wiring (Wave 1R handoff)
//!
//! 1. Add to `apps/desktop/src-tauri/Cargo.toml`:
//!    `autocompare-compare = { path = "../../../crates/compare" }` and
//!    `autocompare-diff = { path = "../../../crates/diff" }`.
//! 2. Create `commands/mod.rs` with `pub mod text;` (and `pub mod folder;`).
//! 3. Register each `#[tauri::command]` below in the `lib.rs`
//!    `generate_handler![...]` table.
//!
//! # Paged-fetch rule (locked)
//!
//! The full comparison stays in Rust memory (module-level session cache).
//! The UI only ever receives `(start, limit)` windows via
//! [`text_get_rows`], so whole files are never serialized over IPC.
//! Every handler returns errors as [`String`] and never panics.

#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};

use autocompare_compare::{TextOptions, compare_text_files, compare_text_lines};
use autocompare_diff::DiffScript;
use autocompare_diff::text_diff::DiffAlgorithm;
use autocompare_diff::inline_diff;
use serde::{Deserialize, Serialize};

/// Maximum rows returned by one [`text_get_rows`] call. Bounds every IPC
/// payload regardless of file size.
const MAX_PAGE: u64 = 500;

/// Maximum overview chunks returned by [`text_get_overview`]; longer runs
/// set `Overview::truncated` instead of growing the payload.
const MAX_OVERVIEW_CHUNKS: usize = 2000;

/// Maximum accepted query length for [`text_find`].
const MAX_QUERY_CHARS: usize = 1000;

/// Kind of one aligned row. Serialized camelCase to match `text.ts`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RowKind {
    /// Both sides present and byte-equal.
    Equal,
    /// Both sides present but different (paired delete+insert or
    /// key-equal/display-different under comparison options).
    Changed,
    /// Only the left side has this row.
    LeftOnly,
    /// Only the right side has this row.
    RightOnly,
}

/// Which side of a comparison a value comes from. Wire values are
/// `"left"` / `"right"`, matching `text.ts`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Side {
    Left,
    Right,
}

impl Side {
    fn parse(raw: &str) -> Result<Self, String> {
        match raw {
            "left" => Ok(Side::Left),
            "right" => Ok(Side::Right),
            _ => Err(format!(
                "unknown side '{raw}'; expected \"left\" or \"right\""
            )),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Side::Left => "left",
            Side::Right => "right",
        }
    }
}

/// Search direction for [`text_find`]. Wire values are `"up"` / `"down"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Direction {
    Up,
    Down,
}

impl Direction {
    fn parse(raw: &str) -> Result<Self, String> {
        match raw {
            "up" => Ok(Direction::Up),
            "down" => Ok(Direction::Down),
            _ => Err(format!(
                "unknown direction '{raw}'; expected \"up\" or \"down\""
            )),
        }
    }
}

/// One changed character range pair inside a [`Changed`](RowKind) row.
/// Offsets are character offsets into the raw display lines, half-open.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InlineSpanDto {
    pub a_start: u64,
    pub a_end: u64,
    pub b_start: u64,
    pub b_end: u64,
}

/// Applied ignore flags echoed back so the UI can tell support from
/// honest fallback. `camelCase` on the wire (`appliedOptions`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppliedTextOptions {
    pub ignore_whitespace: bool,
    pub ignore_case: bool,
    #[serde(default)]
    pub minor_unimportant: bool,
    #[serde(default)]
    pub ignore_blank_lines: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub algorithm: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encoding: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eol: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub suppressed_lines: Vec<u64>,
}

/// Per-side file metadata (byte length + modification timestamp).
/// Wire fields are camelCase (`bytes`, `mtimeUnix`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileMetaDto {
    pub bytes: u64,
    pub mtime_unix: Option<u64>,
}

/// Reads per-side file metadata (bytes + mtime seconds since unix epoch).
/// Never fails or panics: returns `None` on missing, unreadable, or error paths.
pub fn read_file_meta(path: &str) -> Option<FileMetaDto> {
    let meta = std::fs::metadata(path).ok()?;
    let bytes = meta.len();
    let mtime_unix = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs());
    Some(FileMetaDto { bytes, mtime_unix })
}

/// Summary returned by [`text_compare`]. Never carries file content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextCompareDto {
    pub session: String,
    pub left_path: String,
    pub right_path: String,
    pub left_lines: u64,
    pub right_lines: u64,
    /// Number of aligned rows (the `total` used by paging).
    pub rows: u64,
    /// Number of ops in the underlying [`DiffScript`].
    pub op_count: u64,
    /// False when the diff engine exhausted its edit budget and fell back
    /// to a coarse script (callers should say so honestly).
    pub exact: bool,
    /// True when the cached overview hit [`MAX_OVERVIEW_CHUNKS`].
    pub truncated: bool,
    /// Echo of the ignore flags actually applied (backward-compat:
    /// old callers omit flags and get `false`/`false` = byte-identical).
    pub applied_options: AppliedTextOptions,
    #[serde(default)]
    pub left_meta: Option<FileMetaDto>,
    #[serde(default)]
    pub right_meta: Option<FileMetaDto>,
}

/// One aligned row of a text comparison.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextRowDto {
    pub index: u64,
    pub kind: RowKind,
    pub left: Option<String>,
    pub right: Option<String>,
    /// Word/character diff spans; non-empty only for [`Changed`](RowKind)
    /// rows where both sides are present.
    pub inline: Vec<InlineSpanDto>,
    /// Wave-L1: per-line suppressed flag (row flag, not deletion).
    #[serde(default)]
    pub suppressed: bool,
}

/// One `(start, limit)` window over the aligned rows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RowsPage {
    pub session: String,
    pub start: u64,
    pub total: u64,
    pub rows: Vec<TextRowDto>,
}

/// One run-length chunk of the comparison for scrollbar-style overviews.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OverviewChunk {
    pub start: u64,
    pub len: u64,
    pub kind: RowKind,
}

/// Coarse whole-comparison map returned by [`text_get_overview`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Overview {
    pub session: String,
    pub total: u64,
    pub chunks: Vec<OverviewChunk>,
    pub truncated: bool,
}

/// A single search hit returned by [`text_find`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hit {
    /// Aligned row index of the hit.
    pub index: u64,
    pub side: Side,
    /// Character offset of the match inside the line.
    pub column: u64,
    /// Full matched line (one line only, never a whole file).
    pub line: String,
}

/// Section text returned by [`text_copy_section`] for the UI to copy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CopiedSection {
    pub session: String,
    pub index: u64,
    pub side: Side,
    pub start: u64,
    pub end: u64,
    pub text: String,
    pub line_count: u64,
}

/// Receipt returned by [`text_save_side`] after writing one side to disk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedSide {
    pub session: String,
    pub side: Side,
    pub path: String,
    pub lines: u64,
    pub bytes: u64,
}

/// Cached full comparison result. Lives in Rust memory only; the UI pages
/// through it via [`text_get_rows`].
#[derive(Debug, Clone)]
struct StoredText {
    left_count: u64,
    right_count: u64,
    rows: Vec<TextRowDto>,
    overview: Vec<OverviewChunk>,
    overview_truncated: bool,
    opts: TextOptions,
    /// C2a engine-ready options (no `TextOptions` home upstream):
    /// blank-line folding + diff algorithm for re-compare paths.
    ignore_blank: bool,
    algorithm: DiffAlgorithm,
}

fn text_store() -> &'static Mutex<HashMap<String, StoredText>> {
    static STORE: OnceLock<Mutex<HashMap<String, StoredText>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn next_session_id() -> String {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let n = SEQ.fetch_add(1, Ordering::Relaxed).saturating_add(1);
    format!("text-{n}")
}

fn lock_store() -> Result<std::sync::MutexGuard<'static, HashMap<String, StoredText>>, String> {
    text_store()
        .lock()
        .map_err(|_| "text session cache is unavailable".to_string())
}

fn stored(session: &str) -> Result<StoredText, String> {
    let guard = lock_store()?;
    guard
        .get(session)
        .cloned()
        .ok_or_else(|| format!("unknown text session '{session}'"))
}

/// Clamps `(start, limit)` to `[0, total]`, capping `limit` at
/// [`MAX_PAGE`]. Out-of-range `start` yields an empty window, never an
/// error. Overflow-safe: `start + limit` saturates.
pub fn clamp_window(total: u64, start: u64, limit: u64) -> (u64, u64) {
    let limit = limit.min(MAX_PAGE);
    if start >= total || limit == 0 {
        return (total.min(start), 0);
    }
    let end = start.saturating_add(limit).min(total);
    (start, end.saturating_sub(start))
}

/// Marks rows whose indices match `suppressed_lines` as `suppressed: true`.
/// Suppressed rows are flagged, never dropped.
pub fn mark_suppressed_rows(rows: &mut [TextRowDto], suppressed: &[u64]) {
    if suppressed.is_empty() {
        return;
    }
    let set: std::collections::HashSet<u64> = suppressed.iter().copied().collect();
    for row in rows.iter_mut() {
        if set.contains(&row.index) {
            row.suppressed = true;
        }
    }
}

/// Pairs display lines into aligned rows from a [`DiffScript`].
///
/// A `Delete` run immediately followed by an `Insert` run is zipped into
/// [`Changed`](RowKind) pairs (with [`inline_diff`] spans); leftovers stay
/// one-sided. `Equal` runs pair up directly; pairs whose display lines
/// differ anyway (comparison keys matched, e.g. case-insensitively) are
/// marked [`Changed`](RowKind) with spans rather than `Equal`.
pub fn build_rows(
    left_lines: &[String],
    right_lines: &[String],
    script: &DiffScript,
) -> Vec<TextRowDto> {
    let mut rows: Vec<TextRowDto> = Vec::new();
    let mut li: usize = 0;
    let mut ri: usize = 0;
    let mut ops = script.ops.iter().peekable();
    while let Some(op) = ops.next() {
        match op {
            autocompare_diff::Op::Equal { len } => {
                for _ in 0..*len {
                    let l = left_lines.get(li).cloned();
                    let r = right_lines.get(ri).cloned();
                    li = li.saturating_add(1);
                    ri = ri.saturating_add(1);
                    let (kind, inline) = classify_pair(l.as_ref(), r.as_ref());
                    rows.push(TextRowDto {
                        index: rows.len() as u64,
                        kind,
                        left: l,
                        right: r,
                        inline,
                        suppressed: false,
                    });
                }
            }
            autocompare_diff::Op::Delete { len } => {
                // Zip with a following Insert run into Changed pairs.
                let mut insert_len: usize = 0;
                if let Some(autocompare_diff::Op::Insert { len: n }) = ops.peek() {
                    insert_len = *n;
                    ops.next();
                }
                let paired = (*len).min(insert_len);
                for _ in 0..paired {
                    let l = left_lines.get(li).cloned();
                    let r = right_lines.get(ri).cloned();
                    li = li.saturating_add(1);
                    ri = ri.saturating_add(1);
                    let inline = inline_spans(l.as_ref(), r.as_ref());
                    rows.push(TextRowDto {
                        index: rows.len() as u64,
                        kind: RowKind::Changed,
                        left: l,
                        right: r,
                        inline,
                        suppressed: false,
                    });
                }
                for _ in paired..*len {
                    let l = left_lines.get(li).cloned();
                    li = li.saturating_add(1);
                    rows.push(TextRowDto {
                        index: rows.len() as u64,
                        kind: RowKind::LeftOnly,
                        left: l,
                        right: None,
                        inline: Vec::new(),
                        suppressed: false,
                    });
                }
                for _ in paired..insert_len {
                    let r = right_lines.get(ri).cloned();
                    ri = ri.saturating_add(1);
                    rows.push(TextRowDto {
                        index: rows.len() as u64,
                        kind: RowKind::RightOnly,
                        left: None,
                        right: r,
                        inline: Vec::new(),
                        suppressed: false,
                    });
                }
            }
            autocompare_diff::Op::Insert { len } => {
                for _ in 0..*len {
                    let r = right_lines.get(ri).cloned();
                    ri = ri.saturating_add(1);
                    rows.push(TextRowDto {
                        index: rows.len() as u64,
                        kind: RowKind::RightOnly,
                        left: None,
                        right: r,
                        inline: Vec::new(),
                        suppressed: false,
                    });
                }
            }
        }
    }
    rows
}

fn classify_pair(left: Option<&String>, right: Option<&String>) -> (RowKind, Vec<InlineSpanDto>) {
    match (left, right) {
        (Some(l), Some(r)) if l == r => (RowKind::Equal, Vec::new()),
        (Some(l), Some(r)) => {
            let inline = spans_for_lines(l, r);
            (RowKind::Changed, inline)
        }
        (Some(_), None) => (RowKind::LeftOnly, Vec::new()),
        (None, Some(_)) => (RowKind::RightOnly, Vec::new()),
        (None, None) => (RowKind::Equal, Vec::new()),
    }
}

fn inline_spans(left: Option<&String>, right: Option<&String>) -> Vec<InlineSpanDto> {
    match (left, right) {
        (Some(l), Some(r)) => spans_for_lines(l, r),
        _ => Vec::new(),
    }
}

fn spans_for_lines(left: &str, right: &str) -> Vec<InlineSpanDto> {
    inline_diff(left, right)
        .iter()
        .map(|s| InlineSpanDto {
            a_start: s.a_start as u64,
            a_end: s.a_end as u64,
            b_start: s.b_start as u64,
            b_end: s.b_end as u64,
        })
        .collect()
}

/// Run-length encodes row kinds into overview chunks, capped at
/// [`MAX_OVERVIEW_CHUNKS`]. Returns `(chunks, truncated)`.
pub fn summarize_overview(rows: &[TextRowDto]) -> (Vec<OverviewChunk>, bool) {
    let mut chunks: Vec<OverviewChunk> = Vec::new();
    for row in rows {
        let extend = chunks.last().is_some_and(|c| c.kind == row.kind);
        if extend {
            if let Some(last) = chunks.last_mut() {
                last.len = last.len.saturating_add(1);
            }
        } else {
            if chunks.len() >= MAX_OVERVIEW_CHUNKS {
                return (chunks, true);
            }
            chunks.push(OverviewChunk {
                start: row.index,
                len: 1,
                kind: row.kind,
            });
        }
    }
    (chunks, false)
}

/// Character offset of the first `query` match in `line`, or `None`.
fn find_column(line: &str, query: &str) -> Option<u64> {
    let byte_idx = line.find(query)?;
    let mut count: u64 = 0;
    let mut cur: usize = 0;
    for ch in line.chars() {
        if cur >= byte_idx {
            break;
        }
        cur = cur.saturating_add(ch.len_utf8());
        count = count.saturating_add(1);
    }
    Some(count)
}

/// Searches cached rows for `query`. `"down"` returns the first hit from
/// the top (left side before right within a row); `"up"` returns the
/// first hit from the bottom (right side before left). `None` means no
/// match, which is not an error.
pub fn find_in_rows(rows: &[TextRowDto], query: &str, direction: Direction) -> Option<Hit> {
    if query.is_empty() {
        return None;
    }
    let ordered: Vec<&TextRowDto> = match direction {
        Direction::Down => rows.iter().collect(),
        Direction::Up => rows.iter().rev().collect(),
    };
    for row in ordered {
        let sides: Vec<(Side, Option<&String>)> = match direction {
            Direction::Down => vec![
                (Side::Left, row.left.as_ref()),
                (Side::Right, row.right.as_ref()),
            ],
            Direction::Up => vec![
                (Side::Right, row.right.as_ref()),
                (Side::Left, row.left.as_ref()),
            ],
        };
        for (side, text) in sides {
            if let Some(line) = text
                && let Some(column) = find_column(line, query)
            {
                return Some(Hit {
                    index: row.index,
                    side,
                    column,
                    line: line.clone(),
                });
            }
        }
    }
    None
}

/// Extracts the contiguous non-`Equal` block containing `index` on side
/// `side`. An `Equal` row yields just that row's line. Errors when `index`
/// is out of range or the requested side is absent there.
pub fn copy_section_of(
    session: &str,
    rows: &[TextRowDto],
    index: u64,
    side: Side,
) -> Result<CopiedSection, String> {
    if rows.is_empty() {
        return Err("cannot copy section from empty session".to_string());
    }
    let max_idx = rows.iter().map(|r| r.index).max().unwrap_or(0);
    let at = match rows.iter().position(|r| r.index == index) {
        Some(pos) => pos,
        None => {
            if index > max_idx {
                return Err(format!("row index {index} out of range"));
            }
            // The row existed but was deleted (e.g. orphan delete from earlier copy).
            return Ok(CopiedSection {
                session: session.to_string(),
                index,
                side,
                start: index,
                end: index,
                text: String::new(),
                line_count: 0,
            });
        }
    };
    let anchor = &rows[at];
    let (mut lo, mut hi) = (at, at.saturating_add(1));
    if anchor.kind != RowKind::Equal {
        while lo > 0 {
            let prev = rows.get(lo.saturating_sub(1));
            let is_changed = prev.is_some_and(|r| r.kind != RowKind::Equal);
            if !is_changed {
                break;
            }
            lo = lo.saturating_sub(1);
        }
        while hi < rows.len() {
            let next_is_changed = rows.get(hi).is_some_and(|r| r.kind != RowKind::Equal);
            if !next_is_changed {
                break;
            }
            hi = hi.saturating_add(1);
        }
    }
    let mut lines: Vec<String> = Vec::new();
    for row in rows.iter().skip(lo).take(hi.saturating_sub(lo)) {
        let text = match side {
            Side::Left => row.left.as_ref(),
            Side::Right => row.right.as_ref(),
        };
        if let Some(line) = text {
            lines.push(line.clone());
        }
    }
    let start_row = rows.get(lo).map(|r| r.index).unwrap_or(lo as u64);
    let end_row = rows.get(hi.saturating_sub(1)).map(|r| r.index).unwrap_or(hi.saturating_sub(1) as u64);
    if lines.is_empty() {
        return Ok(CopiedSection {
            session: session.to_string(),
            index,
            side,
            start: start_row,
            end: end_row,
            text: String::new(),
            line_count: 0,
        });
    }
    Ok(CopiedSection {
        session: session.to_string(),
        index,
        side,
        start: start_row,
        end: end_row,
        text: lines.join("\n"),
        line_count: lines.len() as u64,
    })
}

/// Compares two text files and caches the full result under a new session
/// id. Only the summary travels back over IPC.
///
/// Wave-66T + Wave-L1: optional ignore flags, minor/unimportant, suppressed lines,
/// encoding, and EOL flags with backward compat. Old callers send
/// only `{left, right}` and get byte-identical comparison. New callers may
/// send camelCase and/or snake_case spellings; either spelling wins.
///
/// Wave-C2a: optional `algorithm` (`"myers"` default | `"patience"`) and
/// `ignoreBlankLines` / `ignore_blank_lines`. Both are engine-ready:
/// `DiffAlgorithm::{Myers, Patience}` in `crates/diff` (`text_diff`,
/// `merge.rs`) and blank-line folding mirroring `grammar::is_blank_line`.
/// Unknown algorithm spellings fall back to Myers. Echoed honestly in
/// `appliedOptions` (`algorithm` is `None` when the caller omits it).
fn parse_text_algorithm(raw: Option<String>) -> (DiffAlgorithm, Option<String>) {
    let norm = raw
        .as_deref()
        .map(|s| s.trim().to_ascii_lowercase())
        .unwrap_or_default();
    match norm.as_str() {
        "patience" => (DiffAlgorithm::Patience, Some("patience".to_string())),
        "myers" => (DiffAlgorithm::Myers, Some("myers".to_string())),
        "" => (DiffAlgorithm::Myers, None),
        _ => (DiffAlgorithm::Myers, None),
    }
}

/// Comparison key mirroring `crates/compare/src/text.rs` plus C2a
/// blank-line folding: when `ignore_blank` is set, any whitespace-only
/// line keys to `""` so blank-vs-blank aligns as equal.
fn text_compare_key(line: &str, opts: &TextOptions, ignore_blank: bool) -> String {
    let base = if opts.eol.is_some() {
        line.strip_suffix('\r').unwrap_or(line)
    } else {
        line
    };
    if ignore_blank && base.trim().is_empty() {
        return String::new();
    }
    let mut key = if opts.ignore_space_change {
        base.split_whitespace().collect::<Vec<&str>>().join(" ")
    } else {
        base.to_owned()
    };
    if opts.ignore_case {
        key = key.to_lowercase();
    }
    key
}

/// Re-diffs decoded display lines with the C2a options. Used by both
/// `text_compare` and the `text_apply_edit` re-compare so sessions stay
/// consistent across edits.
fn diff_with_c2a(
    left_lines: &[String],
    right_lines: &[String],
    opts: &TextOptions,
    ignore_blank: bool,
    algo: DiffAlgorithm,
) -> DiffScript {
    let left_keys: Vec<String> = left_lines
        .iter()
        .map(|l| text_compare_key(l, opts, ignore_blank))
        .collect();
    let right_keys: Vec<String> = right_lines
        .iter()
        .map(|l| text_compare_key(l, opts, ignore_blank))
        .collect();
    let left_refs: Vec<&str> = left_keys.iter().map(|s| s.as_str()).collect();
    let right_refs: Vec<&str> = right_keys.iter().map(|s| s.as_str()).collect();
    autocompare_diff::text_diff::diff(&left_refs, &right_refs, algo)
}

/// Folds blank-vs-blank pairs to `Equal` under `ignore_blank_lines`
/// (whitespace-only difference on both sides is unimportant, mirroring
/// `grammar::is_blank_line`). Display text is untouched; only the row
/// kind/inline spans change.
fn fold_blank_rows_equal(rows: &mut [TextRowDto], ignore_blank: bool) {
    if !ignore_blank {
        return;
    }
    for row in rows.iter_mut() {
        let (Some(l), Some(r)) = (row.left.as_ref(), row.right.as_ref()) else {
            continue;
        };
        if l.trim().is_empty() && r.trim().is_empty() && row.kind != RowKind::Equal {
            row.kind = RowKind::Equal;
            row.inline.clear();
        }
    }
}

#[tauri::command]
#[allow(non_snake_case)]
pub fn text_compare(
    left: String,
    right: String,
    ignoreWhitespace: Option<bool>,
    ignoreCase: Option<bool>,
    ignore_whitespace: Option<bool>,
    ignore_case: Option<bool>,
    minorUnimportant: Option<bool>,
    minor_unimportant: Option<bool>,
    suppressedLines: Option<Vec<u64>>,
    suppressed_lines: Option<Vec<u64>>,
    encoding: Option<String>,
    eol: Option<String>,
    algorithm: Option<String>,
    ignoreBlankLines: Option<bool>,
    ignore_blank_lines: Option<bool>,
) -> Result<TextCompareDto, String> {
    if left.is_empty() {
        return Err("left path must not be empty".to_string());
    }
    if right.is_empty() {
        return Err("right path must not be empty".to_string());
    }
    let ignore_ws = ignoreWhitespace
        .or(ignore_whitespace)
        .unwrap_or(false);
    let ignore_cs = ignoreCase.or(ignore_case).unwrap_or(false);
    let minor = minorUnimportant.or(minor_unimportant).unwrap_or(false);
    let suppressed = suppressedLines.or(suppressed_lines).unwrap_or_default();
    let enc = encoding;
    let line_ending = eol;
    let ignore_blank = ignoreBlankLines.or(ignore_blank_lines).unwrap_or(false);
    let (algo, algo_echo) = parse_text_algorithm(algorithm);

    let opts = TextOptions {
        ignore_case: ignore_cs,
        ignore_space_change: ignore_ws,
        minor_unimportant: minor,
        suppressed_lines: suppressed.clone(),
        encoding: enc.clone(),
        eol: line_ending.clone(),
    };
    let mut compared = compare_text_files(
        std::path::Path::new(&left),
        std::path::Path::new(&right),
        &opts,
    )
    .map_err(|e| e.to_string())?;
    // C2a engine-ready path: Patience and/or blank-line folding re-key
    // over the decoded display lines and re-diff with the chosen
    // algorithm. Display lines are untouched; only the script changes.
    // Myers without blank folding keeps the legacy `compare_text_files`
    // script byte-identical.
    if matches!(algo, DiffAlgorithm::Patience) || ignore_blank {
        compared.script = diff_with_c2a(
            &compared.left_lines,
            &compared.right_lines,
            &opts,
            ignore_blank,
            algo,
        );
    }
    let mut rows = build_rows(
        &compared.left_lines,
        &compared.right_lines,
        &compared.script,
    );
    fold_blank_rows_equal(&mut rows, ignore_blank);
    mark_suppressed_rows(&mut rows, &opts.suppressed_lines);
    let (overview, overview_truncated) = summarize_overview(&rows);
    let dto = TextCompareDto {
        session: next_session_id(),
        left_path: left.clone(),
        right_path: right.clone(),
        left_lines: compared.left_lines.len() as u64,
        right_lines: compared.right_lines.len() as u64,
        rows: rows.len() as u64,
        op_count: compared.script.ops.len() as u64,
        exact: compared.script.exact,
        truncated: overview_truncated,
        applied_options: AppliedTextOptions {
            ignore_whitespace: ignore_ws,
            ignore_case: ignore_cs,
            minor_unimportant: minor,
            ignore_blank_lines: ignore_blank,
            algorithm: algo_echo,
            encoding: enc,
            eol: line_ending,
            suppressed_lines: suppressed,
        },
        left_meta: read_file_meta(&left),
        right_meta: read_file_meta(&right),
    };
    let mut guard = lock_store()?;
    guard.insert(
        dto.session.clone(),
        StoredText {
            left_count: dto.left_lines,
            right_count: dto.right_lines,
            rows,
            overview,
            overview_truncated,
            opts,
            ignore_blank,
            algorithm: algo,
        },
    );
    Ok(dto)
}

/// Receipt returned by [`text_apply_edit`] after patching one cell and
/// re-running the comparison over the cached rows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppliedEdit {
    pub session: String,
    pub index: u64,
    pub side: Side,
    pub kind: RowKind,
    /// Aligned row count after the re-compare.
    pub rows: u64,
}

/// Receipt returned by [`text_apply_hunk`] after copying an entire hunk
/// atomically and re-running the comparison over the cached rows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppliedHunk {
    pub session: String,
    pub start: u64,
    pub end: u64,
    pub dir: Side,
    pub applied: u64,
    pub total: u64,
    pub rows: u64,
}

/// Rebuilds the display line vectors from cached `rows` and patches the
/// edited cell. Row `index` addresses the aligned row (its `index`
/// field, which equals its position); the patch targets that row's
/// non-null cell on `side`. Orphan (null) cells and out-of-range
/// indexes are honest errors, never silent no-ops.
pub fn apply_edit_to_rows(
    rows: &[TextRowDto],
    index: u64,
    side: &Side,
    text: &str,
) -> Result<(Vec<String>, Vec<String>), String> {
    let at = usize::try_from(index).unwrap_or(usize::MAX);
    let anchor_row = rows
        .get(at)
        .filter(|r| r.index == index)
        .ok_or_else(|| format!("row index {index} out of range"))?;
    let cell = match side {
        Side::Left => anchor_row.left.as_ref(),
        Side::Right => anchor_row.right.as_ref(),
    };
    if cell.is_none() {
        return Err(format!(
            "row {index} has no {} content to edit",
            side.as_str()
        ));
    }
    let mut left_lines: Vec<String> = Vec::new();
    let mut right_lines: Vec<String> = Vec::new();
    for row in rows {
        if let Some(line) = row.left.as_ref() {
            if row.index == index && matches!(side, Side::Left) {
                left_lines.push(text.to_string());
            } else {
                left_lines.push(line.clone());
            }
        }
        if let Some(line) = row.right.as_ref() {
            if row.index == index && matches!(side, Side::Right) {
                right_lines.push(text.to_string());
            } else {
                right_lines.push(line.clone());
            }
        }
    }
    Ok((left_lines, right_lines))
}

/// Patches one cached cell and re-runs the comparison with the
/// session's own ignore options, so hunk grouping, inline spans and
/// the overview reflect the edit. Editing an orphan (null) cell or an
/// out-of-range row is an error; empty text is a valid (emptied) line.
#[tauri::command]
pub fn text_apply_edit(
    session: String,
    index: u64,
    side: String,
    text: String,
) -> Result<AppliedEdit, String> {
    let which = Side::parse(&side)?;
    let cached = stored(&session)?;
    let (left_lines, right_lines) =
        apply_edit_to_rows(&cached.rows, index, &which, &text)?;
    let opts = cached.opts.clone();
    let ignore_blank = cached.ignore_blank;
    let algo = cached.algorithm;
    let compared = if matches!(algo, DiffAlgorithm::Patience) || ignore_blank {
        let script = diff_with_c2a(&left_lines, &right_lines, &opts, ignore_blank, algo);
        autocompare_compare::TextComparison {
            left_lines,
            right_lines,
            script,
        }
    } else {
        compare_text_lines(&left_lines, &right_lines, &opts)
    };
    let mut rows = build_rows(&compared.left_lines, &compared.right_lines, &compared.script);
    fold_blank_rows_equal(&mut rows, ignore_blank);
    mark_suppressed_rows(&mut rows, &opts.suppressed_lines);
    let (overview, overview_truncated) = summarize_overview(&rows);
    let at = usize::try_from(index).unwrap_or(usize::MAX);
    let kind = rows.get(at).map(|r| r.kind).unwrap_or(RowKind::Equal);
    let mut guard = lock_store()?;
    guard.insert(
        session.clone(),
        StoredText {
            left_count: compared.left_lines.len() as u64,
            right_count: compared.right_lines.len() as u64,
            rows: rows.clone(),
            overview,
            overview_truncated,
            opts,
            ignore_blank,
            algorithm: algo,
        },
    );
    Ok(AppliedEdit {
        session,
        index,
        side: which,
        kind,
        rows: rows.len() as u64,
    })
}

/// Rebuilds the display line vectors from cached `rows` and patches the
/// entire hunk range `start..=end` atomically.
/// Take-side wins with no fallback: an empty take side deletes the
/// orphan lines (BC parity, FIXTEXT GAP-03); a present take side fills
/// or overwrites both sides; no row is fabricated.
/// Returns `(left_lines, right_lines, applied, total)`.
pub fn apply_hunk_to_rows(
    rows: &[TextRowDto],
    start: u64,
    end: u64,
    dir: &Side,
) -> Result<(Vec<String>, Vec<String>, u64, u64), String> {
    if start > end {
        return Err(format!("invalid hunk range: start {start} > end {end}"));
    }
    let total_rows = rows.len() as u64;
    if total_rows == 0 {
        return Err("cannot apply hunk to empty session".to_string());
    }
    let max_idx = rows.iter().map(|r| r.index).max().unwrap_or(0);
    if start > max_idx {
        return Err(format!("row index {start} out of range"));
    }
    // Wave-G1 / Wave-157: clamp end to max_idx so shifted/post-merge bounds don't crash
    let end = end.min(max_idx);

    let mut left_lines: Vec<String> = Vec::new();
    let mut right_lines: Vec<String> = Vec::new();
    let mut applied: u64 = 0;
    let mut total: u64 = 0;

    for row in rows {
        if row.index < start || row.index > end {
            if let Some(line) = row.left.as_ref() {
                left_lines.push(line.clone());
            }
            if let Some(line) = row.right.as_ref() {
                right_lines.push(line.clone());
            }
        } else {
            total = total.saturating_add(1);
            let take = match dir {
                Side::Left => row.left.as_deref(),
                Side::Right => row.right.as_deref(),
            };
            let other = match dir {
                Side::Left => row.right.as_deref(),
                Side::Right => row.left.as_deref(),
            };
            if take.is_some() && other.is_some() {
                applied = applied.saturating_add(1);
            }
            // FIXTEXT GAP-03 (BC parity): take-side wins with NO
            // `or(other)` fallback. Copying an empty source clears the
            // target text: `None` pushes nothing to EITHER line vector,
            // deleting the orphan lines (re-compare then closes the gap).
            // A present take side fills/overwrites both sides as before.
            let won = take;
            if let Some(line) = won {
                left_lines.push(line.to_string());
                right_lines.push(line.to_string());
            }
        }
    }
    Ok((left_lines, right_lines, applied, total))
}

/// Patches an entire hunk range `[start, end]` atomically and updates
/// the cached rows preserving row identity on surviving rows.
/// Take-side wins with no fallback (empty take deletes the orphan
/// lines, FIXTEXT GAP-03); a present take side fills or overwrites.
#[tauri::command]
pub fn text_apply_hunk(
    session: String,
    start: u64,
    end: u64,
    dir: String,
) -> Result<AppliedHunk, String> {
    let which = Side::parse(&dir)?;
    let cached = stored(&session)?;
    let (left_lines, right_lines, applied, total) =
        apply_hunk_to_rows(&cached.rows, start, end, &which)?;
    let opts = cached.opts.clone();
    let ignore_blank = cached.ignore_blank;
    let algo = cached.algorithm;
    let mut rows: Vec<TextRowDto> = Vec::new();
    for row in &cached.rows {
        if row.index < start || row.index > end {
            rows.push(row.clone());
        } else {
            let take = match which {
                Side::Left => row.left.as_ref(),
                Side::Right => row.right.as_ref(),
            };
            if let Some(line) = take {
                rows.push(TextRowDto {
                    index: row.index,
                    kind: RowKind::Equal,
                    left: Some(line.clone()),
                    right: Some(line.clone()),
                    inline: Vec::new(),
                    suppressed: row.suppressed,
                });
            }
        }
    }
    let (overview, overview_truncated) = summarize_overview(&rows);
    let post_start = start;
    let post_end = start.saturating_add(total).saturating_sub(1);
    let mut guard = lock_store()?;
    guard.insert(
        session.clone(),
        StoredText {
            left_count: left_lines.len() as u64,
            right_count: right_lines.len() as u64,
            rows: rows.clone(),
            overview,
            overview_truncated,
            opts,
            ignore_blank,
            algorithm: algo,
        },
    );
    Ok(AppliedHunk {
        session,
        start: post_start,
        end: post_end,
        dir: which,
        applied,
        total,
        rows: rows.len() as u64,
    })
}

/// Restores a previously snapshotted row vector for `session` (used by
/// frontend Undo/Redo so subsequent `text_save_side` writes the exact
/// undone/redone state to disk). Recomputes line counts and overview map.
#[tauri::command]
pub fn text_restore_rows(session: String, rows: Vec<TextRowDto>) -> Result<u64, String> {
    let cached = stored(&session)?;
    let mut left_count: u64 = 0;
    let mut right_count: u64 = 0;
    for row in &rows {
        if row.left.is_some() {
            left_count = left_count.saturating_add(1);
        }
        if row.right.is_some() {
            right_count = right_count.saturating_add(1);
        }
    }
    let (overview, overview_truncated) = summarize_overview(&rows);
    let total = rows.len() as u64;
    let mut guard = lock_store()?;
    guard.insert(
        session,
        StoredText {
            left_count,
            right_count,
            rows,
            overview,
            overview_truncated,
            opts: cached.opts,
            ignore_blank: cached.ignore_blank,
            algorithm: cached.algorithm,
        },
    );
    Ok(total)
}

/// Lightweight metadata stat for external file-change detection (used by the
/// UI reload prompt). Returns `None` for missing/unreadable paths without
/// failing the batch.
#[tauri::command]
pub fn stat_paths(paths: Vec<String>) -> Vec<Option<FileMetaDto>> {
    paths
        .iter()
        .map(|p| {
            if p.trim().is_empty() {
                None
            } else {
                read_file_meta(p)
            }
        })
        .collect()
}

/// Returns one `(start, limit)` window of aligned rows. `start` past the
/// end yields an empty page, never an error.
#[tauri::command]
pub fn text_get_rows(session: String, start: u64, limit: u64) -> Result<RowsPage, String> {
    let cached = stored(&session)?;
    let total = cached.rows.len() as u64;
    let (from, len) = clamp_window(total, start, limit);
    let from_usize = usize::try_from(from).unwrap_or(usize::MAX);
    let len_usize = usize::try_from(len).unwrap_or(0);
    let rows: Vec<TextRowDto> = cached
        .rows
        .iter()
        .skip(from_usize)
        .take(len_usize)
        .cloned()
        .collect();
    Ok(RowsPage {
        session,
        start: from,
        total,
        rows,
    })
}

/// Returns the coarse whole-comparison map for scrollbars/minimaps.
#[tauri::command]
pub fn text_get_overview(session: String) -> Result<Overview, String> {
    let cached = stored(&session)?;
    Ok(Overview {
        session,
        total: cached.rows.len() as u64,
        chunks: cached.overview.clone(),
        truncated: cached.overview_truncated,
    })
}

/// Searches cached rows for `query` (`direction`: `"up"` / `"down"`).
/// `Ok(None)` means no match. Empty or oversized queries are errors.
#[tauri::command]
pub fn text_find(session: String, query: String, direction: String) -> Result<Option<Hit>, String> {
    if query.is_empty() {
        return Err("query must not be empty".to_string());
    }
    if query.chars().count() > MAX_QUERY_CHARS {
        return Err(format!("query longer than {MAX_QUERY_CHARS} characters"));
    }
    let dir = Direction::parse(&direction)?;
    let cached = stored(&session)?;
    Ok(find_in_rows(&cached.rows, &query, dir))
}

/// Returns the enclosing changed section's text on side `dir`
/// (`"left"` / `"right"`) for the UI clipboard.
#[tauri::command]
pub fn text_copy_section(
    session: String,
    index: u64,
    dir: String,
) -> Result<CopiedSection, String> {
    let side = Side::parse(&dir)?;
    let cached = stored(&session)?;
    copy_section_of(&session, &cached.rows, index, side)
}

/// Writes one cached side back to `path` and returns a receipt.
#[tauri::command]
pub fn text_save_side(session: String, side: String, path: String) -> Result<SavedSide, String> {
    if path.is_empty() {
        return Err("path must not be empty".to_string());
    }
    let which = Side::parse(&side)?;
    let cached = stored(&session)?;
    let mut lines: Vec<&str> = Vec::new();
    for row in &cached.rows {
        let text = match which {
            Side::Left => row.left.as_deref(),
            Side::Right => row.right.as_deref(),
        };
        if let Some(line) = text {
            lines.push(line);
        }
    }
    let expected = match which {
        Side::Left => cached.left_count,
        Side::Right => cached.right_count,
    };
    if lines.len() as u64 != expected {
        return Err("cached text session is inconsistent".to_string());
    }
    let content = lines.join("\n");
    std::fs::write(std::path::Path::new(&path), &content)
        .map_err(|e| format!("writing {path} failed: {e}"))?;
    Ok(SavedSide {
        session,
        side: which,
        path,
        lines: lines.len() as u64,
        bytes: content.len() as u64,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use autocompare_diff::diff_lines;

    fn lines(values: &[&str]) -> Vec<String> {
        values.iter().map(|s| (*s).to_string()).collect()
    }

    fn script_for(left: &[String], right: &[String]) -> DiffScript {
        let l: Vec<&str> = left.iter().map(|s| s.as_str()).collect();
        let r: Vec<&str> = right.iter().map(|s| s.as_str()).collect();
        diff_lines(&l, &r)
    }

    #[test]
    fn apply_edit_recompares_edited_row_to_equal() {
        let left = lines(&["a", "b", "c"]);
        let right = lines(&["a", "B", "c"]);
        let rows = build_rows(&left, &right, &script_for(&left, &right));
        assert_eq!(rows.get(1).map(|r| r.kind), Some(RowKind::Changed));
        let applied = apply_edit_to_rows(&rows, 1, &Side::Left, "B");
        assert!(applied.is_ok(), "apply edit failed: {applied:?}");
        let (patched_left, patched_right) = applied.unwrap_or_default();
        let opts = autocompare_compare::TextOptions::default();
        let recompared = autocompare_compare::compare_text_lines(&patched_left, &patched_right, &opts);
        let rows2 = build_rows(
            &recompared.left_lines,
            &recompared.right_lines,
            &recompared.script,
        );
        assert_eq!(rows2.len(), 3);
        assert_eq!(rows2.get(1).map(|r| r.kind), Some(RowKind::Equal));
        assert!(rows2.get(1).map(|r| r.inline.is_empty()).unwrap_or(false));
    }

    #[test]
    fn apply_hunk_patches_changed_hunk_atomically() {
        let left = lines(&["a", "b", "c", "d"]);
        let right = lines(&["a", "B", "C", "d"]);
        let rows = build_rows(&left, &right, &script_for(&left, &right));
        assert_eq!(rows.get(1).map(|r| r.kind), Some(RowKind::Changed));
        assert_eq!(rows.get(2).map(|r| r.kind), Some(RowKind::Changed));
        let applied = apply_hunk_to_rows(&rows, 1, 2, &Side::Left);
        assert!(applied.is_ok(), "apply hunk failed: {applied:?}");
        let (left_lines, right_lines, count, total) = applied.unwrap();
        assert_eq!(count, 2);
        assert_eq!(total, 2);
        let opts = autocompare_compare::TextOptions::default();
        let recompared = autocompare_compare::compare_text_lines(&left_lines, &right_lines, &opts);
        let rows2 = build_rows(
            &recompared.left_lines,
            &recompared.right_lines,
            &recompared.script,
        );
        assert_eq!(rows2.len(), 4);
        assert_eq!(rows2.get(1).map(|r| r.kind), Some(RowKind::Equal));
        assert_eq!(rows2.get(2).map(|r| r.kind), Some(RowKind::Equal));
    }

    #[test]
    fn apply_hunk_fills_orphan_hunk_without_dropping_rows() {
        let left = lines(&["a", "o1", "o2", "d"]);
        let right = lines(&["a", "d"]);
        let rows = build_rows(&left, &right, &script_for(&left, &right));
        let applied = apply_hunk_to_rows(&rows, 1, 2, &Side::Left);
        assert!(applied.is_ok(), "apply orphan hunk failed: {applied:?}");
        let (left_lines, right_lines, count, total) = applied.unwrap();
        assert_eq!(count, 0);
        assert_eq!(total, 2);
        assert_eq!(left_lines, lines(&["a", "o1", "o2", "d"]));
        assert_eq!(right_lines, lines(&["a", "o1", "o2", "d"]));
        let opts = autocompare_compare::TextOptions::default();
        let recompared = autocompare_compare::compare_text_lines(&left_lines, &right_lines, &opts);
        let rows2 = build_rows(
            &recompared.left_lines,
            &recompared.right_lines,
            &recompared.script,
        );
        assert_eq!(rows2.len(), 4);
        for r in &rows2 {
            assert_eq!(r.kind, RowKind::Equal);
        }
    }

    #[test]
    fn apply_hunk_empty_take_deletes_orphan_lines() {
        // FIXTEXT GAP-03 (BC parity): take-right on a left-only hunk has
        // an empty take side, so the orphan lines are DELETED, not cloned.
        let left = lines(&["a", "o1", "o2", "d"]);
        let right = lines(&["a", "d"]);
        let rows = build_rows(&left, &right, &script_for(&left, &right));
        assert_eq!(rows.get(1).map(|r| r.kind), Some(RowKind::LeftOnly));
        assert_eq!(rows.get(2).map(|r| r.kind), Some(RowKind::LeftOnly));
        let applied = apply_hunk_to_rows(&rows, 1, 2, &Side::Right);
        assert!(applied.is_ok(), "delete orphan hunk failed: {applied:?}");
        let (left_lines, right_lines, count, total) = applied.unwrap();
        assert_eq!(count, 0);
        assert_eq!(total, 2);
        assert_eq!(left_lines, lines(&["a", "d"]));
        assert_eq!(right_lines, lines(&["a", "d"]));
        let opts = autocompare_compare::TextOptions::default();
        let recompared = autocompare_compare::compare_text_lines(&left_lines, &right_lines, &opts);
        let rows2 = build_rows(
            &recompared.left_lines,
            &recompared.right_lines,
            &recompared.script,
        );
        assert_eq!(rows2.len(), 2);
        for r in &rows2 {
            assert_eq!(r.kind, RowKind::Equal);
        }
    }

    #[test]
    fn apply_hunk_clamps_end_out_of_range() {
        let left = lines(&["a", "b1", "c1", "d"]);
        let right = lines(&["a", "b2", "c2", "d"]);
        let rows = build_rows(&left, &right, &script_for(&left, &right));
        // End index beyond total rows clamps instead of hard-failing
        let applied = apply_hunk_to_rows(&rows, 1, 999, &Side::Left);
        assert!(applied.is_ok(), "clamped apply hunk should succeed: {applied:?}");
        let (left_lines, right_lines, count, total) = applied.unwrap();
        assert_eq!(count, 3);
        assert_eq!(total, 3);
        assert_eq!(left_lines, lines(&["a", "b1", "c1", "d"]));
        assert_eq!(right_lines, lines(&["a", "b1", "c1", "d"]));
    }

    #[test]
    fn apply_edit_rejects_orphan_and_range() {
        let left = lines(&["a", "b"]);
        let right = lines(&["a", "b", "c"]);
        let rows = build_rows(&left, &right, &script_for(&left, &right));
        // `rightOnly` tail row has a null left cell.
        let orphan_index = rows
            .iter()
            .find(|r| r.kind == RowKind::RightOnly)
            .map(|r| r.index);
        assert!(orphan_index.is_some(), "expected a rightOnly fixture row");
        assert!(apply_edit_to_rows(&rows, orphan_index.unwrap_or(u64::MAX), &Side::Left, "x").is_err());
        assert!(apply_edit_to_rows(&rows, 99, &Side::Left, "x").is_err());
    }

    #[test]
    fn apply_hunk_double_apply_is_idempotent() {
        // FIXT2 N4: same-range re-copy (gutter then toolbar on the same
        // hunk) must not corrupt the session. The second apply over the
        // already-resolved rows is a safe no-op-ish re-resolve.
        let left = lines(&["a", "b1", "c1", "d"]);
        let right = lines(&["a", "b2", "c2", "d"]);
        let rows = build_rows(&left, &right, &script_for(&left, &right));
        let (l1, r1, _, total) = apply_hunk_to_rows(&rows, 1, 2, &Side::Left)
            .expect("first apply must succeed");
        assert_eq!(total, 2);
        let opts = autocompare_compare::TextOptions::default();
        let recompared = autocompare_compare::compare_text_lines(&l1, &r1, &opts);
        let rows2 = build_rows(
            &recompared.left_lines,
            &recompared.right_lines,
            &recompared.script,
        );
        // Same range, same dir again: rows already equal, re-apply holds.
        let (l2, r2, _, _) = apply_hunk_to_rows(&rows2, 1, 2, &Side::Left)
            .expect("second apply must succeed");
        assert_eq!(l2, l1);
        assert_eq!(r2, r1);
    }

    #[test]
    fn clamp_window_edges() {
        assert_eq!(clamp_window(10, 0, 5), (0, 5));
        assert_eq!(clamp_window(10, 8, 5), (8, 2));
        assert_eq!(clamp_window(10, 10, 5), (10, 0));
        assert_eq!(clamp_window(10, 99, 5), (10, 0));
        assert_eq!(clamp_window(10, 0, 0), (0, 0));
        assert_eq!(clamp_window(0, 0, 5), (0, 0));
        assert_eq!(clamp_window(3, 0, 10_000), (0, 3));
        assert_eq!(clamp_window(10, u64::MAX, u64::MAX), (10, 0));
        assert_eq!(clamp_window(u64::MAX, u64::MAX - 1, 10), (u64::MAX - 1, 1));
    }

    #[test]
    fn build_rows_pairs_delete_insert_into_changed() {
        let left = lines(&["a", "b", "c"]);
        let right = lines(&["a", "B", "c"]);
        let rows = build_rows(&left, &right, &script_for(&left, &right));
        assert_eq!(rows.len(), 3);
        assert_eq!(rows.first().map(|r| r.kind), Some(RowKind::Equal));
        assert_eq!(rows.get(1).map(|r| r.kind), Some(RowKind::Changed));
        assert_eq!(rows.get(2).map(|r| r.kind), Some(RowKind::Equal));
        let changed = rows.get(1).map(|r| r.inline.is_empty());
        assert_eq!(changed, Some(false));
    }

    #[test]
    fn build_rows_pure_insert_is_right_only() {
        let left = lines(&["a"]);
        let right = lines(&["a", "b"]);
        let rows = build_rows(&left, &right, &script_for(&left, &right));
        assert_eq!(rows.len(), 2);
        assert_eq!(rows.get(1).map(|r| r.kind), Some(RowKind::RightOnly));
        assert_eq!(rows.get(1).and_then(|r| r.left.clone()), None);
    }

    #[test]
    fn summarize_overview_run_length_and_total() {
        let left = lines(&["a", "x", "x", "d"]);
        let right = lines(&["a", "y", "y", "d"]);
        let rows = build_rows(&left, &right, &script_for(&left, &right));
        let (chunks, truncated) = summarize_overview(&rows);
        assert!(!truncated);
        assert_eq!(chunks.len(), 3);
        assert_eq!(
            chunks.first().map(|c| (c.start, c.len, c.kind)),
            Some((0, 1, RowKind::Equal))
        );
        let covered: u64 = chunks.iter().map(|c| c.len).sum();
        assert_eq!(covered, rows.len() as u64);
    }

    #[test]
    fn find_direction_picks_first_from_each_end() {
        let left = lines(&["hit one", "mid", "hit two"]);
        let right = lines(&["same", "same", "same"]);
        let rows = build_rows(&left, &right, &script_for(&left, &right));
        let down = find_in_rows(&rows, "hit", Direction::Down).map(|h| h.line);
        let up = find_in_rows(&rows, "hit", Direction::Up).map(|h| h.line);
        assert_eq!(down, Some("hit one".to_string()));
        assert_eq!(up, Some("hit two".to_string()));
        assert_eq!(find_in_rows(&rows, "absent", Direction::Down), None);
        assert_eq!(find_in_rows(&rows, "", Direction::Down), None);
    }

    #[test]
    fn find_column_counts_characters_not_bytes() {
        assert_eq!(find_column("héllo", "llo"), Some(2));
        assert_eq!(find_column("abc", "z"), None);
    }

    #[test]
    fn copy_section_returns_enclosing_block() {
        let left = lines(&["a", "b1", "c1", "d"]);
        let right = lines(&["a", "b2", "c2", "d"]);
        let rows = build_rows(&left, &right, &script_for(&left, &right));
        let section = copy_section_of("s", &rows, 1, Side::Left);
        assert!(section.is_ok());
        if let Ok(section) = section {
            assert_eq!(section.text, "b1\nc1");
            assert_eq!(section.line_count, 2);
            assert_eq!((section.start, section.end), (1, 2));
        }
        let equal = copy_section_of("s", &rows, 0, Side::Right);
        assert!(equal.is_ok());
        if let Ok(equal) = equal {
            assert_eq!(equal.text, "a");
        }
    }

    #[test]
    fn copy_section_errors_on_bad_index_only() {
        let left = lines(&["only-left"]);
        let right = lines(&[]);
        let rows = build_rows(&left, &right, &script_for(&left, &right));
        assert!(copy_section_of("s", &rows, 7, Side::Left).is_err());
        // Wave-T: empty take side is an honest empty receipt, not an error.
        let empty = copy_section_of("s", &rows, 0, Side::Right)
            .expect("empty-source copy must be Ok");
        assert_eq!(empty.line_count, 0);
        assert_eq!(empty.text, "");
    }

    #[test]
    fn side_and_direction_reject_unknown_values() {
        assert!(Side::parse("middle").is_err());
        assert!(Direction::parse("sideways").is_err());
    }

    #[allow(clippy::too_many_arguments)]
    fn text_compare_test(
        left: String,
        right: String,
        ws: Option<bool>,
        cs: Option<bool>,
        ws_snake: Option<bool>,
        cs_snake: Option<bool>,
    ) -> Result<TextCompareDto, String> {
        text_compare(
            left, right, ws, cs, ws_snake, cs_snake, None, None, None, None, None, None, None, None,
            None,
        )
    }

    #[test]
    fn save_side_persists_applied_hunk_to_disk() {
        // FIXTEXT GAP-01 end-to-end: the Save Phase 2 path
        // (text_apply_hunk -> text_save_side) must land the copied hunk
        // in the disk file. `text_copy_section` alone never mutates the
        // stored session, so a save gated on it writes unmutated bytes.
        use std::io::Write;
        let dir = std::env::temp_dir();
        let left = dir.join("fixtext-gap01-left.txt");
        let right = dir.join("fixtext-gap01-right.txt");
        let out = dir.join("fixtext-gap01-saved.txt");
        {
            let mut f = std::fs::File::create(&left).expect("create left");
            f.write_all(b"a\nAAA\nc\n").expect("write left");
            let mut f = std::fs::File::create(&right).expect("create right");
            f.write_all(b"a\nBBB\nc\n").expect("write right");
        }
        let dto = text_compare_test(
            left.display().to_string(),
            right.display().to_string(),
            None,
            None,
            None,
            None,
        )
        .expect("compare must succeed");
        let sess = dto.session.clone();
        // Row 1 is the changed hunk; take-left must apply.
        let hunk = text_apply_hunk(sess.clone(), 1, 1, "left".to_string())
            .expect("apply hunk must succeed");
        assert_eq!((hunk.applied, hunk.total), (1, 1));
        let receipt =
            text_save_side(sess, "right".to_string(), out.display().to_string())
                .expect("save must succeed");
        assert_eq!(receipt.lines, 3);
        let content = std::fs::read_to_string(&out).expect("read saved file");
        assert!(
            content.contains("AAA"),
            "disk file must contain the copied hunk, got: {content:?}"
        );
        assert!(
            !content.contains("BBB"),
            "disk file must not keep the overwritten text, got: {content:?}"
        );
        let _ = std::fs::remove_file(&left);
        let _ = std::fs::remove_file(&right);
        let _ = std::fs::remove_file(&out);
    }

    #[test]
    fn dto_shapes_round_trip_through_json() {
        let dto = TextCompareDto {
            session: "text-1".to_string(),
            left_path: "/a".to_string(),
            right_path: "/b".to_string(),
            left_lines: 2,
            right_lines: 3,
            rows: 3,
            op_count: 4,
            exact: true,
            truncated: false,
            applied_options: AppliedTextOptions {
                ignore_whitespace: false,
                ignore_case: false,
                minor_unimportant: false,
                ignore_blank_lines: false,
                algorithm: None,
                encoding: None,
                eol: None,
                suppressed_lines: Vec::new(),
            },
            left_meta: Some(FileMetaDto {
                bytes: 42,
                mtime_unix: Some(1700000000),
            }),
            right_meta: None,
        };
        let json = serde_json::to_string(&dto).unwrap_or_default();
        assert!(json.contains("\"leftPath\""));
        assert!(json.contains("\"rightPath\""));
        assert!(json.contains("\"leftLines\""));
        assert!(json.contains("\"opCount\""));
        assert!(json.contains("\"appliedOptions\""));
        assert!(json.contains("\"leftMeta\":{"));
        assert!(json.contains("\"bytes\":42"));
        assert!(json.contains("\"mtimeUnix\":1700000000"));
        assert!(json.contains("\"rightMeta\":null"));
        let back: TextCompareDto = serde_json::from_str(&json).unwrap_or_else(|_| TextCompareDto {
            session: String::new(),
            left_path: String::new(),
            right_path: String::new(),
            left_lines: 0,
            right_lines: 0,
            rows: 0,
            op_count: 0,
            exact: false,
            truncated: false,
            applied_options: AppliedTextOptions {
                ignore_whitespace: false,
                ignore_case: false,
                minor_unimportant: false,
                ignore_blank_lines: false,
                algorithm: None,
                encoding: None,
                eol: None,
                suppressed_lines: Vec::new(),
            },
            left_meta: None,
            right_meta: None,
        });
        assert_eq!(back, dto);
    }

    #[test]
    fn text_compare_populates_file_meta_for_fixtures_and_null_for_missing() {
        use std::io::Write;
        let dir = std::env::temp_dir();
        let left = dir.join("wave82-meta-left.txt");
        let right = dir.join("wave82-meta-right.txt");
        let left_content = "hello left\n";
        let right_content = "hello right world\n";
        {
            let mut f1 = std::fs::File::create(&left).expect("create left fixture");
            f1.write_all(left_content.as_bytes()).expect("write left");
            let mut f2 = std::fs::File::create(&right).expect("create right fixture");
            f2.write_all(right_content.as_bytes()).expect("write right");
        }
        let ls = left.display().to_string();
        let rs = right.display().to_string();
        let res = text_compare_test(ls.clone(), rs.clone(), None, None, None, None)
            .expect("text_compare must succeed");

        let left_meta = res.left_meta.expect("left_meta should be populated");
        assert_eq!(left_meta.bytes, left_content.len() as u64);
        assert!(left_meta.mtime_unix.is_some());
        assert!(left_meta.mtime_unix.unwrap() > 0);

        let right_meta = res.right_meta.expect("right_meta should be populated");
        assert_eq!(right_meta.bytes, right_content.len() as u64);
        assert!(right_meta.mtime_unix.is_some());
        assert!(right_meta.mtime_unix.unwrap() > 0);

        // Missing path yields None / null meta without failing
        let missing_meta = read_file_meta("/definitely/not/a/real/path/wave82-missing.txt");
        assert_eq!(missing_meta, None);

        // Verify JSON wire spelling matches contract
        let json = serde_json::to_string(&res).expect("serialize text_compare result");
        assert!(json.contains("\"leftMeta\":{"));
        assert!(json.contains("\"rightMeta\":{"));
        assert!(json.contains("\"bytes\":"));
        assert!(json.contains("\"mtimeUnix\":"));

        let _ = std::fs::remove_file(&left);
        let _ = std::fs::remove_file(&right);
    }

    #[test]
    fn text_compare_ignores_case_and_whitespace() {
        use std::io::Write;
        let dir = std::env::temp_dir();
        let left = dir.join("wave66t-left.txt");
        let right = dir.join("wave66t-right.txt");
        let write = |p: &std::path::PathBuf, s: &str| {
            if let Ok(mut f) = std::fs::File::create(p) {
                let _ = f.write_all(s.as_bytes());
            }
        };
        write(&left, "Hello   World\nsame\n");
        write(&right, "hello world\nsame\n");
        let ls = left.display().to_string();
        let rs = right.display().to_string();
        // Default (all None): byte-identical wire, flags echo false/false.
        if let Ok(exact) = text_compare_test(ls.clone(), rs.clone(), None, None, None, None) {
            assert_eq!(
                (exact.applied_options.ignore_whitespace, exact.applied_options.ignore_case),
                (false, false)
            );
            assert_eq!(exact.rows, 2);
        } else {
            assert!(false, "default compare must succeed");
        }
        // CamelCase flags: case+space differences collapse.
        if let Ok(ignored) = text_compare_test(
            ls.clone(),
            rs.clone(),
            Some(true),
            Some(true),
            None,
            None,
        ) {
            assert_eq!(
                (ignored.applied_options.ignore_whitespace, ignored.applied_options.ignore_case),
                (true, true)
            );
        } else {
            assert!(false, "ignore compare must succeed");
        }
        // Snake_case spellings win too (typed wrapper path).
        if let Ok(snake) = text_compare_test(ls, rs, None, None, Some(true), Some(true)) {
            assert_eq!(
                (snake.applied_options.ignore_whitespace, snake.applied_options.ignore_case),
                (true, true)
            );
        } else {
            assert!(false, "snake compare must succeed");
        }
        let _ = std::fs::remove_file(&left);
        let _ = std::fs::remove_file(&right);
    }

    #[test]
    fn error_paths_on_missing_files_and_sessions() {
        assert!(text_compare_test(String::new(), "/b".to_string(), None, None, None, None).is_err());
        assert!(text_compare_test("/a".to_string(), String::new(), None, None, None, None).is_err());
        let missing = "/definitely/not/here-wave1a-left.txt".to_string();
        assert!(
            text_compare_test(missing, "/definitely/not/here-wave1a-right.txt".to_string(), None, None, None, None).is_err()
        );
        assert!(text_get_rows("no-such-session".to_string(), 0, 10).is_err());
        assert!(text_get_overview("no-such-session".to_string()).is_err());
        assert!(
            text_find(
                "no-such-session".to_string(),
                "q".to_string(),
                "down".to_string()
            )
            .is_err()
        );
        assert!(
            text_find(
                "no-such-session".to_string(),
                String::new(),
                "down".to_string()
            )
            .is_err()
        );
        assert!(text_copy_section("no-such-session".to_string(), 0, "left".to_string()).is_err());
        assert!(
            text_save_side(
                "no-such-session".to_string(),
                "left".to_string(),
                "/tmp/x".to_string()
            )
            .is_err()
        );
        assert!(
            text_save_side(
                "no-such-session".to_string(),
                "left".to_string(),
                String::new()
            )
            .is_err()
        );
    }

    #[test]
    fn repro_file1_file2_sequential_copy() {
        let f1 = "/Users/ektasaini/Documents/File 1.txt";
        let f2 = "/Users/ektasaini/Documents/File 2.txt";
        if !std::path::Path::new(f1).exists() || !std::path::Path::new(f2).exists() {
            println!("repro_file1_file2: files not found, skipping");
            return;
        }
        let comp = text_compare_test(f1.to_string(), f2.to_string(), None, None, None, None)
            .expect("text_compare failed");
        let session = comp.session;
        let cached = stored(&session).expect("session stored");
        println!("Initial rows count: {}", cached.rows.len());

        // Find hunks in initial rows
        let mut hunks: Vec<(u64, u64, RowKind)> = Vec::new();
        let mut in_hunk: Option<(u64, u64, RowKind)> = None;
        for r in &cached.rows {
            if r.kind != RowKind::Equal {
                if let Some((_start, end, _)) = &mut in_hunk {
                    *end = r.index;
                } else {
                    in_hunk = Some((r.index, r.index, r.kind));
                }
            } else if let Some(h) = in_hunk.take() {
                hunks.push(h);
            }
        }
        if let Some(h) = in_hunk.take() {
            hunks.push(h);
        }

        println!("Found {} hunks:", hunks.len());
        for (i, (s, e, k)) in hunks.iter().enumerate().take(10) {
            println!("  Hunk {i}: rows {s}..={e} (kind {k:?})");
            for r in &cached.rows[*s as usize..=*e as usize] {
                println!("    row {}: left={:?} right={:?}", r.index, r.left, r.right);
            }
        }

        // Test text_copy_section for all hunks (dir = "left", l2r)
        println!("\nTesting text_copy_section (l2r):");
        for (i, (s, _, _)) in hunks.iter().enumerate() {
            let res = text_copy_section(session.clone(), *s, "left".to_string());
            assert!(res.is_ok(), "text_copy_section failed at hunk {i} (row {s}): {:?}", res.err());
        }

        // Wave-157: Test sequential text_copy_section and text_apply_hunk alternating L2R and R2L
        println!("\nTesting sequential text_apply_hunk alternating L2R and R2L across hunks:");
        let sim_session = session.clone();
        for (i, (s, e, _)) in hunks.iter().enumerate() {
            let dir = if i % 2 == 0 { "left" } else { "right" };
            let copy_res = text_copy_section(sim_session.clone(), *s, dir.to_string());
            assert!(copy_res.is_ok(), "text_copy_section failed at hunk {i} (row {s}): {:?}", copy_res.err());
            let res = text_apply_hunk(sim_session.clone(), *s, *e, dir.to_string());
            assert!(res.is_ok(), "text_apply_hunk failed at hunk {i} ({s}..={e}, dir={dir}): {:?}", res.err());
        }
        let c = stored(&sim_session).expect("sim_session stored");
        let rem: Vec<_> = c.rows.iter().filter(|r| r.kind != RowKind::Equal).collect();
        assert_eq!(rem.len(), 0, "all hunks must be resolved to equal with 0 non-equal rows remaining (alternating)");

        // Test full sequential L2R across all hunks
        let l2r_session = text_compare_test(f1.to_string(), f2.to_string(), None, None, None, None)
            .expect("text_compare failed").session;
        for (i, (s, e, _)) in hunks.iter().enumerate() {
            let res = text_apply_hunk(l2r_session.clone(), *s, *e, "left".to_string());
            assert!(res.is_ok(), "text_apply_hunk l2r failed at hunk {i} ({s}..={e}): {:?}", res.err());
        }
        let c_l2r = stored(&l2r_session).expect("l2r_session stored");
        let rem_l2r: Vec<_> = c_l2r.rows.iter().filter(|r| r.kind != RowKind::Equal).collect();
        assert_eq!(rem_l2r.len(), 0, "all hunks must be resolved to equal with 0 non-equal rows remaining (l2r)");

        // Test full sequential R2L across all hunks
        let r2l_session = text_compare_test(f1.to_string(), f2.to_string(), None, None, None, None)
            .expect("text_compare failed").session;
        for (i, (s, e, _)) in hunks.iter().enumerate() {
            let res = text_apply_hunk(r2l_session.clone(), *s, *e, "right".to_string());
            assert!(res.is_ok(), "text_apply_hunk r2l failed at hunk {i} ({s}..={e}): {:?}", res.err());
        }
        let c_r2l = stored(&r2l_session).expect("r2l_session stored");
        let rem_r2l: Vec<_> = c_r2l.rows.iter().filter(|r| r.kind != RowKind::Equal).collect();
        assert_eq!(rem_r2l.len(), 0, "all hunks must be resolved to equal with 0 non-equal rows remaining (r2l)");
    }

    // Wave-T: empty take-side sections are honest empty receipts, not
    // errors — this is what unblocked mid-sequence l2r/r2l failures.
    #[test]
    fn copy_section_empty_source_is_ok() {
        let f1 = "/Users/ektasaini/Documents/File 1.txt";
        let f2 = "/Users/ektasaini/Documents/File 2.txt";
        if !std::path::Path::new(f1).exists() || !std::path::Path::new(f2).exists() {
            println!("copy_section_empty_source: files not found, skipping");
            return;
        }
        let comp = text_compare_test(f1.to_string(), f2.to_string(), None, None, None, None)
            .expect("text_compare failed");
        // Hunk 2 (rows 25..=27) is RightOnly: l2r take side is empty.
        let empty = text_copy_section(comp.session.clone(), 25, "left".to_string())
            .expect("empty-source copy must not err");
        assert_eq!(empty.line_count, 0);
        assert_eq!(empty.text, "");
        // Hunk 0 (rows 0..=2) is LeftOnly: l2r carries real lines.
        let full = text_copy_section(comp.session.clone(), 0, "left".to_string())
            .expect("content copy must succeed");
        assert!(full.line_count > 0);
        // Unknown rows still reject honestly.
        assert!(text_copy_section(comp.session, 99999, "left".to_string()).is_err());
    }

    #[test]
    fn text_compare_minor_unimportant_and_suppressed_lines() {
        use std::io::Write;
        let dir = std::env::temp_dir();
        let left = dir.join("contract-l1-left.txt");
        let right = dir.join("contract-l1-right.txt");
        let write = |p: &std::path::PathBuf, s: &str| {
            if let Ok(mut f) = std::fs::File::create(p) {
                let _ = f.write_all(s.as_bytes());
            }
        };
        write(&left, "line 1\nline 2\nline 3\n");
        write(&right, "line 1\nline 2 changed\nline 3\n");
        let ls = left.display().to_string();
        let rs = right.display().to_string();

        // 1. camelCase options in -> echoed appliedOptions + row flag
        let res = text_compare(
            ls.clone(),
            rs.clone(),
            None,
            None,
            None,
            None,
            Some(true),
            None,
            Some(vec![1]),
            None,
            Some("utf-8".to_string()),
            Some("lf".to_string()),
            None,
            None,
            None,
        )
        .expect("text_compare must succeed");

        assert_eq!(res.applied_options.minor_unimportant, true);
        assert_eq!(res.applied_options.suppressed_lines, vec![1]);
        assert_eq!(res.applied_options.encoding, Some("utf-8".to_string()));
        assert_eq!(res.applied_options.eol, Some("lf".to_string()));
        assert_eq!(res.rows, 3); // rows are NOT dropped!

        let page = text_get_rows(res.session.clone(), 0, 10).expect("text_get_rows");
        assert_eq!(page.rows.len(), 3);
        assert!(!page.rows[0].suppressed);
        assert!(page.rows[1].suppressed); // row 1 flagged as suppressed!
        assert!(!page.rows[2].suppressed);

        // 2. snake_case options in -> echoed appliedOptions
        let res_snake = text_compare(
            ls.clone(),
            rs.clone(),
            None,
            None,
            None,
            None,
            None,
            Some(true),
            None,
            Some(vec![2]),
            Some("windows-1252".to_string()),
            Some("crlf".to_string()),
            None,
            None,
            None,
        )
        .expect("text_compare with snake_case must succeed");

        assert_eq!(res_snake.applied_options.minor_unimportant, true);
        assert_eq!(res_snake.applied_options.suppressed_lines, vec![2]);
        assert_eq!(res_snake.applied_options.encoding, Some("windows-1252".to_string()));
        assert_eq!(res_snake.applied_options.eol, Some("crlf".to_string()));
        assert_eq!(res_snake.rows, 3);

        let page_snake = text_get_rows(res_snake.session.clone(), 0, 10).expect("text_get_rows");
        assert!(!page_snake.rows[0].suppressed);
        assert!(!page_snake.rows[1].suppressed);
        assert!(page_snake.rows[2].suppressed); // row 2 flagged!

        let _ = std::fs::remove_file(&left);
        let _ = std::fs::remove_file(&right);
    }

    #[test]
    fn sequential_apply_and_copy_preserves_options_snapshot() {
        use std::io::Write;
        let dir = std::env::temp_dir();
        let left = dir.join("contract-l1-seq-left.txt");
        let right = dir.join("contract-l1-seq-right.txt");
        let write = |p: &std::path::PathBuf, s: &str| {
            if let Ok(mut f) = std::fs::File::create(p) {
                let _ = f.write_all(s.as_bytes());
            }
        };
        write(&left, "first\nsecond\nthird\n");
        write(&right, "first\nchanged second\nthird\n");
        let ls = left.display().to_string();
        let rs = right.display().to_string();

        let comp = text_compare(
            ls,
            rs,
            None,
            None,
            None,
            None,
            Some(true),
            None,
            Some(vec![0]),
            None,
            Some("utf-8".to_string()),
            Some("lf".to_string()),
            None,
            None,
            None,
        )
        .expect("text_compare must succeed");

        // Apply hunk at row 1
        let applied = text_apply_hunk(comp.session.clone(), 1, 1, "left".to_string())
            .expect("text_apply_hunk must succeed");
        assert_eq!(applied.applied, 1);

        // Verify options and suppressed lines are preserved across snapshot in StoredText
        let cached = stored(&comp.session).expect("stored session");
        assert_eq!(cached.opts.minor_unimportant, true);
        assert_eq!(cached.opts.suppressed_lines, vec![0]);
        assert_eq!(cached.opts.encoding, Some("utf-8".to_string()));
        assert_eq!(cached.opts.eol, Some("lf".to_string()));
        assert!(cached.rows[0].suppressed);

        // Sequential copy re-resolves against snapshot
        let section = text_copy_section(comp.session.clone(), 1, "left".to_string())
            .expect("text_copy_section must succeed");
        assert_eq!(section.start, 1);
        assert_eq!(section.end, 1);

        let _ = std::fs::remove_file(&left);
        let _ = std::fs::remove_file(&right);
    }

    #[test]
    fn c2a_algorithm_and_blank_lines_round_trip() {
        // C2a: Patience parses case-insensitively, unknown/omitted falls
        // back to Myers; blank-only lines fold to the same key.
        let (patience, echo) = parse_text_algorithm(Some("Patience".to_string()));
        assert!(matches!(patience, DiffAlgorithm::Patience));
        assert_eq!(echo, Some("patience".to_string()));
        let (myers, no_echo) = parse_text_algorithm(None);
        assert!(matches!(myers, DiffAlgorithm::Myers));
        assert_eq!(no_echo, None);
        let (fallback, _) = parse_text_algorithm(Some("bogus".to_string()));
        assert!(matches!(fallback, DiffAlgorithm::Myers));

        let opts = TextOptions::default();
        assert_eq!(text_compare_key("   ", &opts, true), String::new());
        assert_eq!(text_compare_key("", &opts, true), String::new());
        assert_ne!(text_compare_key("   ", &opts, false), String::new());

        // End-to-end: blank-only whitespace diff collapses under the
        // flag and echoes honestly; patience echoes its name.
        use std::io::Write;
        let dir = std::env::temp_dir();
        let left = dir.join("c2a-blank-left.txt");
        let right = dir.join("c2a-blank-right.txt");
        {
            let mut f = std::fs::File::create(&left).expect("create left");
            f.write_all(b"a\n   \nb\n").expect("write left");
            let mut f = std::fs::File::create(&right).expect("create right");
            f.write_all(b"a\n\nb\n").expect("write right");
        }
        let ls = left.display().to_string();
        let rs = right.display().to_string();
        let exact = text_compare(
            ls.clone(),
            rs.clone(),
            None, None, None, None, None, None, None, None, None, None, None, None, None,
        )
        .expect("exact compare must succeed");
        assert!(!exact.applied_options.ignore_blank_lines);
        assert_eq!(exact.applied_options.algorithm, None);
        let folded = text_compare(
            ls.clone(),
            rs.clone(),
            None, None, None, None, None, None, None, None, None, None,
            Some("patience".to_string()),
            None,
            Some(true),
        )
        .expect("folded compare must succeed");
        assert!(folded.applied_options.ignore_blank_lines);
        assert_eq!(folded.applied_options.algorithm, Some("patience".to_string()));
        assert_eq!(folded.rows, 3);
        let page = text_get_rows(folded.session.clone(), 0, 10).expect("rows");
        assert_eq!(page.rows.len(), 3);
        assert_eq!(page.rows.get(1).map(|r| r.kind), Some(RowKind::Equal));
        let _ = std::fs::remove_file(&left);
        let _ = std::fs::remove_file(&right);
    }
}
