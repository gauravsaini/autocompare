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
use autocompare_diff::{DiffScript, inline_diff};
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppliedTextOptions {
    pub ignore_whitespace: bool,
    pub ignore_case: bool,
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
    ignore_ws: bool,
    ignore_case: bool,
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
    let at = usize::try_from(index).unwrap_or(usize::MAX);
    let anchor = rows
        .get(at)
        .ok_or_else(|| format!("row index {index} out of range"))?;
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
    if lines.is_empty() {
        return Err(format!("no {} content at row {index}", side.as_str()));
    }
    Ok(CopiedSection {
        session: session.to_string(),
        index,
        side,
        start: lo as u64,
        end: hi.saturating_sub(1) as u64,
        text: lines.join("\n"),
        line_count: lines.len() as u64,
    })
}

/// Compares two text files and caches the full result under a new session
/// id. Only the summary travels back over IPC.
///
/// Wave-66T: optional ignore flags, backward compat. Old callers send
/// only `{left, right}` and get byte-identical comparison
/// (`ignoreWhitespace`/`ignoreCase` default `false`). New callers may
/// send camelCase (`ignoreWhitespace`, `ignoreCase`, what the
/// TextView `recompareWithIgnores` probe sends) and/or snake_case
/// (`ignore_whitespace`, `ignore_case`, what the typed `textCompare`
/// wrapper sends); either spelling wins when `true`.
#[tauri::command]
#[allow(non_snake_case)]
pub fn text_compare(
    left: String,
    right: String,
    ignoreWhitespace: Option<bool>,
    ignoreCase: Option<bool>,
    ignore_whitespace: Option<bool>,
    ignore_case: Option<bool>,
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
    let opts = TextOptions {
        ignore_case: ignore_cs,
        ignore_space_change: ignore_ws,
    };
    let compared = compare_text_files(
        std::path::Path::new(&left),
        std::path::Path::new(&right),
        &opts,
    )
    .map_err(|e| e.to_string())?;
    let rows = build_rows(
        &compared.left_lines,
        &compared.right_lines,
        &compared.script,
    );
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
            ignore_ws,
            ignore_case: ignore_cs,
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
    let opts = TextOptions {
        ignore_case: cached.ignore_case,
        ignore_space_change: cached.ignore_ws,
    };
    let compared = compare_text_lines(&left_lines, &right_lines, &opts);
    let rows = build_rows(&compared.left_lines, &compared.right_lines, &compared.script);
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
            ignore_ws: cached.ignore_ws,
            ignore_case: cached.ignore_case,
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
    fn copy_section_errors_on_bad_index_or_side() {
        let left = lines(&["only-left"]);
        let right = lines(&[]);
        let rows = build_rows(&left, &right, &script_for(&left, &right));
        assert!(copy_section_of("s", &rows, 7, Side::Left).is_err());
        assert!(copy_section_of("s", &rows, 0, Side::Right).is_err());
    }

    #[test]
    fn side_and_direction_reject_unknown_values() {
        assert!(Side::parse("middle").is_err());
        assert!(Direction::parse("sideways").is_err());
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
        let res = text_compare(ls.clone(), rs.clone(), None, None, None, None)
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
        if let Ok(exact) = text_compare(ls.clone(), rs.clone(), None, None, None, None) {
            assert_eq!(
                (exact.applied_options.ignore_whitespace, exact.applied_options.ignore_case),
                (false, false)
            );
            assert_eq!(exact.rows, 2);
        } else {
            assert!(false, "default compare must succeed");
        }
        // CamelCase flags: case+space differences collapse.
        if let Ok(ignored) = text_compare(
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
        if let Ok(snake) = text_compare(ls, rs, None, None, Some(true), Some(true)) {
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
        assert!(text_compare(String::new(), "/b".to_string(), None, None, None, None).is_err());
        assert!(text_compare("/a".to_string(), String::new(), None, None, None, None).is_err());
        let missing = "/definitely/not/here-wave1a-left.txt".to_string();
        assert!(
            text_compare(missing, "/definitely/not/here-wave1a-right.txt".to_string(), None, None, None, None).is_err()
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
}
