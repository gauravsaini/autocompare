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
pub const HEX_BYTES_PER_ROW: usize = 16;

/// Per-byte cursor over one side's byte length: absolute offset plus
/// which nibble the next keystroke edits. `total_len` bounds every
/// move so the cursor never leaves the input.
/// Test-only until paint wiring lands (orchestrator owns paint).
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
pub fn format_hex_offset(offset: u64) -> String {
    format!("{offset:#010x}")
}

/// Grid row count for `total_len` bytes (partial tail counts as one row).
/// Test-only until paint wiring lands (orchestrator owns paint).
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
pub fn hex_row_start(row: u64) -> u64 {
    row.saturating_mul(HEX_BYTES_PER_ROW as u64)
}

/// Whether any displayed hunk touches the 16-byte row at `row_start`.
/// Hunk ranges are `[offset, offset + max(left_len, right_len))`.
/// Test-only until paint wiring lands (orchestrator owns paint).
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
pub fn hex_offset_to_row_col(offset: u64) -> (u64, u64) {
    (
        offset / (HEX_BYTES_PER_ROW as u64),
        offset % (HEX_BYTES_PER_ROW as u64),
    )
}

/// Grid `(row, col)` se byte offset. `col >= HEX_BYTES_PER_ROW` par `None`.
/// Test-only until paint wiring lands (orchestrator owns paint).
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
// me ek pad ke saath Small-role typo me). Typography second: section label
// Body-strong (helper + paint-side `.strong()`), needle Mono family
// (`.font(Monospace)`), badge Small-role — label(13-strong) > content(12) >
// hint(11-weak) hierarchy, koi naya size/token/color nahi. Paint caller ke locals
// (`&mut String` needle + badge `&str`) par chalti hai, isliye App/viewer
// struct me koi field edit nahi hota; returns `(prev_clicked, next_clicked)`
// taaki cursor/badge advance caller ke paas rahe. No I/O; colors frozen
// (koi color touch nahi, sizes sirf TypoRole tokens se).
// ----------------------------------------------------------------------------

/// Hex find bar ki height (table precedent reuse, koi naya token nahi).
/// Test-only until paint wiring lands (orchestrator owns paint).
pub const HEX_FIND_BAR_H: f32 = 28.0;
/// Hex find needle field ki chaudai (table precedent reuse).
/// Test-only until paint wiring lands (orchestrator owns paint).
pub const HEX_FIND_NEEDLE_W: f32 = 120.0;
/// Hex find badge ka right-end pad (table precedent reuse).
/// Test-only until paint wiring lands (orchestrator owns paint).
pub const HEX_FIND_BADGE_PAD: f32 = 4.0;

/// Hex find-bar section-label role: Body (13 default proportional).
/// Paint-side `.strong()` composition (table header Mono+strong precedent):
/// section label hint/badge (Small 11-weak) se ek step upar taaki flat
/// hierarchy tute; input values Mono family me (needle field
/// `.font(Monospace)`) taaki label-vs-input confuse na ho. Koi naya
/// size/token/color nahi. Pure; unit-tested.
pub const fn hex_find_label_text_role() -> crate::theme::TypoRole {
    crate::theme::TypoRole::Body
}

/// Needle text se bytes: separators hatakar hex jode decode hota hai,
/// warna raw utf-8 bytes. Khali par khali vec. App-side
/// `parse_hex_needle` ka views_hex-local mirror hai (views_hex app par
/// depend nahi kar sakta, isliye same semantics yahin rehti hai taaki
/// agla wiring wave dono taraf ek jaisi needle samjhe). Pure; unit-tested.
/// Test-only until paint wiring lands (orchestrator owns paint).
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
/// saath Small-role typo me; label Body-strong, needle Mono family).
/// Returns `(prev_clicked, next_clicked)` —
/// cursor/badge advance caller ke locals me hota hai (App fields untouched).
/// Caller signature same rehti hai (hex tab wiring isi ko chalaye, KEVL
/// caller me koi edit nahi).
/// Wiring-pending until orchestrator wires the hex tab caller.
pub fn paint_hex_find_bar(ui: &mut egui::Ui, needle: &mut String, badge: &str) -> (bool, bool) {
    let mut prev = false;
    let mut next = false;
    let avail = ui.available_width();
    ui.horizontal_wrapped(|ui| {
        ui.set_min_size(egui::vec2(avail.max(0.0), HEX_FIND_BAR_H));
        // Dense action-row rhythm: hunk-nav row precedent (`GEO_TIGHT_GAP`
        // 4px) — needle/buttons/badge compact, ek hi gap source.
        ui.spacing_mut().item_spacing.x = crate::theme::GEO_TIGHT_GAP;
        ui.label(hex_find_label_text_role().rich("Find:").strong());
        ui.add(
            egui::TextEdit::singleline(needle)
                .desired_width(HEX_FIND_NEEDLE_W)
                .font(egui::TextStyle::Monospace)
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
            ui.label(crate::theme::TypoRole::Small.rich(badge));
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
pub fn hex_gutter_total_rows(total_len: u64) -> usize {
    hex_row_count(total_len).min(usize::MAX as u64) as usize
}

/// Hex diff rows ka gutter geometry: har `hex_row_has_diff` wali row ek
/// band hai, lagatar diff rows merge hote hain. Wahi `GutterGeometry` type,
/// wahi ruler floor (`gutter::GUTTER_SEGMENT_MIN_H` = 4.0) + `GUTTER_MERGE_GAP`
/// tokens, kind `Changed` (text `Changed` jaisa paired-diff marker).
/// Single source: [`hex_gutter_geometry_fast`] par delegate karta hai taaki
/// row-wise aur virtualized (`gutter_geometry_from_ranges`) bands kabhi
/// alag na hon — tall files me 2px vs 4px drift nahi. Pure; unit-tested.
/// Test-only until paint wiring lands (orchestrator owns paint).
pub fn hex_gutter_geometry(
    total_rows: usize,
    height: f32,
    hunks: &[HexHunkView],
) -> crate::gutter::GutterGeometry {
    hex_gutter_geometry_fast(total_rows, height, hunks)
}

/// Hex gutter bar width: hamesha wide (12.0). Text pane line numbers
/// chhupa sakta hai (narrow 6.0), par hex grid ka offset ruler hamesha
/// paint hota hai — marker strip ruler se patli ho to markers weak/click
/// mushkil hote hain, isliye flag se narrow nahi hota. Single source
/// `gutter_bar_width(true)` taaki text wide-bar se drift na ho.
/// Test-only until paint wiring lands (orchestrator owns paint).
pub fn hex_gutter_bar_width(_show_line_numbers: bool) -> f32 {
    crate::gutter::gutter_bar_width(true)
}

/// Font size se scaled gutter width: text precedent ka seedha reuse.
/// Test-only until paint wiring lands (orchestrator owns paint).
pub fn hex_gutter_bar_width_for_font(font_size: f32, show_line_numbers: bool) -> f32 {
    crate::gutter::gutter_bar_width_for_font(font_size, show_line_numbers)
}

/// Hex gutter text role: text precedent ka seedha reuse (Mono).
/// Test-only until paint wiring lands (orchestrator owns paint).
pub const fn hex_gutter_text_role() -> crate::theme::TypoRole {
    crate::gutter::gutter_text_role()
}

/// Hex gutter text size: text precedent ka seedha reuse (mono token).
/// Test-only until paint wiring lands (orchestrator owns paint).
pub const fn hex_gutter_text_size() -> f32 {
    crate::gutter::gutter_text_size()
}

/// Hex gutter line-height source: text precedent ka seedha reuse.
/// Test-only until paint wiring lands (orchestrator owns paint).
pub const fn hex_gutter_line_height_source() -> &'static str {
    crate::gutter::gutter_line_height_source()
}

/// Gutter-local y se grid row: text mapping ka seedha reuse.
/// Test-only until paint wiring lands (orchestrator owns paint).
pub fn hex_gutter_row_at_y(y: f32, total_rows: usize, height: f32) -> Option<usize> {
    crate::gutter::gutter_row_at_y(y, total_rows, height)
}

/// Gutter-local y se segment hit: text hit-test ka seedha reuse.
/// Test-only until paint wiring lands (orchestrator owns paint).
pub fn hex_gutter_segment_at_y(segments: &[crate::gutter::GutterSegment], y: f32) -> Option<usize> {
    crate::gutter::gutter_segment_at_y(segments, y)
}

/// Grid row se gutter-local y: text inverse mapping ka seedha reuse.
/// Test-only until paint wiring lands (orchestrator owns paint).
pub fn hex_gutter_y_for_row(row: usize, total_rows: usize, height: f32) -> Option<f32> {
    crate::gutter::gutter_y_for_row(row, total_rows, height)
}

/// Gutter row marker label: us row ka offset ruler text
/// (`format_hex_offset(hex_row_start(row))` reuse). Marker alignment
/// geometry se aati hai (row bands), ye sirf label text hai. Pure.
/// Test-only until paint wiring lands (orchestrator owns paint).
pub fn hex_gutter_row_label(row: u64) -> String {
    format_hex_offset(hex_row_start(row))
}

/// Hex gutter click-to-jump: y se row (`gutter_row_at_y` reuse), row se
/// covering hunk (`hex_row_to_hunk` reuse); khali row par aage ki rows
/// me pehla covering hunk, warna peeche ka aakhri. Khali hunks ya
/// out-of-range click par `None`. Pure; unit-tested.
/// Test-only until paint wiring lands (orchestrator owns paint).
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

// ----------------------------------------------------------------------------
// Hex minimap hit-rect + offset-proportional bands (headless; paint wiring
// stays with the orchestrator).
// AX2 §10.4: live paint ka 24px minimap strip (`views::HEX_MINIMAP_W`)
// gutter bar ka double hai; ye section us hit-rect ka headless judvaan
// hai — strip width gutter single-source se, band list offset-proportional
// (paint ka inline hunk loop isi mapping par aata hai), aur row->y inverse
// selection marker ke liye. Koi naya token ya color nahi; sab pure fns,
// `.get()`/saturating only, no egui, no I/O. Paint allocate rect
// `hex_minimap_strip_width` se banata hai, click y ko locked
// `hex_minimap_row_at_y` (views.rs) se row me badalta hai, bands
// `hex_minimap_bands` se nikalta hai.
// ----------------------------------------------------------------------------

/// Hex minimap strip width: gutter bar ka double (legacy 12px bar ->
/// 24px hit-rect). Single source gutter contract hai
/// (`gutter_bar_width(true)`); test paint ke `HEX_MINIMAP_W` se equality
/// lock karta hai taaki allocate rect aur hit-rect kabhi alag na hon.
/// Pure; unit-tested.
pub fn hex_minimap_strip_width() -> f32 {
    crate::gutter::gutter_bar_width(true) * 2.0
}

/// Ek hunk ka offset-proportional minimap band `(y_start, y_end)`.
/// `first = offset / 16`, `last = (offset + run - 1) / 16`, y fraction
/// se; sub-row bands ruler floor (`gutter::GUTTER_SEGMENT_MIN_H` = 4.0)
/// par taaki tall files me gutter bands aur minimap bands ek jaise dikhen
/// (text gutter + `gutter_geometry_from_ranges` parity, 2px vs 4px drift
/// nahi). Khali run, row se bahar hunk ya kharab input par `None`.
/// Pure; unit-tested.
pub fn hex_minimap_band_for_hunk(
    hunk: &HexHunkView,
    total_rows: u64,
    height: f32,
) -> Option<(f32, f32)> {
    let run = (hunk.left_len).max(hunk.right_len);
    if run == 0 || total_rows == 0 || !height.is_finite() || height <= 0.0 {
        return None;
    }
    let step = HEX_BYTES_PER_ROW as u64;
    let first = hunk.offset.saturating_div(step);
    if first >= total_rows {
        return None;
    }
    let last = hunk
        .offset
        .saturating_add(run as u64)
        .saturating_sub(1)
        .saturating_div(step)
        .min(total_rows.saturating_sub(1));
    let total_f = total_rows as f32;
    let y_start = (first as f32 / total_f) * height;
    let raw_end = ((last.saturating_add(1)) as f32 / total_f) * height;
    let floored = raw_end.max(y_start + crate::gutter::GUTTER_SEGMENT_MIN_H);
    Some((y_start, floored.min(height)))
}

/// Poori minimap band list offset order me: `total_len` se
/// `hex_row_count`, har hunk par [`hex_minimap_band_for_hunk`]. Paint ka
/// inline hunk loop isi list par aata hai (track rect + `CHANGED_BG`
/// frozen rehte hain). Khali/kharab input par khali vec.
/// Pure; unit-tested.
pub fn hex_minimap_bands(total_len: u64, height: f32, hunks: &[HexHunkView]) -> Vec<(f32, f32)> {
    let total_rows = hex_row_count(total_len);
    if total_rows == 0 || !height.is_finite() || height <= 0.0 {
        return Vec::new();
    }
    let mut out = Vec::new();
    for hunk in hunks {
        if let Some(band) = hex_minimap_band_for_hunk(hunk, total_rows, height) {
            out.push(band);
        }
    }
    out
}

/// Grid row -> minimap-local y (band top): `(row / total_rows) * height`.
/// Gutter `gutter_y_for_row` (usize) ka u64 grid judvaan; selection marker
/// paint isi par lagta hai taaki marker clicked band par baithe. Wahi
/// `None` contract. Pure; unit-tested.
pub fn hex_minimap_y_for_row(row: u64, total_rows: u64, height: f32) -> Option<f32> {
    if total_rows == 0 || !height.is_finite() || height <= 0.0 {
        return None;
    }
    if row >= total_rows {
        return None;
    }
    let y = (row as f32 / total_rows as f32) * height;
    if y.is_finite() && y >= 0.0 && y < height {
        Some(y)
    } else {
        None
    }
}

// ----------------------------------------------------------------------------
// Nibble parity cell geometry (headless; paint wiring stays with the
// orchestrator).
// AX2 §10.3: changed row poori-row paint hoti hai — kaun sa nibble badla
// geometry se pata nahi chalta. Ye section per-byte nibble diff nikalta
// hai taaki paint sirf badle cells highlight kare (poori row nahi).
// Model me byte values nahi (sirf offset/len), isliye caller slices deta
// hai; barabar bytes aur missing slots skip hote hain (saturating, kabhi
// panic nahi). Geometry first (offset/col + high/low flags), typography
// second (wahi mono role, koi naya token nahi). No egui, no I/O; colors
// frozen (koi Color32/Visuals nahi).
// ----------------------------------------------------------------------------

/// Ek badle byte ka nibble-level cell: grid column + kaun sa nibble alag
/// hai. Paint Grid cell isi par accent lagata hai (poori row par nahi).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HexNibbleCell {
    /// Absolute byte offset.
    pub offset: u64,
    /// Grid column (0..16).
    pub col: u64,
    /// High nibble alag hai.
    pub high_diff: bool,
    /// Low nibble alag hai.
    pub low_diff: bool,
}

/// Do byte values me kaun sa nibble alag hai: `(high, low)`. Barabar
/// bytes par `(false, false)`. Pure; unit-tested.
pub fn hex_nibbles_differ(left: u8, right: u8) -> (bool, bool) {
    (left >> 4 != right >> 4, left & 0x0F != right & 0x0F)
}

/// 16-byte row (`row_start` se) ke badle bytes nibble cells ke roop me,
/// offset order me. Barabar bytes aur missing slots skip hote hain
/// (saturating, kabhi panic nahi). Khali vec = is row me koi nibble antar
/// nahi — paint poori-row accent chhodkar inhi cells par lagata hai.
/// Pure; unit-tested.
pub fn hex_diff_nibble_cells(row_start: u64, left: &[u8], right: &[u8]) -> Vec<HexNibbleCell> {
    let mut out = Vec::new();
    let mut col: u64 = 0;
    while col < HEX_BYTES_PER_ROW as u64 {
        let off = row_start.saturating_add(col);
        let idx = off.min(usize::MAX as u64) as usize;
        match (left.get(idx).copied(), right.get(idx).copied()) {
            (Some(lv), Some(rv)) if lv != rv => {
                let (high_diff, low_diff) = hex_nibbles_differ(lv, rv);
                out.push(HexNibbleCell {
                    offset: off,
                    col,
                    high_diff,
                    low_diff,
                });
            }
            _ => {}
        }
        col = col.saturating_add(1);
    }
    out
}

/// Nibble cell text role: grid cells monospace paint hote hain, wahi
/// shared mono token (koi naya token nahi). Pure; unit-tested.
pub const fn hex_nibble_text_role() -> crate::theme::TypoRole {
    crate::theme::TypoRole::Mono
}

// ----------------------------------------------------------------------------
// Hex minimap pure-rect + nibble changed-accent rects (headless; paint
// wiring stays with the orchestrator).
// AX2 S10.3/S10.4 + localrun-bb P1-2: live paint ka 24px minimap strip
// (`views::HEX_MINIMAP_W` x `views::HEX_MINIMAP_H`) aur changed-nibble
// accent is file me pure `egui::Rect` math ke roop me. Tuple layer
// (`hex_minimap_bands`, `hex_diff_nibble_cells`) offset/fraction deti hai;
// ye section usko track-anchored rects me badalta hai taaki paint ka inline
// first/last/y0/y1 loop aur `??` whole-row marks isi mapping par aayen.
// Koi naya token ya color nahi: width gutter se, height paint token se,
// floor ruler `GUTTER_SEGMENT_MIN_H` (gutter parity, 4.0) se, typo `Mono` se.
// Sab pure fns, `.get()`/saturating only, no `Ui`/painter/I/O, no paint
// colors. Paint allocate rect `hex_minimap_track_rect` se
// banata hai, bands `hex_minimap_band_rects` se nikalta hai, click y ko
// `hex_minimap_row_at_pos` se row me badalta hai, changed nibbles par
// `hex_row_nibble_accent_rects` se accent rects lagata hai.
// ----------------------------------------------------------------------------

/// Minimap allocate height: paint token `views::HEX_MINIMAP_H` ka
/// headless judvaan (220px fixed strip height). Test paint token se
/// equality lock karta hai. Pure; unit-tested.
/// Test-only until paint wiring lands (orchestrator owns paint).
pub fn hex_minimap_alloc_height() -> f32 {
    220.0
}

/// Minimap allocate size: `(strip_width, alloc_height)` — paint ka
/// `allocate_exact_size(vec2(bar_w, HEX_MINIMAP_H))` isi size par aata hai.
/// Pure; unit-tested.
/// Test-only until paint wiring lands (orchestrator owns paint).
pub fn hex_minimap_alloc_size() -> egui::Vec2 {
    egui::vec2(hex_minimap_strip_width(), hex_minimap_alloc_height())
}

/// Minimap track/hit rect: `origin` se `alloc_size` tak — paint ka allocate
/// rect + click/drag sense isi rect par hai (24px hit-rect). Pure;
/// unit-tested.
/// Test-only until paint wiring lands (orchestrator owns paint).
pub fn hex_minimap_track_rect(origin: egui::Pos2) -> egui::Rect {
    egui::Rect::from_min_size(origin, hex_minimap_alloc_size())
}

/// Ek hunk ka track-anchored band rect (x me poori track chaudai).
/// `hex_minimap_band_for_hunk` fractions ko `track.min.y` par anchor karke
/// `track.max.y` par clamp karta hai; paint ka `seg` rect wahi hai
/// (`from_min_max((min.x, y0), (max.x, min(y1, max.y)))`). Zero-area ya
/// non-finite track, kharab input par `None`. Pure; unit-tested.
/// Test-only until paint wiring lands (orchestrator owns paint).
pub fn hex_minimap_band_rect(
    track: egui::Rect,
    total_rows: u64,
    hunk: &HexHunkView,
) -> Option<egui::Rect> {
    if !track.width().is_finite()
        || !track.height().is_finite()
        || track.width() <= 0.0
        || track.height() <= 0.0
    {
        return None;
    }
    let (local_start, local_end) = hex_minimap_band_for_hunk(hunk, total_rows, track.height())?;
    let y0 = track.min.y + local_start;
    let y1 = (track.min.y + local_end).min(track.max.y);
    if !y0.is_finite() || !y1.is_finite() || y1 <= y0 {
        return None;
    }
    Some(egui::Rect::from_min_max(
        egui::pos2(track.min.x, y0),
        egui::pos2(track.max.x, y1),
    ))
}

/// Poori minimap band rect list offset order me: `total_len` se
/// `hex_row_count`, har hunk par [`hex_minimap_band_rect`]. Paint ka inline
/// hunk loop isi list par aata hai (track rect + `CHANGED_BG` frozen rehte
/// hain). Kharab input par khali vec. Pure; unit-tested.
/// Test-only until paint wiring lands (orchestrator owns paint).
pub fn hex_minimap_band_rects(
    track: egui::Rect,
    total_len: u64,
    hunks: &[HexHunkView],
) -> Vec<egui::Rect> {
    let total_rows = hex_row_count(total_len);
    if total_rows == 0 {
        return Vec::new();
    }
    let mut out = Vec::new();
    for hunk in hunks {
        if let Some(rect) = hex_minimap_band_rect(track, total_rows, hunk) {
            out.push(rect);
        }
    }
    out
}

/// Track-local hit mapping: absolute `pos_y` ko grid row me badalta hai.
/// Paint path `hex_minimap_row_at_y(pos.y - rect.min.y, rect.height(),
/// total)` ka track-relative judvaan; wahi `None` contract (bahar ya
/// kharab input par `None`). Selection marker paint isi par jump karta hai.
/// Pure; unit-tested.
/// Test-only until paint wiring lands (orchestrator owns paint).
pub fn hex_minimap_row_at_pos(track: egui::Rect, total_rows: u64, pos_y: f32) -> Option<u64> {
    if total_rows == 0 || !track.height().is_finite() || track.height() <= 0.0 {
        return None;
    }
    if !pos_y.is_finite() {
        return None;
    }
    let local = pos_y - track.min.y;
    if !local.is_finite() || local < 0.0 || local >= track.height() {
        return None;
    }
    let row = ((local / track.height()) * total_rows as f32).floor() as u64;
    if row < total_rows { Some(row) } else { None }
}

/// Row rect ka ek byte-cell rect: 16 barabar columns me `col` wala hissa.
/// Grid paint row rect isi par bant-ta hai taaki accent sahi cell par lage.
/// `col >= 16`, zero-area ya non-finite row par `None`. Pure; unit-tested.
/// Test-only until paint wiring lands (orchestrator owns paint).
pub fn hex_nibble_cell_rect(row_rect: egui::Rect, col: u64) -> Option<egui::Rect> {
    if col >= HEX_BYTES_PER_ROW as u64 {
        return None;
    }
    if !row_rect.width().is_finite()
        || !row_rect.height().is_finite()
        || row_rect.width() <= 0.0
        || row_rect.height() <= 0.0
    {
        return None;
    }
    let step = row_rect.width() / HEX_BYTES_PER_ROW as f32;
    if !step.is_finite() || step <= 0.0 {
        return None;
    }
    let x0 = row_rect.min.x + col as f32 * step;
    let x1 = x0 + step;
    if !x0.is_finite() || !x1.is_finite() || x1 <= x0 {
        return None;
    }
    Some(egui::Rect::from_min_max(
        egui::pos2(x0, row_rect.min.y),
        egui::pos2(x1, row_rect.max.y),
    ))
}

/// Ek nibble cell ke accent rects: `(high, low)` — high = cell ka left
/// aadha, low = right aadha. Sirf alag nibble par `Some`, barabar par
/// `None` (paint poori-row accent chhodkar inhi halves par lagata hai).
/// Zero-area cell par `None` joda. Pure; unit-tested.
/// Test-only until paint wiring lands (orchestrator owns paint).
pub fn hex_nibble_accent_rects(
    cell_rect: egui::Rect,
    cell: &HexNibbleCell,
) -> (Option<egui::Rect>, Option<egui::Rect>) {
    if !cell_rect.width().is_finite()
        || !cell_rect.height().is_finite()
        || cell_rect.width() <= 0.0
        || cell_rect.height() <= 0.0
    {
        return (None, None);
    }
    let mid_x = cell_rect.min.x + cell_rect.width() / 2.0;
    if !mid_x.is_finite() || mid_x <= cell_rect.min.x || mid_x >= cell_rect.max.x {
        return (None, None);
    }
    let high = if cell.high_diff {
        Some(egui::Rect::from_min_max(
            egui::pos2(cell_rect.min.x, cell_rect.min.y),
            egui::pos2(mid_x, cell_rect.max.y),
        ))
    } else {
        None
    };
    let low = if cell.low_diff {
        Some(egui::Rect::from_min_max(
            egui::pos2(mid_x, cell_rect.min.y),
            egui::pos2(cell_rect.max.x, cell_rect.max.y),
        ))
    } else {
        None
    };
    (high, low)
}

/// Ek grid row ke sab changed-nibble accent rects col order me: har cell ka
/// cell-rect (`hex_nibble_cell_rect`) phir alag halves
/// (`hex_nibble_accent_rects`). Khali cells ya kharab row par khali vec —
/// paint ke paas accent karne ko kuch nahi to poori-row band bhi nahi.
/// Pure; unit-tested.
/// Test-only until paint wiring lands (orchestrator owns paint).
pub fn hex_row_nibble_accent_rects(
    row_rect: egui::Rect,
    cells: &[HexNibbleCell],
) -> Vec<egui::Rect> {
    let mut out = Vec::new();
    for cell in cells {
        let Some(cell_rect) = hex_nibble_cell_rect(row_rect, cell.col) else {
            continue;
        };
        let (high, low) = hex_nibble_accent_rects(cell_rect, cell);
        if let Some(rect) = high {
            out.push(rect);
        }
        if let Some(rect) = low {
            out.push(rect);
        }
    }
    out
}

// ----------------------------------------------------------------------------
// Hex grid text with real byte content (headless; paint wiring stays with
// the orchestrator).
// Gap localrun-bh P1-1: `views::format_hex_grid_cells` sirf offset/len se
// `??` marks banata hai; accent-rect plumbing + callsite slices pehle se
// live hain (BG-nibble), sirf grid text baaki tha. Ye section uska pure
// judvaan hai — wahi 3-cell shape (ruler address, 16-byte hex, 16-char
// ASCII), wahi midline gap, par covered byte par real hex digits (lowercase,
// fixture style) aur real ASCII (printable char, warna `.`). Caller slices
// App-owned snap se aayengi; model me byte values nahi, isliye left byte
// prefer hota hai, missing par right fallback, dono missing par blank.
// Uncovered slot blank rehta hai taaki `hex.len()` purane shape se barabar
// rahe (Grid chaudai stable). No egui, no I/O; colors frozen (koi palette
// change nahi). Paint wave `views::format_hex_grid_cells` call ko
// isi se replace karega (views.rs callsite is wave me FROZEN — yahan sirf
// pure helper + tests).
// ----------------------------------------------------------------------------

/// Kya koi displayed hunk `offset` byte ko cover karta hai.
/// Hunk range `[offset, offset + max(left_len, right_len))` hai —
/// `views::hunk_covers_offset` ka views_hex-local judvaan (views_hex views
/// par depend nahi kar sakta, isliye same semantics yahin).
/// Test-only until paint wiring lands (orchestrator owns paint).
fn hex_grid_covers_offset(offset: u64, hunks: &[HexHunkView]) -> bool {
    for hunk in hunks {
        let run = (hunk.left_len).max(hunk.right_len);
        if run == 0 {
            continue;
        }
        let end = hunk.offset.saturating_add(run as u64);
        if hunk.offset <= offset && offset < end {
            return true;
        }
    }
    false
}

/// Ek covered offset ka byte: left prefer, missing par right fallback,
/// dono missing par `None`. Insertion hunk (ek side khali) isi se honest
/// rehta hai — jo side maujood hai uske digits dikhte hain.
/// Test-only until paint wiring lands (orchestrator owns paint).
fn hex_grid_cell_byte(offset: u64, left: &[u8], right: &[u8]) -> Option<u8> {
    let idx = offset.min(usize::MAX as u64) as usize;
    if let Some(byte) = left.get(idx).copied() {
        return Some(byte);
    }
    right.get(idx).copied()
}

/// Ek covered byte ka ASCII mark: printable `0x20..=0x7E` wahi char,
/// warna `.` (fixture style). Pure; unit-tested.
/// Test-only until paint wiring lands (orchestrator owns paint).
pub fn hex_grid_ascii_mark(byte: u8) -> char {
    if (0x20..=0x7Eu8).contains(&byte) {
        byte as char
    } else {
        '.'
    }
}

/// One grid row split into three paint cells with REAL byte content:
/// ruler address, 16-byte hex digits, 16-char ASCII. Shape
/// `views::format_hex_grid_cells` jaisa (wahi midline gap, wahi trailing
/// space, wahi blank uncovered slots) — sirf covered `??` ki jagah real
/// digits aur covered `?` ki jagah real ASCII. `left` prefer, `right`
/// fallback; dono missing par blank (Grid chaudai stable). Pure;
/// unit-tested.
/// Test-only until paint wiring lands (orchestrator owns paint).
pub fn format_hex_grid_cells_with_bytes(
    row_start: u64,
    left: &[u8],
    right: &[u8],
    hunks: &[HexHunkView],
) -> (String, String, String) {
    let addr = format_hex_offset(row_start);
    let mut hex = String::new();
    let mut ascii = String::with_capacity(HEX_BYTES_PER_ROW);
    for col in 0..HEX_BYTES_PER_ROW {
        if col == HEX_BYTES_PER_ROW / 2 {
            for _ in 0..HEX_MIDLINE_GAP_CHARS {
                hex.push(' ');
            }
        }
        let off = row_start.saturating_add(col as u64);
        if hex_grid_covers_offset(off, hunks) {
            match hex_grid_cell_byte(off, left, right) {
                Some(byte) => {
                    hex.push_str(&format!("{byte:02x}"));
                    ascii.push(hex_grid_ascii_mark(byte));
                }
                None => {
                    hex.push_str("  ");
                    ascii.push(' ');
                }
            }
        } else {
            hex.push_str("  ");
            ascii.push(' ');
        }
        hex.push(' ');
    }
    (addr, hex, ascii)
}

/// One grid row of the hex byte view with real content: ruler offset plus
/// per-byte hex/ASCII for the 16-byte row at `row_start`. Shape
/// `views::format_hex_grid_row` jaisa join; Grid paint isi ko cells me
/// baant-ta hai. Back-compat join (logs/tests); Grid cells use karenge.
/// Pure; unit-tested.
/// Test-only until paint wiring lands (orchestrator owns paint).
pub fn format_hex_grid_row_with_bytes(
    row_start: u64,
    left: &[u8],
    right: &[u8],
    hunks: &[HexHunkView],
) -> String {
    let (addr, hex, ascii) = format_hex_grid_cells_with_bytes(row_start, left, right, hunks);
    format!("{addr}  {hex} |{ascii}|")
}

// ----------------------------------------------------------------------------
// Hex 64-bit goto + byte-edit (headless; paint wiring stays with the
// orchestrator).
// Gap EXQ-L hex half: hex tab me 64-bit offset jump + nibble byte-edit
// nahi hai — manual scroll se badi file me jump/edit slow hai. Ye section
// pure goto/edit helpers hai: text se u64 offset, total_len clamp, nibble
// apply, slice edit. Geometry first: goto bar find-bar geometry reuse
// (wahi bar height, wahi field chaudai, wahi badge pad — koi naya rhythm
// nahi). Typography second: label Body-strong, value Mono family, badge
// Small-role (find-bar hierarchy reuse: label(13-strong) > value(12-mono)
// > hint(11-weak)); koi naya size/token/color nahi. No I/O; colors frozen
// (koi Color32/Visuals nahi). Paint caller ke locals par chalti hai
// (`&mut String` goto text + badge `&str`), App/viewer struct me koi field
// edit nahi hota; returns `go_clicked` taaki cursor advance caller ke paas
// rahe. Unit-tested.
// ----------------------------------------------------------------------------

/// Hex goto bar height: find-bar precedent reuse (koi naya token nahi).
pub const HEX_GOTO_BAR_H: f32 = HEX_FIND_BAR_H;
/// Hex goto field chaudai: find needle precedent reuse.
pub const HEX_GOTO_FIELD_W: f32 = HEX_FIND_NEEDLE_W;
/// Hex goto badge right-end pad: find badge precedent reuse.
pub const HEX_GOTO_BADGE_PAD: f32 = HEX_FIND_BADGE_PAD;

/// Hex goto section-label role: Body (13 default proportional).
/// Paint-side `.strong()` composition (find-bar label precedent): section
/// label hint/badge (Small 11-weak) se ek step upar taaki flat hierarchy
/// tute; input values Mono family me (field `.font(Monospace)`) taaki
/// label-vs-input confuse na ho. Koi naya size/token/color nahi. Pure.
pub const fn hex_goto_label_text_role() -> crate::theme::TypoRole {
    crate::theme::TypoRole::Body
}

/// Hex goto value role: Mono (12 regular monospace, tabular).
/// Goto offset ruler text jaisa dikhta hai (`format_hex_offset` precedent),
/// isliye value field Mono me taaki label(Body) vs value confuse na ho.
/// Koi naya size/token/color nahi. Pure.
pub const fn hex_goto_value_text_role() -> crate::theme::TypoRole {
    crate::theme::TypoRole::Mono
}

/// Goto text se 64-bit offset: `0x`/`0X` prefix par hex, warna decimal.
/// Visual separators (`_`, `,`, space, tab) hat-te hain taaki `1_000`,
/// `0xFF_FF` jaise paste kaam karen. Khali/invalid/overflow par `None`
/// (kabhi panic nahi, `u64::from_str_radix` honest `None` deta hai).
/// Pure; unit-tested.
pub fn parse_hex_goto(text: &str) -> Option<u64> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    let mut compact = String::new();
    for ch in trimmed.chars() {
        if ch == '_' || ch == ',' || ch == ' ' || ch == '\t' {
            continue;
        }
        compact.push(ch);
    }
    if compact.is_empty() {
        return None;
    }
    let (digits, radix) = match compact
        .strip_prefix("0x")
        .or_else(|| compact.strip_prefix("0X"))
    {
        Some(rest) => (rest, 16),
        None => (compact.as_str(), 10),
    };
    if digits.is_empty() {
        return None;
    }
    u64::from_str_radix(digits, radix).ok()
}

/// Goto offset ko `total_len` me clamp: valid par `Some(offset)`,
/// khali input (`total_len == 0`) ya bahar (`offset >= total_len`) par
/// `None` (paint bahar jump par honest no-op dikhata hai). Pure.
pub fn clamp_hex_goto(offset: u64, total_len: u64) -> Option<u64> {
    if total_len == 0 || offset >= total_len {
        return None;
    }
    Some(offset)
}

/// Ek hex char ki nibble value (0..=15). Non-hex par `None`.
/// Pure; unit-tested.
pub fn hex_nibble_value(ch: char) -> Option<u8> {
    ch.to_digit(16).and_then(|v| u8::try_from(v).ok())
}

/// Ek byte me ek nibble lagata hai: `high` par upper 4 bits, warna lower
/// 4 bits; dusra nibble untouched rehta hai. Non-hex char par `None`
/// (paint invalid keystroke honest ignore karta hai). Pure; unit-tested.
pub fn apply_hex_nibble(byte: u8, hex: char, high: bool) -> Option<u8> {
    let v = hex_nibble_value(hex)?;
    if high {
        Some((v << 4) | (byte & 0x0F))
    } else {
        Some((byte & 0xF0) | v)
    }
}

/// Slice me ek byte likhta hai (`ByteEditBuffer::set_byte` ka headless
/// judvaan bina history ke): valid offset par `true`, OOB par `false`
/// (kabhi panic nahi — `usize::try_from` + `.get_mut()` only).
/// Caller undo/history engine buffer par rakhta hai; ye paint-preview
/// aur tests ke liye single-byte path hai. Pure.
pub fn edit_byte_at(bytes: &mut [u8], offset: u64, new_val: u8) -> bool {
    let idx = match usize::try_from(offset) {
        Ok(i) => i,
        Err(_) => return false,
    };
    match bytes.get_mut(idx) {
        Some(slot) => {
            *slot = new_val;
            true
        }
        None => false,
    }
}

/// Hex window ke andar goto row: label + offset field + Go + badge.
/// Find bar (`paint_hex_find_bar`) ka geometry/typo mirror (wahi bar
/// height, wahi field chaudai, wahi badge right-end Small-role me; label
/// Body-strong, value Mono family). Returns `go_clicked` — cursor/row
/// advance caller ke locals me hota hai (App fields untouched).
/// Wiring-pending until orchestrator wires the hex tab caller.
pub fn paint_hex_goto_bar(ui: &mut egui::Ui, goto_text: &mut String, badge: &str) -> bool {
    let mut go = false;
    let avail = ui.available_width();
    ui.horizontal_wrapped(|ui| {
        ui.set_min_size(egui::vec2(avail.max(0.0), HEX_GOTO_BAR_H));
        // Dense action-row rhythm: find-bar jaisa ek hi tight gap source.
        ui.spacing_mut().item_spacing.x = crate::theme::GEO_TIGHT_GAP;
        ui.label(hex_goto_label_text_role().rich("Goto:").strong());
        ui.add(
            egui::TextEdit::singleline(goto_text)
                .desired_width(HEX_GOTO_FIELD_W)
                .font(egui::TextStyle::Monospace)
                .hint_text("0x10 / 16"),
        );
        if ui.small_button("Go ▶").clicked() {
            go = true;
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(HEX_GOTO_BADGE_PAD);
            ui.label(crate::theme::TypoRole::Small.rich(badge));
        });
    });
    go
}

