//! Hex compare view model: headless-first pure state over the
//! aligned binary engine.
//!
//! Observed behavior parity (clean-room, no copied strings): a hex
//! session compares two files byte by byte with an alignment strategy
//! (positional, fast-resync heuristic, or optimal), caps aligned input
//! per file, chunks differing runs into bounded hunks, and reports an
//! honest truncation flag plus a one-line status. This module holds
//! that state without egui or I/O so it stays unit-testable; painting
//! and engine driving stay the orchestrator's wiring job.
//!
//! Engine mapping: [`HexCompareMode`] maps 1:1 onto
//! [`autocompare_compare::HexAlignment`]; hunk chunking mirrors
//! `compare_binary_files_aligned` (differing runs cut into at most 64
//! byte hunks, `truncated` when more chunks exist than `max_hunks`
//! allows).

#![forbid(unsafe_code)]

use autocompare_compare::{BinaryComparison, HexAlignment};
use autocompare_diff::hex::{HexOp, diff_hex};
use eframe::egui;

/// Per-file byte cap for aligned hex compare, mirroring the engine's
/// 16 MiB limit. Aligned modes hold both inputs in memory, so files
/// beyond this cap report [`HexCompareView::over_cap`] instead of
/// comparing.
pub const ALIGNED_HEX_MAX_BYTES: usize = 16 * 1024 * 1024;

/// Maximum bytes carried per hunk side, mirroring the engine.
pub const HEX_HUNK_LEN: usize = 64;

/// Default hunk budget per compare, matching the app's current call.
pub const HEX_DEFAULT_MAX_HUNKS: usize = 50;

/// Alignment strategy for one hex session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HexCompareMode {
    /// Offset-by-offset compare from byte 0, no shift detection.
    None,
    /// Windowed heuristic resynchronization after an insertion.
    Fast,
    /// Optimal minimal edit script (Myers over bytes).
    #[default]
    Complete,
}

impl HexCompareMode {
    /// All modes in selector order.
    pub fn all() -> [HexCompareMode; 3] {
        [
            HexCompareMode::None,
            HexCompareMode::Fast,
            HexCompareMode::Complete,
        ]
    }

    /// Short label for selectors and status lines.
    pub fn label(self) -> &'static str {
        match self {
            HexCompareMode::None => "None",
            HexCompareMode::Fast => "Fast",
            HexCompareMode::Complete => "Complete",
        }
    }

    /// Engine alignment behind this mode.
    pub fn to_alignment(self) -> HexAlignment {
        match self {
            HexCompareMode::None => HexAlignment::None,
            HexCompareMode::Fast => HexAlignment::Fast,
            HexCompareMode::Complete => HexAlignment::Complete,
        }
    }

    /// Parses a selector label back into a mode. Test-only: the binary
    /// selector drives the typed [`HexCompareMode`] via [`Self::all`], so
    /// no runtime path parses labels; unit tests guard the round-trip.
    #[allow(dead_code)]
    pub fn from_label(label: &str) -> Option<HexCompareMode> {
        match label.trim() {
            "None" => Some(HexCompareMode::None),
            "Fast" => Some(HexCompareMode::Fast),
            "Complete" => Some(HexCompareMode::Complete),
            _ => None,
        }
    }
}

/// One displayed hunk: offset plus per-side byte counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HexHunkView {
    /// Absolute offset of the first differing byte in this hunk.
    pub offset: u64,
    /// Bytes carried from the left file (at most [`HEX_HUNK_LEN`]).
    pub left_len: usize,
    /// Bytes carried from the right file (at most [`HEX_HUNK_LEN`]).
    pub right_len: usize,
}

/// Headless hex compare state: mode, sizes, hunks, selection.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HexCompareView {
    /// Alignment strategy used for this compare.
    pub mode: HexCompareMode,
    /// Hunk budget the compare ran with.
    pub max_hunks: usize,
    /// Left input size in bytes.
    pub left_size: u64,
    /// Right input size in bytes.
    pub right_size: u64,
    /// Offset of the first differing byte, if any.
    pub first_diff: Option<u64>,
    /// Displayed hunks, at most `max_hunks`.
    pub hunks: Vec<HexHunkView>,
    /// Whether more differing chunks exist beyond `max_hunks`.
    pub truncated: bool,
    /// Whether an input exceeded [`ALIGNED_HEX_MAX_BYTES`], so no
    /// compare ran and the status line says so honestly.
    pub over_cap: bool,
    /// Selected hunk index into [`HexCompareView::hunks`].
    pub selected: Option<usize>,
}

impl HexCompareView {
    /// Empty view before any compare ran.
    pub fn empty(mode: HexCompareMode, max_hunks: usize) -> Self {
        Self {
            mode,
            max_hunks,
            left_size: 0,
            right_size: 0,
            first_diff: None,
            hunks: Vec::new(),
            truncated: false,
            over_cap: false,
            selected: None,
        }
    }

    /// Builds a view from an engine [`BinaryComparison`] produced with
    /// `mode` (e.g. via `compare_binary_files_aligned`). Pure and
    /// total; a stale selection is cleared.
    pub fn from_comparison(cmp: &BinaryComparison, mode: HexCompareMode, max_hunks: usize) -> Self {
        let mut hunks = Vec::new();
        // Engine hunks already arrive chunked, but run them through the
        // same 64-byte chunker the slice path uses so an over-long or
        // over-budget engine hunk still reports honestly.
        let mut needed: usize = 0;
        for hunk in cmp.hunks.iter() {
            chunk_run(
                hunk.offset,
                hunk.left.len(),
                hunk.right.len(),
                max_hunks,
                &mut hunks,
                &mut needed,
            );
        }
        Self {
            mode,
            max_hunks,
            left_size: cmp.left_size,
            right_size: cmp.right_size,
            first_diff: cmp.first_diff,
            hunks,
            truncated: cmp.truncated || needed > max_hunks,
            over_cap: false,
            selected: None,
        }
    }

    /// Headless compare over two byte slices with the view's mode,
    /// chunking exactly like `compare_binary_files_aligned`. Inputs
    /// beyond [`ALIGNED_HEX_MAX_BYTES`] yield an [`Self::over_cap`]
    /// view with no hunks instead of comparing. Test-only: the binary
    /// compares files via the engine plus [`Self::from_comparison`];
    /// unit tests drive this pure slice path.
    #[allow(dead_code)]
    pub fn build_from_slices(
        left: &[u8],
        right: &[u8],
        mode: HexCompareMode,
        max_hunks: usize,
    ) -> Self {
        if Self::is_over_cap(left.len(), right.len()) {
            return Self {
                mode,
                max_hunks,
                left_size: left.len() as u64,
                right_size: right.len() as u64,
                first_diff: None,
                hunks: Vec::new(),
                truncated: false,
                over_cap: true,
                selected: None,
            };
        }
        let diff = diff_hex(left, right, mode.to_alignment());
        let mut hunks: Vec<HexHunkView> = Vec::new();
        let mut needed: usize = 0;
        for op in diff.ops.iter() {
            match op {
                HexOp::Equal { .. } => {}
                HexOp::Different {
                    left_offset, len, ..
                } => {
                    chunk_run(*left_offset, *len, *len, max_hunks, &mut hunks, &mut needed);
                }
                HexOp::LeftOnly { left_offset, len } => {
                    chunk_run(*left_offset, *len, 0, max_hunks, &mut hunks, &mut needed);
                }
                HexOp::RightOnly { right_offset, len } => {
                    chunk_run(*right_offset, 0, *len, max_hunks, &mut hunks, &mut needed);
                }
            }
        }
        Self {
            mode,
            max_hunks,
            left_size: left.len() as u64,
            right_size: right.len() as u64,
            first_diff: diff.first_diff,
            hunks,
            truncated: needed > max_hunks,
            over_cap: false,
            selected: None,
        }
    }

    /// Whether either input length exceeds the aligned cap.
    pub fn is_over_cap(left_len: usize, right_len: usize) -> bool {
        left_len > ALIGNED_HEX_MAX_BYTES || right_len > ALIGNED_HEX_MAX_BYTES
    }

    /// Currently selected hunk, if the index is still valid.
    pub fn selected_hunk(&self) -> Option<HexHunkView> {
        match self.selected {
            Some(idx) => self.hunks.get(idx).copied(),
            None => None,
        }
    }

    /// Moves selection to the next hunk, wrapping like a find loop.
    /// Empty hunk lists clear the selection.
    pub fn select_next_hunk(&mut self) {
        if self.hunks.is_empty() {
            self.selected = None;
            return;
        }
        let next = match self.selected {
            Some(idx) => match self.hunks.get(idx) {
                Some(_) => (idx.saturating_add(1)) % self.hunks.len(),
                None => 0,
            },
            None => 0,
        };
        self.selected = Some(next);
    }

    /// Moves selection to the previous hunk, wrapping like a find loop.
    pub fn select_prev_hunk(&mut self) {
        if self.hunks.is_empty() {
            self.selected = None;
            return;
        }
        let prev = match self.selected {
            Some(idx) => match self.hunks.get(idx) {
                Some(_) => {
                    if idx == 0 {
                        self.hunks.len().saturating_sub(1)
                    } else {
                        idx.saturating_sub(1)
                    }
                }
                None => 0,
            },
            None => self.hunks.len().saturating_sub(1),
        };
        self.selected = Some(prev);
    }

