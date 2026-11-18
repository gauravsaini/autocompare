//! Pure diff algorithms for autocompare.
//!
//! Layer 0: no I/O, no panics, no unsafe code. Every function accepts
//! hostile input (empty, huge, arbitrary bytes already decoded by the
//! caller) and always returns a valid result.

#![forbid(unsafe_code)]

mod audio;
mod engine;
mod image;
mod merge3;
mod table;
mod word_diff;

pub use audio::{
    AudioDiff, MAX_SAMPLES, PcmAudio, decode_audio, decode_mp3, decode_wav, diff_audio,
};
pub use image::{PictureDiff, RgbaImage, decode_png, diff_pictures, encode_png, picture_heatmap};
pub use table::{Table, diff_cells, diff_rows, parse_delimited};

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
/// Indices are character offsets into the raw lines, half-open
/// `[start, end)`. Empty when the lines are equal. This is a visual
/// aid: highlighting is computed on the raw lines, whatever the
/// comparison options were.
pub fn inline_diff(a: &str, b: &str) -> Vec<InlineSpan> {
    word_diff::inline_diff(a, b)
}

/// Three-way merge of `base`, `left` and `right` line sequences.
///
/// Edits from either side apply when they do not overlap; overlapping
/// edits become conflict hunks delimited by `<<<<<<<` / `>>>>>>>`
/// markers. Never panics; the result is always complete lines.
pub fn merge3(base: &[&str], left: &[&str], right: &[&str]) -> Vec<String> {
    merge3::merge3(base, left, right)
}