// ----------------------------------------------------------------------------
// Hex exquisite geometry wave 7 (headless; paint wiring stays with the
// orchestrator).
// Geometry FIRST: hex+ASCII sync panes share one visible-row window,
// 64-bit goto anchors to the grid ruler, the gutter virtualizes from
// hunk row-ranges (O(hunks), no per-row vec), grid rows keep a fixed
// char budget so nothing clips, and the 960x600 canvas plus a narrow
// stacked mode cover wide and wrapped windows. Typography SECOND:
// sync panes and the ruler read the shared Mono role (family/weight/
// line-height/letter-spacing carry hierarchy), section labels stay Body.
// Koi naya size/token/color nahi — widths reuse gutter/minimap/SPACE
// tokens, sizes reuse TypoRole. No egui paint, no I/O; colors frozen.
// ----------------------------------------------------------------------------

/// Hex canvas reference width (px): the 960px budget the gutter docs
/// lock (`12 + 460 + 4 + 460 + 16 + 8`). Grid + chrome never exceed it
/// side-by-side; narrower windows stack instead of clipping.
pub const HEX_CANVAS_W: f32 = 960.0;

/// Hex canvas reference height (px): rows + find/goto bars + status fit
/// without scrolling the chrome at this height on the reference window.
pub const HEX_CANVAS_H: f32 = 600.0;

/// Narrow stacking breakpoint (px): below this the hex+ASCII panes stack
/// (ASCII under hex, shared scroll) instead of crushing side-by-side.
/// Reuses the documented 720px paint fallback precedent (views.rs
/// merge-apply row), koi naya magic rhythm nahi.
pub const HEX_STACK_BREAKPOINT_W: f32 = 720.0;

/// Hex pane arrangement for the available width.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HexLayoutMode {
    /// Hex + ASCII panes side-by-side, one shared row window.
    #[default]
    SideBySide,
    /// ASCII wraps under hex, same shared row window (narrow windows).
    Stacked,
}

/// Pane arrangement for `avail_w`: finite widths at or above
/// [`HEX_STACK_BREAKPOINT_W`] stay side-by-side; narrower, zero, or
/// non-finite widths stack so panes scroll instead of clipping.
/// Pure; unit-tested.
pub fn hex_layout_mode(avail_w: f32) -> HexLayoutMode {
    if avail_w.is_finite() && avail_w >= HEX_STACK_BREAKPOINT_W {
        HexLayoutMode::SideBySide
    } else {
        HexLayoutMode::Stacked
    }
}

/// Chrome reserve beside the hex grid (px): gutter bar + minimap strip +
/// two [`crate::theme::SPACE_SM`] gaps. Paint subtracts this from `avail`
/// so the grid never slides under the ruler at 960px. Pure; unit-tested.
pub fn hex_chrome_reserve_w(show_line_numbers: bool) -> f32 {
    hex_gutter_bar_width(show_line_numbers)
        + hex_minimap_strip_width()
        + 2.0 * crate::theme::SPACE_SM
}

/// Grid width left for hex+ASCII cells (px): `avail` minus
/// [`hex_chrome_reserve_w`], floored at 0 so narrow windows scroll
/// instead of feeding negative widths into layout. Pure; unit-tested.
pub fn hex_grid_avail_w(avail_w: f32, show_line_numbers: bool) -> f32 {
    if !avail_w.is_finite() {
        return 0.0;
    }
    (avail_w - hex_chrome_reserve_w(show_line_numbers)).max(0.0)
}

/// Hex sync panes min height (px): text/merge panes floor reuse
/// (`viewmodel::TEXT_PANES_MIN_H` = 100.0) taaki dono hex panes + text
/// panes ek hi budget par fill karen — ek pane sukadkar gutter/minimap
/// math degrade nahi karta. Koi naya token nahi. Pure; unit-tested.
pub const HEX_PANES_MIN_H: f32 = crate::viewmodel::TEXT_PANES_MIN_H;

/// Hex byte-cell ka ek taraf ka horizontal padding (px): `SPACE_SM` reuse
/// (8.0, table `TABLE_CELL_PAD_X` precedent — wahi value, koi naya rhythm
/// nahi). Grid `HEX_GRID_ROW_CHARS` columns ke beech nahi; ye pad har byte
/// cell box ke andar hai (digits saans lete hain, gutter/minimap align
/// rehte hain). Pure.
pub const HEX_CELL_PAD_X: f32 = crate::theme::SPACE_SM;

/// Remaining-height budget for the hex+ASCII sync panes row: paint captures
/// `ui.available_height()` before the panes row (find/goto bars already
/// spent above) and both panes keep at least [`HEX_PANES_MIN_H`] from the
/// one shared window, so hex digits aur ASCII kabhi alag height par desync
/// nahi hote. `viewmodel::text_panes_height` delegation — do budgets kabhi
/// drift nahi karte. Pure; unit-tested.
pub fn hex_panes_height(avail_h: f32) -> f32 {
    crate::viewmodel::text_panes_height(avail_h)
}

/// Cell ka ek taraf ka horizontal padding (px). Pure; unit-tested.
pub fn hex_cell_pad_x() -> f32 {
    HEX_CELL_PAD_X
}

/// Text chaudai se outer byte-cell box chaudai (px): `text + 2 * pad`.
/// Non-finite ya negative text par pad floor (khali cell bhi click-target
/// rehta hai). Table `table_cell_outer_w` precedent reuse. Pure; unit-tested.
pub fn hex_cell_outer_w(text_w: f32) -> f32 {
    if !text_w.is_finite() || text_w < 0.0 {
        return HEX_CELL_PAD_X * 2.0;
    }
    text_w + HEX_CELL_PAD_X * 2.0
}

/// Outer byte-cell box se text chaudai (px): `(outer - 2 * pad)` floor 0.
/// `hex_cell_outer_w` ka inverse (round-trip); kharab outer par 0.
/// Pure; unit-tested.
pub fn hex_cell_text_w(outer_w: f32) -> f32 {
    if !outer_w.is_finite() {
        return 0.0;
    }
    (outer_w - HEX_CELL_PAD_X * 2.0).max(0.0)
}

/// Panes height me kitni grid rows fit hoti hain: `floor(panes_h / row_h)`.
/// Kharab ya zero `row_h` par 0 (paint honest empty dikhata hai); negative
/// ya non-finite `panes_h` par 0. Saturating, kabhi panic nahi.
/// Tail-truncate edge: aakhri partial page isi se cut hoti hai — caller
/// `hex_sync_visible_range` me clamp karta hai. Pure; unit-tested.
pub fn hex_visible_rows(panes_h: f32, row_h: f32) -> u64 {
    if !panes_h.is_finite() || !row_h.is_finite() || panes_h <= 0.0 || row_h <= 0.0 {
        return 0;
    }
    (panes_h / row_h).floor().max(0.0) as u64
}

/// Rows se panes height (px): `rows * row_h`, floor [`HEX_PANES_MIN_H`] par
/// taaki ek chhoti window me gutter/minimap math degrade na ho. Kharab
/// input par floor. `hex_visible_rows` ka inverse (round-trip ceil nahi,
/// floor parity). Pure; unit-tested.
pub fn hex_panes_height_for_rows(rows: u64, row_h: f32) -> f32 {
    if !row_h.is_finite() || row_h <= 0.0 {
        return HEX_PANES_MIN_H;
    }
    let want = rows as f32 * row_h;
    if !want.is_finite() {
        return HEX_PANES_MIN_H;
    }
    want.max(HEX_PANES_MIN_H)
}

/// Gutter marker rows: har hunk range ka pehla grid row, row order me,
/// dedup ke saath. `hex_hunk_row_ranges` reuse — paint gutter markers isi
/// list par lagata hai taaki row-wise aur virtualized bands kabhi alag na
/// hon. Khali hunks par khali vec (paint koi marker nahi dikhata).
/// Pure; unit-tested.
pub fn hex_gutter_marker_rows(hunks: &[HexHunkView], total_rows: usize) -> Vec<usize> {
    let mut out = Vec::new();
    for (start, _end) in hex_hunk_row_ranges(hunks, total_rows) {
        if out.last().copied() != Some(start) {
            out.push(start);
        }
    }
    out
}

/// Byte-cell text role: Mono grid content (ruler/sync-pane precedent).
/// Koi naya size/token nahi. Pure; unit-tested.
pub const fn hex_cell_text_role() -> crate::theme::TypoRole {
    crate::theme::TypoRole::Mono
}

/// Pane-label role: Body (find/goto bar label precedent reuse).
/// Paint-side `.strong()` composition; koi naya size/token/color nahi.
/// Pure; unit-tested.
pub const fn hex_pane_label_text_role() -> crate::theme::TypoRole {
    crate::theme::TypoRole::Body
}