    /// Selects the first hunk at or after `offset`, if any.
    pub fn navigate_to_offset(&mut self, offset: u64) -> Option<usize> {
        let mut found: Option<usize> = None;
        for (idx, hunk) in self.hunks.iter().enumerate() {
            if hunk.offset >= offset {
                found = Some(idx);
                break;
            }
        }
        self.selected = found;
        found
    }

    /// First-difference label mirroring the CLI wording.
    pub fn first_diff_label(&self) -> String {
        match self.first_diff {
            Some(off) => format!("first difference at offset {off:#010x}"),
            None => String::from("no differences"),
        }
    }

    /// Honest one-line status for the status bar.
    pub fn status_line(&self) -> String {
        if self.over_cap {
            return format!(
                "hex {}: left {} bytes, right {} bytes, over {} MiB cap, compare skipped",
                self.mode.label(),
                self.left_size,
                self.right_size,
                ALIGNED_HEX_MAX_BYTES / (1024 * 1024)
            );
        }
        let mut line = format!(
            "hex {}: {} hunk(s), {}",
            self.mode.label(),
            self.hunks.len(),
            self.first_diff_label()
        );
        if self.truncated {
            line.push_str(" (more runs exist, hunk limit reached)");
        }
        match self.selected {
            // Validity goes through the single accessor so a stale index
            // reads as no selection everywhere the status line paints.
            Some(idx) if self.selected_hunk().is_some() => {
                line.push_str(&format!(" | selected hunk {}", idx.saturating_add(1)));
            }
            _ => line.push_str(" | no selection"),
        }
        line
    }
}

/// Copies one aligned run into at most 64-byte hunk views, counting
/// every chunk the run needs so the caller can mark truncation
/// honestly. Shared by the slice path and [`HexCompareView::from_comparison`];
/// mirrors the engine's `push_hunk_run`.
fn chunk_run(
    offset: u64,
    left_len: usize,
    right_len: usize,
    max_hunks: usize,
    hunks: &mut Vec<HexHunkView>,
    needed: &mut usize,
) {
    let run_len = left_len.max(right_len);
    let mut done: usize = 0;
    while done < run_len {
        let take = (run_len.saturating_sub(done)).min(HEX_HUNK_LEN);
        *needed = needed.saturating_add(1);
        if hunks.len() < max_hunks {
            let left_take = (left_len.saturating_sub(done)).min(take);
            let right_take = (right_len.saturating_sub(done)).min(take);
            hunks.push(HexHunkView {
                offset: offset.saturating_add(done as u64),
                left_len: left_take,
                right_len: right_take,
            });
        }
        done = done.saturating_add(take);
        if take == 0 {
            break;
        }
    }
}

// ----------------------------------------------------------------------------
// Hex grid (headless; paint wiring stays with the orchestrator).
// Gap 2: the hex tab paints hunk summary lines only. This section is
// the pure grid spec behind a future byte grid: 16 bytes per row,
// an offset ruler, a per-byte cursor with nibble position, and a
// hunk-aware row flag. No egui, no I/O; paint lays out one row per
// 16-byte slice with `format_hex_offset` as the ruler label.
// ----------------------------------------------------------------------------

/// Bytes per hex grid row: the classic 16-byte ruler step.
/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
pub const HEX_BYTES_PER_ROW: usize = 16;

/// Per-byte cursor over one side's byte length: absolute offset plus
/// which nibble the next keystroke edits. `total_len` bounds every
/// move so the cursor never leaves the input.
/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HexCursor {
    /// Absolute byte offset of the cursor.
    pub offset: u64,
    /// True for the high nibble, false for the low nibble.
    pub high_nibble: bool,
    /// Length of the side the cursor roams over.
    pub total_len: u64,
}

/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
impl HexCursor {
    /// Cursor at offset 0 (or nowhere when `total_len` is 0).
    pub fn at_start(total_len: u64) -> Self {
        Self {
            offset: 0,
            high_nibble: true,
            total_len,
        }
    }

    /// Whether the cursor names a real byte.
    pub fn is_valid(self) -> bool {
        self.offset < self.total_len
    }

    /// Grid row of the cursor (0-based).
    pub fn row(self) -> u64 {
        self.offset / (HEX_BYTES_PER_ROW as u64)
    }

    /// Grid column of the cursor (0..16).
    pub fn col(self) -> u64 {
        self.offset % (HEX_BYTES_PER_ROW as u64)
    }

    /// Builds a cursor from a grid position. Returns `None` when the
    /// position names no byte (past the end or empty input).
    pub fn from_row_col(row: u64, col: u64, total_len: u64) -> Option<Self> {
        if col >= HEX_BYTES_PER_ROW as u64 {
            return None;
        }
        let offset = row
            .saturating_mul(HEX_BYTES_PER_ROW as u64)
            .saturating_add(col);
        if offset < total_len {
            Some(Self {
                offset,
                high_nibble: true,
                total_len,
            })
        } else {
            None
        }
    }

    /// Clamps the cursor into the input after the length changed.
    pub fn clamp_len(&mut self) {
        if self.total_len == 0 {
            self.offset = 0;
            self.high_nibble = true;
            return;
        }
        let last = self.total_len.saturating_sub(1);
        if self.offset > last {
            self.offset = last;
        }
    }

    /// Steps one byte toward the start, crossing rows honestly.
    pub fn move_left(&mut self) {
        if self.offset > 0 && self.offset <= self.total_len {
            self.offset = self.offset.saturating_sub(1);
        }
        self.high_nibble = true;
        self.clamp_len();
    }

    /// Steps one byte toward the end, stopping on the last byte.
    pub fn move_right(&mut self) {
        if self.total_len == 0 {
            return;
        }
        let last = self.total_len.saturating_sub(1);
        if self.offset < last {
            self.offset = self.offset.saturating_add(1);
        }
        self.high_nibble = true;
    }

    /// Steps one grid row up (16 bytes), stopping at row 0.
    pub fn move_up(&mut self) {
        self.offset = self.offset.saturating_sub(HEX_BYTES_PER_ROW as u64);
        self.high_nibble = true;
        self.clamp_len();
    }

    /// Steps one grid row down (16 bytes), stopping on the last byte.
    pub fn move_down(&mut self) {
        if self.total_len == 0 {
            return;
        }
        let next = self.offset.saturating_add(HEX_BYTES_PER_ROW as u64);
        let last = self.total_len.saturating_sub(1);
        self.offset = if next > last { last } else { next };
        self.high_nibble = true;
    }

    /// Flips between the high and low nibble of the current byte.
    pub fn toggle_nibble(&mut self) {
        self.high_nibble = !self.high_nibble;
    }
}

/// Offset ruler label for one grid row, e.g. `0x00000010`.
/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
pub fn format_hex_offset(offset: u64) -> String {
    format!("{offset:#010x}")
}

/// Grid row count for `total_len` bytes (partial tail counts as one row).
/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
pub fn hex_row_count(total_len: u64) -> u64 {
    if total_len == 0 {
        return 0;
    }
    total_len
        .saturating_sub(1)
        .saturating_div(HEX_BYTES_PER_ROW as u64)
        .saturating_add(1)
}

/// Start offset of grid row `row`.
/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
pub fn hex_row_start(row: u64) -> u64 {
    row.saturating_mul(HEX_BYTES_PER_ROW as u64)
}

/// Whether any displayed hunk touches the 16-byte row at `row_start`.
/// Hunk ranges are `[offset, offset + max(left_len, right_len))`.
/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
pub fn hex_row_has_diff(row_start: u64, hunks: &[HexHunkView]) -> bool {
    let row_end = row_start.saturating_add(HEX_BYTES_PER_ROW as u64);
    for hunk in hunks {
        let run = (hunk.left_len).max(hunk.right_len);
        if run == 0 {
            continue;
        }
        let end = hunk.offset.saturating_add(run as u64);
        if hunk.offset < row_end && end > row_start {
            return true;
        }
    }
    false
}

// ----------------------------------------------------------------------------
// Gutter-click jump (headless; paint wiring stays with the orchestrator).
// Gap local-run-l #3/#4: long diffs me manual scroll ke bajaye gutter
// click ya next/prev control seedha change par jump kare. Ye section
// pure offset math hai: kaunsa hunk ek offset/row ko cover karta hai
// aur uske aage/peeche kaunsa hunk hai. No egui, no I/O; paint ek
// gutter row click ko `hex_row_start(row)` se offset banakar in
// helpers ko deta hai, orchestrator `navigate_to_offset` se selection
// move karta hai.
// ----------------------------------------------------------------------------

/// Hunk index jo `offset` byte ko cover karta hai, agar koi hai.
/// Hunk range `[offset, offset + max(left_len, right_len))` hai.
/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
pub fn hex_hunk_at_offset(offset: u64, hunks: &[HexHunkView]) -> Option<usize> {
    for (idx, hunk) in hunks.iter().enumerate() {
        let run = (hunk.left_len).max(hunk.right_len);
        if run == 0 {
            continue;
        }
        let end = hunk.offset.saturating_add(run as u64);
        if hunk.offset <= offset && offset < end {
            return Some(idx);
        }
    }
    None
}

