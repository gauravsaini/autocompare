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
    let (ops, exact) = engine::diff_slices(a, b, Some(EDIT_BUDGET));
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
    word_diff::word_char_diff(a, b)
}

/// Three-way merge of `base`, `left` and `right` line sequences.
///
/// Edits from either side apply when they do not overlap; overlapping
/// edits become conflict hunks delimited by `<<<<<<<` / `>>>>>>>`
/// markers. Never panics; the result is always complete lines.
pub fn merge3(base: &[&str], left: &[&str], right: &[&str]) -> Vec<String> {
    merge3::merge3(base, left, right)
}

/// Three-way merge with explicit diff algorithm (Myers/Patience).
/// Never panics; always complete lines.
pub fn merge3_with_algorithm(
    base: &[&str],
    left: &[&str],
    right: &[&str],
    algorithm: crate::text_diff::DiffAlgorithm,
) -> Vec<String> {
    merge3::merge3_with_algorithm(base, left, right, algorithm)
}