/// Fixed grid-row text budget (chars): 10 ruler + 2 pad + 49 hex cells
/// (16 x 3 incl. the midline gap) + 1 pad + 16 ASCII + 2 pipes = 80.
/// Covered, uncovered, insertion and tail rows all render exactly this
/// many chars (`format_hex_grid_row_with_bytes` shape parity), so paint
/// sizes Grid columns once and no content variance ever clips.
/// Pure; unit-tested.
pub const HEX_GRID_ROW_CHARS: usize = 80;

/// Shared visible-row window for the hex + ASCII sync panes:
/// `[top, end)` clamped to `total_rows`, `end` at most one page past
/// `top`. Both panes iterate [`hex_sync_row_starts`] from this one
/// window, so hex digits and ASCII can never desync. Mirrors
/// [`HexPager::visible_range`] for in-range states; out-of-range tops
/// clamp to `total_rows` instead of overrunning. Pure; unit-tested.
pub fn hex_sync_visible_range(top_row: u64, visible_rows: u64, total_rows: u64) -> (u64, u64) {
    let per = visible_rows.max(1);
    let top = top_row.min(total_rows);
    let end = top_row.saturating_add(per).min(total_rows);
    (top, end.max(top))
}

/// Row-start offsets both sync panes paint: 16-aligned ruler labels
/// from the shared [`hex_sync_visible_range`] window. Empty window or
/// empty input yields an empty vec (paint shows the honest empty state).
/// Pure; unit-tested.
pub fn hex_sync_row_starts(top_row: u64, visible_rows: u64, total_len: u64) -> Vec<u64> {
    let total_rows = hex_row_count(total_len);
    if total_rows == 0 {
        return Vec::new();
    }
    let (start, end) = hex_sync_visible_range(top_row, visible_rows, total_rows);
    let mut out = Vec::new();
    let mut row = start;
    while row < end {
        out.push(hex_row_start(row));
        row = row.saturating_add(1);
    }
    out
}

/// 64-bit goto anchor: grid `(row, col)` for an absolute byte offset.
/// [`hex_offset_to_row_col`] reuse — the goto bar lands on the same grid
/// the ruler labels, so typed offsets and clicks share one alignment.
/// Pure; unit-tested.
pub fn hex_goto_anchor(offset: u64) -> (u64, u64) {
    hex_offset_to_row_col(offset)
}

/// Ruler row-start the goto bar scrolls to for `offset`: anchor row x 16.
/// Saturating so `u64::MAX` still names the last grid row, never wraps.
/// Pure; unit-tested.
pub fn hex_goto_row_start(offset: u64) -> u64 {
    hex_goto_anchor(offset)
        .0
        .saturating_mul(HEX_BYTES_PER_ROW as u64)
}

/// Hunk row-ranges (half-open `[first, end)`, 16-byte rows) clamped to
/// `total_rows`: the virtualized gutter input. Empty runs contribute
/// nothing; past-the-end hunks clip instead of overrunning. Pure;
/// unit-tested.
pub fn hex_hunk_row_ranges(hunks: &[HexHunkView], total_rows: usize) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let total_u64 = total_rows as u64;
    for hunk in hunks {
        let run = (hunk.left_len).max(hunk.right_len);
        if run == 0 {
            continue;
        }
        let first = hunk.offset.saturating_div(HEX_BYTES_PER_ROW as u64);
        if first >= total_u64 {
            continue;
        }
        let last = hunk
            .offset
            .saturating_add(run as u64)
            .saturating_sub(1)
            .saturating_div(HEX_BYTES_PER_ROW as u64)
            .min(total_u64.saturating_sub(1));
        let start = first.min(usize::MAX as u64) as usize;
        let stop = last.saturating_add(1).min(usize::MAX as u64) as usize;
        if stop > start {
            out.push((start, stop));
        }
    }
    out
}

/// Virtualized hex gutter geometry: hunk row-ranges straight into
/// [`crate::gutter::gutter_geometry_from_ranges`] — O(hunks), no per-row
/// vec, identical bands to [`hex_gutter_geometry`]. Kind is always
/// `Changed` (paired-diff marker, text `Changed` precedent). Pure;
/// unit-tested.
pub fn hex_gutter_geometry_fast(
    total_rows: usize,
    height: f32,
    hunks: &[HexHunkView],
) -> crate::gutter::GutterGeometry {
    let ranges = hex_hunk_row_ranges(hunks, total_rows);
    let mut tagged: Vec<(usize, usize, crate::viewmodel::TextRowKind)> = Vec::new();
    for (start, end) in ranges {
        tagged.push((start, end, crate::viewmodel::TextRowKind::Changed));
    }
    crate::gutter::gutter_geometry_from_ranges(total_rows, height, &tagged)
}

/// Sync-pane text role: Mono grid content (12 regular monospace,
/// tabular) — hex digits and ASCII paint in the same family as the
/// ruler so the three cells read as one row. Koi naya token nahi.
/// Pure; unit-tested.
pub const fn hex_sync_pane_text_role() -> crate::theme::TypoRole {
    crate::theme::TypoRole::Mono
}

/// Ruler text role: Mono grid content — offset labels match the cells
/// they head (BC hierarchy: ruler text matches diff-body mono).
/// Pure; unit-tested.
pub const fn hex_ruler_text_role() -> crate::theme::TypoRole {
    crate::theme::TypoRole::Mono
}

// ----------------------------------------------------------------------------
// Hex exquisite geometry wave hextable3 (headless; paint wiring stays with
// the orchestrator).
// Gap BC hex half: sync hex+ASCII panes need a locked width split (hex
// digits pane vs ASCII pane plus one gutter), one shared row height, and
// a goto bar that never clips at 960px. Geometry FIRST: char budgets come
// from `HEX_GRID_ROW_CHARS` (80 = 10 ruler + 2 pad + 49 hex + 1 pad +
// 16 ASCII + 2 pipes), widths reuse `hex_cell_outer_w` + `SPACE_SM`
// gutter, row height reuses `TypoRole::Mono.line_height_px` (no new
// size). Typography SECOND: row/pane/ruler all Mono (single role, no new
// sizes). Colors frozen (koi Color32/Visuals nahi). No egui paint, no I/O.
// ----------------------------------------------------------------------------

/// Ruler address chars (`0x00000000` = 10). Pure.
pub const HEX_RULER_CHARS: usize = 10;
/// Hex digits pane chars per row: 16 bytes x 3 incl. spaces + midline gap.
pub const HEX_HEX_PANE_CHARS: usize = 49;
/// ASCII pane chars per row: 16 one-char marks. Pure.
pub const HEX_ASCII_PANE_CHARS: usize = 16;

/// Hex digits pane aur ASCII pane ke beech ka gutter (px): `SPACE_SM`
/// reuse (table `TABLE_CELL_PAD_X` precedent — wahi value, koi naya
/// rhythm nahi). Paint sync row isi gap se baant-ta hai. Pure.
pub fn hex_ascii_gutter_w() -> f32 {
    crate::theme::SPACE_SM
}

/// Grid row height (px): Mono role ka line-height (12.0 x 1.30 = 15.6).
/// `TypoRole::Mono.line_height_px` reuse — koi naya size/token nahi.
/// Paint hex rows + ASCII rows + ruler isi height par hain taaki sync
/// panes kabhi desync na hon. Pure; unit-tested.
pub fn hex_grid_row_h() -> f32 {
    crate::theme::TypoRole::Mono.line_height_px()
}

/// Grid row text role: sync-pane Mono reuse (ruler/cells ek hi family).
/// Koi naya size/token nahi. Pure.
pub const fn hex_row_text_role() -> crate::theme::TypoRole {
    crate::theme::TypoRole::Mono
}

/// Pane text widths (px): `(hex_text_w, ascii_text_w)` = chars x char_w.
/// Kharab `char_w` par `None` (paint honest fallback dikhata hai).
/// Pure; unit-tested.
pub fn hex_pane_text_widths(char_w: f32) -> Option<(f32, f32)> {
    if !char_w.is_finite() || char_w <= 0.0 {
        return None;
    }
    let hex_w = (HEX_HEX_PANE_CHARS as f32) * char_w;
    let ascii_w = (HEX_ASCII_PANE_CHARS as f32) * char_w;
    if !hex_w.is_finite() || !ascii_w.is_finite() {
        return None;
    }
    Some((hex_w, ascii_w))
}

/// Pane outer widths (px): text + `HEX_CELL_PAD_X` dono taraf
/// (`hex_cell_outer_w` reuse). Kharab `char_w` par `None`. Pure.
pub fn hex_pane_outer_widths(char_w: f32) -> Option<(f32, f32)> {
    let (hex_text, ascii_text) = hex_pane_text_widths(char_w)?;
    Some((hex_cell_outer_w(hex_text), hex_cell_outer_w(ascii_text)))
}

/// Kya dono sync panes `grid_avail_w` me fit hote hain (outers + ek
/// gutter). Kharab input par false (paint stacked mode dikhata hai).
/// Pure; unit-tested.
pub fn hex_panes_fit(grid_avail_w: f32, char_w: f32) -> bool {
    if !grid_avail_w.is_finite() || grid_avail_w <= 0.0 {
        return false;
    }
    let Some((hex_outer, ascii_outer)) = hex_pane_outer_widths(char_w) else {
        return false;
    };
    let total = hex_outer + hex_ascii_gutter_w() + ascii_outer;
    total.is_finite() && total <= grid_avail_w
}

/// Goto bar min width (px): field + badge pad + label/button gaps
/// (`SPACE_SM` x 2 reuse, koi naya rhythm nahi). Paint goto row isi se
/// decide karta hai ki 960px canvas me clip hoga ya nahi. Pure.
pub fn hex_goto_bar_min_w() -> f32 {
    HEX_GOTO_FIELD_W + HEX_GOTO_BADGE_PAD + 2.0 * crate::theme::SPACE_SM
}

// ----------------------------------------------------------------------------
// Hex exquisite geometry wave hexview10: scrollable sync panes plus ruler
// (paint; caller wiring orchestrator ke paas, is file me paint fns +
// headless ruler helpers).
// Gap BC hex half: BC hex compare scrollable sync hex+ASCII panes with ruler
// dikhata hai — ek taraf hex digits, dusri taraf ASCII, dono ek hi row
// window par scroll hote hain, upar offset ruler. Ours me sirf hunk summary
// + sparse `??` grid tha. Ye wave panes + ruler paint karta hai: ruler header
// (offset + 00..0F + ASCII), dono panes ek hi ScrollArea me (ek scroll = dono
// sync), narrow <720px par stacked (ASCII hex ke neeche, wahi shared scroll).
// Geometry FIRST: ruler/body ek hi Grid spacing
// (`GEO_SECTION_GAP` x, `GEO_TIGHT_GAP` y — `paint_hex` body precedent),
// header shapes body cells se char-barabar (10/49/18) taaki alag Grids hone
// par bhi columns align rahen, panes height `hex_panes_height` reuse, row
// height `hex_grid_row_h` reuse (virtualized `show_rows`), stacked breakpoint
// `HEX_STACK_BREAKPOINT_W` reuse, beech me `GEO_TIGHT_GAP`. Koi naya token,
// naya rhythm, naya width nahi.
// Typography SECOND: ruler + dono panes + stacked lines sab Mono
// (`hex_row_text_role` reuse), empty note Small — label(13-strong) >
// content(12-mono) > hint(11) hierarchy, koi naya size nahi.
// Colors FROZEN: koi Color32/Visuals/painter nahi — sirf labels.
// Caller signature same rehti hai (slices + hunks caller ke paas, App/viewer
// struct me koi field edit nahi). Wiring-pending until orchestrator wires
// the hex tab caller.
// ----------------------------------------------------------------------------

/// Ruler offset-column header text: `offset` + 4 pad = 10 chars, body ruler
/// address (`0x00000000` = 10) se char-barabar taaki ruler Grid aur body Grid
/// alag hone par bhi offset column align rahe (Mono fixed width). Pure.
pub fn hex_ruler_offset_header() -> String {
    String::from("offset    ")
}

/// Ruler me ek hex column ka label (`00`..`0F`, uppercase, 2 chars).
/// `col >= 16` par `None` (kabhi panic nahi). Pure; unit-tested.
pub fn hex_ruler_col_label(col: u64) -> Option<String> {
    if col >= HEX_BYTES_PER_ROW as u64 {
        return None;
    }
    Some(format!("{col:02X}"))
}

/// Ruler hex-digits header: 16 column labels body hex cell shape me (midline
/// gap samet 49 chars = `HEX_HEX_PANE_CHARS`), taaki ruler Grid column body
/// Grid column se char-barabar ho. Pure; unit-tested.
pub fn hex_ruler_hex_header() -> String {
    let mut out = String::new();
    let mut col: u64 = 0;
    while col < HEX_BYTES_PER_ROW as u64 {
        if col == HEX_BYTES_PER_ROW as u64 / 2 {
            for _ in 0..HEX_MIDLINE_GAP_CHARS {
                out.push(' ');
            }
        }
        let Some(label) = hex_ruler_col_label(col) else {
            break;
        };
        out.push_str(&label);
        out.push(' ');
        col = col.saturating_add(1);
    }
    out
}

/// Ruler ASCII header: 16 column nibbles body ASCII cell shape me
/// (`|...|` samet 18 chars), taaki ruler Grid column body se barabar ho.
/// Pure; unit-tested.
pub fn hex_ruler_ascii_header() -> String {
    String::from("|0123456789ABCDEF|")
}

/// Ruler header row: offset + hex + ASCII, ek hi Grid me body jaisi spacing
/// (`GEO_SECTION_GAP` x, `GEO_TIGHT_GAP` y) taaki header columns body columns
/// par baithe. Header shapes body cells se char-barabar (10/49/18), isliye
/// alag Grid hone par bhi align rehta hai; header scroll ke bahar fixed
/// rehta hai. Mono role (`hex_row_text_role`), koi naya size/color nahi.
/// Wiring-pending until orchestrator wires the hex tab caller.
pub fn paint_hex_ruler(ui: &mut egui::Ui) {
    egui::Grid::new("hex_ruler_grid")
        .num_columns(3)
        .spacing([hex_ruler_grid_gap_x(), hex_ruler_grid_gap_y()])
        .show(ui, |ui| {
            ui.label(hex_row_text_role().rich(hex_ruler_offset_header()));
            ui.label(hex_row_text_role().rich(hex_ruler_hex_header()));
            ui.label(hex_row_text_role().rich(hex_ruler_ascii_header()));
            ui.end_row();
        });
}

/// Scrollable sync hex+ASCII panes: dono panes ek hi vertical ScrollArea me,
/// isliye ek scroll dono ko ek saath le jata hai (desync nahi). Wide par
/// 3-column Grid (address | hex | ASCII — body precedent, real bytes
/// `format_hex_grid_cells_with_bytes` se); narrow (`hex_layout_mode`
/// Stacked, <720px) par har grid row do visual lines me (hex line, phir ASCII
/// line, wahi address ke saath) — wahi shared scroll, crush nahi. Virtualized
/// `show_rows` (`hex_grid_row_h`) taaki lambi file me sirf dikhne wali rows
/// banein. Khali window par honest Small note. Mono role, koi naya
/// size/color nahi. Slices caller-owned snap se (App fields untouched).
/// Wiring-pending until orchestrator wires the hex tab caller.
pub fn paint_hex_sync_panes(
    ui: &mut egui::Ui,
    row_starts: &[u64],
    left: &[u8],
    right: &[u8],
    hunks: &[HexHunkView],
) {
    if row_starts.is_empty() {
        ui.label(crate::theme::TypoRole::Small.rich("(no rows in window)"));
        return;
    }
    let avail_w = ui.available_width();
    let mode = hex_layout_mode(avail_w);
    let panes_h = hex_panes_height(ui.available_height());
    let row_h = hex_grid_row_h();
    match mode {
        HexLayoutMode::SideBySide => {
            egui::ScrollArea::vertical()
                .id_salt("hex_sync_panes")
                .auto_shrink([false, false])
                .max_height(panes_h)
                .show_rows(ui, row_h, row_starts.len(), |ui, range| {
                    egui::Grid::new("hex_sync_grid")
                        .num_columns(3)
                        .spacing([hex_ruler_grid_gap_x(), hex_ruler_grid_gap_y()])
                        .show(ui, |ui| {
                            let mut pos = range.start;
                            while pos < range.end {
                                let Some(off) = row_starts.get(pos).copied() else {
                                    pos = pos.saturating_add(1);
                                    continue;
                                };
                                let (addr, hex, ascii) =
                                    format_hex_grid_cells_with_bytes(off, left, right, hunks);
                                ui.label(hex_row_text_role().rich(addr));
                                ui.label(hex_row_text_role().rich(hex));
                                ui.label(hex_row_text_role().rich(format!("|{ascii}|")));
                                ui.end_row();
                                pos = pos.saturating_add(1);
                            }
                        });
                });
        }
        HexLayoutMode::Stacked => {
            let count = row_starts.len().saturating_mul(2);
            egui::ScrollArea::vertical()
                .id_salt("hex_sync_panes_stacked")
                .auto_shrink([false, false])
                .max_height(panes_h)
                .show_rows(ui, row_h, count, |ui, range| {
                    let mut pos = range.start;
                    while pos < range.end {
                        let grid = pos / 2;
                        let half = pos % 2;
                        let Some(off) = row_starts.get(grid).copied() else {
                            pos = pos.saturating_add(1);
                            continue;
                        };
                        let (addr, hex, ascii) =
                            format_hex_grid_cells_with_bytes(off, left, right, hunks);
                        if half == 0 {
                            ui.label(hex_row_text_role().rich(format!("{addr}  {hex}")));
                        } else {
                            ui.label(hex_row_text_role().rich(format!("{addr}  |{ascii}|")));
                        }
                        pos = pos.saturating_add(1);
                    }
                });
        }
    }
}

/// Hex window ke andar panes-plus-ruler block: ruler header upar fixed, ek
/// tight gap, phir scrollable sync panes. Dono callee fns ka geometry/typo
/// mirror (wahi spacing, wahi Mono roles); yahan koi naya token/color nahi.
/// Slices + hunks caller-owned (App fields untouched).
/// Wiring-pending until orchestrator wires the hex tab caller.
pub fn paint_hex_panes_plus_ruler(
    ui: &mut egui::Ui,
    row_starts: &[u64],
    left: &[u8],
    right: &[u8],
    hunks: &[HexHunkView],
) {
    paint_hex_ruler(ui);
    ui.add_space(hex_ruler_grid_gap_y());
    paint_hex_sync_panes(ui, row_starts, left, right, hunks);
}

// ----------------------------------------------------------------------------
// Hex exquisite geometry wave hextable10: byte-grid cells + gutter alignment.
// Gap BC hex half: BC hex compare view has exact column alignment between the
// overview gutter / minimap strip, the offset ruler (10 chars), the 16 byte
// cells (with a midline gap between byte 7 and 8), and the ASCII pane.
// In narrow windows (<720px breakpoint), layout stacks into 2 visual lines
// per grid row (hex line + ASCII line) while keeping the gutter alignment intact.
// Geometry FIRST:
// - Byte cells: 16 columns (0..15). At char_w, byte cells 0..7 start at col*3*char_w;
//   byte cells 8..15 start at (col*3 + 1)*char_w due to the 1-char midline gap.
//   Cell text width = 2*char_w; outer box width = hex_cell_outer_w(2*char_w)
//   reusing HEX_CELL_PAD_X (SPACE_SM, 8.0px).
// - ASCII cells: chars 0..15 inside |...| start at (1 + col)*char_w.
// - Gutter alignment: hex grid origin x starts immediately after chrome reserve
//   (hex_chrome_reserve_w = 52.0px without line numbers, 12 gutter + 24 minimap + 2x8 gaps).
//   Grid row y positions align with gutter markers: row * hex_grid_row_h() (15.6px).
// - Stacked mode: 2 visual rows per grid row (hex_stacked_visual_rows); row height
//   hex_grid_row_h() reused.
// Typography SECOND:
// - All byte-grid cells use TypoRole::Mono (hex_cell_text_role reuse).
// - No new point sizes or typography tokens.
// Colors FROZEN: zero Color32/Visuals modifications.
// ----------------------------------------------------------------------------

/// Byte column x-offset within the hex bytes pane: 16 bytes per row (col 0..15).
/// Columns 0..7 are spaced 3 chars apart (`XX `). At column 8, the shared
/// nibble group gap derived from [`hex_nibble_group_gap_w`] separates the two
/// 8-byte nibble groups (`XX  XX`), so columns 8..15 start at
/// `col * 3 * char_w + hex_nibble_group_gap_w(char_w)`. Out of range or non-finite
/// `char_w` returns `None`. Pure; unit-tested.
pub fn hex_grid_byte_col_x(col: usize, char_w: f32) -> Option<f32> {
    if col >= HEX_BYTES_PER_ROW || !char_w.is_finite() || char_w <= 0.0 {
        return None;
    }
    let base_x = (col.saturating_mul(3) as f32) * char_w;
    let x = if col < HEX_BYTES_PER_ROW / 2 {
        base_x
    } else {
        base_x + hex_nibble_group_gap_w(char_w)
    };
    if !x.is_finite() {
        return None;
    }
    Some(x)
}

/// Text width of an individual 2-digit hex byte cell (px): `2.0 * char_w`.
/// Non-finite or negative `char_w` returns 0.0. Pure; unit-tested.
pub fn hex_grid_byte_cell_text_w(char_w: f32) -> f32 {
    if !char_w.is_finite() || char_w <= 0.0 {
        return 0.0;
    }
    2.0 * char_w
}

/// Outer box width of an individual hex byte cell (px): text width plus two
/// padding sides ([`hex_cell_pad_x`]). Reuses [`hex_cell_outer_w`]. Pure; unit-tested.
pub fn hex_grid_byte_cell_box_w(char_w: f32) -> f32 {
    hex_cell_outer_w(hex_grid_byte_cell_text_w(char_w))
}

/// ASCII character x-offset within the ASCII pane (`|0123456789ABCDEF|`).
/// Column 0..15 sits after the opening pipe delimiter at `(1 + col) * char_w`.
/// Out of range or non-finite `char_w` returns `None`. Pure; unit-tested.
pub fn hex_grid_ascii_col_x(col: usize, char_w: f32) -> Option<f32> {
    if col >= HEX_BYTES_PER_ROW || !char_w.is_finite() || char_w <= 0.0 {
        return None;
    }
    let char_offset = col.saturating_add(1);
    let x = (char_offset as f32) * char_w;
    if !x.is_finite() {
        return None;
    }
    Some(x)
}

/// Grid horizontal origin x (px): begins immediately adjacent to the gutter and
/// overview minimap chrome reserve ([`hex_chrome_reserve_w`]). Pure; unit-tested.
pub fn hex_grid_origin_x(show_line_numbers: bool) -> f32 {
    hex_chrome_reserve_w(show_line_numbers)
}

/// Grid row y-offset (px) for a given grid row index: `row * hex_grid_row_h()`.
/// Vertical rhythm single source from [`hex_grid_row_h`] (15.6px). Pure; unit-tested.
pub fn hex_grid_row_y(row: usize) -> f32 {
    (row as f32) * hex_grid_row_h()
}

/// Stacked layout visual row count: each logical grid row produces 2 visual lines
/// (hex line, then ASCII line). Pure; unit-tested.
pub fn hex_stacked_visual_rows(grid_rows: usize) -> usize {
    grid_rows.saturating_mul(2)
}