/// Pehla hunk jo `offset` par ya uske baad shuru hota hai.
/// Gutter next-change button isi ko `navigate_to_offset` jaisa use karta hai.
/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
pub fn hex_next_hunk_from(offset: u64, hunks: &[HexHunkView]) -> Option<usize> {
    let mut found: Option<usize> = None;
    for (idx, hunk) in hunks.iter().enumerate() {
        if hunk.offset >= offset {
            found = Some(idx);
            break;
        }
    }
    found
}

/// Aakhri hunk jo `offset` se pehle shuru hota hai (strictly `<`).
/// Gutter prev-change button isko use karta hai; already-selected hunk
/// par dobara rukne ke bajaye peeche jata hai.
/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
pub fn hex_prev_hunk_from(offset: u64, hunks: &[HexHunkView]) -> Option<usize> {
    let mut found: Option<usize> = None;
    for (idx, hunk) in hunks.iter().enumerate() {
        if hunk.offset < offset {
            found = Some(idx);
        } else {
            break;
        }
    }
    found
}

/// Grid row `row` ka pehla covering hunk, gutter row-click ke liye.
/// Row range `[hex_row_start(row), +16)` hai; jo hunk is range ko
/// chhuta hai wahi jump target hai.
/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
pub fn hex_row_to_hunk(row: u64, hunks: &[HexHunkView]) -> Option<usize> {
    let row_start = hex_row_start(row);
    let row_end = row_start.saturating_add(HEX_BYTES_PER_ROW as u64);
    for (idx, hunk) in hunks.iter().enumerate() {
        let run = (hunk.left_len).max(hunk.right_len);
        if run == 0 {
            continue;
        }
        let end = hunk.offset.saturating_add(run as u64);
        if hunk.offset < row_end && end > row_start {
            return Some(idx);
        }
    }
    None
}

// ----------------------------------------------------------------------------
// Find-box (headless; paint wiring stays with the orchestrator).
// Gap local-run-l #15: bade diff me symbol dhoondhna manual scan hai.
// Ye pure search helpers hain: forward/backward, wrapping, khali
// needle par `None`. Paint ek text field se needle lekar inhe
// current cursor se chalata hai. No egui, no I/O.
// ----------------------------------------------------------------------------

/// Agli needle occurrence `from` se (inclusive), wrap ke saath.
/// Khali needle ya khali haystack par `None`.
/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
pub fn hex_find_next(haystack: &[u8], needle: &[u8], from: u64) -> Option<u64> {
    if needle.is_empty() || haystack.is_empty() || needle.len() > haystack.len() {
        return None;
    }
    let total = haystack.len() as u64;
    let start = from.min(total.saturating_sub(1));
    let mut idx = start;
    for _ in 0..total {
        let begin = idx as usize;
        let Some(window) = haystack.get(begin..begin.saturating_add(needle.len())) else {
            idx = idx.saturating_add(1) % total.max(1);
            continue;
        };
        if window == needle {
            return Some(idx);
        }
        idx = idx.saturating_add(1) % total.max(1);
    }
    None
}

/// Pichhli needle occurrence `from` se peeche (exclusive), wrap ke saath.
/// Khali needle ya khali haystack par `None`.
/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
pub fn hex_find_prev(haystack: &[u8], needle: &[u8], from: u64) -> Option<u64> {
    if needle.is_empty() || haystack.is_empty() || needle.len() > haystack.len() {
        return None;
    }
    let total = haystack.len() as u64;
    if total == 0 {
        return None;
    }
    let mut idx = if from == 0 {
        total.saturating_sub(1)
    } else {
        (from.saturating_sub(1)) % total.max(1)
    };
    for _ in 0..total {
        let begin = idx as usize;
        if let Some(window) = haystack.get(begin..begin.saturating_add(needle.len()))
            && window == needle
        {
            return Some(idx);
        }
        idx = if idx == 0 {
            total.saturating_sub(1)
        } else {
            idx.saturating_sub(1)
        };
    }
    None
}

// ----------------------------------------------------------------------------
// Hex find positions + count label (headless; paint wiring stays with the
// orchestrator). Gap #12 hex half: find box ko byte offset ke saath grid
// `(row, col)` hit chahiye taaki paint sahi row par scroll karke sahi column
// highlight kare, aur N/M badge ke liye count + label. Offset layer
// (`hex_find_next` / `hex_find_prev`, wrap samet) reuse hota hai; ye sirf
// grid conversion + counting hai. No egui, no I/O.
// ----------------------------------------------------------------------------

/// Grid `(row, col)` ek byte offset ke liye (`HEX_BYTES_PER_ROW` = 16).
/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
pub fn hex_offset_to_row_col(offset: u64) -> (u64, u64) {
    (
        offset / (HEX_BYTES_PER_ROW as u64),
        offset % (HEX_BYTES_PER_ROW as u64),
    )
}

/// Grid `(row, col)` se byte offset. `col >= HEX_BYTES_PER_ROW` par `None`.
/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
pub fn hex_row_col_to_offset(row: u64, col: u64) -> Option<u64> {
    if col >= HEX_BYTES_PER_ROW as u64 {
        return None;
    }
    Some(
        row.saturating_mul(HEX_BYTES_PER_ROW as u64)
            .saturating_add(col),
    )
}

/// Agli needle occurrence `from (row, col)` se (inclusive), wrap ke saath,
/// grid `(row, col)` hit ke roop me. Khali needle/haystack ya invalid `col`
/// par `None`. Andar `hex_find_next` reuse hota hai.
/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
pub fn hex_find_next_position(
    haystack: &[u8],
    needle: &[u8],
    from_row: u64,
    from_col: u64,
) -> Option<(u64, u64)> {
    let from = hex_row_col_to_offset(from_row, from_col)?;
    let hit = hex_find_next(haystack, needle, from)?;
    Some(hex_offset_to_row_col(hit))
}

/// Pichhli needle occurrence `from (row, col)` se peeche (exclusive), wrap
/// ke saath, grid `(row, col)` hit ke roop me. Khali needle/haystack ya
/// invalid `col` par `None`. Andar `hex_find_prev` reuse hota hai.
/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
pub fn hex_find_prev_position(
    haystack: &[u8],
    needle: &[u8],
    from_row: u64,
    from_col: u64,
) -> Option<(u64, u64)> {
    let from = hex_row_col_to_offset(from_row, from_col)?;
    let hit = hex_find_prev(haystack, needle, from)?;
    Some(hex_offset_to_row_col(hit))
}

/// Needle ki overlapping occurrence count (ek-ek byte aage badhkar, taaki
/// koi hit chhute nahi). Khali needle/haystack ya badi needle par `0`.
/// Badge ke `total` ke liye; position cursor se aati hai.
/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
pub fn hex_find_count(haystack: &[u8], needle: &[u8]) -> usize {
    if needle.is_empty() || haystack.is_empty() || needle.len() > haystack.len() {
        return 0;
    }
    let mut count: usize = 0;
    let mut idx: usize = 0;
    while let Some(window) = haystack.get(idx..idx.saturating_add(needle.len())) {
        if window == needle {
            count = count.saturating_add(1);
        }
        idx = idx.saturating_add(1);
    }
    count
}

/// N/M badge text for the hex find bar: "2/5", no hit par "0/0".
/// Pure; `viewmodel::find_count_label` ka hex-side judvaan — agla paint
/// wave isko badge label ke liye use karega. Unit-tested.
/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
pub fn hex_find_count_label(position: Option<usize>, total: usize) -> String {
    match position {
        Some(pos) => format!("{pos}/{total}"),
        None => format!("0/{total}"),
    }
}

// ----------------------------------------------------------------------------
// Hex find bar (paint; caller wiring orchestrator ke paas, is file me sirf
// paint fn + local-state helpers). Gap #12 hex hissa: hex window ke andar
// find row — table `paint_table_find_bar` precedent ka geometry/typo mirror
// (wahi bar height, wahi needle chaudai, wahi Prev/Next labels, badge right
// me ek pad ke saath small weak typo me). Paint caller ke locals
// (`&mut String` needle + badge `&str`) par chalti hai, isliye App/viewer
// struct me koi field edit nahi hota; returns `(prev_clicked, next_clicked)`
// taaki cursor/badge advance caller ke paas rahe. No I/O; colors frozen
// (koi Color32/Visuals/.size nahi, sirf small/weak/small_button).
// ----------------------------------------------------------------------------

/// Hex find bar ki height (table precedent reuse, koi naya token nahi).
/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
pub const HEX_FIND_BAR_H: f32 = 28.0;
/// Hex find needle field ki chaudai (table precedent reuse).
/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
pub const HEX_FIND_NEEDLE_W: f32 = 120.0;
/// Hex find badge ka right-end pad (table precedent reuse).
/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
pub const HEX_FIND_BADGE_PAD: f32 = 4.0;

