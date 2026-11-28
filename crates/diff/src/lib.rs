//! Pure diff algorithms for autocompare.
//!
//! Layer 0: no I/O, no panics, no unsafe code. Every function accepts
//! hostile input (empty, huge, arbitrary bytes already decoded by the
//! caller) and always returns a valid result.

#![forbid(unsafe_code)]

mod audio;
mod engine;
pub mod grammar;
pub mod hex;
mod image;
pub mod media_tags;
pub mod merge;
mod merge3;
pub(crate) mod odd;
mod table;
pub mod text_diff;
mod word_diff;

pub use audio::{
    AudioDiff, MAX_SAMPLES, PcmAudio, decode_audio, decode_mp3, decode_wav, diff_audio,
    diff_audio_with_tolerance,
};
pub use grammar::{
    CommentRule, CommentState, DisplayFilter, GrammarDiffScript, GrammarInlineSpan,
    GrammarLineDiff, GrammarOptions, GrammarToken, Importance, TokenKind, WhitespaceRule,
    compare_lines_importance, diff_lines_with_grammar, grammar_inline_diff, is_blank_line,
    normalize_line, normalize_lines, strip_comments,
};
pub use image::{
    PictureDiff, RgbaImage, decode_image, decode_png, diff_pictures, encode_png, picture_heatmap,
};
pub use table::{
    KeyRowDiff, Sheet, SheetDiff, SheetDiffStatus, Table, TableCompareOptions, TableKeyDiff,
    Workbook, WorkbookDiff, cells_equal_with_options, cells_equal_with_tolerance,
    detect_key_columns, diff_cells, diff_cells_with_options, diff_rows, diff_table_by_key,
    diff_table_by_key_with_options, diff_workbooks, parse_delimited, sort_table_rows,
};

/// One edit operation over two sequences of lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Op {
    /// The next `len` lines are equal in both inputs.
    Equal { len: usize },
    /// The next `len` lines exist only in the left input.
    Delete { len: usize },
    /// The next `len` lines exist only in the right input.
    Insert { len: usize },
}

/// A minimal edit script between two line sequences.
///
/// `exact` is `false` when the edit budget was exhausted and the script
/// degraded to a coarse replace of the differing middle. Callers should
/// show that limitation honestly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffScript {
    /// The edit operations, in order.
    pub ops: Vec<Op>,
    /// Whether the script is a true minimal diff (no budget fallback).
    pub exact: bool,
}

/// Maximum edit distance attempted before falling back to a coarse script.
pub const EDIT_BUDGET: usize = 50_000;

/// Compares two line sequences (Myers diff, linear space).
///
/// Falls back to common prefix/suffix trimming plus a single
/// delete+insert when the edit distance exceeds [`EDIT_BUDGET`].
/// Never panics.
pub fn diff_lines(a: &[&str], b: &[&str]) -> DiffScript {
    // ODD Phase 6: orchestration span (redacted summaries only).
    let odd_span = tracing::info_span!(
        "diff.lines",
        component = "rs.diff/lines",
        a_lines = a.len() as u64,
        b_lines = b.len() as u64,
        ops = tracing::field::Empty,
        exact = tracing::field::Empty,
    );
    let _odd_guard = if crate::odd::enabled() {
        Some(odd_span.enter())
    } else {
        None
    };
    let (ops, exact) = engine::diff_slices(a, b, Some(EDIT_BUDGET));
    odd_span.record("ops", ops.len() as u64);
    odd_span.record("exact", exact);
    DiffScript { ops, exact }
}

/// A pair of changed character ranges within one line pair (half-open).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineSpan {
    /// Range in the left line.
    pub a_start: usize,
    pub a_end: usize,
    /// Range in the right line.
    pub b_start: usize,
    pub b_end: usize,
}

/// Changed character spans for one pair of lines, usually per word.
///
/// Hierarchical word-and-character diffing: replaced words that share
/// substantial character structure are refined to the exact changed
/// characters, while wholly rewritten words keep one coarse span.
/// Indices are character offsets into the raw lines, half-open
/// `[start, end)`. Empty when the lines are equal. This is a visual
/// aid: highlighting is computed on the raw lines, whatever the
/// comparison options were.
pub fn inline_diff(a: &str, b: &str) -> Vec<InlineSpan> {
    // ODD Phase 6: orchestration span. Only shapes recorded, never text.
    let odd_span = tracing::info_span!(
        "diff.inline",
        component = "rs.diff/inline",
        a_chars = a.chars().count() as u64,
        b_chars = b.chars().count() as u64,
        spans = tracing::field::Empty,
    );
    let _odd_guard = if crate::odd::enabled() {
        Some(odd_span.enter())
    } else {
        None
    };
    let out = word_diff::word_char_diff(a, b);
    odd_span.record("spans", out.len() as u64);
    out
}

/// Three-way merge of `base`, `left` and `right` line sequences.
///
/// Edits from either side apply when they do not overlap; overlapping
/// edits become conflict hunks delimited by `<<<<<<<` / `>>>>>>>`
/// markers. Never panics; the result is always complete lines.
pub fn merge3(base: &[&str], left: &[&str], right: &[&str]) -> Vec<String> {
    // ODD Phase 6: orchestration span (counts only).
    let odd_span = tracing::info_span!(
        "diff.merge3",
        component = "rs.diff/merge3",
        base_lines = base.len() as u64,
        left_lines = left.len() as u64,
        right_lines = right.len() as u64,
        out_lines = tracing::field::Empty,
        has_conflict = tracing::field::Empty,
    );
    let _odd_guard = if crate::odd::enabled() {
        Some(odd_span.enter())
    } else {
        None
    };
    let out = merge3::merge3(base, left, right);
    odd_span.record("out_lines", out.len() as u64);
    odd_span.record("has_conflict", crate::odd::has_conflict_markers(&out));
    out
}

/// Three-way merge with explicit diff algorithm (Myers/Patience).
/// Never panics; always complete lines.
pub fn merge3_with_algorithm(
    base: &[&str],
    left: &[&str],
    right: &[&str],
    algorithm: crate::text_diff::DiffAlgorithm,
) -> Vec<String> {
    // ODD Phase 6: orchestration span (counts + algorithm, never contents).
    let algo_name = match algorithm {
        crate::text_diff::DiffAlgorithm::Myers => "myers",
        crate::text_diff::DiffAlgorithm::Patience => "patience",
    };
    let odd_span = tracing::info_span!(
        "diff.merge3",
        component = "rs.diff/merge3",
        algorithm = algo_name,
        base_lines = base.len() as u64,
        left_lines = left.len() as u64,
        right_lines = right.len() as u64,
        out_lines = tracing::field::Empty,
        has_conflict = tracing::field::Empty,
    );
    let _odd_guard = if crate::odd::enabled() {
        Some(odd_span.enter())
    } else {
        None
    };
    let out = merge3::merge3_with_algorithm(base, left, right, algorithm);
    odd_span.record("out_lines", out.len() as u64);
    odd_span.record("has_conflict", crate::odd::has_conflict_markers(&out));
    out
}