/// Stacked layout visual row y-offset (px): `(grid_row * 2 + is_ascii) * row_h`.
/// Pure; unit-tested.
pub fn hex_stacked_row_y(grid_row: usize, is_ascii_line: bool) -> f32 {
    let visual_idx = grid_row
        .saturating_mul(2)
        .saturating_add(if is_ascii_line { 1 } else { 0 });
    (visual_idx as f32) * hex_grid_row_h()
}

/// Checks whether stacked lines fit within available width at a given `char_w`.
/// Hex line is 61 chars (`10 addr + 2 pad + 49 hex`), ASCII line is 30 chars.
/// Must fit inside `hex_grid_avail_w(avail_w, false)`. Pure; unit-tested.
pub fn hex_stacked_panes_fit(avail_w: f32, char_w: f32) -> bool {
    if !avail_w.is_finite() || !char_w.is_finite() || avail_w <= 0.0 || char_w <= 0.0 {
        return false;
    }
    let avail = hex_grid_avail_w(avail_w, false);
    let needed = 61.0 * char_w;
    needed <= avail
}

// ----------------------------------------------------------------------------
// Hex exquisite geometry wave hex2 round-2: grid origin + ruler ticks
// (headless; paint wiring stays with the orchestrator).
// Gap after-hex.png round-1: panes read cramped vs BC — the byte grid
// origin floated one gap off the gutter/minimap chrome, ruler ticks had no
// shared tick spacing, the ruler row height was not locked to the Mono
// line-height, and the nibble-group midline gap was an inline `+1`.
// Geometry FIRST: grid origin (x = chrome reserve, y = ruler + one tight
// gap), ruler Grid gaps reuse `GEO_SECTION_GAP`/`GEO_TIGHT_GAP` (SPACE
// tokens, no new rhythm), row height reuses `TypoRole::Mono.line_height_px`
// (no new size), midline gap is one shared char (`HEX_MIDLINE_GAP_CHARS`).
// Typography SECOND: grid + ruler stay Mono (`hex_row_text_role` reuse),
// labels stay Body — koi naya size/token nahi. Colors FROZEN: koi
// Color32/Visuals nahi. No egui paint math here except existing paint fns
// adopting the gap single-sources below; helpers pure, saturating only.
// ----------------------------------------------------------------------------

/// Nibble-group midline gap between byte 7 and 8 (chars): one blank column
/// (`XX  XX`), so post-gap byte columns shift by one char. Single source
/// for [`hex_grid_byte_col_x`] and [`hex_ruler_hex_header`]. Pure.
pub const HEX_MIDLINE_GAP_CHARS: usize = 1;

/// Midline gap width (px) at `char_w`: `HEX_MIDLINE_GAP_CHARS * char_w`.
/// Non-finite or non-positive `char_w` yields 0.0 (paint honest fallback).
/// Pure; unit-tested.
pub fn hex_midline_gap_w(char_w: f32) -> f32 {
    if !char_w.is_finite() || char_w <= 0.0 {
        return 0.0;
    }
    let w = (HEX_MIDLINE_GAP_CHARS as f32) * char_w;
    if !w.is_finite() || w <= 0.0 {
        return 0.0;
    }
    w
}

/// Ruler/body Grid horizontal tick spacing (px): one section gap
/// (`GEO_SECTION_GAP` = `SPACE_SM`, 8.0). Single source so the ruler header
/// Grid and the body sync Grid never drift. Pure; unit-tested.
pub fn hex_ruler_grid_gap_x() -> f32 {
    crate::theme::GEO_SECTION_GAP
}

/// Ruler/body Grid vertical tick spacing (px): one tight gap
/// (`GEO_TIGHT_GAP` = `SPACE_XS`, 4.0). Single source for Grid row gaps
/// plus the ruler-to-body gap. Pure; unit-tested.
pub fn hex_ruler_grid_gap_y() -> f32 {
    crate::theme::GEO_TIGHT_GAP
}

/// Ruler row height (px): the Mono line-height, same single source as the
/// body rows ([`hex_grid_row_h`]) so ruler ticks and byte rows share one
/// vertical rhythm and never desync. Pure; unit-tested.
pub fn hex_ruler_row_h() -> f32 {
    hex_grid_row_h()
}

/// Grid vertical origin y (px): ruler row plus one tight gap — the byte
/// grid starts exactly below the fixed ruler header. Pure; unit-tested.
pub fn hex_grid_origin_y() -> f32 {
    hex_ruler_row_h() + hex_ruler_grid_gap_y()
}

/// Byte grid origin (px): `(origin_x, origin_y)` — x adjacent to the
/// gutter/minimap chrome reserve, y below the ruler header. Paint anchors
/// the first byte row here; gutter markers (`row * row_h`) share the same
/// row height so markers stay aligned. Pure; unit-tested.
pub fn hex_grid_origin(show_line_numbers: bool) -> (f32, f32) {
    (hex_grid_origin_x(show_line_numbers), hex_grid_origin_y())
}

// ----------------------------------------------------------------------------
// Hex exquisite geometry wave hex3 round-3: ruler tick spacing + grid at 960px.
// Gap after-hex.png round-2: ruler ticks still uneven vs BC — ruler tick marks
// lacked explicit symmetric SPACE token spacing, the nibble-group gap was
// inline math instead of single-sourced from the midline helper, and the row
// height was not formally locked to the Mono line-height across the 960px
// canvas budget.
// Geometry FIRST:
// - Ruler tick spacing symmetric via SPACE tokens: `hex_ruler_tick_spacing()`
//   uses `GEO_SECTION_GAP` (`SPACE_SM`, 8.0px), `hex_ruler_tick_sym_pad()` uses
//   `GEO_TIGHT_GAP` (`SPACE_XS`, 4.0px).
// - Nibble group gap: `hex_nibble_group_gap_w()` delegates directly to the
//   `hex_midline_gap_w()` midline helper so both grid columns and ruler columns
//   match pixel-for-pixel at `(col * 3) * char_w + hex_nibble_group_gap_w(char_w)`.
// - Row height locked at 960px wide: `hex_wide_row_h()` locked to `TypoRole::Mono.line_height_px()`
//   (15.6px) at `HEX_CANVAS_W` (960.0px), yielding exactly 30 visible rows in
//   the 470px panes budget without clipping or desync.
// Typography SECOND: grid + ruler stay Mono (`hex_row_text_role` reuse),
// labels stay Body — koi naya size/token nahi. Colors FROZEN: zero modifications.
// ----------------------------------------------------------------------------

/// Nibble group gap width (px) at `char_w`: single source derived from
/// [`hex_midline_gap_w`]. Separates the two 8-byte nibble groups (col 0..7
/// and col 8..15) symmetrically in both byte grid and ruler header. Pure; unit-tested.
pub fn hex_nibble_group_gap_w(char_w: f32) -> f32 {
    hex_midline_gap_w(char_w)
}

/// Ruler tick symmetric spacing (px): one section gap (`GEO_SECTION_GAP` = `SPACE_SM`, 8.0px).
/// Reuses SPACE tokens so ruler tick marks maintain symmetric rhythm across columns.
/// Pure; unit-tested.
pub fn hex_ruler_tick_spacing() -> f32 {
    crate::theme::GEO_SECTION_GAP
}

/// Ruler tick symmetric padding (px): one tight gap (`GEO_TIGHT_GAP` = `SPACE_XS`, 4.0px).
/// Centers tick marks with symmetric clearance. Pure; unit-tested.
pub fn hex_ruler_tick_sym_pad() -> f32 {
    crate::theme::GEO_TIGHT_GAP
}

/// Ruler column x-offset within the ruler hex header: 16 column labels (col 0..15).
/// Columns 0..7 start at `col * 3 * char_w`. At column 8, the shared nibble group gap
/// derived from the midline helper shifts the second 8-byte group by `hex_nibble_group_gap_w(char_w)`.
/// Matches [`hex_grid_byte_col_x`] pixel-for-pixel so ruler ticks align above byte cells.
/// Pure; unit-tested.
pub fn hex_ruler_col_x(col: usize, char_w: f32) -> Option<f32> {
    if col >= HEX_BYTES_PER_ROW || !char_w.is_finite() || char_w <= 0.0 {
        return None;
    }
    let base_x = (col.saturating_mul(3) as f32) * char_w;
    let x = if col < HEX_BYTES_PER_ROW / 2 {
        base_x
    } else {
        base_x + hex_nibble_group_gap_w(char_w)
    };
    if !x.is_finite() {
        return None;
    }
    Some(x)
}

/// Hex row height at reference 960px canvas width (px): locked to [`crate::theme::TypoRole::Mono`]
/// line-height (15.6px). Single source ensures grid rows and ruler header never desync.
/// Pure; unit-tested.
pub fn hex_wide_row_h() -> f32 {
    hex_grid_row_h()
}

/// Hex grid visible rows count at reference 960x600 canvas:
/// `panes_h = hex_panes_height(600.0)` (470.0px), divided by `hex_wide_row_h()` (15.6px),
/// yielding 30 fully visible rows without vertical clipping. Pure; unit-tested.
pub fn hex_wide_960_visible_rows() -> u64 {
    hex_visible_rows(470.0, hex_wide_row_h())
}

// ----------------------------------------------------------------------------
// Hex exquisite geometry wave hex4 round-1: even byte grid plus offset
// (headless; paint wiring stays with the orchestrator).
// Gap BC hex half: BC hex compare has an even byte grid (every byte cell
// the same width), an offset gutter aligned to the ruler, and an ASCII
// pane in sync; ours had uneven gaps (midline math inline, gutter pads
// ad-hoc). Geometry FIRST:
// - Row height locked to `TypoRole::Mono.line_height_px` (15.6px):
//   `hex_even_row_h` / `hex_offset_row_h` both delegate to
//   `hex_grid_row_h` so byte rows, ruler rows and offset-gutter rows
//   share one vertical rhythm at 960px wide.
// - Byte col widths from char metrics: `hex_byte_col_step_w` (3 chars)
//   is the even step, `hex_offset_gutter_text_w` (10 ruler chars) and
//   ASCII cols (1 char) come from the same `char_w`, so columns never
//   drift per-pane.
// - Offset gutter from SPACE tokens: `hex_offset_gutter_pad` reuses
//   `GEO_TIGHT_GAP` (`SPACE_XS`, 4.0px), `hex_offset_gutter_gap` reuses
//   `GEO_SECTION_GAP` (`SPACE_SM`, 8.0px); the gutter box is text plus
//   symmetric pads, the gutter-to-grid gap is the ruler Grid gap.
// Typography SECOND: gutter + grid + ASCII all Mono
// (`hex_offset_text_role` reuse), labels stay Body — koi naya size/token
// nahi. Colors FROZEN: koi Color32/Visuals nahi. No egui paint, no I/O.
// ----------------------------------------------------------------------------

/// Offset-gutter inner pad per side (px): one tight gap (`SPACE_XS`, 4.0).
/// Single source so the offset box never drifts from ruler Grid gaps.
/// Pure; unit-tested.
pub fn hex_offset_gutter_pad() -> f32 {
    crate::theme::GEO_TIGHT_GAP
}

/// Offset-gutter to byte-grid gap (px): one section gap (`SPACE_SM`, 8.0).
/// Same single source as the ruler/body Grid gap (`hex_ruler_grid_gap_x`).
/// Pure; unit-tested.
pub fn hex_offset_gutter_gap() -> f32 {
    crate::theme::GEO_SECTION_GAP
}

/// Offset-gutter text width (px): 10 ruler chars x `char_w`.
/// Bad `char_w` yields 0.0 (paint honest fallback). Pure; unit-tested.
pub fn hex_offset_gutter_text_w(char_w: f32) -> f32 {
    if !char_w.is_finite() || char_w <= 0.0 {
        return 0.0;
    }
    let w = (HEX_RULER_CHARS as f32) * char_w;
    if !w.is_finite() || w <= 0.0 {
        return 0.0;
    }
    w
}

/// Offset-gutter outer box width (px): text plus symmetric SPACE pads.
/// Bad `char_w` keeps the pad floor so the click target never vanishes.
/// Pure; unit-tested.
pub fn hex_offset_gutter_w(char_w: f32) -> f32 {
    hex_offset_gutter_text_w(char_w) + 2.0 * hex_offset_gutter_pad()
}

/// Even byte-col step (px): 3 chars (`XX `) x `char_w`.
/// Every byte cell 0..15 starts on a multiple of this step except the
/// shared one-char midline shift at col 8. Bad `char_w` yields 0.0.
/// Pure; unit-tested.
pub fn hex_byte_col_step_w(char_w: f32) -> f32 {
    if !char_w.is_finite() || char_w <= 0.0 {
        return 0.0;
    }
    let w = 3.0 * char_w;
    if !w.is_finite() || w <= 0.0 {
        return 0.0;
    }
    w
}

/// Even grid row height (px): `hex_grid_row_h` reuse (Mono 15.6).
/// Byte rows share one rhythm so gaps stay even vertically. Pure.
pub fn hex_even_row_h() -> f32 {
    hex_grid_row_h()
}

/// Offset-gutter row height (px): same single source, so the offset
/// gutter never desyncs from byte rows. Pure; unit-tested.
pub fn hex_offset_row_h() -> f32 {
    hex_grid_row_h()
}

/// Offset-gutter + grid text role: Mono (ruler/cells precedent reuse).
/// Koi naya size/token nahi. Pure.
pub const fn hex_offset_text_role() -> crate::theme::TypoRole {
    crate::theme::TypoRole::Mono
}

/// Even-grid check at `char_w`: col 0 at 0, cols 1..7 step evenly by
/// `hex_byte_col_step_w`, col 8 shifted by exactly one midline char,
/// cols 8..15 step evenly again, and ruler cols match byte cols.
/// Bad `char_w` returns false. Pure; unit-tested.
pub fn hex_even_grid_ok(char_w: f32) -> bool {
    if !char_w.is_finite() || char_w <= 0.0 {
        return false;
    }
    let step = hex_byte_col_step_w(char_w);
    if !step.is_finite() || step <= 0.0 {
        return false;
    }
    let gap = hex_nibble_group_gap_w(char_w);
    if !gap.is_finite() || gap <= 0.0 {
        return false;
    }
    if (gap - char_w).abs() > 0.001 {
        return false;
    }
    let mut col: usize = 0;
    while col < HEX_BYTES_PER_ROW {
        let Some(x) = hex_grid_byte_col_x(col, char_w) else {
            return false;
        };
        let Some(rx) = hex_ruler_col_x(col, char_w) else {
            return false;
        };
        if (x - rx).abs() > 0.001 {
            return false;
        }
        let want = if col < HEX_BYTES_PER_ROW / 2 {
            (col as f32) * step
        } else {
            (col as f32) * step + gap
        };
        if (x - want).abs() > 0.001 {
            return false;
        }
        col = col.saturating_add(1);
    }
    true
}

/// ASCII-in-sync check at `char_w`: cols 0..15 sit at `(1 + col) * char_w`
/// (one pipe char lead) with an even one-char step, matching the body
/// ASCII cell shape. Bad `char_w` returns false. Pure; unit-tested.
pub fn hex_ascii_sync_ok(char_w: f32) -> bool {
    if !char_w.is_finite() || char_w <= 0.0 {
        return false;
    }
    let mut col: usize = 0;
    while col < HEX_BYTES_PER_ROW {
        let Some(x) = hex_grid_ascii_col_x(col, char_w) else {
            return false;
        };
        let want = ((col.saturating_add(1)) as f32) * char_w;
        if (x - want).abs() > 0.001 {
            return false;
        }
        col = col.saturating_add(1);
    }
    true
}

// ----------------------------------------------------------------------------
// Hex exquisite geometry wave hex5 round-1: grid plus ascii at 960px.
// Gap BC hex half: nibble gap uneven, ASCII pane offset drift, ruler tick
// spacing asymmetric. Geometry FIRST:
// - Nibble group gap stays single-sourced from the midline helper
//   (`hex_nibble_group_gap_w` -> `hex_midline_gap_w`), so grid cols and
//   ruler cols match pixel-for-pixel.
// - ASCII pane x comes from the grid origin (`hex_ascii_pane_x` =
//   grid origin + offset gutter box + gutter gap + hex outer +
//   ascii gutter), so the ASCII pane never drifts from the byte grid.
// - Ruler tick spacing reuses SPACE tokens (`hex_ruler_tick_spacing` =
//   `GEO_SECTION_GAP` = `SPACE_SM` 8.0, sym pad = `GEO_TIGHT_GAP` 4.0);
//   absolute tick x (`hex_ruler_tick_abs_x`) shares the byte-pane origin
//   so ticks sit above byte cells.
// - Row height locked to `TypoRole::Mono.line_height_px` (15.6) at the
//   960px canvas (`hex_wide_row_h` reuse), 30 visible rows in 470px.
// Typography SECOND: grid + ruler + offset + ASCII all Mono, labels Body
// — koi naya size/token nahi. Colors FROZEN: koi Color32/Visuals nahi.
// No egui paint, no I/O.
// ----------------------------------------------------------------------------

/// Byte pane absolute x (px) from window origin: grid origin plus the
/// offset gutter box plus the gutter-to-grid gap. Single source so byte
/// cells and ruler ticks share one origin. Bad `char_w` yields `None`.
/// Pure; unit-tested.
pub fn hex_byte_pane_x(show_line_numbers: bool, char_w: f32) -> Option<f32> {
    if !char_w.is_finite() || char_w <= 0.0 {
        return None;
    }
    let x = hex_grid_origin_x(show_line_numbers)
        + hex_offset_gutter_w(char_w)
        + hex_offset_gutter_gap();
    if !x.is_finite() {
        return None;
    }
    Some(x)
}

/// ASCII pane absolute x (px) from window origin: byte pane plus hex pane
/// outer plus the ascii gutter (`SPACE_SM`). Single source from the grid
/// origin so the ASCII pane never drifts from the byte grid. Bad `char_w`
/// yields `None`. Pure; unit-tested.
pub fn hex_ascii_pane_x(show_line_numbers: bool, char_w: f32) -> Option<f32> {
    if !char_w.is_finite() || char_w <= 0.0 {
        return None;
    }
    let (hex_outer, _) = hex_pane_outer_widths(char_w)?;
    let byte_x = hex_byte_pane_x(show_line_numbers, char_w)?;
    let x = byte_x + hex_outer + hex_ascii_gutter_w();
    if !x.is_finite() {
        return None;
    }
    Some(x)
}

/// Ruler tick absolute x (px) for byte col: byte pane origin plus the
/// pane-relative tick (`hex_ruler_col_x`, itself midline single-sourced).
/// Shares the byte-pane origin with [`hex_grid_byte_col_x`], so ruler
/// ticks sit exactly above byte cells. Out-of-range or bad `char_w`
/// yields `None`. Pure; unit-tested.
pub fn hex_ruler_tick_abs_x(show_line_numbers: bool, col: usize, char_w: f32) -> Option<f32> {
    let base = hex_byte_pane_x(show_line_numbers, char_w)?;
    let rel = hex_ruler_col_x(col, char_w)?;
    let x = base + rel;
    if !x.is_finite() {
        return None;
    }
    Some(x)
}

/// Grid-plus-offset sync check at `char_w`: even byte grid, ASCII sync,
/// nibble gap exactly one char, and ruler cols matching byte cols. Bad
/// `char_w` returns false. Pure; unit-tested.
pub fn hex_grid_plus_offset_sync_ok(char_w: f32) -> bool {
    if !char_w.is_finite() || char_w <= 0.0 {
        return false;
    }
    if !hex_even_grid_ok(char_w) {
        return false;
    }
    if !hex_ascii_sync_ok(char_w) {
        return false;
    }
    let gap = hex_nibble_group_gap_w(char_w);
    if !gap.is_finite() || (gap - char_w).abs() > 0.001 {
        return false;
    }
    if hex_ruler_tick_spacing() != crate::theme::SPACE_SM {
        return false;
    }
    hex_byte_pane_x(false, char_w).is_some() && hex_ascii_pane_x(false, char_w).is_some()
}

// ----------------------------------------------------------------------------
// Hex exquisite geometry wave hex6 round-1: nibble plus ascii at 960px.
// Gap BC hex half: hex grid spacing drifts on wide files — the nibble
// group gap floated off its single source, the ASCII pane x drifted off
// the grid origin, and ruler ticks had no char-metric step at 960px wide.
// Geometry FIRST:
// - Nibble gap via SPACE-token rhythm: `hex_nibble_gap_space_rhythm_ok`
//   locks the 1-char midline gap (`hex_nibble_group_gap_w` ->
//   `hex_midline_gap_w`) alongside the SPACE-token spacings (tick spacing
//   `SPACE_SM`, sym pad `SPACE_XS`, ascii gutter `SPACE_SM`) so the gap
//   never drifts from the surrounding chrome at 960px wide.
// - ASCII pane x from grid origin helper: `hex_ascii_pane_x` already sums
//   `hex_grid_origin_x` + offset gutter box + gutter gap + hex outer +
//   ascii gutter; `hex_ascii_pane_from_origin_ok` proves the decomposition
//   pixel-for-pixel so the pane never drifts from the byte grid.
// - Ruler tick step from char metrics: `hex_ruler_tick_step_w` is the even
//   3-char step (`hex_byte_col_step_w` single source), and
//   `hex_wide_960_ruler_tick_step` locks it to 21.0px at the 7px reference
//   char on the 960px canvas; `hex_ruler_tick_abs_x` shares the byte-pane
//   origin so ticks sit above byte cells.
// Typography SECOND: grid + ruler + offset + ASCII all Mono, labels Body
// — koi naya size/token nahi. Colors FROZEN: koi Color32/Visuals nahi.
// No egui paint, no I/O.
// ----------------------------------------------------------------------------

/// Ruler tick step (px): one even byte-col step (3 chars x `char_w`).
/// Single source over [`hex_byte_col_step_w`] so ruler ticks advance on
/// the same char-metric step as byte cells; post-gap columns add the
/// shared one-char nibble gap via [`hex_ruler_col_x`]. Bad `char_w`
/// yields 0.0 (paint honest fallback). Pure; unit-tested.
pub fn hex_ruler_tick_step_w(char_w: f32) -> f32 {
    hex_byte_col_step_w(char_w)
}

/// Ruler tick step at the 960px reference canvas (px): 21.0 at the 7px
/// reference char (`3 x 7`). Locks the wide-file tick rhythm so 960px
/// ruler columns never drift from byte cells. Pure; unit-tested.
pub fn hex_wide_960_ruler_tick_step() -> f32 {
    hex_ruler_tick_step_w(7.0)
}

/// Nibble-gap symmetry check at `char_w`: the col7->col8 jump is exactly
/// one even step plus one midline char, every ruler col matches its byte
/// col, and both halves step evenly. Bad `char_w` returns false. Pure;
/// unit-tested.
pub fn hex_nibble_gap_symmetry_ok(char_w: f32) -> bool {
    if !char_w.is_finite() || char_w <= 0.0 {
        return false;
    }
    let step = hex_ruler_tick_step_w(char_w);
    if !step.is_finite() || step <= 0.0 {
        return false;
    }
    let gap = hex_nibble_group_gap_w(char_w);
    if !gap.is_finite() || (gap - char_w).abs() > 0.001 {
        return false;
    }
    let (Some(x7), Some(x8)) = (
        hex_grid_byte_col_x(7, char_w),
        hex_grid_byte_col_x(8, char_w),
    ) else {
        return false;
    };
    if ((x8 - x7) - (step + gap)).abs() > 0.001 {
        return false;
    }
    let mut col: usize = 0;
    while col < HEX_BYTES_PER_ROW {
        let (Some(bx), Some(rx)) = (
            hex_grid_byte_col_x(col, char_w),
            hex_ruler_col_x(col, char_w),
        ) else {
            return false;
        };
        if (bx - rx).abs() > 0.001 {
            return false;
        }
        col = col.saturating_add(1);
    }
    true
}