/// Needle text se bytes: separators hatakar hex jode decode hota hai,
/// warna raw utf-8 bytes. Khali par khali vec. App-side
/// `parse_hex_needle` ka views_hex-local mirror hai (views_hex app par
/// depend nahi kar sakta, isliye same semantics yahin rehti hai taaki
/// agla wiring wave dono taraf ek jaisi needle samjhe). Pure; unit-tested.
/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
pub fn parse_hex_needle_text(text: &str) -> Vec<u8> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    let mut compact = String::new();
    for ch in trimmed.chars() {
        if ch == ' ' || ch == '\t' || ch == ',' || ch == ';' || ch == '_' || ch == '-' || ch == ':'
        {
            continue;
        }
        compact.push(ch);
    }
    let even = compact.len().is_multiple_of(2);
    let all_hex = !compact.is_empty() && compact.chars().all(|c| c.is_ascii_hexdigit());
    if even && all_hex {
        let bytes = compact.as_bytes();
        let mut out = Vec::new();
        let mut i = 0usize;
        while i < bytes.len() {
            let hi = bytes.get(i).copied().unwrap_or(b'0');
            let lo = bytes.get(i.saturating_add(1)).copied().unwrap_or(b'0');
            let hex = [hi, lo];
            let mut ok = false;
            if let Ok(pair) = std::str::from_utf8(&hex)
                && let Ok(byte) = u8::from_str_radix(pair, 16)
            {
                out.push(byte);
                ok = true;
            }
            if !ok {
                return trimmed.as_bytes().to_vec();
            }
            i = i.saturating_add(2);
        }
        return out;
    }
    trimmed.as_bytes().to_vec()
}

/// Needle ki sab overlapping occurrences offset order me (ek-ek byte
/// aage badhkar, taaki koi hit chhute nahi). Khali needle/haystack ya
/// badi needle par khali vec. `hex_find_count` ka poora-hit judvaan
/// (`len()` count ke barabar aata hai); N/M total + Next/Prev nav isi
/// par chalta hai. Pure; unit-tested.
/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
pub fn hex_find_match_offsets(haystack: &[u8], needle: &[u8]) -> Vec<u64> {
    let mut out = Vec::new();
    if needle.is_empty() || haystack.is_empty() || needle.len() > haystack.len() {
        return out;
    }
    let mut idx: usize = 0;
    while let Some(window) = haystack.get(idx..idx.saturating_add(needle.len())) {
        if window == needle {
            out.push(idx as u64);
        }
        idx = idx.saturating_add(1);
    }
    out
}

/// Hex window ke andar find row: needle field + Prev/Next + N/M badge.
/// Table `paint_table_find_bar` ka geometry/typo mirror (wahi bar height,
/// wahi needle chaudai, wahi button labels, badge right me ek pad ke
/// saath small weak typo me). Returns `(prev_clicked, next_clicked)` —
/// cursor/badge advance caller ke locals me hota hai (App fields untouched).
/// Caller signature same rehti hai (hex tab wiring isi ko chalaye, KEVL
/// caller me koi edit nahi).
/// Wiring-pending until orchestrator wires the hex tab caller.
#[allow(dead_code)]
pub fn paint_hex_find_bar(ui: &mut egui::Ui, needle: &mut String, badge: &str) -> (bool, bool) {
    let mut prev = false;
    let mut next = false;
    let avail = ui.available_width();
    ui.horizontal_wrapped(|ui| {
        ui.set_min_size(egui::vec2(avail.max(0.0), HEX_FIND_BAR_H));
        ui.label(egui::RichText::new("Find:").small().weak());
        ui.add(
            egui::TextEdit::singleline(needle)
                .desired_width(HEX_FIND_NEEDLE_W)
                .hint_text("needle"),
        );
        if ui.small_button("◀ Prev").clicked() {
            prev = true;
        }
        if ui.small_button("Next ▶").clicked() {
            next = true;
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(HEX_FIND_BADGE_PAD);
            ui.label(egui::RichText::new(badge).small().weak());
        });
    });
    (prev, next)
}

// ----------------------------------------------------------------------------
// Per-hunk merge (headless; paint wiring stays with the orchestrator).
// Gap local-run-l #10: product view-only hai, comparable tools har hunk
// par take-left/take-right dete hain. Ye pure byte choice hai: hunk ke
// liye left ya right side ke bytes. Poori file apply karna (I/O) wiring
// ka kaam hai aur OPEN rehta hai. No egui, no I/O.
// ----------------------------------------------------------------------------

/// Per-hunk merge choice: kaunsi side ke bytes rakhe jayen.
/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HunkSide {
    /// Left side ke bytes rakho.
    #[default]
    Left,
    /// Right side ke bytes rakho.
    Right,
}

/// Hunk range ke bytes ek side se surakshit nikaalta hai.
/// Out-of-range bytes skip hote hain (saturating, kabhi panic nahi).
fn slice_hunk_bytes(data: &[u8], offset: u64, len: usize) -> Vec<u8> {
    let mut out = Vec::new();
    let start = offset.min(usize::MAX as u64) as usize;
    for i in 0..len {
        let Some(byte) = data.get(start.saturating_add(i)) else {
            break;
        };
        out.push(*byte);
    }
    out
}

/// Ek hunk ka merge result: chosen side ke bytes.
/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
pub fn hex_resolve_hunk(left: &[u8], right: &[u8], hunk: &HexHunkView, side: HunkSide) -> Vec<u8> {
    match side {
        HunkSide::Left => slice_hunk_bytes(left, hunk.offset, hunk.left_len),
        HunkSide::Right => slice_hunk_bytes(right, hunk.offset, hunk.right_len),
    }
}

// ----------------------------------------------------------------------------
// Paging (headless; paint wiring stays with the orchestrator).
// Gap local-run-l #3: lambi hex grid me page-wise move. Ye chhota pager
// top row rakhta hai; paint PageUp/PageDown aur scrollbar se isko
// chalata hai. Pure math, saturating, kabhi panic nahi. No egui, no I/O.
// ----------------------------------------------------------------------------

/// Page-wise scroll state ek hex grid ke liye (rows = `hex_row_count`).
/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HexPager {
    /// Kul grid rows.
    pub total_rows: u64,
    /// Ek page me kitni rows.
    pub rows_per_page: u64,
    /// Visible window ki pehli row.
    pub top: u64,
}

/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
impl HexPager {
    /// Naya pager: page size kam se kam 1, top 0.
    pub fn new(total_rows: u64, rows_per_page: u64) -> Self {
        let per = rows_per_page.max(1);
        Self {
            total_rows,
            rows_per_page: per,
            top: 0,
        }
    }

    /// Total badalne par top clamp hota hai.
    pub fn set_total(&mut self, total_rows: u64) {
        self.total_rows = total_rows;
        self.clamp_top();
    }

    /// Page size badalne par top clamp hota hai.
    pub fn set_page_size(&mut self, rows_per_page: u64) {
        self.rows_per_page = rows_per_page.max(1);
        self.clamp_top();
    }

    /// Kul pages (khali grid = 0).
    pub fn page_count(self) -> u64 {
        if self.total_rows == 0 {
            return 0;
        }
        self.total_rows
            .saturating_sub(1)
            .saturating_div(self.rows_per_page.max(1))
            .saturating_add(1)
    }

    /// Aakhri valid top (aakhri page ki pehli row).
    pub fn last_top(self) -> u64 {
        let count = self.page_count();
        if count == 0 {
            return 0;
        }
        count
            .saturating_sub(1)
            .saturating_mul(self.rows_per_page.max(1))
    }

    /// Top ko valid range me rakhta hai.
    pub fn clamp_top(&mut self) {
        let last = self.last_top();
        if self.top > last {
            self.top = last;
        }
    }

    /// Ek page aage, aakhri page par clamp.
    pub fn next_page(&mut self) {
        self.top = self
            .top
            .saturating_add(self.rows_per_page.max(1))
            .min(self.last_top());
    }

    /// Ek page peeche, 0 par clamp.
    pub fn prev_page(&mut self) {
        self.top = self.top.saturating_sub(self.rows_per_page.max(1));
    }

    /// Visible window `[top, end)`: end kabhi total se aage nahi.
    pub fn visible_range(self) -> (u64, u64) {
        let end = self
            .top
            .saturating_add(self.rows_per_page.max(1))
            .min(self.total_rows);
        (self.top, end.max(self.top))
    }

    /// `row` dikhe isliye top adjust karta hai; bahar ki row par top
    /// clamp hota hai. Returns whether the top moved.
    pub fn bring_into_view(&mut self, row: u64) -> bool {
        if self.total_rows == 0 {
            return false;
        }
        let row = row.min(self.total_rows.saturating_sub(1));
        let (start, end) = self.visible_range();
        if row >= start && row < end {
            return false;
        }
        if row < self.top {
            self.top = row;
        } else {
            self.top = row
                .saturating_add(1)
                .saturating_sub(self.rows_per_page.max(1));
        }
        self.clamp_top();
        true
    }
}

// ----------------------------------------------------------------------------
// Hex gutter parity (headless; paint wiring stays with the orchestrator).
// Text gutter precedent (`crate::gutter`) ka hex judvaan: offsets/row
// markers + click-to-jump. Geometry first (wahi bar widths, wahi
// min-height/merge-gap tokens, marker alignment wahi row bands par),
// typography second (wahi mono role/size/source). Koi naya token ya
// color nahi; sab pure fns, `.get()`/saturating only. No egui, no I/O;
// paint ek gutter click ki local y ko `hex_gutter_jump_target` deta hai
// aur mili hunk index ko `navigate_to_offset` jaisa selection me le jata hai.
// ----------------------------------------------------------------------------

