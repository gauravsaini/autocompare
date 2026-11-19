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
}