/// ASCII pane x decomposition check at `char_w`: `hex_ascii_pane_x` equals
/// grid origin + offset gutter box + gutter gap + hex outer + ascii
/// gutter, pixel-for-pixel. Bad `char_w` returns false. Pure; unit-tested.
pub fn hex_ascii_pane_from_origin_ok(show_line_numbers: bool, char_w: f32) -> bool {
    if !char_w.is_finite() || char_w <= 0.0 {
        return false;
    }
    let Some(want) = hex_ascii_pane_x(show_line_numbers, char_w) else {
        return false;
    };
    let Some((hex_outer, _)) = hex_pane_outer_widths(char_w) else {
        return false;
    };
    let Some(byte_x) = hex_byte_pane_x(show_line_numbers, char_w) else {
        return false;
    };
    let recomposed = byte_x + hex_outer + hex_ascii_gutter_w();
    if !recomposed.is_finite() {
        return false;
    }
    (want - recomposed).abs() < 0.001
        && (byte_x
            - (hex_grid_origin_x(show_line_numbers)
                + hex_offset_gutter_w(char_w)
                + hex_offset_gutter_gap()))
        .abs()
            < 0.001
}

/// Nibble gap plus SPACE-token rhythm check at `char_w`: the 1-char gap
/// (`hex_nibble_group_gap_w` -> `hex_midline_gap_w`) holds exactly one
/// `char_w` while the surrounding spacings stay on SPACE tokens (tick
/// spacing `SPACE_SM`, sym pad `SPACE_XS`, ascii gutter `SPACE_SM`,
/// grid gaps `GEO_SECTION_GAP`/`GEO_TIGHT_GAP`). Bad `char_w` returns
/// false. Pure; unit-tested.
pub fn hex_nibble_gap_space_rhythm_ok(char_w: f32) -> bool {
    if !char_w.is_finite() || char_w <= 0.0 {
        return false;
    }
    let gap = hex_nibble_group_gap_w(char_w);
    if !gap.is_finite() || (gap - char_w).abs() > 0.001 {
        return false;
    }
    if hex_midline_gap_w(char_w) != gap {
        return false;
    }
    hex_ruler_tick_spacing() == crate::theme::SPACE_SM
        && hex_ruler_tick_sym_pad() == crate::theme::SPACE_XS
        && hex_ascii_gutter_w() == crate::theme::SPACE_SM
        && hex_ruler_grid_gap_x() == crate::theme::GEO_SECTION_GAP
        && hex_ruler_grid_gap_y() == crate::theme::GEO_TIGHT_GAP
}

// ----------------------------------------------------------------------------
// Hex exquisite geometry wave hex7 round-2: offset plus gap at 960px.
// Gap BC hex half: the offset column reads tight on wide (8-digit)
// addresses and the ASCII pane sits without a clear gap — ours matched the
// ruler chars but never locked the offset box to the 8-digit address width
// or the ASCII gap to SPACE rhythm at 960px wide.
// Geometry FIRST:
// - Offset column from char metrics: `HEX_OFFSET_HEX_DIGITS` (8) plus the
//   `0x` prefix reuse `HEX_RULER_CHARS` (10); `hex_offset_960_text_w`
//   (70.0) and `hex_offset_960_gutter_w` (78.0) lock the 960px box, and
//   `hex_offset_width_fit_ok` proves text + symmetric SPACE pads fit the
//   widest 8-digit label (`format_hex_offset(u32::MAX)`, 10 chars).
// - ASCII pane gap from SPACE tokens: `hex_ascii_gap_space_ok` locks the
//   gutter to `SPACE_SM`/`GEO_SECTION_GAP` and proves the ASCII pane x
//   leaves exactly that gap past the hex outer box.
// - Byte cell size from TypoRole Mono metrics: `hex_byte_cell_960_ok`
//   locks the 2-char text (14.0) plus symmetric cell pads (box 30.0) at
//   the 7px reference char on the 960px canvas.
// Typography SECOND: offset + grid + ruler + ASCII all Mono
// (`hex_offset_text_role` reuse), labels Body — koi naya size/token nahi.
// Colors FROZEN: koi Color32/Visuals nahi. No egui paint, no I/O.
// ----------------------------------------------------------------------------

/// Hex digits in one offset label: 8 (`0x00000000` tail). Pure.
pub const HEX_OFFSET_HEX_DIGITS: usize = 8;

/// `0x` prefix chars in one offset label. Pure.
pub const HEX_OFFSET_PREFIX_CHARS: usize = 2;

/// Full offset label chars: prefix + digits. Must equal
/// [`HEX_RULER_CHARS`] (10) so the gutter text never clips the widest
/// 8-digit address. Pure.
pub const HEX_OFFSET_ADDR_CHARS: usize = HEX_OFFSET_PREFIX_CHARS + HEX_OFFSET_HEX_DIGITS;

/// Offset label chars (10): `0x` prefix + 8 address digits. Pure.
pub fn hex_offset_addr_chars() -> usize {
    HEX_OFFSET_ADDR_CHARS
}

/// Offset-gutter text width (px) at the 960px reference char (7.0): 70.0.
/// Single source over [`hex_offset_gutter_text_w`]. Pure; unit-tested.
pub fn hex_offset_960_text_w() -> f32 {
    hex_offset_gutter_text_w(7.0)
}

/// Offset-gutter outer box (px) at the 960px reference char: 78.0.
/// Single source over [`hex_offset_gutter_w`]. Pure; unit-tested.
pub fn hex_offset_960_gutter_w() -> f32 {
    hex_offset_gutter_w(7.0)
}

/// Byte-cell text width (px) at the 960px reference char: 14.0 (2 chars).
/// Single source over [`hex_grid_byte_cell_text_w`]. Pure; unit-tested.
pub fn hex_byte_cell_960_text_w() -> f32 {
    hex_grid_byte_cell_text_w(7.0)
}

/// Byte-cell outer box (px) at the 960px reference char: 30.0 (text plus
/// symmetric [`HEX_CELL_PAD_X`] pads). Single source over
/// [`hex_grid_byte_cell_box_w`]. Pure; unit-tested.
pub fn hex_byte_cell_960_box_w() -> f32 {
    hex_grid_byte_cell_box_w(7.0)
}

/// Offset-width fit at `char_w`: the label budget is exactly the 10-char
/// address (`HEX_OFFSET_ADDR_CHARS == HEX_RULER_CHARS`), text width is
/// `chars * char_w`, the outer box adds symmetric SPACE pads, and the
/// widest 8-digit label (`format_hex_offset(u32::MAX)`) still fits.
/// Bad `char_w` returns false. Pure; unit-tested.
pub fn hex_offset_width_fit_ok(char_w: f32) -> bool {
    if !char_w.is_finite() || char_w <= 0.0 {
        return false;
    }
    if HEX_OFFSET_ADDR_CHARS != HEX_RULER_CHARS {
        return false;
    }
    if HEX_OFFSET_ADDR_CHARS != HEX_OFFSET_PREFIX_CHARS + HEX_OFFSET_HEX_DIGITS {
        return false;
    }
    let widest = format_hex_offset(u32::MAX as u64);
    if widest.len() != HEX_OFFSET_ADDR_CHARS {
        return false;
    }
    let text = hex_offset_gutter_text_w(char_w);
    if !text.is_finite() {
        return false;
    }
    if (text - (HEX_OFFSET_ADDR_CHARS as f32) * char_w).abs() > 0.001 {
        return false;
    }
    let outer = hex_offset_gutter_w(char_w);
    if !outer.is_finite() {
        return false;
    }
    if (outer - (text + 2.0 * hex_offset_gutter_pad())).abs() > 0.001 {
        return false;
    }
    if hex_offset_gutter_pad() != crate::theme::GEO_TIGHT_GAP {
        return false;
    }
    hex_offset_gutter_gap() == crate::theme::GEO_SECTION_GAP
}

/// ASCII-pane gap from SPACE tokens at `char_w`: the gutter is exactly
/// `SPACE_SM` (`GEO_SECTION_GAP`), and the ASCII pane x leaves exactly
/// that gap past the hex outer box. Bad `char_w` returns false. Pure;
/// unit-tested.
pub fn hex_ascii_gap_space_ok(show_line_numbers: bool, char_w: f32) -> bool {
    if !char_w.is_finite() || char_w <= 0.0 {
        return false;
    }
    if hex_ascii_gutter_w() != crate::theme::SPACE_SM {
        return false;
    }
    if hex_ascii_gutter_w() != crate::theme::GEO_SECTION_GAP {
        return false;
    }
    let Some(byte_x) = hex_byte_pane_x(show_line_numbers, char_w) else {
        return false;
    };
    let Some(ascii_x) = hex_ascii_pane_x(show_line_numbers, char_w) else {
        return false;
    };
    let Some((hex_outer, _)) = hex_pane_outer_widths(char_w) else {
        return false;
    };
    ((ascii_x - byte_x - hex_outer) - hex_ascii_gutter_w()).abs() < 0.001
}

/// Byte-cell size from TypoRole Mono metrics at `char_w`: text is 2 chars,
/// the box adds symmetric [`HEX_CELL_PAD_X`] pads, the even step is
/// 3 chars, and the grid rhythm rides the Mono line-height (12.0 x 1.30
/// = 15.6). Bad `char_w` returns false. Pure; unit-tested.
pub fn hex_byte_cell_960_ok(char_w: f32) -> bool {
    if !char_w.is_finite() || char_w <= 0.0 {
        return false;
    }
    let text = hex_grid_byte_cell_text_w(char_w);
    if !text.is_finite() || (text - 2.0 * char_w).abs() > 0.001 {
        return false;
    }
    let boxed = hex_grid_byte_cell_box_w(char_w);
    if !boxed.is_finite() || (boxed - (text + 2.0 * HEX_CELL_PAD_X)).abs() > 0.001 {
        return false;
    }
    let step = hex_byte_col_step_w(char_w);
    if !step.is_finite() || (step - 3.0 * char_w).abs() > 0.001 {
        return false;
    }
    if hex_cell_text_role() != crate::theme::TypoRole::Mono {
        return false;
    }
    (crate::theme::TypoRole::Mono.line_height_px() - 15.6).abs() < 0.001
}