/// Hex grid rows kul bytes se (`hex_row_count` reuse, `usize` me cap).
/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
pub fn hex_gutter_total_rows(total_len: u64) -> usize {
    hex_row_count(total_len).min(usize::MAX as u64) as usize
}

/// Hex diff rows ka gutter geometry: har `hex_row_has_diff` wali row ek
/// band hai, lagatar diff rows merge hote hain. Wahi `GutterGeometry` type,
/// wahi `GUTTER_MIN_SEGMENT_H`/`GUTTER_MERGE_GAP` tokens, kind `Changed`
/// (text `Changed` jaisa paired-diff marker). Pure; unit-tested.
/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
pub fn hex_gutter_geometry(
    total_rows: usize,
    height: f32,
    hunks: &[HexHunkView],
) -> crate::gutter::GutterGeometry {
    if total_rows == 0 || !height.is_finite() || height <= 0.0 {
        return crate::gutter::GutterGeometry {
            total_height: height.max(0.0),
            segments: Vec::new(),
        };
    }
    let mut segments: Vec<crate::gutter::GutterSegment> = Vec::new();
    let total_f = total_rows as f32;
    let mut idx: usize = 0;
    while idx < total_rows {
        let row_start = hex_row_start(idx as u64);
        if hex_row_has_diff(row_start, hunks) {
            let y_start = (idx as f32 / total_f) * height;
            let mut y_end = ((idx.saturating_add(1)) as f32 / total_f) * height;
            if y_end < y_start + crate::theme::GUTTER_MIN_SEGMENT_H {
                y_end = (y_start + crate::theme::GUTTER_MIN_SEGMENT_H).min(height);
            }
            let mut merged = false;
            if let Some(last) = segments.last_mut()
                && last.kind == crate::viewmodel::TextRowKind::Changed
                && y_start <= last.y_end + crate::theme::GUTTER_MERGE_GAP
            {
                last.y_end = last.y_end.max(y_end);
                merged = true;
            }
            if !merged {
                segments.push(crate::gutter::GutterSegment {
                    y_start,
                    y_end,
                    kind: crate::viewmodel::TextRowKind::Changed,
                });
            }
        }
        idx = idx.saturating_add(1);
    }
    crate::gutter::GutterGeometry {
        total_height: height,
        segments,
    }
}

/// Hex gutter bar width: text precedent ka seedha reuse (12.0/6.0).
/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
pub fn hex_gutter_bar_width(show_line_numbers: bool) -> f32 {
    crate::gutter::gutter_bar_width(show_line_numbers)
}

/// Font size se scaled gutter width: text precedent ka seedha reuse.
/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
pub fn hex_gutter_bar_width_for_font(font_size: f32, show_line_numbers: bool) -> f32 {
    crate::gutter::gutter_bar_width_for_font(font_size, show_line_numbers)
}

/// Hex gutter text role: text precedent ka seedha reuse (Mono).
/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
pub const fn hex_gutter_text_role() -> crate::theme::TypoRole {
    crate::gutter::gutter_text_role()
}

/// Hex gutter text size: text precedent ka seedha reuse (mono token).
/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
pub const fn hex_gutter_text_size() -> f32 {
    crate::gutter::gutter_text_size()
}

/// Hex gutter line-height source: text precedent ka seedha reuse.
/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
pub const fn hex_gutter_line_height_source() -> &'static str {
    crate::gutter::gutter_line_height_source()
}

/// Gutter-local y se grid row: text mapping ka seedha reuse.
/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
pub fn hex_gutter_row_at_y(y: f32, total_rows: usize, height: f32) -> Option<usize> {
    crate::gutter::gutter_row_at_y(y, total_rows, height)
}

/// Gutter-local y se segment hit: text hit-test ka seedha reuse.
/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
pub fn hex_gutter_segment_at_y(segments: &[crate::gutter::GutterSegment], y: f32) -> Option<usize> {
    crate::gutter::gutter_segment_at_y(segments, y)
}

/// Grid row se gutter-local y: text inverse mapping ka seedha reuse.
/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
pub fn hex_gutter_y_for_row(row: usize, total_rows: usize, height: f32) -> Option<f32> {
    crate::gutter::gutter_y_for_row(row, total_rows, height)
}

/// Gutter row marker label: us row ka offset ruler text
/// (`format_hex_offset(hex_row_start(row))` reuse). Marker alignment
/// geometry se aati hai (row bands), ye sirf label text hai. Pure.
/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
pub fn hex_gutter_row_label(row: u64) -> String {
    format_hex_offset(hex_row_start(row))
}

/// Hex gutter click-to-jump: y se row (`gutter_row_at_y` reuse), row se
/// covering hunk (`hex_row_to_hunk` reuse); khali row par aage ki rows
/// me pehla covering hunk, warna peeche ka aakhri. Khali hunks ya
/// out-of-range click par `None`. Pure; unit-tested.
/// Test-only until paint wiring lands (orchestrator owns paint).
#[allow(dead_code)]
pub fn hex_gutter_jump_target(
    hunks: &[HexHunkView],
    total_rows: usize,
    height: f32,
    y: f32,
) -> Option<usize> {
    let row = crate::gutter::gutter_row_at_y(y, total_rows, height)?;
    let row_u64 = row as u64;
    if let Some(hit) = hex_row_to_hunk(row_u64, hunks) {
        return Some(hit);
    }
    let total_u64 = total_rows as u64;
    let mut fwd = row_u64.saturating_add(1);
    while fwd < total_u64 {
        if let Some(hit) = hex_row_to_hunk(fwd, hunks) {
            return Some(hit);
        }
        fwd = fwd.saturating_add(1);
    }
    let mut back = row_u64;
    while back > 0 {
        back = back.saturating_sub(1);
        if let Some(hit) = hex_row_to_hunk(back, hunks) {
            return Some(hit);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modes_map_to_engine_alignments() {
        assert_eq!(HexCompareMode::None.to_alignment(), HexAlignment::None);
        assert_eq!(HexCompareMode::Fast.to_alignment(), HexAlignment::Fast);
        assert_eq!(
            HexCompareMode::Complete.to_alignment(),
            HexAlignment::Complete
        );
        assert_eq!(HexCompareMode::default(), HexCompareMode::Complete);
    }

    #[test]
    fn mode_labels_round_trip() {
        for mode in HexCompareMode::all() {
            let label = mode.label();
            assert_eq!(HexCompareMode::from_label(label), Some(mode));
        }
        assert_eq!(HexCompareMode::from_label("bogus"), None);
        assert_eq!(
            HexCompareMode::from_label("  Fast "),
            Some(HexCompareMode::Fast)
        );
    }

    #[test]
    fn identical_slices_report_no_differences() {
        let bytes = vec![1u8, 2, 3, 4];
        for mode in HexCompareMode::all() {
            let view = HexCompareView::build_from_slices(&bytes, &bytes, mode, 8);
            assert_eq!(view.first_diff, None);
            assert!(view.hunks.is_empty());
            assert!(!view.truncated);
            assert!(!view.over_cap);
            assert_eq!(view.first_diff_label(), "no differences");
        }
    }

    #[test]
    fn single_byte_diff_reports_offset_and_hunk() {
        let left = vec![0u8, 1, 2, 3, 4];
        let right = vec![0u8, 1, 2, 9, 4];
        let view = HexCompareView::build_from_slices(&left, &right, HexCompareMode::None, 8);
        assert_eq!(view.first_diff, Some(3));
        assert_eq!(view.hunks.len(), 1);
        assert_eq!(view.hunks.first().map(|h| h.offset), Some(3));
        assert!(view.status_line().contains("first difference at offset"));
        assert!(!view.truncated);
    }

    #[test]
    fn complete_mode_detects_insertion_shift() {
        // Inserting one byte shifts the tail: Complete realigns and
        // reports a gap, None reports positional noise from offset 2.
        let left = vec![10u8, 20, 30, 40];
        let right = vec![10u8, 20, 99, 30, 40];
        let complete =
            HexCompareView::build_from_slices(&left, &right, HexCompareMode::Complete, 8);
        assert_eq!(complete.first_diff, Some(2));
        let positional = HexCompareView::build_from_slices(&left, &right, HexCompareMode::None, 8);
        assert_eq!(positional.first_diff, Some(2));
        // Complete isolates the insertion into fewer hunk bytes than
        // the positional smear over the shifted tail.
        let complete_bytes: usize = complete
            .hunks
            .iter()
            .map(|h| h.left_len.saturating_add(h.right_len))
            .sum();
        assert!(complete_bytes <= 2);
    }

    #[test]
    fn zero_hunk_budget_marks_any_difference_truncated() {
        let left = vec![1u8, 2, 3];
        let right = vec![1u8, 2, 4];
        let view = HexCompareView::build_from_slices(&left, &right, HexCompareMode::None, 0);
        assert!(view.hunks.is_empty());
        assert_eq!(view.first_diff, Some(2));
        assert!(view.truncated);
        assert!(view.status_line().contains("hunk limit reached"));
    }

    #[test]
    fn hunk_budget_clips_long_runs_honestly() {
        let left = vec![0u8; 200];
        let right = vec![1u8; 200];
        let view = HexCompareView::build_from_slices(&left, &right, HexCompareMode::None, 1);
        assert_eq!(view.hunks.len(), 1);
        assert!(view.truncated);
        // 200 differing bytes need four 64-byte chunks.
        let full = HexCompareView::build_from_slices(&left, &right, HexCompareMode::None, 8);
        assert_eq!(full.hunks.len(), 4);
        assert!(!full.truncated);
        assert_eq!(full.hunks.first().map(|h| h.left_len), Some(HEX_HUNK_LEN));
    }

    #[test]
    fn over_cap_skips_compare_and_says_so() {
        assert!(HexCompareView::is_over_cap(
            ALIGNED_HEX_MAX_BYTES.saturating_add(1),
            4
        ));
        assert!(!HexCompareView::is_over_cap(4, 4));
        let big = vec![0u8; ALIGNED_HEX_MAX_BYTES.saturating_add(1)];
        let small = vec![0u8; 4];
        let view = HexCompareView::build_from_slices(&big, &small, HexCompareMode::Fast, 8);
        assert!(view.over_cap);
        assert!(view.hunks.is_empty());
        assert!(view.status_line().contains("cap"));
    }

    #[test]
    fn hunk_selection_wraps_like_a_find_loop() {
        let left = vec![0u8; 200];
        let right = vec![1u8; 200];
        let mut view = HexCompareView::build_from_slices(&left, &right, HexCompareMode::None, 8);
        assert_eq!(view.selected, None);
        view.select_next_hunk();
        assert_eq!(view.selected, Some(0));
        view.select_next_hunk();
        assert_eq!(view.selected, Some(1));
        view.select_prev_hunk();
        assert_eq!(view.selected, Some(0));
        // Wrap: prev from first lands on last, next from last on first.
        view.select_prev_hunk();
        assert_eq!(view.selected, Some(view.hunks.len().saturating_sub(1)));
        view.select_next_hunk();
        assert_eq!(view.selected, Some(0));
        assert!(view.selected_hunk().is_some());
    }

    #[test]
    fn empty_hunks_clear_selection() {
        let mut view = HexCompareView::empty(HexCompareMode::Complete, 8);
        view.select_next_hunk();
        assert_eq!(view.selected, None);
        view.select_prev_hunk();
        assert_eq!(view.selected, None);
        assert_eq!(view.selected_hunk(), None);
    }

    #[test]
    fn navigate_to_offset_picks_first_hunk_at_or_after() {
        let left = vec![0u8; 200];
        let right = vec![1u8; 200];
        let mut view = HexCompareView::build_from_slices(&left, &right, HexCompareMode::None, 8);
        assert_eq!(view.navigate_to_offset(64), Some(1));
        assert_eq!(view.selected, Some(1));
        // Past the last hunk selects nothing.
        assert_eq!(view.navigate_to_offset(10_000), None);
        assert_eq!(view.selected, None);
    }

    #[test]
    fn from_comparison_mirrors_engine_result() {
        let cmp = BinaryComparison {
            left_size: 5,
            right_size: 6,
            first_diff: Some(2),
            hunks: vec![autocompare_compare::BinaryHunk {
                offset: 2,
                left: vec![1, 2],
                right: vec![3, 4, 5],
            }],
            truncated: true,
        };
        let view = HexCompareView::from_comparison(&cmp, HexCompareMode::Fast, 8);
        assert_eq!(view.left_size, 5);
        assert_eq!(view.right_size, 6);
        assert_eq!(view.first_diff, Some(2));
        assert_eq!(view.hunks.len(), 1);
        assert_eq!(
            view.hunks.first().map(|h| (h.left_len, h.right_len)),
            Some((2, 3))
        );
        assert!(view.truncated);
        assert!(view.status_line().contains("Fast"));
    }

    #[test]
    fn from_comparison_chunks_overlong_engine_hunks() {
        let cmp = BinaryComparison {
            left_size: 200,
            right_size: 200,
            first_diff: Some(0),
            hunks: vec![autocompare_compare::BinaryHunk {
                offset: 0,
                left: vec![0u8; 200],
                right: vec![1u8; 200],
            }],
            truncated: false,
        };
        let full = HexCompareView::from_comparison(&cmp, HexCompareMode::None, 8);
        assert_eq!(full.hunks.len(), 4);
        assert!(full.hunks.iter().all(|h| h.left_len <= HEX_HUNK_LEN));
        assert!(!full.truncated);
        let clipped = HexCompareView::from_comparison(&cmp, HexCompareMode::None, 1);
        assert_eq!(clipped.hunks.len(), 1);
        assert!(clipped.truncated);
    }

    #[test]
    fn stale_selection_reads_as_no_selection() {
        let left = vec![0u8, 1, 2, 3, 4];
        let right = vec![0u8, 1, 2, 9, 4];
        let mut view = HexCompareView::build_from_slices(&left, &right, HexCompareMode::None, 8);
        view.selected = Some(99);
        assert_eq!(view.selected_hunk(), None);
        assert!(view.status_line().contains("no selection"));
    }

    #[test]
    fn hex_grid_ruler_and_row_math() {
        assert_eq!(HEX_BYTES_PER_ROW, 16);
        assert_eq!(format_hex_offset(0), "0x00000000");
        assert_eq!(format_hex_offset(16), "0x00000010");
        assert_eq!(hex_row_count(0), 0);
        assert_eq!(hex_row_count(1), 1);
        assert_eq!(hex_row_count(16), 1);
        assert_eq!(hex_row_count(17), 2);
        assert_eq!(hex_row_count(32), 2);
        assert_eq!(hex_row_start(0), 0);
        assert_eq!(hex_row_start(1), 16);
        assert_eq!(hex_row_start(2), 32);
    }

    #[test]
    fn hex_cursor_moves_over_grid() {
        let mut cursor = HexCursor::at_start(32);
        assert!(cursor.is_valid());
        assert_eq!((cursor.row(), cursor.col()), (0, 0));
        cursor.move_right();
        assert_eq!(cursor.offset, 1);
        cursor.move_down();
        assert_eq!(cursor.offset, 17);
        assert_eq!((cursor.row(), cursor.col()), (1, 1));
        cursor.move_up();
        assert_eq!(cursor.offset, 1);
        cursor.move_left();
        assert_eq!(cursor.offset, 0);
        // Left at zero stays, right at end clamps.
        cursor.move_left();
        assert_eq!(cursor.offset, 0);
        let mut end = HexCursor::at_start(32);
        end.offset = 31;
        end.move_right();
        assert_eq!(end.offset, 31);
        end.move_down();
        assert_eq!(end.offset, 31);
        // Nibble toggle flips only the nibble flag.
        let mut nib = HexCursor::at_start(8);
        assert!(nib.high_nibble);
        nib.toggle_nibble();
        assert!(!nib.high_nibble);
        nib.toggle_nibble();
        assert!(nib.high_nibble);
        // Row/col round-trip.
        let placed = HexCursor::from_row_col(1, 1, 32);
        assert_eq!(placed.map(|c| c.offset), Some(17));
        assert_eq!(HexCursor::from_row_col(0, 16, 32), None);
        assert_eq!(HexCursor::from_row_col(5, 0, 32), None);
        // Empty input has no valid cursor.
        let empty = HexCursor::at_start(0);
        assert!(!empty.is_valid());
    }

    #[test]
    fn hex_row_diff_flag_follows_hunks() {
        let left = vec![0u8; 48];
        let mut right = vec![0u8; 48];
        if let Some(slot) = right.get_mut(20) {
            *slot = 9;
        }
        let view = HexCompareView::build_from_slices(&left, &right, HexCompareMode::None, 8);
        assert!(!view.hunks.is_empty());
        assert!(!hex_row_has_diff(0, &view.hunks));
        assert!(hex_row_has_diff(16, &view.hunks));
        assert!(!hex_row_has_diff(32, &view.hunks));
        assert!(!hex_row_has_diff(0, &[]));
    }

    #[test]
    fn hex_find_wraps_forward_and_backward() {
        let hay = vec![10u8, 20, 30, 20, 30, 40];
        let needle = vec![20u8, 30];
        assert_eq!(hex_find_next(&hay, &needle, 0), Some(1));
        assert_eq!(hex_find_next(&hay, &needle, 2), Some(3));
        // Wrap: aakhri occurrence ke baad pehli.
        assert_eq!(hex_find_next(&hay, &needle, 4), Some(1));
        assert_eq!(hex_find_prev(&hay, &needle, 4), Some(3));
        assert_eq!(hex_find_prev(&hay, &needle, 3), Some(1));
        // Wrap peeche: shuru se peeche aakhri.
        assert_eq!(hex_find_prev(&hay, &needle, 0), Some(3));
        // Khali needle/haystack ya badi needle: None.
        assert_eq!(hex_find_next(&hay, &[], 0), None);
        assert_eq!(hex_find_next(&[], &needle, 0), None);
        assert_eq!(hex_find_next(&[1u8], &[1u8, 2u8], 0), None);
        assert_eq!(hex_find_prev(&hay, &[], 0), None);
        assert_eq!(hex_find_prev(&[], &needle, 0), None);
        // Missing needle: None.
        assert_eq!(hex_find_next(&hay, &[99u8], 0), None);
        assert_eq!(hex_find_prev(&hay, &[99u8], 2), None);
    }

    #[test]
    fn hex_merge_resolves_chosen_side_bytes() {
        let left = vec![1u8, 2, 3, 4];
        let right = vec![1u8, 9, 3, 4];
        let view = HexCompareView::build_from_slices(&left, &right, HexCompareMode::None, 8);
        assert_eq!(view.hunks.len(), 1);
        let Some(hunk) = view.hunks.first() else {
            return;
        };
        assert_eq!(
            hex_resolve_hunk(&left, &right, hunk, HunkSide::Left),
            vec![2u8]
        );
        assert_eq!(
            hex_resolve_hunk(&left, &right, hunk, HunkSide::Right),
            vec![9u8]
        );
        // Out-of-range hunk kabhi panic nahi karta.
        let past = HexHunkView {
            offset: 100,
            left_len: 4,
            right_len: 4,
        };
        assert!(hex_resolve_hunk(&left, &right, &past, HunkSide::Left).is_empty());
        assert!(hex_resolve_hunk(&left, &right, &past, HunkSide::Right).is_empty());
        // Insertion hunk: ek side khali.
        let ins = HexHunkView {
            offset: 2,
            left_len: 0,
            right_len: 2,
        };
        assert!(hex_resolve_hunk(&left, &right, &ins, HunkSide::Left).is_empty());
        assert_eq!(
            hex_resolve_hunk(&right, &right, &ins, HunkSide::Right).len(),
            2
        );
    }

    #[test]
    fn hex_pager_pages_and_brings_rows_into_view() {
        let mut pager = HexPager::new(100, 16);
        assert_eq!(pager.page_count(), 7);
        assert_eq!(pager.visible_range(), (0, 16));
        pager.next_page();
        assert_eq!(pager.top, 16);
        pager.prev_page();
        assert_eq!(pager.top, 0);
        // Aakhri page par clamp: 96-99.
        for _ in 0..20 {
            pager.next_page();
        }
        assert_eq!(pager.top, 96);
        assert_eq!(pager.visible_range(), (96, 100));
        pager.next_page();
        assert_eq!(pager.top, 96);
        // Visible row par koi move nahi.
        assert!(!pager.bring_into_view(97));
        // Upar ki row par top jump.
        assert!(pager.bring_into_view(10));
        assert_eq!(pager.top, 10);
        // Neeche ki row par window ka aakhri row banta hai.
        let mut small = HexPager::new(100, 10);
        assert!(small.bring_into_view(50));
        assert_eq!(small.top, 41);
        // Khali grid: pages 0, koi move nahi.
        let mut empty = HexPager::new(0, 10);
        assert_eq!(empty.page_count(), 0);
        assert!(!empty.bring_into_view(5));
        // Page size 0 matlab 1.
        let one = HexPager::new(5, 0);
        assert_eq!(one.rows_per_page, 1);
        assert_eq!(one.page_count(), 5);
    }

    #[test]
    fn hex_gutter_jump_targets_covering_hunks() {
        let left = vec![0u8; 64];
        let mut right = vec![0u8; 64];
        if let Some(slot) = right.get_mut(20) {
            *slot = 9;
        }
        if let Some(slot) = right.get_mut(50) {
            *slot = 7;
        }
        let view = HexCompareView::build_from_slices(&left, &right, HexCompareMode::None, 8);
        assert!(view.hunks.len() >= 2);
        // Covering hunk milta hai, bahar ka offset nahi.
        let first = view.hunks.first().map(|h| h.offset).unwrap_or(0);
        assert_eq!(hex_hunk_at_offset(first, &view.hunks), Some(0));
        assert_eq!(hex_hunk_at_offset(0, &view.hunks), None);
        assert_eq!(hex_hunk_at_offset(10_000, &view.hunks), None);
        // Next/prev offset se.
        assert_eq!(hex_next_hunk_from(0, &view.hunks), Some(0));
        assert_eq!(
            hex_next_hunk_from(first.saturating_add(1), &view.hunks),
            Some(1)
        );
        assert_eq!(hex_next_hunk_from(10_000, &view.hunks), None);
        assert_eq!(hex_prev_hunk_from(0, &view.hunks), None);
        let second = view.hunks.get(1).map(|h| h.offset).unwrap_or(0);
        assert_eq!(hex_prev_hunk_from(second, &view.hunks), Some(0));
        assert_eq!(
            hex_prev_hunk_from(second.saturating_add(1), &view.hunks),
            Some(1)
        );
        // Gutter row-click: row 1 (16-31) pehla hunk, row 0 khali.
        assert_eq!(hex_row_to_hunk(1, &view.hunks), Some(0));
        assert_eq!(hex_row_to_hunk(0, &view.hunks), None);
        assert_eq!(hex_row_to_hunk(99, &view.hunks), None);
        assert_eq!(hex_row_to_hunk(0, &[]), None);
    }

    #[test]
    fn hex_find_positions_report_row_col_with_wrap() {
        let hay = vec![10u8, 20, 30, 20, 30, 40];
        let needle = vec![20u8, 30];
        // Offset <-> grid round-trip (16 bytes per row).
        assert_eq!(hex_offset_to_row_col(1), (0, 1));
        assert_eq!(hex_offset_to_row_col(17), (1, 1));
        assert_eq!(hex_row_col_to_offset(0, 1), Some(1));
        assert_eq!(hex_row_col_to_offset(1, 1), Some(17));
        assert_eq!(hex_row_col_to_offset(0, 16), None);
        // Aage: from inclusive, row+col hit.
        assert_eq!(hex_find_next_position(&hay, &needle, 0, 0), Some((0, 1)));
        assert_eq!(hex_find_next_position(&hay, &needle, 0, 2), Some((0, 3)));
        // Wrap: aakhri occurrence ke baad pehli.
        assert_eq!(hex_find_next_position(&hay, &needle, 0, 4), Some((0, 1)));
        assert_eq!(hex_find_prev_position(&hay, &needle, 0, 4), Some((0, 3)));
        assert_eq!(hex_find_prev_position(&hay, &needle, 0, 3), Some((0, 1)));
        // Wrap peeche: shuru se peeche aakhri.
        assert_eq!(hex_find_prev_position(&hay, &needle, 0, 0), Some((0, 3)));
        // Dusri row ka hit: offset 17 -> (1, 1).
        let mut big = vec![0u8; 20];
        if let Some(slot) = big.get_mut(17) {
            *slot = 9;
        }
        assert_eq!(hex_find_next_position(&big, &[9u8], 0, 0), Some((1, 1)));
        assert_eq!(hex_find_prev_position(&big, &[9u8], 1, 2), Some((1, 1)));
        // Invalid col / khali needle / missing needle: None.
        assert_eq!(hex_find_next_position(&hay, &needle, 0, 16), None);
        assert_eq!(hex_find_prev_position(&hay, &needle, 0, 16), None);
        assert_eq!(hex_find_next_position(&hay, &[], 0, 0), None);
        assert_eq!(hex_find_prev_position(&hay, &[], 0, 0), None);
        assert_eq!(hex_find_next_position(&hay, &[99u8], 0, 0), None);
        assert_eq!(hex_find_prev_position(&hay, &[99u8], 0, 1), None);
    }

    #[test]
    fn hex_find_count_and_label_formats_n_of_m() {
        let hay = vec![10u8, 20, 30, 20, 30, 40];
        let needle = vec![20u8, 30];
        assert_eq!(hex_find_count(&hay, &needle), 2);
        // Overlapping: 3 same bytes me 2-wide needle do baar.
        assert_eq!(hex_find_count(&[7u8, 7, 7], &[7u8, 7]), 2);
        assert_eq!(hex_find_count(&hay, &[]), 0);
        assert_eq!(hex_find_count(&[], &needle), 0);
        assert_eq!(hex_find_count(&[1u8], &[1u8, 2u8]), 0);
        assert_eq!(hex_find_count(&hay, &[99u8]), 0);
        assert_eq!(hex_find_count_label(Some(2), 5), "2/5");
        assert_eq!(hex_find_count_label(Some(1), 1), "1/1");
        assert_eq!(hex_find_count_label(None, 5), "0/5");
        assert_eq!(hex_find_count_label(None, 0), "0/0");
    }

    #[test]
    fn hex_find_needle_text_parses_hex_pairs_or_raw_bytes() {
        assert!(parse_hex_needle_text("").is_empty());
        assert!(parse_hex_needle_text("   ").is_empty());
        // Hex jode: separators hatkar decode hota hai.
        assert_eq!(parse_hex_needle_text("14 1e"), vec![0x14u8, 0x1e]);
        assert_eq!(parse_hex_needle_text("14:1e"), vec![0x14u8, 0x1e]);
        assert_eq!(parse_hex_needle_text("14-1e"), vec![0x14u8, 0x1e]);
        assert_eq!(parse_hex_needle_text("14,1e"), vec![0x14u8, 0x1e]);
        assert_eq!(parse_hex_needle_text("FF 00"), vec![0xFFu8, 0x00]);
        assert_eq!(parse_hex_needle_text("ff"), vec![0xFFu8]);
        // Odd length ya gair-hex par raw utf-8 bytes girte hain.
        assert_eq!(parse_hex_needle_text("abc"), b"abc".to_vec());
        assert_eq!(parse_hex_needle_text("zz"), b"zz".to_vec());
        assert_eq!(parse_hex_needle_text("banana"), b"banana".to_vec());
        // Bahar ki space trim hoti hai, andar wali separator hai.
        assert_eq!(parse_hex_needle_text("  14 1e  "), vec![0x14u8, 0x1e]);
    }

    #[test]
    fn hex_find_match_offsets_collects_all_hits_in_order() {
        let hay = vec![10u8, 20, 30, 20, 30, 40];
        let needle = vec![20u8, 30];
        assert_eq!(hex_find_match_offsets(&hay, &needle), vec![1u64, 3]);
        // Overlapping: 3 same bytes me 2-wide needle do baar.
        assert_eq!(
            hex_find_match_offsets(&[7u8, 7, 7], &[7u8, 7]),
            vec![0u64, 1]
        );
        // Poora-hit judvaan: len count ke barabar.
        assert_eq!(
            hex_find_match_offsets(&hay, &needle).len(),
            hex_find_count(&hay, &needle)
        );
        // Khali needle/haystack, badi ya missing needle: khali.
        assert!(hex_find_match_offsets(&hay, &[]).is_empty());
        assert!(hex_find_match_offsets(&[], &needle).is_empty());
        assert!(hex_find_match_offsets(&hay, &[99u8]).is_empty());
        assert!(hex_find_match_offsets(&[1u8], &[1u8, 2u8]).is_empty());
    }

    #[test]
    fn hex_find_bar_geometry_reuses_table_precedent() {
        // Table precedent values reuse hote hain, koi naya token nahi.
        assert_eq!(HEX_FIND_BAR_H, 28.0);
        assert_eq!(HEX_FIND_NEEDLE_W, 120.0);
        assert_eq!(HEX_FIND_BADGE_PAD, 4.0);
        // Badge label shape count label jaisa hai.
        assert_eq!(hex_find_count_label(Some(2), 5), "2/5");
        assert_eq!(hex_find_count_label(None, 0), "0/0");
    }

    #[test]
    fn hex_gutter_geometry_marks_diff_rows_and_merges() {
        use crate::viewmodel::TextRowKind;
        // 64 bytes = 4 rows; byte 20 (row 1) aur byte 50 (row 3) diff.
        let left = vec![0u8; 64];
        let mut right = vec![0u8; 64];
        if let Some(slot) = right.get_mut(20) {
            *slot = 9;
        }
        if let Some(slot) = right.get_mut(50) {
            *slot = 7;
        }
        let view = HexCompareView::build_from_slices(&left, &right, HexCompareMode::None, 8);
        assert!(view.hunks.len() >= 2);
        let total = hex_gutter_total_rows(64);
        assert_eq!(total, 4);
        let geom = hex_gutter_geometry(total, 400.0, &view.hunks);
        // Row 1 (16-31) aur row 3 (48-63) diff: do segments.
        assert_eq!(geom.segments.len(), 2);
        for seg in geom.segments.iter() {
            assert_eq!(seg.kind, TextRowKind::Changed);
        }
        let Some(first) = geom.segments.first() else {
            return;
        };
        assert_eq!(first.y_start, 100.0);
        assert_eq!(first.y_end, 200.0);
        let Some(second) = geom.segments.get(1) else {
            return;
        };
        assert_eq!(second.y_start, 300.0);
        assert_eq!(second.y_end, 400.0);
        // Lagatar diff rows merge: 32 bytes (2 rows) ek hi run.
        let l2 = vec![0u8; 32];
        let r2 = vec![1u8; 32];
        let v2 = HexCompareView::build_from_slices(&l2, &r2, HexCompareMode::None, 8);
        let g2 = hex_gutter_geometry(hex_gutter_total_rows(32), 200.0, &v2.hunks);
        assert_eq!(g2.segments.len(), 1);
        // Khali/zero par koi segment nahi.
        assert!(
            hex_gutter_geometry(0, 400.0, &view.hunks)
                .segments
                .is_empty()
        );
        assert!(
            hex_gutter_geometry(total, 0.0, &view.hunks)
                .segments
                .is_empty()
        );
        assert!(hex_gutter_geometry(total, 400.0, &[]).segments.is_empty());
    }

    #[test]
    fn hex_gutter_width_and_typo_follow_text_precedent() {
        use crate::gutter::{
            gutter_bar_width, gutter_bar_width_for_font, gutter_text_role, gutter_text_size,
        };
        use crate::theme::{TYPO_LINE_HEIGHT_SOURCE, TYPO_MONO_SIZE};
        assert_eq!(hex_gutter_bar_width(true), gutter_bar_width(true));
        assert_eq!(hex_gutter_bar_width(false), gutter_bar_width(false));
        assert_eq!(hex_gutter_bar_width(true), 12.0);
        assert_eq!(hex_gutter_bar_width(false), 6.0);
        assert_eq!(
            hex_gutter_bar_width_for_font(TYPO_MONO_SIZE, true),
            gutter_bar_width_for_font(TYPO_MONO_SIZE, true)
        );
        assert_eq!(hex_gutter_text_role(), gutter_text_role());
        assert_eq!(hex_gutter_text_size(), gutter_text_size());
        assert_eq!(hex_gutter_text_size(), TYPO_MONO_SIZE);
        assert_eq!(hex_gutter_line_height_source(), TYPO_LINE_HEIGHT_SOURCE);
        assert_eq!(
            hex_gutter_line_height_source(),
            crate::gutter::gutter_line_height_source()
        );
    }

    #[test]
    fn hex_gutter_row_at_y_delegates_to_text_mapping() {
        // 4 rows, height 400: har row 100px band.
        assert_eq!(hex_gutter_row_at_y(0.0, 4, 400.0), Some(0));
        assert_eq!(hex_gutter_row_at_y(150.0, 4, 400.0), Some(1));
        assert_eq!(hex_gutter_row_at_y(399.9, 4, 400.0), Some(3));
        assert_eq!(hex_gutter_row_at_y(400.0, 4, 400.0), None);
        assert_eq!(hex_gutter_row_at_y(10.0, 0, 400.0), None);
        assert_eq!(hex_gutter_row_at_y(f32::NAN, 4, 400.0), None);
        // Inverse + segment hit bhi text precedent jaisa.
        assert_eq!(hex_gutter_y_for_row(1, 4, 400.0), Some(100.0));
        assert_eq!(hex_gutter_y_for_row(4, 4, 400.0), None);
        let geom = hex_gutter_geometry(4, 400.0, &[]);
        assert_eq!(hex_gutter_segment_at_y(&geom.segments, 150.0), None);
    }

    #[test]
    fn hex_gutter_jump_target_snaps_to_covering_hunk() {
        let left = vec![0u8; 64];
        let mut right = vec![0u8; 64];
        if let Some(slot) = right.get_mut(20) {
            *slot = 9;
        }
        if let Some(slot) = right.get_mut(50) {
            *slot = 7;
        }
        let view = HexCompareView::build_from_slices(&left, &right, HexCompareMode::None, 8);
        assert!(view.hunks.len() >= 2);
        // Row 1 par click covering hunk 0 deta hai.
        assert_eq!(
            hex_gutter_jump_target(&view.hunks, 4, 400.0, 150.0),
            Some(0)
        );
        // Row 0 khali: aage snap karke hunk 0.
        assert_eq!(hex_gutter_jump_target(&view.hunks, 4, 400.0, 50.0), Some(0));
        // Row 2 khali (32-47): aage snap karke hunk 1.
        assert_eq!(
            hex_gutter_jump_target(&view.hunks, 4, 400.0, 250.0),
            Some(1)
        );
        // Row 3 covering hunk 1.
        assert_eq!(
            hex_gutter_jump_target(&view.hunks, 4, 400.0, 350.0),
            Some(1)
        );
        // Khali hunks / out-of-range: None.
        assert_eq!(hex_gutter_jump_target(&[], 4, 400.0, 150.0), None);
        assert_eq!(hex_gutter_jump_target(&view.hunks, 4, 400.0, 999.0), None);
        assert_eq!(hex_gutter_jump_target(&[], 0, 400.0, 50.0), None);
    }

    #[test]
    fn hex_gutter_row_labels_use_offset_ruler() {
        assert_eq!(hex_gutter_row_label(0), format_hex_offset(0));
        assert_eq!(hex_gutter_row_label(1), format_hex_offset(16));
        assert_eq!(hex_gutter_row_label(1), "0x00000010");
        assert_eq!(hex_gutter_row_label(2), "0x00000020");
        assert_eq!(hex_gutter_total_rows(0), 0);
        assert_eq!(hex_gutter_total_rows(1), 1);
        assert_eq!(hex_gutter_total_rows(16), 1);
        assert_eq!(hex_gutter_total_rows(17), 2);
    }
}