/// Offset-plus-gap sync at `char_w`: the offset box fits the widest
/// 8-digit address, the ASCII pane leaves a SPACE-token gap, byte cells
/// come from Mono char metrics, and the 960px canvas stays side-by-side.
/// Bad `char_w` returns false. Pure; unit-tested.
pub fn hex_offset_plus_gap_sync_ok(char_w: f32) -> bool {
    if !hex_offset_width_fit_ok(char_w) {
        return false;
    }
    if !hex_ascii_gap_space_ok(false, char_w) {
        return false;
    }
    if !hex_byte_cell_960_ok(char_w) {
        return false;
    }
    if hex_layout_mode(HEX_CANVAS_W) != HexLayoutMode::SideBySide {
        return false;
    }
    hex_grid_plus_offset_sync_ok(char_w)
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
        assert_eq!(hex_gutter_bar_width(false), gutter_bar_width(true));
        assert_eq!(hex_gutter_bar_width(true), 12.0);
        assert_eq!(hex_gutter_bar_width(false), 12.0);
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

    #[test]
    fn hex_minimap_strip_width_is_double_gutter_bar() {
        use crate::gutter::gutter_bar_width;
        // Single source: gutter contract ka double, paint token parity.
        assert_eq!(hex_minimap_strip_width(), gutter_bar_width(true) * 2.0);
        assert_eq!(hex_minimap_strip_width(), 24.0);
        assert_eq!(hex_minimap_strip_width(), crate::views::HEX_MINIMAP_W);
    }

    #[test]
    fn hex_minimap_bands_are_offset_proportional() {
        // 64 bytes = 4 rows, height 400: har row 100px band.
        let hunks = vec![HexHunkView {
            offset: 20,
            left_len: 1,
            right_len: 1,
        }];
        let bands = hex_minimap_bands(64, 400.0, &hunks);
        assert_eq!(bands.len(), 1);
        assert_eq!(bands.first(), Some(&(100.0, 200.0)));
        // Multi-row hunk: row 1 se row 3 tak failta hai.
        let wide = vec![HexHunkView {
            offset: 20,
            left_len: 30,
            right_len: 30,
        }];
        let wide_bands = hex_minimap_bands(64, 400.0, &wide);
        assert_eq!(wide_bands.len(), 1);
        assert_eq!(wide_bands.first(), Some(&(100.0, 400.0)));
        // Tall file: sub-row band floor par (y0 + 4px ruler floor).
        // 16000 bytes = 1000 rows, height 400: har row 0.4px band.
        let tall = vec![HexHunkView {
            offset: 8000,
            left_len: 1,
            right_len: 1,
        }];
        let tall_bands = hex_minimap_bands(16_000, 400.0, &tall);
        assert_eq!(tall_bands.len(), 1);
        let Some((y0, y1)) = tall_bands.first().copied() else {
            return;
        };
        assert_eq!(y0, 200.0);
        assert!((y1 - y0 - crate::gutter::GUTTER_SEGMENT_MIN_H).abs() < 0.001);
        // Khali run / row se bahar / kharab input: koi band nahi.
        let empty_run = vec![HexHunkView {
            offset: 20,
            left_len: 0,
            right_len: 0,
        }];
        assert!(hex_minimap_bands(64, 400.0, &empty_run).is_empty());
        let past_end = vec![HexHunkView {
            offset: 10_000,
            left_len: 1,
            right_len: 1,
        }];
        assert!(hex_minimap_bands(64, 400.0, &past_end).is_empty());
        assert!(hex_minimap_bands(0, 400.0, &hunks).is_empty());
        assert!(hex_minimap_bands(64, 0.0, &hunks).is_empty());
        assert!(hex_minimap_bands(64, 400.0, &[]).is_empty());
        let single = HexHunkView {
            offset: 20,
            left_len: 1,
            right_len: 1,
        };
        assert_eq!(hex_minimap_band_for_hunk(&single, 0, 400.0), None);
    }

    #[test]
    fn hex_minimap_y_for_row_inverts_locked_row_mapping() {
        // 4 rows over 400px: band tops 0/100/200/300.
        assert_eq!(hex_minimap_y_for_row(0, 4, 400.0), Some(0.0));
        assert_eq!(hex_minimap_y_for_row(1, 4, 400.0), Some(100.0));
        assert_eq!(hex_minimap_y_for_row(3, 4, 400.0), Some(300.0));
        // Bad input maps nothing.
        assert_eq!(hex_minimap_y_for_row(4, 4, 400.0), None);
        assert_eq!(hex_minimap_y_for_row(0, 0, 400.0), None);
        assert_eq!(hex_minimap_y_for_row(1, 4, 0.0), None);
        assert_eq!(hex_minimap_y_for_row(1, 4, f32::NAN), None);
        // Round-trip locked paint mapping ke saath: marker y wapas usi row.
        let mut row: u64 = 0;
        while row < 4 {
            if let Some(y) = hex_minimap_y_for_row(row, 4, 400.0) {
                assert_eq!(
                    crate::views::hex_minimap_row_at_y(y + 0.5, 400.0, 4),
                    Some(row)
                );
            }
            row = row.saturating_add(1);
        }
    }

    #[test]
    fn hex_nibbles_differ_flags_each_half_byte() {
        assert_eq!(hex_nibbles_differ(0xAB, 0xAB), (false, false));
        assert_eq!(hex_nibbles_differ(0xAB, 0xCB), (true, false));
        assert_eq!(hex_nibbles_differ(0xAB, 0xAD), (false, true));
        assert_eq!(hex_nibbles_differ(0xAB, 0xCD), (true, true));
        assert_eq!(hex_nibbles_differ(0x00, 0xFF), (true, true));
    }

    #[test]
    fn hex_diff_nibble_cells_report_only_changed_bytes() {
        // Row 0: col 3 low-only, col 5 high-only.
        let mut left = Vec::new();
        let mut right = Vec::new();
        let mut i: u8 = 0;
        while i < 16 {
            left.push(i);
            right.push(i);
            i = i.saturating_add(1);
        }
        if let Some(slot) = right.get_mut(3) {
            *slot = 0x02;
        }
        if let Some(slot) = right.get_mut(5) {
            *slot = 0xF5;
        }
        let cells = hex_diff_nibble_cells(0, &left, &right);
        assert_eq!(cells.len(), 2);
        assert_eq!(
            cells.first(),
            Some(&HexNibbleCell {
                offset: 3,
                col: 3,
                high_diff: false,
                low_diff: true,
            })
        );
        assert_eq!(
            cells.get(1),
            Some(&HexNibbleCell {
                offset: 5,
                col: 5,
                high_diff: true,
                low_diff: false,
            })
        );
        // Dusri row barabar: koi cell nahi (poori-row accent ka koi kaaran nahi).
        let big_left = vec![0u8; 32];
        let mut big_right = vec![0u8; 32];
        if let Some(slot) = big_right.get_mut(3) {
            *slot = 9;
        }
        assert!(hex_diff_nibble_cells(16, &big_left, &big_right).is_empty());
        assert_eq!(hex_diff_nibble_cells(0, &big_left, &big_right).len(), 1);
        // Chhoti slices: missing slots skip, panic nahi.
        let tiny = vec![7u8, 7, 7, 7];
        let tiny_r = vec![7u8, 7, 8, 7];
        let tiny_cells = hex_diff_nibble_cells(0, &tiny, &tiny_r);
        assert_eq!(tiny_cells.len(), 1);
        assert_eq!(tiny_cells.first().map(|c| c.col), Some(2));
        // Khali slices: khali vec.
        assert!(hex_diff_nibble_cells(0, &[], &[]).is_empty());
    }

    #[test]
    fn hex_nibble_text_role_reuses_mono_token() {
        use crate::theme::{TYPO_MONO_SIZE, TypoRole};
        assert_eq!(hex_nibble_text_role(), TypoRole::Mono);
        assert!(hex_nibble_text_role().is_mono());
        assert_eq!(hex_nibble_text_role().size(), TYPO_MONO_SIZE);
    }

    #[test]
    fn hex_find_label_role_steps_above_hint() {
        use crate::theme::{TYPO_BODY_SIZE, TYPO_SMALL_SIZE, TypoRole};
        assert_eq!(hex_find_label_text_role(), TypoRole::Body);
        assert_eq!(hex_find_label_text_role().size(), TYPO_BODY_SIZE);
        // Hierarchy: section label hint se ek step upar, mono content
        // se alag family (label proportional, input Mono).
        assert!(hex_find_label_text_role().size() > TYPO_SMALL_SIZE);
        assert!(!hex_find_label_text_role().is_mono());
        assert!(hex_find_label_text_role().is_chrome());
    }

    #[test]
    fn hex_minimap_alloc_tokens_lock_paint() {
        // Contract: width gutter double (24px), height paint strip (220px).
        assert_eq!(hex_minimap_alloc_height(), crate::views::HEX_MINIMAP_H);
        assert_eq!(hex_minimap_alloc_height(), 220.0);
        let size = hex_minimap_alloc_size();
        assert_eq!(size.x, 24.0);
        assert_eq!(size.y, 220.0);
        assert_eq!(size.x, hex_minimap_strip_width());
        // Track rect origin se size tak: hit-rect wahi allocate rect hai.
        let origin = egui::pos2(100.0, 50.0);
        let track = hex_minimap_track_rect(origin);
        assert_eq!(track.min, origin);
        assert_eq!(track.width(), 24.0);
        assert_eq!(track.height(), 220.0);
        assert_eq!(track.max.x, 124.0);
        assert_eq!(track.max.y, 270.0);
    }

    #[test]
    fn hex_minimap_band_rect_anchors_to_track() {
        // 4 rows over 400px track at (10, 50): row 1 band exact.
        let track = egui::Rect::from_min_size(egui::pos2(10.0, 50.0), egui::vec2(24.0, 400.0));
        let hunk = HexHunkView {
            offset: 20,
            left_len: 1,
            right_len: 1,
        };
        let rect = hex_minimap_band_rect(track, 4, &hunk);
        assert_eq!(
            rect,
            Some(egui::Rect::from_min_max(
                egui::pos2(10.0, 150.0),
                egui::pos2(34.0, 250.0),
            ))
        );
        // X poori track chaudai, Y track ke andar clamp.
        let wide = HexHunkView {
            offset: 20,
            left_len: 30,
            right_len: 30,
        };
        let wide_rect = hex_minimap_band_rect(track, 4, &wide);
        assert_eq!(
            wide_rect,
            Some(egui::Rect::from_min_max(
                egui::pos2(10.0, 150.0),
                egui::pos2(34.0, 450.0),
            ))
        );
        // Tall file: sub-row floor (y0 + 4px ruler floor) track-anchored.
        let tall_track = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(24.0, 400.0));
        let tall = HexHunkView {
            offset: 8000,
            left_len: 1,
            right_len: 1,
        };
        let Some(tall_rect) = hex_minimap_band_rect(tall_track, 1000, &tall) else {
            return;
        };
        assert_eq!(tall_rect.min.x, 0.0);
        assert_eq!(tall_rect.max.x, 24.0);
        assert_eq!(tall_rect.min.y, 200.0);
        assert!((tall_rect.height() - crate::gutter::GUTTER_SEGMENT_MIN_H).abs() < 0.001);
        // Bad inputs: khali run / row se bahar / zero-area track -> None.
        let empty_run = HexHunkView {
            offset: 20,
            left_len: 0,
            right_len: 0,
        };
        assert_eq!(hex_minimap_band_rect(track, 4, &empty_run), None);
        let past_end = HexHunkView {
            offset: 10_000,
            left_len: 1,
            right_len: 1,
        };
        assert_eq!(hex_minimap_band_rect(track, 4, &past_end), None);
        assert_eq!(hex_minimap_band_rect(track, 0, &hunk), None);
        let flat = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(24.0, 0.0));
        assert_eq!(hex_minimap_band_rect(flat, 4, &hunk), None);
    }

    #[test]
    fn hex_minimap_band_rects_follow_offset_order() {
        let track = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(24.0, 400.0));
        let first = HexHunkView {
            offset: 20,
            left_len: 1,
            right_len: 1,
        };
        let second = HexHunkView {
            offset: 50,
            left_len: 1,
            right_len: 1,
        };
        let rects = hex_minimap_band_rects(track, 64, &[first, second]);
        assert_eq!(rects.len(), 2);
        // Offset order barkarar (paint loop jaisa), y badhta hai.
        assert!(rects.first().map(|r| r.min.y) < rects.get(1).map(|r| r.min.y));
        // Tuple layer se parity: local bands track offset par.
        let tuples = hex_minimap_bands(64, 400.0, &[first, second]);
        assert_eq!(rects.len(), tuples.len());
        for (rect, (y0, y1)) in rects.iter().zip(tuples.iter()) {
            assert_eq!(rect.min.y, *y0);
            assert_eq!(rect.max.y, *y1);
            assert_eq!(rect.min.x, 0.0);
            assert_eq!(rect.max.x, 24.0);
        }
        // Khali/kharab: khali vec.
        assert!(hex_minimap_band_rects(track, 64, &[]).is_empty());
        assert!(hex_minimap_band_rects(track, 0, &[first]).is_empty());
    }

    #[test]
    fn hex_minimap_row_at_pos_matches_locked_mapping() {
        let track = egui::Rect::from_min_size(egui::pos2(10.0, 50.0), egui::vec2(24.0, 400.0));
        // Track-relative: min.y + 0 -> row 0, +100 -> row 1, +399.9 -> row 3.
        assert_eq!(hex_minimap_row_at_pos(track, 4, 50.0), Some(0));
        assert_eq!(hex_minimap_row_at_pos(track, 4, 150.0), Some(1));
        assert_eq!(hex_minimap_row_at_pos(track, 4, 449.9), Some(3));
        // Bahar ya kharab: None (paint mapping jaisa contract).
        assert_eq!(hex_minimap_row_at_pos(track, 4, 450.0), None);
        assert_eq!(hex_minimap_row_at_pos(track, 4, 49.9), None);
        assert_eq!(hex_minimap_row_at_pos(track, 0, 150.0), None);
        assert_eq!(hex_minimap_row_at_pos(track, 4, f32::NAN), None);
        // Locked paint mapping se parity: local y par views fn wahi row.
        let mut row: u64 = 0;
        while row < 4 {
            if let Some(y) = hex_minimap_y_for_row(row, 4, 400.0) {
                assert_eq!(hex_minimap_row_at_pos(track, 4, 50.0 + y + 0.5), Some(row));
                assert_eq!(
                    crate::views::hex_minimap_row_at_y(y + 0.5, 400.0, 4),
                    Some(row)
                );
            }
            row = row.saturating_add(1);
        }
    }

    #[test]
    fn hex_nibble_cell_rect_splits_row_into_sixteen() {
        let row = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(160.0, 20.0));
        // Har column 10px: col 0 -> [0,10], col 15 -> [150,160].
        assert_eq!(
            hex_nibble_cell_rect(row, 0),
            Some(egui::Rect::from_min_max(
                egui::pos2(0.0, 0.0),
                egui::pos2(10.0, 20.0),
            ))
        );
        assert_eq!(
            hex_nibble_cell_rect(row, 15),
            Some(egui::Rect::from_min_max(
                egui::pos2(150.0, 0.0),
                egui::pos2(160.0, 20.0),
            ))
        );
        // Y poori row height rakhta hai.
        let Some(mid) = hex_nibble_cell_rect(row, 3) else {
            return;
        };
        assert_eq!(mid.min.y, 0.0);
        assert_eq!(mid.max.y, 20.0);
        assert_eq!(mid.min.x, 30.0);
        assert_eq!(mid.max.x, 40.0);
        // col >= 16 ya zero-area row: None.
        assert_eq!(hex_nibble_cell_rect(row, 16), None);
        let flat = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(0.0, 20.0));
        assert_eq!(hex_nibble_cell_rect(flat, 0), None);
    }

    #[test]
    fn hex_nibble_accent_rects_split_halves_only_for_diffs() {
        let cell_rect = egui::Rect::from_min_max(egui::pos2(30.0, 0.0), egui::pos2(40.0, 20.0));
        // Dono nibble alag: dono halves (poori cell cover).
        let both = HexNibbleCell {
            offset: 3,
            col: 3,
            high_diff: true,
            low_diff: true,
        };
        let (high, low) = hex_nibble_accent_rects(cell_rect, &both);
        assert_eq!(
            high,
            Some(egui::Rect::from_min_max(
                egui::pos2(30.0, 0.0),
                egui::pos2(35.0, 20.0),
            ))
        );
        assert_eq!(
            low,
            Some(egui::Rect::from_min_max(
                egui::pos2(35.0, 0.0),
                egui::pos2(40.0, 20.0),
            ))
        );
        // Sirf low: high None.
        let low_only = HexNibbleCell {
            offset: 3,
            col: 3,
            high_diff: false,
            low_diff: true,
        };
        let (high_none, low_some) = hex_nibble_accent_rects(cell_rect, &low_only);
        assert_eq!(high_none, None);
        assert!(low_some.is_some());
        // Sirf high: low None.
        let high_only = HexNibbleCell {
            offset: 5,
            col: 5,
            high_diff: true,
            low_diff: false,
        };
        let (high_some, low_none) = hex_nibble_accent_rects(cell_rect, &high_only);
        assert!(high_some.is_some());
        assert_eq!(low_none, None);
        // Barabar byte: dono None (koi accent nahi).
        let same = HexNibbleCell {
            offset: 1,
            col: 1,
            high_diff: false,
            low_diff: false,
        };
        assert_eq!(hex_nibble_accent_rects(cell_rect, &same), (None, None));
    }

    #[test]
    fn hex_row_nibble_accent_rects_collect_in_col_order() {
        let row = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(160.0, 20.0));
        // Row 0: col 3 low-only (1 rect), col 5 high-only (1 rect).
        let left: Vec<u8> = (0..16).collect();
        let mut right: Vec<u8> = (0..16).collect();
        if let Some(slot) = right.get_mut(3) {
            *slot = 0x02;
        }
        if let Some(slot) = right.get_mut(5) {
            *slot = 0xF5;
        }
        let cells = hex_diff_nibble_cells(0, &left, &right);
        assert_eq!(cells.len(), 2);
        let rects = hex_row_nibble_accent_rects(row, &cells);
        assert_eq!(rects.len(), 2);
        // Col order: pehla col 3 ka low half [35,40], dusra col 5 ka high [50,55].
        assert_eq!(rects.first().map(|r| r.min.x), Some(35.0));
        assert_eq!(rects.first().map(|r| r.max.x), Some(40.0));
        assert_eq!(rects.get(1).map(|r| r.min.x), Some(50.0));
        assert_eq!(rects.get(1).map(|r| r.max.x), Some(55.0));
        // Barabar row: koi accent nahi (poori-row band ka koi kaaran nahi).
        let big_left = vec![0u8; 32];
        let big_right = vec![0u8; 32];
        let same_cells = hex_diff_nibble_cells(16, &big_left, &big_right);
        assert!(hex_row_nibble_accent_rects(row, &same_cells).is_empty());
        assert!(hex_row_nibble_accent_rects(row, &[]).is_empty());
    }

    #[test]
    fn hex_grid_cells_with_bytes_show_real_digits() {
        // Row 0-15 cover: left bytes 0..16 ki real lowercase digits.
        let left: Vec<u8> = (0..16).collect();
        let right: Vec<u8> = (0..16).collect();
        let hunks = vec![HexHunkView {
            offset: 0,
            left_len: 16,
            right_len: 16,
        }];
        let (addr, hex, ascii) = format_hex_grid_cells_with_bytes(0, &left, &right, &hunks);
        assert_eq!(addr, format_hex_offset(0));
        assert!(hex.contains("00"));
        assert!(hex.contains("0f"));
        assert!(!hex.contains("??"));
        // "Hello wo" jaisa printable ASCII seedha dikhta hai.
        let text_left = b"Hello wo!!!!!!!!".to_vec();
        let (.., text_ascii) = format_hex_grid_cells_with_bytes(0, &text_left, &text_left, &hunks);
        assert!(text_ascii.contains("Hello wo"));
        // Non-printable par `.` (fixture style), `?` nahi.
        assert!(!ascii.contains('?'));
        assert_eq!(ascii.chars().next(), Some('.'));
        // Single-string join wahi shape hai jo Grid paint karta hai.
        assert_eq!(
            format_hex_grid_row_with_bytes(0, &left, &right, &hunks),
            format!("{addr}  {hex} |{ascii}|")
        );
    }

    #[test]
    fn hex_grid_cells_with_bytes_fallback_and_shape_parity() {
        // Insertion hunk: left khali, right se digits (right fallback).
        let left: Vec<u8> = Vec::new();
        let right: Vec<u8> = (0..16).collect();
        let hunks = vec![HexHunkView {
            offset: 0,
            left_len: 0,
            right_len: 16,
        }];
        let (.., hex, ascii) = format_hex_grid_cells_with_bytes(0, &left, &right, &hunks);
        assert!(hex.contains("00"));
        assert!(!hex.contains("??"));
        assert_eq!(ascii.len(), 16);
        // Shape parity: purane offset-only cells jitni hi chaudai.
        let (old_addr, old_hex, old_ascii) = crate::views::format_hex_grid_cells(0, &hunks);
        let (addr, _, _) = format_hex_grid_cells_with_bytes(0, &left, &right, &hunks);
        assert_eq!(addr, old_addr);
        assert_eq!(hex.len(), old_hex.len());
        assert_eq!(ascii.len(), old_ascii.len());
        // Uncovered row blank: koi digit nahi, ASCII 16 spaces.
        let (.., e_hex, e_ascii) = format_hex_grid_cells_with_bytes(32, &left, &right, &hunks);
        assert!(!e_hex.contains("??"));
        assert_eq!(e_hex.trim().len(), 0);
        assert_eq!(e_ascii, "                ");
        // Dono missing par blank (stale bytes kabhi nahi).
        let past = vec![HexHunkView {
            offset: 10_000,
            left_len: 4,
            right_len: 4,
        }];
        let (.., p_hex, p_ascii) = format_hex_grid_cells_with_bytes(10_000, &left, &[], &past);
        assert_eq!(p_hex.trim().len(), 0);
        assert_eq!(p_ascii, "                ");
        // ASCII mark contract: printable wahi char, control/DEL par `.`.
        assert_eq!(hex_grid_ascii_mark(b'A'), 'A');
        assert_eq!(hex_grid_ascii_mark(0x20), ' ');
        assert_eq!(hex_grid_ascii_mark(0x00), '.');
        assert_eq!(hex_grid_ascii_mark(0x7F), '.');
    }

    #[test]
    fn hex_goto_parses_64bit_decimal_and_hex() {
        assert_eq!(parse_hex_goto("16"), Some(16));
        assert_eq!(parse_hex_goto("0x10"), Some(16));
        assert_eq!(parse_hex_goto("0XFF"), Some(255));
        assert_eq!(parse_hex_goto("1_000"), Some(1000));
        assert_eq!(parse_hex_goto("0xFF_FF"), Some(65535));
        assert_eq!(parse_hex_goto(&u64::MAX.to_string()), Some(u64::MAX));
        assert_eq!(parse_hex_goto("0xFFFFFFFFFFFFFFFF"), Some(u64::MAX));
        assert_eq!(parse_hex_goto(""), None);
        assert_eq!(parse_hex_goto("   "), None);
        assert_eq!(parse_hex_goto("0x"), None);
        assert_eq!(parse_hex_goto("banana"), None);
        assert_eq!(parse_hex_goto("18446744073709551616"), None);
        // Geometry parity: goto bar find-bar rhythm reuse karta hai.
        assert_eq!(HEX_GOTO_BAR_H, HEX_FIND_BAR_H);
        assert_eq!(HEX_GOTO_FIELD_W, HEX_FIND_NEEDLE_W);
        assert_eq!(HEX_GOTO_BADGE_PAD, HEX_FIND_BADGE_PAD);
        assert_eq!(hex_goto_label_text_role(), crate::theme::TypoRole::Body);
        assert_eq!(hex_goto_value_text_role(), crate::theme::TypoRole::Mono);
    }

    #[test]
    fn hex_goto_clamps_and_edits_bytes_honestly() {
        assert_eq!(clamp_hex_goto(5, 10), Some(5));
        assert_eq!(clamp_hex_goto(10, 10), None);
        assert_eq!(clamp_hex_goto(0, 0), None);
        assert_eq!(hex_nibble_value('f'), Some(15));
        assert_eq!(hex_nibble_value('G'), None);
        assert_eq!(apply_hex_nibble(0xAB, '0', true), Some(0x0B));
        assert_eq!(apply_hex_nibble(0xAB, 'C', false), Some(0xAC));
        assert_eq!(apply_hex_nibble(0xAB, 'z', true), None);
        let mut buf = vec![1u8, 2, 3];
        assert!(edit_byte_at(&mut buf, 1, 9));
        assert_eq!(buf, vec![1, 9, 3]);
        assert!(!edit_byte_at(&mut buf, 9, 0));
        assert_eq!(buf, vec![1, 9, 3]);
    }

    #[test]
    fn hex_wave7_canvas_layout_modes_and_chrome_budget() {
        // Reference canvas + breakpoint: narrow stacks, wide splits.
        assert_eq!(HEX_CANVAS_W, 960.0);
        assert_eq!(HEX_CANVAS_H, 600.0);
        assert_eq!(HEX_STACK_BREAKPOINT_W, 720.0);
        assert_eq!(hex_layout_mode(960.0), HexLayoutMode::SideBySide);
        assert_eq!(hex_layout_mode(720.0), HexLayoutMode::SideBySide);
        assert_eq!(hex_layout_mode(719.0), HexLayoutMode::Stacked);
        assert_eq!(hex_layout_mode(320.0), HexLayoutMode::Stacked);
        assert_eq!(hex_layout_mode(0.0), HexLayoutMode::Stacked);
        assert_eq!(hex_layout_mode(f32::NAN), HexLayoutMode::Stacked);
        assert_eq!(hex_layout_mode(f32::INFINITY), HexLayoutMode::Stacked);
        // Chrome reserve reuses gutter + minimap + SPACE_SM gaps, no drift.
        // Hex bar hamesha wide (ruler hamesha visible): false par bhi 12.
        assert_eq!(
            hex_chrome_reserve_w(true),
            12.0 + 24.0 + 2.0 * crate::theme::SPACE_SM
        );
        assert_eq!(
            hex_chrome_reserve_w(false),
            12.0 + 24.0 + 2.0 * crate::theme::SPACE_SM
        );
        // 960 canvas leaves a positive grid; narrow floors at 0, no clip.
        assert!(hex_grid_avail_w(960.0, true) > 0.0);
        assert_eq!(
            hex_grid_avail_w(960.0, true),
            960.0 - hex_chrome_reserve_w(true)
        );
        assert_eq!(hex_grid_avail_w(10.0, true), 0.0);
        assert_eq!(hex_grid_avail_w(f32::NAN, true), 0.0);
    }

    #[test]
    fn hex_wave7_sync_panes_share_one_row_window() {
        // 64 bytes = 4 rows: full window lists all 16-aligned starts.
        assert_eq!(hex_sync_visible_range(0, 4, 4), (0, 4));
        assert_eq!(hex_sync_row_starts(0, 4, 64), vec![0, 16, 32, 48]);
        // Partial window: hex and ASCII iterate the same starts.
        assert_eq!(hex_sync_visible_range(1, 2, 4), (1, 3));
        assert_eq!(hex_sync_row_starts(1, 2, 64), vec![16, 32]);
        // Pager parity: in-range states match HexPager exactly.
        let mut pager = HexPager::new(4, 2);
        pager.bring_into_view(3);
        assert_eq!(
            hex_sync_visible_range(pager.top, 2, 4),
            pager.visible_range()
        );
        // Out-of-range top clamps; empty input is empty, never panics.
        assert_eq!(hex_sync_visible_range(99, 4, 4), (4, 4));
        assert!(hex_sync_row_starts(99, 4, 64).is_empty());
        assert!(hex_sync_row_starts(0, 4, 0).is_empty());
        assert_eq!(hex_sync_visible_range(0, 0, 4), (0, 1));
    }

    #[test]
    fn hex_wave7_goto_anchors_to_grid_and_clamps_u64() {
        // Goto anchor is the grid (row, col): ruler and bar agree.
        assert_eq!(hex_goto_anchor(0), (0, 0));
        assert_eq!(hex_goto_anchor(17), (1, 1));
        assert_eq!(hex_goto_anchor(17), hex_offset_to_row_col(17));
        assert_eq!(hex_goto_row_start(17), 16);
        assert_eq!(hex_goto_row_start(17), hex_row_start(1));
        // 64-bit top: saturates on the last grid row, never wraps.
        let (max_row, max_col) = hex_goto_anchor(u64::MAX);
        assert_eq!((max_row, max_col), hex_offset_to_row_col(u64::MAX));
        assert_eq!(max_col, 15);
        assert_eq!(hex_goto_row_start(u64::MAX), max_row * 16);
        assert_eq!(hex_goto_row_start(u64::MAX), u64::MAX - 15);
    }

    #[test]
    fn hex_wave7_gutter_fast_matches_row_wise_bands() {
        // 64 bytes = 4 rows; byte 20 (row 1) and byte 50 (row 3) differ.
        let left = vec![0u8; 64];
        let mut right = vec![0u8; 64];
        if let Some(slot) = right.get_mut(20) {
            *slot = 9;
        }
        if let Some(slot) = right.get_mut(50) {
            *slot = 7;
        }
        let view = HexCompareView::build_from_slices(&left, &right, HexCompareMode::None, 8);
        let total = hex_gutter_total_rows(64);
        assert_eq!(total, 4);
        let slow = hex_gutter_geometry(total, 400.0, &view.hunks);
        let fast = hex_gutter_geometry_fast(total, 400.0, &view.hunks);
        assert_eq!(fast, slow);
        assert_eq!(fast.segments.len(), 2);
        // Contiguous run fuses: 32 differing bytes = one band, both paths.
        let l2 = vec![0u8; 32];
        let r2 = vec![1u8; 32];
        let v2 = HexCompareView::build_from_slices(&l2, &r2, HexCompareMode::None, 8);
        let t2 = hex_gutter_total_rows(32);
        assert_eq!(
            hex_gutter_geometry_fast(t2, 200.0, &v2.hunks),
            hex_gutter_geometry(t2, 200.0, &v2.hunks)
        );
        // Ranges clamp honestly: past-the-end hunk clips, empty is empty.
        assert_eq!(
            hex_hunk_row_ranges(
                &[HexHunkView {
                    offset: 10_000,
                    left_len: 1,
                    right_len: 1,
                }],
                4
            ),
            Vec::new()
        );
        assert!(
            hex_gutter_geometry_fast(0, 400.0, &view.hunks)
                .segments
                .is_empty()
        );
        assert!(
            hex_gutter_geometry_fast(total, 400.0, &[])
                .segments
                .is_empty()
        );
    }

    #[test]
    fn hex_wave7_grid_rows_keep_fixed_char_budget() {
        // Zero clipped text: every row renders exactly 80 chars.
        let left: Vec<u8> = (0..16).collect();
        let right: Vec<u8> = (0..16).collect();
        let hunks = vec![HexHunkView {
            offset: 0,
            left_len: 16,
            right_len: 16,
        }];
        assert_eq!(HEX_GRID_ROW_CHARS, 80);
        for row in [0u64, 16, 32, 10_000] {
            let text = format_hex_grid_row_with_bytes(row, &left, &right, &hunks);
            assert_eq!(text.chars().count(), HEX_GRID_ROW_CHARS);
        }
        // Uncovered and insertion rows hold the same width (blanks, no `?`).
        let (.., e_hex, e_ascii) = format_hex_grid_cells_with_bytes(32, &left, &right, &hunks);
        assert_eq!(e_hex.chars().count(), 49);
        assert_eq!(e_ascii.chars().count(), 16);
    }

    #[test]
    fn hex_wave7_edge_fixture_tail_insertion_parity() {
        // Mirrors checks/gui-exq7/comparison/hex-sync-tail.json: 48 bytes,
        // diff at 20, tail insertion at 47; 3 grid rows.
        let left: Vec<u8> = (0..48).collect();
        let mut right: Vec<u8> = (0..48).collect();
        if let Some(slot) = right.get_mut(20) {
            *slot = 9;
        }
        right.push(0xFF);
        let view = HexCompareView::build_from_slices(&left, &right, HexCompareMode::None, 8);
        // Right side is 49 bytes: honest row count covers the tail band.
        assert_eq!(hex_gutter_total_rows(49), 4);
        assert_eq!(hex_sync_row_starts(0, 8, 49), vec![0, 16, 32, 48]);
        assert!(!view.hunks.is_empty());
        // Goto lands on the tail row; gutter fast path agrees with slow.
        let (tail_row, _) = hex_goto_anchor(47);
        assert_eq!(tail_row, 2);
        assert_eq!(hex_goto_row_start(47), 32);
        let total = hex_gutter_total_rows(49);
        assert_eq!(
            hex_gutter_geometry_fast(total, 300.0, &view.hunks),
            hex_gutter_geometry(total, 300.0, &view.hunks)
        );
        assert_eq!(
            hex_gutter_geometry(total, 300.0, &view.hunks)
                .segments
                .len(),
            2
        );
        // Fixed-width rows: tail included.
        let text = format_hex_grid_row_with_bytes(32, &left, &right, &view.hunks);
        assert_eq!(text.chars().count(), HEX_GRID_ROW_CHARS);
    }

    #[test]
    fn hex_wave7_sync_typo_reuses_mono_hierarchy() {
        use crate::theme::{TYPO_LINE_HEIGHT_MONO, TYPO_MONO_SIZE, TYPO_WEIGHT_REGULAR, TypoRole};
        for role in [hex_sync_pane_text_role(), hex_ruler_text_role()] {
            assert_eq!(role, TypoRole::Mono);
            assert!(role.is_mono());
            assert!(!role.is_chrome());
            assert!(role.is_tabular());
            assert_eq!(role.size(), TYPO_MONO_SIZE);
            assert_eq!(role.family(), "monospace");
            assert_eq!(role.weight_value(), TYPO_WEIGHT_REGULAR);
            assert_eq!(role.line_height(), TYPO_LINE_HEIGHT_MONO);
            assert_eq!(
                role.line_height_px(),
                TYPO_MONO_SIZE * TYPO_LINE_HEIGHT_MONO
            );
            assert_eq!(role.letter_spacing_em(), 0.0);
            // Hierarchy: grid/ruler content sits below the Body section
            // label (label 13 > content 12 > Small hint 11).
            assert!(role.rank() > TypoRole::Body.rank());
            assert!(TypoRole::Small.rank() > role.rank());
        }
    }

    #[test]
    fn hex_exq_hextable_pane_heights_share_text_budget() {
        use crate::viewmodel::{TEXT_PANES_MIN_H, text_panes_height};
        // Single source: hex floor text floor ke barabar, koi naya token nahi.
        assert_eq!(HEX_PANES_MIN_H, TEXT_PANES_MIN_H);
        assert_eq!(HEX_PANES_MIN_H, 100.0);
        // Delegation: hex budget text budget se kabhi drift nahi karta.
        assert_eq!(hex_panes_height(600.0), text_panes_height(600.0));
        assert_eq!(hex_panes_height(600.0), 600.0);
        assert_eq!(hex_panes_height(99.9), TEXT_PANES_MIN_H);
        assert_eq!(hex_panes_height(0.0), TEXT_PANES_MIN_H);
        assert_eq!(hex_panes_height(-50.0), TEXT_PANES_MIN_H);
        assert_eq!(hex_panes_height(f32::NAN), TEXT_PANES_MIN_H);
        assert_eq!(hex_panes_height(f32::INFINITY), TEXT_PANES_MIN_H);
    }

    #[test]
    fn hex_exq_hextable_gutter_markers_use_ruler_floor() {
        use crate::gutter::GUTTER_SEGMENT_MIN_H;
        // Tall file: 16_000 bytes = 1000 rows, height 400 -> 0.4px/row.
        // Slow (row-wise) aur fast (virtualized) dono 4px floor par.
        let left = vec![0u8; 16_000];
        let mut right = vec![0u8; 16_000];
        if let Some(slot) = right.get_mut(8000) {
            *slot = 9;
        }
        let view = HexCompareView::build_from_slices(&left, &right, HexCompareMode::None, 8);
        assert!(!view.hunks.is_empty());
        let total = hex_gutter_total_rows(16_000);
        assert_eq!(total, 1000);
        let slow = hex_gutter_geometry(total, 400.0, &view.hunks);
        let fast = hex_gutter_geometry_fast(total, 400.0, &view.hunks);
        assert_eq!(slow, fast);
        assert_eq!(slow.segments.len(), 1);
        let Some(seg) = slow.segments.first() else {
            return;
        };
        assert!((seg.y_end - seg.y_start - GUTTER_SEGMENT_MIN_H).abs() < 0.001);
        assert_eq!(GUTTER_SEGMENT_MIN_H, 4.0);
        // Minimap band bhi wahi floor: gutter marker se drift nahi.
        let bands = hex_minimap_bands(16_000, 400.0, &view.hunks);
        assert_eq!(bands.len(), 1);
        let Some((_, y1)) = bands.first().copied() else {
            return;
        };
        let Some((y0, _)) = bands.first().copied() else {
            return;
        };
        assert!((y1 - y0 - GUTTER_SEGMENT_MIN_H).abs() < 0.001);
    }

    #[test]
    fn hex_exq_hextable2_pane_heights_plus_cell_gaps() {
        // Geometry first: cell gaps table precedent reuse, pane heights
        // text floor reuse. Typography second: cell Mono, pane label Body.
        assert_eq!(HEX_CELL_PAD_X, 8.0);
        assert_eq!(hex_cell_pad_x(), HEX_CELL_PAD_X);
        assert_eq!(crate::theme::SPACE_SM, 8.0);
        assert_eq!(hex_cell_outer_w(100.0), 116.0);
        assert_eq!(hex_cell_text_w(116.0), 100.0);
        assert_eq!(hex_cell_outer_w(f32::NAN), 16.0);
        assert_eq!(hex_cell_text_w(f32::NAN), 0.0);
        // Pane heights: 18px rows me 600px -> 33 rows, floor 100 keeps 100.
        assert_eq!(hex_visible_rows(600.0, 18.0), 33);
        assert_eq!(hex_visible_rows(90.0, 18.0), 5);
        assert_eq!(hex_visible_rows(0.0, 18.0), 0);
        assert_eq!(hex_visible_rows(100.0, 0.0), 0);
        assert_eq!(hex_visible_rows(f32::NAN, 18.0), 0);
        assert_eq!(hex_panes_height_for_rows(33, 18.0), 594.0);
        assert_eq!(hex_panes_height_for_rows(1, 18.0), HEX_PANES_MIN_H);
        assert_eq!(hex_panes_height_for_rows(0, 18.0), HEX_PANES_MIN_H);
        // Gutter markers: single diff at 8000 -> row 500 marker.
        let left = vec![0u8; 16_000];
        let mut right = vec![0u8; 16_000];
        if let Some(slot) = right.get_mut(8000) {
            *slot = 9;
        }
        let view = HexCompareView::build_from_slices(&left, &right, HexCompareMode::None, 8);
        assert_eq!(hex_gutter_marker_rows(&view.hunks, 1000), vec![500]);
        assert!(hex_gutter_marker_rows(&[], 1000).is_empty());
        // Typo second: no new sizes, only role reuse.
        assert_eq!(
            hex_cell_text_role() as u8,
            crate::theme::TypoRole::Mono as u8
        );
        assert_eq!(
            hex_pane_label_text_role() as u8,
            crate::theme::TypoRole::Body as u8
        );
    }

    #[test]
    fn hex_exq_hextable2_edge_tail_truncate() {
        // Tail-truncate: 200 differing bytes need 4 chunks, budget 1
        // truncates honestly; tail row still maps, visible rows cut partial.
        let left = vec![0u8; 200];
        let right = vec![1u8; 200];
        let view = HexCompareView::build_from_slices(&left, &right, HexCompareMode::None, 1);
        assert_eq!(view.hunks.len(), 1);
        assert!(view.truncated);
        assert_eq!(hex_row_count(200), 13);
        // Truncated budget keeps head hunk only: row 0 covered, tail cut.
        assert!(hex_row_has_diff(0, &view.hunks));
        assert!(!hex_row_has_diff(192, &view.hunks));
        assert_eq!(hex_row_to_hunk(0, &view.hunks), Some(0));
        assert_eq!(hex_visible_rows(100.0, 18.0), 5);
        let starts = hex_sync_row_starts(10, 5, 200);
        assert_eq!(starts.len(), 3);
        // Full budget shows all 4 chunks, no truncation.
        let full = HexCompareView::build_from_slices(&left, &right, HexCompareMode::None, 8);
        assert_eq!(full.hunks.len(), 4);
        assert!(!full.truncated);
        assert!(hex_row_has_diff(192, &full.hunks));
        assert_eq!(hex_row_to_hunk(12, &full.hunks), Some(3));
    }

    #[test]
    fn hex_exq_hextable2_edge_empty_vs_nonempty() {
        // Empty-vs-nonempty: left empty, right 16 bytes -> one row, one
        // hunk side empty, markers honest, empty side panes floor.
        let left: Vec<u8> = Vec::new();
        let right = vec![7u8; 16];
        let view = HexCompareView::build_from_slices(&left, &right, HexCompareMode::None, 8);
        assert_eq!(hex_row_count(0), 0);
        assert_eq!(hex_row_count(16), 1);
        assert_eq!(hex_gutter_total_rows(0), 0);
        assert_eq!(hex_gutter_marker_rows(&view.hunks, 1).len(), 1);
        assert!(hex_gutter_marker_rows(&[], 0).is_empty());
        assert!(hex_sync_row_starts(0, 5, 0).is_empty());
        assert_eq!(hex_sync_row_starts(0, 5, 16), vec![0]);
        assert_eq!(hex_panes_height(0.0), HEX_PANES_MIN_H);
        assert_eq!(hex_visible_rows(HEX_PANES_MIN_H, 18.0), 5);
    }

    #[test]
    fn hex_exq_hextable3_edge_empty_vs_nonempty_64bit() {
        // Empty-vs-nonempty + 64-bit goto: empty side 0 rows, 16B side 1
        // row; u64::MAX goto parses, anchors without wrap, clamps honest.
        assert_eq!(hex_row_count(0), 0);
        assert_eq!(hex_row_count(16), 1);
        assert!(hex_sync_row_starts(0, 5, 0).is_empty());
        assert_eq!(parse_hex_goto("0xFFFFFFFFFFFFFFFF"), Some(u64::MAX));
        assert_eq!(parse_hex_goto(&u64::MAX.to_string()), Some(u64::MAX));
        assert_eq!(parse_hex_goto("18446744073709551616"), None);
        let (row, col) = hex_goto_anchor(u64::MAX);
        assert_eq!((row, col), hex_offset_to_row_col(u64::MAX));
        assert_eq!(hex_goto_row_start(u64::MAX), row.saturating_mul(16));
        assert_eq!(clamp_hex_goto(u64::MAX, u64::MAX), None);
        assert_eq!(clamp_hex_goto(u64::MAX - 1, u64::MAX), Some(u64::MAX - 1));
        assert_eq!(clamp_hex_goto(0, 0), None);
        // Geometry: 80-char budget splits 10 + 49 + 16 (+pads/pipes).
        assert_eq!(HEX_RULER_CHARS, 10);
        assert_eq!(HEX_HEX_PANE_CHARS, 49);
        assert_eq!(HEX_ASCII_PANE_CHARS, 16);
        assert_eq!(hex_ascii_gutter_w(), crate::theme::SPACE_SM);
        // Row height is the Mono line-height, no new size.
        assert!((hex_grid_row_h() - crate::theme::TypoRole::Mono.line_height_px()).abs() < 0.001);
        assert_eq!(
            hex_row_text_role() as u8,
            crate::theme::TypoRole::Mono as u8
        );
        // Pane split at 7px char: hex 343 + ascii 112, outers + 8 gutter.
        assert_eq!(hex_pane_text_widths(7.0), Some((343.0, 112.0)));
        assert!(hex_pane_text_widths(0.0).is_none());
        assert!(hex_pane_text_widths(f32::NAN).is_none());
        let Some((hex_outer, ascii_outer)) = hex_pane_outer_widths(7.0) else {
            return;
        };
        assert_eq!((hex_outer, ascii_outer), (359.0, 128.0));
        assert!(hex_pane_outer_widths(0.0).is_none());
        let grid_avail = hex_grid_avail_w(HEX_CANVAS_W, true);
        assert!(hex_panes_fit(grid_avail, 7.0));
        assert!(!hex_panes_fit(100.0, 7.0));
        assert!(!hex_panes_fit(f32::NAN, 7.0));
        // Goto bar min width: 120 field + 4 pad + 2x8 gaps = 140.
        assert_eq!(hex_goto_bar_min_w(), 140.0);
        assert_eq!(HEX_GOTO_FIELD_W, HEX_FIND_NEEDLE_W);
    }

    #[test]
    fn hexdense8_narrow_stacked_gutter_markers_stay_visible() {
        // Mirrors checks/gui-exq-hexdense8/round-1/comparison/
        // hexdense8-narrow-gutter.json: 600px narrow window, 64 bytes,
        // diffs at 20 (row 1) and 50 (row 3), line numbers off.
        // Geometry first: panes stack, wide gutter bar se chrome reserve
        // 52px, markers + jump targets wahi rehte hain. Typography second:
        // roles wahi Mono/Body (koi naya size nahi).
        assert_eq!(hex_layout_mode(600.0), HexLayoutMode::Stacked);
        // Ruler hamesha visible: flag off par bhi wide bar (12px).
        assert_eq!(hex_gutter_bar_width(false), 12.0);
        assert_eq!(hex_chrome_reserve_w(false), 52.0);
        assert_eq!(hex_grid_avail_w(600.0, false), 548.0);
        // Dense panes fit: outers (359 + 128) + 8 gutter = 495 <= 548.
        assert!(hex_panes_fit(hex_grid_avail_w(600.0, false), 7.0));
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
        assert_eq!(hex_gutter_marker_rows(&view.hunks, total), vec![1, 3]);
        // Gutter click row 1 (y=100, 75px rows) par covering hunk 0.
        assert_eq!(
            hex_gutter_jump_target(&view.hunks, total, 300.0, 100.0),
            Some(0)
        );
        // Typo parity: cell/pane/ruler Mono, labels Body — koi naya token nahi.
        assert_eq!(hex_cell_text_role(), crate::theme::TypoRole::Mono);
        assert_eq!(hex_sync_pane_text_role(), crate::theme::TypoRole::Mono);
        assert_eq!(hex_ruler_text_role(), crate::theme::TypoRole::Mono);
        assert_eq!(hex_pane_label_text_role(), crate::theme::TypoRole::Body);
    }

    #[test]
    fn hex_exq_hexview10_ruler_headers_lock_shapes() {
        // Ruler header shapes body cells se char-barabar (10/49/18) taaki
        // alag Grids hone par bhi columns align rahen. Koi naya size nahi.
        assert_eq!(hex_ruler_col_label(0), Some(String::from("00")));
        assert_eq!(hex_ruler_col_label(9), Some(String::from("09")));
        assert_eq!(hex_ruler_col_label(10), Some(String::from("0A")));
        assert_eq!(hex_ruler_col_label(15), Some(String::from("0F")));
        assert_eq!(hex_ruler_col_label(16), None);
        assert_eq!(hex_ruler_col_label(u64::MAX), None);
        let hex_header = hex_ruler_hex_header();
        assert_eq!(hex_header.chars().count(), HEX_HEX_PANE_CHARS);
        assert_eq!(hex_header.chars().count(), 49);
        assert!(hex_header.contains("00"));
        assert!(hex_header.contains("0F"));
        assert_eq!(hex_ruler_offset_header().chars().count(), HEX_RULER_CHARS);
        assert_eq!(hex_ruler_ascii_header().chars().count(), 18);
        assert_eq!(hex_ruler_ascii_header(), String::from("|0123456789ABCDEF|"));
        // Body cell shapes se barabar: addr 10, hex 49, |ascii| 18.
        let left: Vec<u8> = (0..16).collect();
        let right: Vec<u8> = (0..16).collect();
        let hunks = vec![HexHunkView {
            offset: 0,
            left_len: 16,
            right_len: 16,
        }];
        let (addr, hex, ascii) = format_hex_grid_cells_with_bytes(0, &left, &right, &hunks);
        assert_eq!(
            addr.chars().count(),
            hex_ruler_offset_header().chars().count()
        );
        assert_eq!(hex.chars().count(), hex_header.chars().count());
        assert_eq!(
            format!("|{ascii}|").chars().count(),
            hex_ruler_ascii_header().chars().count()
        );
        // Typo second: ruler + row ek hi Mono role, koi naya size nahi.
        assert_eq!(hex_row_text_role(), crate::theme::TypoRole::Mono);
        assert_eq!(hex_ruler_text_role(), crate::theme::TypoRole::Mono);
    }

    #[test]
    fn hex_exq_hexview10_edge_narrow_stacked_tail() {
        // Mirrors checks/gui-exq-hexview10/round-1/comparison/
        // narrow-stacked-tail.json: 700px narrow window (stacked), 48 bytes
        // + tail push = 49 bytes, diff row 1 + tail row 3 me.
        assert_eq!(hex_layout_mode(700.0), HexLayoutMode::Stacked);
        assert_eq!(hex_layout_mode(960.0), HexLayoutMode::SideBySide);
        assert_eq!(hex_row_count(49), 4);
        assert_eq!(hex_sync_row_starts(0, 8, 49), vec![0, 16, 32, 48]);
        // Stacked visual rows = grid rows x 2 (hex line + ASCII line).
        let starts = hex_sync_row_starts(0, 8, 49);
        assert_eq!(starts.len().saturating_mul(2), 8);
        let left: Vec<u8> = (0..48).collect();
        let mut right: Vec<u8> = (0..48).collect();
        if let Some(slot) = right.get_mut(20) {
            *slot = 9;
        }
        right.push(0xFF);
        let view = HexCompareView::build_from_slices(&left, &right, HexCompareMode::None, 8);
        assert!(!view.hunks.is_empty());
        assert_eq!(hex_gutter_marker_rows(&view.hunks, 4), vec![1, 3]);
        // Panes geometry: 300px me 15.6 row se 19 rows, tail row andar.
        assert!((hex_grid_row_h() - 15.6).abs() < 0.001);
        assert_eq!(hex_visible_rows(300.0, hex_grid_row_h()), 19);
        assert_eq!(hex_chrome_reserve_w(false), 52.0);
        assert_eq!(hex_grid_avail_w(700.0, false), 648.0);
        assert!(hex_panes_fit(hex_grid_avail_w(700.0, false), 7.0));
        // Tail row real bytes ke saath poori 80-char budget me.
        let text = format_hex_grid_row_with_bytes(48, &left, &right, &view.hunks);
        assert_eq!(text.chars().count(), HEX_GRID_ROW_CHARS);
    }

    #[test]
    fn hex_exq_hextable10_edge_narrow_gutter_cells() {
        // Mirrors checks/gui-exq-hextable10/round-1/comparison/hex-narrow.json:
        // 640px narrow window stacks (Stacked mode), 32 bytes (2 rows, row 1 diff).
        let narrow_w = 640.0;
        let wide_w = 960.0;
        let char_w = 7.0;

        assert_eq!(hex_layout_mode(narrow_w), HexLayoutMode::Stacked);
        assert_eq!(hex_layout_mode(wide_w), HexLayoutMode::SideBySide);

        // Chrome reserve and grid avail
        assert_eq!(hex_chrome_reserve_w(false), 52.0);
        assert_eq!(hex_grid_origin_x(false), 52.0);
        let avail = hex_grid_avail_w(narrow_w, false);
        assert_eq!(avail, 588.0);
        assert!(hex_stacked_panes_fit(narrow_w, char_w));

        // 32 bytes -> 2 rows, stacked visual rows -> 4
        assert_eq!(hex_row_count(32), 2);
        assert_eq!(hex_stacked_visual_rows(2), 4);
        assert_eq!(hex_stacked_row_y(0, false), 0.0);
        assert!((hex_stacked_row_y(0, true) - 15.6).abs() < 0.001);
        assert!((hex_stacked_row_y(1, false) - 31.2).abs() < 0.001);
        assert!((hex_stacked_row_y(1, true) - 46.8).abs() < 0.001);

        // Byte cell positioning at 7px char:
        // col 0: 0 * 3 * 7 = 0.0
        assert_eq!(hex_grid_byte_col_x(0, char_w), Some(0.0));
        // col 7: 7 * 3 * 7 = 147.0
        assert_eq!(hex_grid_byte_col_x(7, char_w), Some(147.0));
        // col 8: (8 * 3 + 1) * 7 = 175.0 (midline gap of 7px)
        assert_eq!(hex_grid_byte_col_x(8, char_w), Some(175.0));
        assert_eq!(
            hex_grid_byte_col_x(8, char_w).unwrap_or(0.0)
                - hex_grid_byte_col_x(7, char_w).unwrap_or(0.0),
            28.0 // 4 chars (3 + 1 midline) * 7px = 28px
        );
        assert_eq!(hex_grid_byte_col_x(16, char_w), None);

        // ASCII cell positioning at 7px char:
        // col 0 sits after pipe at 1 * 7 = 7.0
        assert_eq!(hex_grid_ascii_col_x(0, char_w), Some(7.0));
        assert_eq!(hex_grid_ascii_col_x(15, char_w), Some(112.0));
        assert_eq!(hex_grid_ascii_col_x(16, char_w), None);

        // Cell sizes and padding
        assert_eq!(hex_cell_pad_x(), crate::theme::SPACE_SM);
        assert_eq!(hex_grid_byte_cell_text_w(char_w), 14.0);
        assert_eq!(hex_grid_byte_cell_box_w(char_w), 30.0); // 14 + 2 * 8

        // Gutter marker on row 1
        let left: Vec<u8> = (0..32).collect();
        let mut right = left.clone();
        if let Some(slot) = right.get_mut(20) {
            *slot = 0xAA;
        }
        let view = HexCompareView::build_from_slices(&left, &right, HexCompareMode::None, 8);
        assert_eq!(hex_gutter_marker_rows(&view.hunks, 2), vec![1]);
        assert!((hex_grid_row_y(1) - 15.6).abs() < 0.001);
    }

    #[test]
    fn hex_exq_hex2_round2_origin_ruler_midline() {
        // Mirrors checks/gui-exq-hex2/round-1/comparison/hex-narrow-720.json:
        // 720px breakpoint edge — 720 stays SideBySide, 719 stacks; 32
        // bytes (2 rows, row 1 diff); origin x = chrome reserve (52px),
        // origin y = ruler row (Mono 15.6) + one tight gap (4.0).
        assert_eq!(hex_layout_mode(720.0), HexLayoutMode::SideBySide);
        assert_eq!(hex_layout_mode(719.0), HexLayoutMode::Stacked);
        // Row height is the Mono line-height, ruler locks to the same row.
        assert!((hex_grid_row_h() - crate::theme::TypoRole::Mono.line_height_px()).abs() < 0.001);
        assert!((hex_grid_row_h() - 15.6).abs() < 0.001);
        assert!((hex_ruler_row_h() - hex_grid_row_h()).abs() < 0.001);
        // Ruler tick spacing reuses SPACE tokens, no new rhythm.
        assert_eq!(hex_ruler_grid_gap_x(), crate::theme::GEO_SECTION_GAP);
        assert_eq!(hex_ruler_grid_gap_x(), crate::theme::SPACE_SM);
        assert_eq!(hex_ruler_grid_gap_y(), crate::theme::GEO_TIGHT_GAP);
        assert_eq!(hex_ruler_grid_gap_y(), crate::theme::SPACE_XS);
        // Grid origin alignment: x = chrome reserve, y = ruler + gap.
        assert_eq!(hex_grid_origin_x(false), 52.0);
        assert_eq!(hex_chrome_reserve_w(false), 52.0);
        assert!((hex_grid_origin_y() - 19.6).abs() < 0.001);
        assert_eq!(hex_grid_origin(false), (52.0, hex_grid_origin_y()));
        let avail = hex_grid_avail_w(720.0, false);
        assert_eq!(avail, 668.0);
        let char_w = 7.0;
        // Midline gap: one shared char, 7px at 7px char.
        assert_eq!(HEX_MIDLINE_GAP_CHARS, 1);
        assert_eq!(hex_midline_gap_w(char_w), 7.0);
        assert_eq!(hex_midline_gap_w(0.0), 0.0);
        assert_eq!(hex_midline_gap_w(f32::NAN), 0.0);
        assert_eq!(hex_grid_byte_col_x(7, char_w), Some(147.0));
        assert_eq!(hex_grid_byte_col_x(8, char_w), Some(175.0));
        assert_eq!(
            hex_grid_byte_col_x(8, char_w).unwrap_or(0.0)
                - hex_grid_byte_col_x(7, char_w).unwrap_or(0.0),
            4.0 * char_w
        );
        // Stacked + gutter parity at the 720 edge.
        assert!(hex_stacked_panes_fit(720.0, char_w));
        assert_eq!(hex_layout_mode(720.0), HexLayoutMode::SideBySide);
        assert_eq!(hex_row_count(32), 2);
        assert_eq!(hex_stacked_visual_rows(2), 4);
        assert!((hex_grid_row_y(1) - hex_ruler_row_h()).abs() < 0.001);
        let left: Vec<u8> = (0..32).collect();
        let mut right = left.clone();
        if let Some(slot) = right.get_mut(20) {
            *slot = 0xAA;
        }
        let view = HexCompareView::build_from_slices(&left, &right, HexCompareMode::None, 8);
        assert_eq!(hex_gutter_marker_rows(&view.hunks, 2), vec![1]);
        // Typo second: grid + ruler stay Mono, no new sizes.
        assert_eq!(hex_row_text_role(), crate::theme::TypoRole::Mono);
        assert_eq!(hex_ruler_text_role(), crate::theme::TypoRole::Mono);
    }

    #[test]
    fn hex_exq_hex3_round3_ruler_ticks_and_grid_wide_960() {
        // Headless test proving edge fixture comparison/hex-wide-960.json:
        // 960px canvas reference width (SideBySide mode); ruler tick spacing
        // symmetric via SPACE tokens (SPACE_SM 8.0px spacing, SPACE_XS 4.0px pad);
        // nibble group gap derived from midline helper (7.0px at char_w 7.0px);
        // row height locked to Mono line-height (15.6px) at 960px wide.
        let fixture_raw = include_str!("../../../checks/gui-exq-hex3/comparison/hex-wide-960.json");
        let fixture_r1 =
            include_str!("../../../checks/gui-exq-hex3/round-1/comparison/hex-wide-960.json");
        assert_eq!(fixture_raw, fixture_r1);
        assert!(fixture_raw.contains("\"name\": \"hex-wide-960\""));
        assert!(fixture_raw.contains("\"canvas_w\": 960.0"));
        assert!(fixture_raw.contains("\"canvas_h\": 600.0"));
        assert!(fixture_raw.contains("\"layout_mode\": \"SideBySide\""));
        assert!(fixture_raw.contains("\"row_h\": 15.6"));
        assert!(fixture_raw.contains("\"ruler_grid_gap_x\": 8.0"));
        assert!(fixture_raw.contains("\"ruler_grid_gap_y\": 4.0"));
        assert!(fixture_raw.contains("\"ruler_tick_spacing\": 8.0"));
        assert!(fixture_raw.contains("\"ruler_tick_sym_pad\": 4.0"));
        assert!(fixture_raw.contains("\"midline_gap_px\": 7.0"));
        assert!(fixture_raw.contains("\"nibble_group_gap_px\": 7.0"));
        assert!(fixture_raw.contains("\"visible_rows\": 30"));

        // 1. 960px canvas layout and row height locked to Mono line-height
        assert_eq!(HEX_CANVAS_W, 960.0);
        assert_eq!(HEX_CANVAS_H, 600.0);
        assert_eq!(hex_layout_mode(HEX_CANVAS_W), HexLayoutMode::SideBySide);
        assert!((hex_wide_row_h() - 15.6).abs() < 0.001);
        assert!((hex_wide_row_h() - hex_grid_row_h()).abs() < 0.001);
        assert!((hex_wide_row_h() - hex_ruler_row_h()).abs() < 0.001);
        assert!((hex_wide_row_h() - crate::theme::TypoRole::Mono.line_height_px()).abs() < 0.001);

        // Chrome reserve and grid availability at 960px wide
        assert_eq!(hex_chrome_reserve_w(false), 52.0);
        assert_eq!(hex_grid_avail_w(960.0, false), 908.0);
        assert_eq!(hex_grid_origin_x(false), 52.0);
        assert!((hex_grid_origin_y() - 19.6).abs() < 0.001);
        assert_eq!(hex_wide_960_visible_rows(), 30);

        // 2. Ruler tick spacing symmetric via SPACE tokens
        assert_eq!(hex_ruler_tick_spacing(), crate::theme::SPACE_SM);
        assert_eq!(hex_ruler_tick_spacing(), crate::theme::GEO_SECTION_GAP);
        assert_eq!(hex_ruler_tick_spacing(), 8.0);
        assert_eq!(hex_ruler_tick_sym_pad(), crate::theme::SPACE_XS);
        assert_eq!(hex_ruler_tick_sym_pad(), crate::theme::GEO_TIGHT_GAP);
        assert_eq!(hex_ruler_tick_sym_pad(), 4.0);
        assert_eq!(hex_ruler_grid_gap_x(), crate::theme::SPACE_SM);
        assert_eq!(hex_ruler_grid_gap_y(), crate::theme::SPACE_XS);

        // 3. Nibble group gap derived from midline helper
        let char_w = 7.0;
        assert_eq!(hex_midline_gap_w(char_w), 7.0);
        assert_eq!(hex_nibble_group_gap_w(char_w), hex_midline_gap_w(char_w));
        assert_eq!(hex_nibble_group_gap_w(char_w), 7.0);
        assert_eq!(hex_nibble_group_gap_w(0.0), 0.0);
        assert_eq!(hex_nibble_group_gap_w(-1.0), 0.0);
        assert_eq!(hex_nibble_group_gap_w(f32::NAN), 0.0);

        // Ruler column and grid column alignment parity (col 0..15)
        for col in 0..16 {
            let r_x = hex_ruler_col_x(col, char_w);
            let g_x = hex_grid_byte_col_x(col, char_w);
            assert_eq!(r_x, g_x);
        }
        assert_eq!(hex_ruler_col_x(0, char_w), Some(0.0));
        assert_eq!(hex_ruler_col_x(7, char_w), Some(147.0));
        assert_eq!(hex_ruler_col_x(8, char_w), Some(175.0));
        assert_eq!(
            hex_ruler_col_x(8, char_w).unwrap() - hex_ruler_col_x(7, char_w).unwrap(),
            28.0 // 4 chars (3 + 1 midline gap) * 7.0px = 28.0px
        );
        assert_eq!(hex_ruler_col_x(16, char_w), None);
    }

    #[test]
    fn hex_exq_hex4_grid_plus_offset_wide_960() {
        // Headless test proving edge fixture comparison/hex-wide-960.json:
        // even byte grid (3-char step from char metrics), offset gutter
        // from SPACE tokens, row height locked to Mono at 960px wide.
        let fixture_raw = include_str!("../../../checks/gui-exq-hex4/comparison/hex-wide-960.json");
        let fixture_r1 =
            include_str!("../../../checks/gui-exq-hex4/round-1/comparison/hex-wide-960.json");
        assert_eq!(fixture_raw, fixture_r1);
        assert!(fixture_raw.contains("\"name\": \"hex-wide-960\""));
        assert!(fixture_raw.contains("\"canvas_w\": 960.0"));
        assert!(fixture_raw.contains("\"layout_mode\": \"SideBySide\""));
        assert!(fixture_raw.contains("\"row_h\": 15.6"));
        assert!(fixture_raw.contains("\"offset_gutter_w\": 78.0"));
        assert!(fixture_raw.contains("\"byte_col_step_px\": 21.0"));
        assert!(fixture_raw.contains("\"visible_rows\": 30"));
        // 1. Row height locked to Mono (grid + ruler + offset share one rhythm).
        assert!((hex_even_row_h() - 15.6).abs() < 0.001);
        assert!((hex_offset_row_h() - hex_grid_row_h()).abs() < 0.001);
        assert!((hex_offset_row_h() - hex_ruler_row_h()).abs() < 0.001);
        assert!((hex_even_row_h() - crate::theme::TypoRole::Mono.line_height_px()).abs() < 0.001);
        assert!((hex_wide_row_h() - hex_even_row_h()).abs() < 0.001);
        assert_eq!(hex_layout_mode(HEX_CANVAS_W), HexLayoutMode::SideBySide);
        assert_eq!(hex_wide_960_visible_rows(), 30);
        // 2. Byte col widths from char metrics (even 3-char step, 7px char).
        let char_w = 7.0;
        assert_eq!(hex_byte_col_step_w(char_w), 21.0);
        assert_eq!(hex_byte_col_step_w(0.0), 0.0);
        assert_eq!(hex_byte_col_step_w(f32::NAN), 0.0);
        assert_eq!(hex_grid_byte_col_x(0, char_w), Some(0.0));
        assert_eq!(hex_grid_byte_col_x(7, char_w), Some(147.0));
        assert_eq!(hex_grid_byte_col_x(8, char_w), Some(175.0));
        assert_eq!(
            hex_grid_byte_col_x(8, char_w).unwrap_or(0.0)
                - hex_grid_byte_col_x(7, char_w).unwrap_or(0.0),
            28.0
        );
        assert!(hex_even_grid_ok(char_w));
        assert!(!hex_even_grid_ok(0.0));
        assert!(!hex_even_grid_ok(f32::NAN));
        // 3. Offset gutter from SPACE tokens at 960px wide.
        assert_eq!(hex_offset_gutter_pad(), crate::theme::SPACE_XS);
        assert_eq!(hex_offset_gutter_pad(), crate::theme::GEO_TIGHT_GAP);
        assert_eq!(hex_offset_gutter_pad(), 4.0);
        assert_eq!(hex_offset_gutter_gap(), crate::theme::SPACE_SM);
        assert_eq!(hex_offset_gutter_gap(), crate::theme::GEO_SECTION_GAP);
        assert_eq!(hex_offset_gutter_gap(), 8.0);
        assert_eq!(hex_offset_gutter_text_w(char_w), 70.0);
        assert_eq!(hex_offset_gutter_text_w(0.0), 0.0);
        assert_eq!(hex_offset_gutter_w(char_w), 78.0);
        assert_eq!(hex_grid_avail_w(960.0, false), 908.0);
        assert_eq!(hex_grid_origin_x(false), 52.0);
        // 4. ASCII pane in sync (even 1-char step behind the pipe).
        assert!(hex_ascii_sync_ok(char_w));
        assert!(!hex_ascii_sync_ok(0.0));
        assert_eq!(hex_grid_ascii_col_x(0, char_w), Some(7.0));
        assert_eq!(hex_grid_ascii_col_x(15, char_w), Some(112.0));
        // Typo second: gutter + grid + ASCII all Mono, no new sizes.
        assert_eq!(hex_offset_text_role(), crate::theme::TypoRole::Mono);
        assert_eq!(hex_row_text_role(), crate::theme::TypoRole::Mono);
        assert_eq!(hex_ruler_text_role(), crate::theme::TypoRole::Mono);
    }

    #[test]
    fn hex_exq_hex5_grid_plus_ascii_wide_960() {
        // Headless test proving edge fixture comparison/hex-wide-960.json:
        // nibble gap single-sourced from midline, ASCII pane x from grid
        // origin, ruler tick spacing via SPACE_SM, row height locked to
        // Mono line-height at 960px wide.
        let fixture_raw = include_str!("../../../checks/gui-exq-hex5/comparison/hex-wide-960.json");
        let fixture_r1 =
            include_str!("../../../checks/gui-exq-hex5/round-1/comparison/hex-wide-960.json");
        assert_eq!(fixture_raw, fixture_r1);
        assert!(fixture_raw.contains("\"name\": \"hex-wide-960\""));
        assert!(fixture_raw.contains("\"canvas_w\": 960.0"));
        assert!(fixture_raw.contains("\"layout_mode\": \"SideBySide\""));
        assert!(fixture_raw.contains("\"row_h\": 15.6"));
        assert!(fixture_raw.contains("\"nibble_group_gap_px\": 7.0"));
        assert!(fixture_raw.contains("\"midline_gap_px\": 7.0"));
        assert!(fixture_raw.contains("\"ruler_tick_spacing\": 8.0"));
        assert!(fixture_raw.contains("\"ascii_pane_x\": 505.0"));
        assert!(fixture_raw.contains("\"visible_rows\": 30"));
        // 1. Nibble group gap from single-source midline helper.
        let char_w = 7.0;
        assert_eq!(HEX_MIDLINE_GAP_CHARS, 1);
        assert_eq!(hex_midline_gap_w(char_w), 7.0);
        assert_eq!(hex_nibble_group_gap_w(char_w), hex_midline_gap_w(char_w));
        assert_eq!(hex_nibble_group_gap_w(char_w), 7.0);
        // 2. ASCII pane x from grid origin (no drift).
        assert_eq!(hex_grid_origin_x(false), 52.0);
        assert_eq!(hex_offset_gutter_w(char_w), 78.0);
        assert_eq!(hex_offset_gutter_gap(), 8.0);
        assert_eq!(hex_byte_pane_x(false, char_w), Some(138.0));
        assert_eq!(hex_ascii_pane_x(false, char_w), Some(505.0));
        assert_eq!(hex_byte_pane_x(false, 0.0), None);
        assert_eq!(hex_ascii_pane_x(false, f32::NAN), None);
        // Byte + ruler absolute ticks share the pane origin.
        assert_eq!(hex_ruler_tick_abs_x(false, 0, char_w), Some(138.0));
        assert_eq!(hex_ruler_tick_abs_x(false, 7, char_w), Some(285.0));
        assert_eq!(hex_ruler_tick_abs_x(false, 8, char_w), Some(313.0));
        assert_eq!(hex_ruler_tick_abs_x(false, 16, char_w), None);
        // 3. Ruler tick spacing symmetric via SPACE_SM.
        assert_eq!(hex_ruler_tick_spacing(), crate::theme::SPACE_SM);
        assert_eq!(hex_ruler_tick_spacing(), 8.0);
        assert_eq!(hex_ruler_tick_sym_pad(), crate::theme::SPACE_XS);
        assert_eq!(hex_ruler_tick_sym_pad(), 4.0);
        // 4. Row height locked to Mono line-height at 960px wide.
        assert_eq!(hex_layout_mode(HEX_CANVAS_W), HexLayoutMode::SideBySide);
        assert!((hex_wide_row_h() - crate::theme::TypoRole::Mono.line_height_px()).abs() < 0.001);
        assert!((hex_wide_row_h() - 15.6).abs() < 0.001);
        assert!((hex_grid_row_h() - hex_ruler_row_h()).abs() < 0.001);
        assert!((hex_even_row_h() - hex_offset_row_h()).abs() < 0.001);
        assert_eq!(hex_wide_960_visible_rows(), 30);
        // 5. Grid plus offset sync as one check.
        assert!(hex_grid_plus_offset_sync_ok(char_w));
        assert!(!hex_grid_plus_offset_sync_ok(0.0));
        assert!(!hex_grid_plus_offset_sync_ok(f32::NAN));
        // Typo second: grid + ruler + offset all Mono.
        assert_eq!(hex_row_text_role(), crate::theme::TypoRole::Mono);
        assert_eq!(hex_ruler_text_role(), crate::theme::TypoRole::Mono);
        assert_eq!(hex_offset_text_role(), crate::theme::TypoRole::Mono);
    }

    #[test]
    fn hex_exq_hex6_nibble_plus_ascii_wide_960() {
        // Headless test proving edge fixture comparison/hex-wide-960.json:
        // nibble gap via SPACE-token rhythm (1-char midline gap beside
        // SPACE_SM/SPACe_XS spacings), ASCII pane x from grid origin
        // helper, ruler tick step from char metrics at 960px wide.
        let fixture_raw = include_str!("../../../checks/gui-exq-hex6/comparison/hex-wide-960.json");
        let fixture_r1 =
            include_str!("../../../checks/gui-exq-hex6/round-1/comparison/hex-wide-960.json");
        assert_eq!(fixture_raw, fixture_r1);
        assert!(fixture_raw.contains("\"name\": \"hex-wide-960\""));
        assert!(fixture_raw.contains("\"canvas_w\": 960.0"));
        assert!(fixture_raw.contains("\"layout_mode\": \"SideBySide\""));
        assert!(fixture_raw.contains("\"row_h\": 15.6"));
        assert!(fixture_raw.contains("\"nibble_group_gap_px\": 7.0"));
        assert!(fixture_raw.contains("\"midline_gap_px\": 7.0"));
        assert!(fixture_raw.contains("\"ruler_tick_spacing\": 8.0"));
        assert!(fixture_raw.contains("\"ruler_tick_step_px\": 21.0"));
        assert!(fixture_raw.contains("\"ascii_pane_x\": 505.0"));
        assert!(fixture_raw.contains("\"visible_rows\": 30"));
        // 1. Nibble gap via SPACE-token rhythm + symmetry.
        let char_w = 7.0;
        assert_eq!(HEX_MIDLINE_GAP_CHARS, 1);
        assert_eq!(hex_midline_gap_w(char_w), 7.0);
        assert_eq!(hex_nibble_group_gap_w(char_w), hex_midline_gap_w(char_w));
        assert!(hex_nibble_gap_space_rhythm_ok(char_w));
        assert!(!hex_nibble_gap_space_rhythm_ok(0.0));
        assert!(!hex_nibble_gap_space_rhythm_ok(f32::NAN));
        assert!(hex_nibble_gap_symmetry_ok(char_w));
        assert!(!hex_nibble_gap_symmetry_ok(0.0));
        assert!(!hex_nibble_gap_symmetry_ok(f32::NAN));
        assert_eq!(hex_grid_byte_col_x(7, char_w), Some(147.0));
        assert_eq!(hex_grid_byte_col_x(8, char_w), Some(175.0));
        assert_eq!(
            hex_grid_byte_col_x(8, char_w).unwrap_or(0.0)
                - hex_grid_byte_col_x(7, char_w).unwrap_or(0.0),
            28.0
        );
        // 2. ASCII pane x from grid origin helper.
        assert_eq!(hex_grid_origin_x(false), 52.0);
        assert_eq!(hex_byte_pane_x(false, char_w), Some(138.0));
        assert_eq!(hex_ascii_pane_x(false, char_w), Some(505.0));
        assert!(hex_ascii_pane_from_origin_ok(false, char_w));
        assert!(!hex_ascii_pane_from_origin_ok(false, 0.0));
        assert_eq!(hex_ruler_tick_abs_x(false, 0, char_w), Some(138.0));
        assert_eq!(hex_ruler_tick_abs_x(false, 7, char_w), Some(285.0));
        assert_eq!(hex_ruler_tick_abs_x(false, 8, char_w), Some(313.0));
        // 3. Ruler tick step from char metrics at 960px wide.
        assert_eq!(hex_ruler_tick_step_w(char_w), 21.0);
        assert_eq!(hex_ruler_tick_step_w(char_w), hex_byte_col_step_w(char_w));
        assert_eq!(hex_ruler_tick_step_w(0.0), 0.0);
        assert_eq!(hex_wide_960_ruler_tick_step(), 21.0);
        assert_eq!(hex_layout_mode(HEX_CANVAS_W), HexLayoutMode::SideBySide);
        assert!((hex_wide_row_h() - crate::theme::TypoRole::Mono.line_height_px()).abs() < 0.001);
        assert_eq!(hex_wide_960_visible_rows(), 30);
        assert!(hex_grid_plus_offset_sync_ok(char_w));
        // Typo second: grid + ruler + offset all Mono.
        assert_eq!(hex_row_text_role(), crate::theme::TypoRole::Mono);
        assert_eq!(hex_ruler_text_role(), crate::theme::TypoRole::Mono);
        assert_eq!(hex_offset_text_role(), crate::theme::TypoRole::Mono);
    }

    #[test]
    fn hex_exq_hex7_offset_plus_gap_wide_960() {
        // Headless test proving edge fixture comparison/hex-offset-960.json:
        // offset column width from char metrics for 8-digit addresses,
        // ASCII pane gap from SPACE tokens, byte cell size from TypoRole
        // Mono metrics at 960px wide.
        let fixture_raw =
            include_str!("../../../checks/gui-exq-hex7/comparison/hex-offset-960.json");
        let fixture_r1 =
            include_str!("../../../checks/gui-exq-hex7/round-1/comparison/hex-offset-960.json");
        assert_eq!(fixture_raw, fixture_r1);
        assert!(fixture_raw.contains("\"name\": \"hex-offset-960\""));
        assert!(fixture_raw.contains("\"canvas_w\": 960.0"));
        assert!(fixture_raw.contains("\"layout_mode\": \"SideBySide\""));
        assert!(fixture_raw.contains("\"row_h\": 15.6"));
        assert!(fixture_raw.contains("\"offset_addr_chars\": 10"));
        assert!(fixture_raw.contains("\"offset_hex_digits\": 8"));
        assert!(fixture_raw.contains("\"offset_widest_label\": \"0xffffffff\""));
        assert!(fixture_raw.contains("\"offset_gutter_text_w\": 70.0"));
        assert!(fixture_raw.contains("\"offset_gutter_w\": 78.0"));
        assert!(fixture_raw.contains("\"byte_cell_text_w\": 14.0"));
        assert!(fixture_raw.contains("\"byte_cell_box_w\": 30.0"));
        assert!(fixture_raw.contains("\"byte_col_step_px\": 21.0"));
        assert!(fixture_raw.contains("\"ascii_pane_gutter_w\": 8.0"));
        assert!(fixture_raw.contains("\"ascii_pane_x\": 505.0"));
        assert!(fixture_raw.contains("\"visible_rows\": 30"));
        // 1. Offset column width from char metrics for 8-digit addresses.
        let char_w = 7.0;
        assert_eq!(HEX_OFFSET_HEX_DIGITS, 8);
        assert_eq!(HEX_OFFSET_PREFIX_CHARS, 2);
        assert_eq!(HEX_OFFSET_ADDR_CHARS, 10);
        assert_eq!(HEX_OFFSET_ADDR_CHARS, HEX_RULER_CHARS);
        assert_eq!(hex_offset_addr_chars(), HEX_RULER_CHARS);
        assert_eq!(format_hex_offset(u32::MAX as u64), "0xffffffff");
        assert_eq!(
            format_hex_offset(u32::MAX as u64).len(),
            HEX_OFFSET_ADDR_CHARS
        );
        assert_eq!(hex_offset_gutter_text_w(char_w), 70.0);
        assert_eq!(hex_offset_960_text_w(), 70.0);
        assert_eq!(hex_offset_gutter_w(char_w), 78.0);
        assert_eq!(hex_offset_960_gutter_w(), 78.0);
        assert_eq!(hex_offset_gutter_pad(), 4.0);
        assert_eq!(hex_offset_gutter_gap(), 8.0);
        assert!(hex_offset_width_fit_ok(char_w));
        assert!(!hex_offset_width_fit_ok(0.0));
        assert!(!hex_offset_width_fit_ok(f32::NAN));
        assert!(!hex_offset_width_fit_ok(f32::INFINITY));
        // 2. ASCII pane gap from SPACE tokens.
        assert_eq!(hex_ascii_gutter_w(), crate::theme::SPACE_SM);
        assert_eq!(hex_ascii_gutter_w(), 8.0);
        assert_eq!(hex_ascii_gutter_w(), crate::theme::GEO_SECTION_GAP);
        assert_eq!(hex_byte_pane_x(false, char_w), Some(138.0));
        assert_eq!(hex_ascii_pane_x(false, char_w), Some(505.0));
        assert!(hex_ascii_gap_space_ok(false, char_w));
        assert!(!hex_ascii_gap_space_ok(false, 0.0));
        assert!(!hex_ascii_gap_space_ok(false, f32::NAN));
        assert!(hex_ascii_pane_from_origin_ok(false, char_w));
        // 3. Byte cell size from TypoRole Mono metrics at 960px wide.
        assert_eq!(crate::theme::TypoRole::Mono.size(), 12.0);
        assert!((crate::theme::TypoRole::Mono.line_height_px() - 15.6).abs() < 0.001);
        assert_eq!(hex_grid_byte_cell_text_w(char_w), 14.0);
        assert_eq!(hex_byte_cell_960_text_w(), 14.0);
        assert_eq!(hex_grid_byte_cell_box_w(char_w), 30.0);
        assert_eq!(hex_byte_cell_960_box_w(), 30.0);
        assert_eq!(hex_byte_col_step_w(char_w), 21.0);
        assert!(hex_byte_cell_960_ok(char_w));
        assert!(!hex_byte_cell_960_ok(0.0));
        assert!(!hex_byte_cell_960_ok(f32::NAN));
        assert_eq!(hex_layout_mode(HEX_CANVAS_W), HexLayoutMode::SideBySide);
        assert_eq!(hex_wide_960_visible_rows(), 30);
        assert_eq!(hex_visible_rows(470.0, hex_grid_row_h()), 30);
        assert!(hex_offset_plus_gap_sync_ok(char_w));
        assert!(!hex_offset_plus_gap_sync_ok(0.0));
        assert!(!hex_offset_plus_gap_sync_ok(f32::NAN));
        // Typo second: offset + grid + ruler + ASCII all Mono.
        assert_eq!(hex_offset_text_role(), crate::theme::TypoRole::Mono);
        assert_eq!(hex_row_text_role(), crate::theme::TypoRole::Mono);
        assert_eq!(hex_ruler_text_role(), crate::theme::TypoRole::Mono);
        assert_eq!(hex_cell_text_role(), crate::theme::TypoRole::Mono);
    }
}
