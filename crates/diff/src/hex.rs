//! Binary byte alignment and hexadecimal difference analysis.
//!
//! Layer 0: pure algorithms, no I/O, no panics, no unsafe code. Every function accepts
//! hostile inputs (empty, huge, random bytes) and always returns valid, honest output.

use crate::Op;
use crate::engine;

/// Budget for Complete alignment edit search before falling back.
const HEX_EDIT_BUDGET: usize = 100_000;

/// Window size for Fast heuristic resynchronization.
const FAST_WINDOW: usize = 512;

/// Minimum matching byte sequence required to confirm a Fast resync.
const FAST_ANCHOR_LEN: usize = 8;

/// Alignment strategy for hex diffing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HexAlignment {
    /// Direct offset-by-offset comparison from byte 0. No shift detection.
    None,
    /// Windowed heuristic lookahead for fast resynchronization.
    Fast,
    /// Optimal minimal edit sequence via Myers diff.
    #[default]
    Complete,
}

/// An aligned difference operation between two byte sequences.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HexOp {
    /// Bytes match on both sides at the given offsets.
    Equal {
        left_offset: u64,
        right_offset: u64,
        len: usize,
    },
    /// Bytes differ in-place (same length) on both sides.
    Different {
        left_offset: u64,
        right_offset: u64,
        len: usize,
    },
    /// Bytes exist only on the left side (gap on the right).
    LeftOnly { left_offset: u64, len: usize },
    /// Bytes exist only on the right side (gap on the left).
    RightOnly { right_offset: u64, len: usize },
}

/// Summary and detailed operations of a binary hex comparison.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HexDiff {
    /// Length of left input in bytes.
    pub left_len: usize,
    /// Length of right input in bytes.
    pub right_len: usize,
    /// Alignment mode used.
    pub alignment: HexAlignment,
    /// Absolute byte offset of the first detected difference.
    pub first_diff: Option<u64>,
    /// Number of differing bytes (or gap bytes).
    pub differing_bytes: usize,
    /// Number of identical bytes.
    pub matching_bytes: usize,
    /// Sequence of aligned operations.
    pub ops: Vec<HexOp>,
    /// True when the alignment was optimal without budget fallback.
    pub exact: bool,
}

impl HexDiff {
    /// True when both sides are byte-identical under this alignment.
    pub fn is_identical(&self) -> bool {
        self.first_diff.is_none() && self.left_len == self.right_len
    }
}

/// One formatted aligned row for side-by-side hex display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HexRow {
    /// Byte offset of the left side (if any bytes present).
    pub left_offset: Option<u64>,
    /// Left bytes for this row; `None` represents alignment padding.
    pub left_bytes: Vec<Option<u8>>,
    /// Byte offset of the right side (if any bytes present).
    pub right_offset: Option<u64>,
    /// Right bytes for this row; `None` represents alignment padding.
    pub right_bytes: Vec<Option<u8>>,
    /// Per-column difference flag.
    pub differs: Vec<bool>,
}

/// Compares two byte slices with the requested alignment strategy.
pub fn diff_hex(left: &[u8], right: &[u8], alignment: HexAlignment) -> HexDiff {
    match alignment {
        HexAlignment::None => diff_hex_none(left, right),
        HexAlignment::Fast => diff_hex_fast(left, right),
        HexAlignment::Complete => diff_hex_complete(left, right),
    }
}

/// Direct byte-by-byte comparison with no alignment/shift detection.
pub fn diff_hex_none(left: &[u8], right: &[u8]) -> HexDiff {
    let left_len = left.len();
    let right_len = right.len();
    let common = left_len.min(right_len);

    let mut ops = Vec::new();
    let mut first_diff = None;
    let mut differing_bytes = 0usize;
    let mut matching_bytes = 0usize;

    let mut i = 0usize;
    while i < common {
        let l_byte = left.get(i).copied().unwrap_or(0);
        let r_byte = right.get(i).copied().unwrap_or(0);
        let matches = l_byte == r_byte;

        let start = i;
        i = i.saturating_add(1);
        while i < common {
            let l = left.get(i).copied().unwrap_or(0);
            let r = right.get(i).copied().unwrap_or(0);
            if (l == r) != matches {
                break;
            }
            i = i.saturating_add(1);
        }
        let len = i.saturating_sub(start);
        let offset = start as u64;

        if matches {
            matching_bytes = matching_bytes.saturating_add(len);
            ops.push(HexOp::Equal {
                left_offset: offset,
                right_offset: offset,
                len,
            });
        } else {
            if first_diff.is_none() {
                first_diff = Some(offset);
            }
            differing_bytes = differing_bytes.saturating_add(len);
            ops.push(HexOp::Different {
                left_offset: offset,
                right_offset: offset,
                len,
            });
        }
    }

    if left_len > common {
        let len = left_len.saturating_sub(common);
        let offset = common as u64;
        if first_diff.is_none() {
            first_diff = Some(offset);
        }
        differing_bytes = differing_bytes.saturating_add(len);
        ops.push(HexOp::LeftOnly {
            left_offset: offset,
            len,
        });
    } else if right_len > common {
        let len = right_len.saturating_sub(common);
        let offset = common as u64;
        if first_diff.is_none() {
            first_diff = Some(offset);
        }
        differing_bytes = differing_bytes.saturating_add(len);
        ops.push(HexOp::RightOnly {
            right_offset: offset,
            len,
        });
    }

    HexDiff {
        left_len,
        right_len,
        alignment: HexAlignment::None,
        first_diff,
        differing_bytes,
        matching_bytes,
        ops,
        exact: true,
    }
}

/// Windowed heuristic lookahead alignment for fast resynchronization.
pub fn diff_hex_fast(left: &[u8], right: &[u8]) -> HexDiff {
    let left_len = left.len();
    let right_len = right.len();

    let mut ops = Vec::new();
    let mut first_diff = None;
    let mut differing_bytes = 0usize;
    let mut matching_bytes = 0usize;

    let mut l_idx = 0usize;
    let mut r_idx = 0usize;

    while l_idx < left_len && r_idx < right_len {
        let l_byte = left.get(l_idx).copied().unwrap_or(0);
        let r_byte = right.get(r_idx).copied().unwrap_or(0);

        if l_byte == r_byte {
            let start_l = l_idx;
            let start_r = r_idx;
            while l_idx < left_len && r_idx < right_len {
                let lb = left.get(l_idx).copied().unwrap_or(0);
                let rb = right.get(r_idx).copied().unwrap_or(0);
                if lb != rb {
                    break;
                }
                l_idx = l_idx.saturating_add(1);
                r_idx = r_idx.saturating_add(1);
            }
            let len = l_idx.saturating_sub(start_l);
            matching_bytes = matching_bytes.saturating_add(len);
            ops.push(HexOp::Equal {
                left_offset: start_l as u64,
                right_offset: start_r as u64,
                len,
            });
            continue;
        }

        // Differing byte encountered: search for resync in window.
        if first_diff.is_none() {
            first_diff = Some(l_idx as u64);
        }

        if let Some((skip_l, skip_r)) = find_fast_resync(left, right, l_idx, r_idx) {
            emit_mismatch_ops(&mut ops, l_idx, r_idx, skip_l, skip_r, &mut differing_bytes);
            l_idx = l_idx.saturating_add(skip_l);
            r_idx = r_idx.saturating_add(skip_r);
        } else {
            // No resync found in window: emit 1 byte different and step forward.
            differing_bytes = differing_bytes.saturating_add(1);
            ops.push(HexOp::Different {
                left_offset: l_idx as u64,
                right_offset: r_idx as u64,
                len: 1,
            });
            l_idx = l_idx.saturating_add(1);
            r_idx = r_idx.saturating_add(1);
        }
    }

    if l_idx < left_len {
        let len = left_len.saturating_sub(l_idx);
        let offset = l_idx as u64;
        if first_diff.is_none() {
            first_diff = Some(offset);
        }
        differing_bytes = differing_bytes.saturating_add(len);
        ops.push(HexOp::LeftOnly {
            left_offset: offset,
            len,
        });
    } else if r_idx < right_len {
        let len = right_len.saturating_sub(r_idx);
        let offset = r_idx as u64;
        if first_diff.is_none() {
            first_diff = Some(offset);
        }
        differing_bytes = differing_bytes.saturating_add(len);
        ops.push(HexOp::RightOnly {
            right_offset: offset,
            len,
        });
    }

    HexDiff {
        left_len,
        right_len,
        alignment: HexAlignment::Fast,
        first_diff,
        differing_bytes,
        matching_bytes,
        ops,
        exact: true,
    }
}

/// Searches ahead up to `FAST_WINDOW` for a matching anchor between left and right.
fn find_fast_resync(
    left: &[u8],
    right: &[u8],
    l_start: usize,
    r_start: usize,
) -> Option<(usize, usize)> {
    let max_l = left.len().saturating_sub(l_start).min(FAST_WINDOW);
    let max_r = right.len().saturating_sub(r_start).min(FAST_WINDOW);

    let anchor_len = FAST_ANCHOR_LEN.min(max_l).min(max_r);
    if anchor_len == 0 {
        return None;
    }

    // Prioritize direct insertions/deletions before cross-diagonal shifts
    for dist in 1..=FAST_WINDOW {
        // Check deletion from left (r stays at 0, l advances by dist)
        if dist < max_l && has_anchor_match(left, right, l_start + dist, r_start, anchor_len) {
            return Some((dist, 0));
        }
        // Check insertion on right (l stays at 0, r advances by dist)
        if dist < max_r && has_anchor_match(left, right, l_start, r_start + dist, anchor_len) {
            return Some((0, dist));
        }
        // Check symmetric diagonal
        if dist < max_l
            && dist < max_r
            && has_anchor_match(left, right, l_start + dist, r_start + dist, anchor_len)
        {
            return Some((dist, dist));
        }
    }

    // Fallback: check 2D combinations bounded by sum
    for sum in 2..=(max_l + max_r).min(64) {
        for dl in 0..=sum.min(max_l) {
            let dr = sum.saturating_sub(dl);
            if dr <= max_r
                && (dl > 0 || dr > 0)
                && has_anchor_match(left, right, l_start + dl, r_start + dr, anchor_len)
            {
                return Some((dl, dr));
            }
        }
    }

    None
}

fn has_anchor_match(
    left: &[u8],
    right: &[u8],
    l_pos: usize,
    r_pos: usize,
    anchor_len: usize,
) -> bool {
    let l_sub = match left.get(l_pos..l_pos.saturating_add(anchor_len)) {
        Some(s) => s,
        None => return false,
    };
    let r_sub = match right.get(r_pos..r_pos.saturating_add(anchor_len)) {
        Some(s) => s,
        None => return false,
    };
    l_sub == r_sub
}

fn emit_mismatch_ops(
    ops: &mut Vec<HexOp>,
    l_idx: usize,
    r_idx: usize,
    skip_l: usize,
    skip_r: usize,
    differing_bytes: &mut usize,
) {
    let common = skip_l.min(skip_r);
    if common > 0 {
        *differing_bytes = differing_bytes.saturating_add(common);
        ops.push(HexOp::Different {
            left_offset: l_idx as u64,
            right_offset: r_idx as u64,
            len: common,
        });
    }
    if skip_l > common {
        let diff_len = skip_l.saturating_sub(common);
        *differing_bytes = differing_bytes.saturating_add(diff_len);
        ops.push(HexOp::LeftOnly {
            left_offset: l_idx.saturating_add(common) as u64,
            len: diff_len,
        });
    } else if skip_r > common {
        let diff_len = skip_r.saturating_sub(common);
        *differing_bytes = differing_bytes.saturating_add(diff_len);
        ops.push(HexOp::RightOnly {
            right_offset: r_idx.saturating_add(common) as u64,
            len: diff_len,
        });
    }
}

/// Optimal minimal edit sequence alignment via Myers diff engine.
pub fn diff_hex_complete(left: &[u8], right: &[u8]) -> HexDiff {
    let left_len = left.len();
    let right_len = right.len();

    let (raw_ops, exact) = engine::diff_slices(left, right, Some(HEX_EDIT_BUDGET));

    let mut ops = Vec::new();
    let mut first_diff = None;
    let mut differing_bytes = 0usize;
    let mut matching_bytes = 0usize;

    let mut l_off: u64 = 0;
    let mut r_off: u64 = 0;

    let mut iter = raw_ops.into_iter().peekable();
    while let Some(op) = iter.next() {
        match op {
            Op::Equal { len } => {
                matching_bytes = matching_bytes.saturating_add(len);
                ops.push(HexOp::Equal {
                    left_offset: l_off,
                    right_offset: r_off,
                    len,
                });
                l_off = l_off.saturating_add(len as u64);
                r_off = r_off.saturating_add(len as u64);
            }
            Op::Delete { len: d_len } => {
                if first_diff.is_none() {
                    first_diff = Some(l_off);
                }
                // Check if immediately followed by an Insert to pair into Different
                if let Some(Op::Insert { len: i_len }) = iter.peek().cloned() {
                    iter.next();
                    let common = d_len.min(i_len);
                    differing_bytes = differing_bytes.saturating_add(common);
                    ops.push(HexOp::Different {
                        left_offset: l_off,
                        right_offset: r_off,
                        len: common,
                    });
                    l_off = l_off.saturating_add(common as u64);
                    r_off = r_off.saturating_add(common as u64);

                    if d_len > common {
                        let rem = d_len.saturating_sub(common);
                        differing_bytes = differing_bytes.saturating_add(rem);
                        ops.push(HexOp::LeftOnly {
                            left_offset: l_off,
                            len: rem,
                        });
                        l_off = l_off.saturating_add(rem as u64);
                    } else if i_len > common {
                        let rem = i_len.saturating_sub(common);
                        differing_bytes = differing_bytes.saturating_add(rem);
                        ops.push(HexOp::RightOnly {
                            right_offset: r_off,
                            len: rem,
                        });
                        r_off = r_off.saturating_add(rem as u64);
                    }
                } else {
                    differing_bytes = differing_bytes.saturating_add(d_len);
                    ops.push(HexOp::LeftOnly {
                        left_offset: l_off,
                        len: d_len,
                    });
                    l_off = l_off.saturating_add(d_len as u64);
                }
            }
            Op::Insert { len: i_len } => {
                if first_diff.is_none() {
                    first_diff = Some(l_off);
                }
                differing_bytes = differing_bytes.saturating_add(i_len);
                ops.push(HexOp::RightOnly {
                    right_offset: r_off,
                    len: i_len,
                });
                r_off = r_off.saturating_add(i_len as u64);
            }
        }
    }

    HexDiff {
        left_len,
        right_len,
        alignment: HexAlignment::Complete,
        first_diff,
        differing_bytes,
        matching_bytes,
        ops,
        exact,
    }
}

/// Generates side-by-side aligned hex rows (typically 8 or 16 bytes per row).
pub fn aligned_hex_rows(
    left: &[u8],
    right: &[u8],
    diff: &HexDiff,
    bytes_per_row: usize,
) -> Vec<HexRow> {
    let row_len = if bytes_per_row == 0 { 8 } else { bytes_per_row };
    let mut rows = Vec::new();

    let mut cur_l_bytes = Vec::new();
    let mut cur_r_bytes = Vec::new();
    let mut cur_differs = Vec::new();
    let mut cur_l_off = None;
    let mut cur_r_off = None;

    for op in &diff.ops {
        match op {
            HexOp::Equal {
                left_offset,
                right_offset,
                len,
            } => {
                for i in 0..*len {
                    if cur_l_bytes.is_empty() {
                        cur_l_off = Some(left_offset.saturating_add(i as u64));
                        cur_r_off = Some(right_offset.saturating_add(i as u64));
                    }
                    let lb = left
                        .get(usize::try_from(left_offset.saturating_add(i as u64)).unwrap_or(0))
                        .copied();
                    let rb = right
                        .get(usize::try_from(right_offset.saturating_add(i as u64)).unwrap_or(0))
                        .copied();
                    cur_l_bytes.push(lb);
                    cur_r_bytes.push(rb);
                    cur_differs.push(false);

                    if cur_l_bytes.len() == row_len {
                        rows.push(HexRow {
                            left_offset: cur_l_off,
                            left_bytes: std::mem::take(&mut cur_l_bytes),
                            right_offset: cur_r_off,
                            right_bytes: std::mem::take(&mut cur_r_bytes),
                            differs: std::mem::take(&mut cur_differs),
                        });
                        cur_l_off = None;
                        cur_r_off = None;
                    }
                }
            }
            HexOp::Different {
                left_offset,
                right_offset,
                len,
            } => {
                for i in 0..*len {
                    if cur_l_bytes.is_empty() {
                        cur_l_off = Some(left_offset.saturating_add(i as u64));
                        cur_r_off = Some(right_offset.saturating_add(i as u64));
                    }
                    let lb = left
                        .get(usize::try_from(left_offset.saturating_add(i as u64)).unwrap_or(0))
                        .copied();
                    let rb = right
                        .get(usize::try_from(right_offset.saturating_add(i as u64)).unwrap_or(0))
                        .copied();
                    cur_l_bytes.push(lb);
                    cur_r_bytes.push(rb);
                    cur_differs.push(true);

                    if cur_l_bytes.len() == row_len {
                        rows.push(HexRow {
                            left_offset: cur_l_off,
                            left_bytes: std::mem::take(&mut cur_l_bytes),
                            right_offset: cur_r_off,
                            right_bytes: std::mem::take(&mut cur_r_bytes),
                            differs: std::mem::take(&mut cur_differs),
                        });
                        cur_l_off = None;
                        cur_r_off = None;
                    }
                }
            }
            HexOp::LeftOnly { left_offset, len } => {
                for i in 0..*len {
                    if cur_l_bytes.is_empty() {
                        cur_l_off = Some(left_offset.saturating_add(i as u64));
                    }
                    let lb = left
                        .get(usize::try_from(left_offset.saturating_add(i as u64)).unwrap_or(0))
                        .copied();
                    cur_l_bytes.push(lb);
                    cur_r_bytes.push(None);
                    cur_differs.push(true);

                    if cur_l_bytes.len() == row_len {
                        rows.push(HexRow {
                            left_offset: cur_l_off,
                            left_bytes: std::mem::take(&mut cur_l_bytes),
                            right_offset: cur_r_off,
                            right_bytes: std::mem::take(&mut cur_r_bytes),
                            differs: std::mem::take(&mut cur_differs),
                        });
                        cur_l_off = None;
                        cur_r_off = None;
                    }
                }
            }
            HexOp::RightOnly { right_offset, len } => {
                for i in 0..*len {
                    if cur_r_bytes.is_empty() {
                        cur_r_off = Some(right_offset.saturating_add(i as u64));
                    }
                    let rb = right
                        .get(usize::try_from(right_offset.saturating_add(i as u64)).unwrap_or(0))
                        .copied();
                    cur_l_bytes.push(None);
                    cur_r_bytes.push(rb);
                    cur_differs.push(true);

                    if cur_l_bytes.len() == row_len {
                        rows.push(HexRow {
                            left_offset: cur_l_off,
                            left_bytes: std::mem::take(&mut cur_l_bytes),
                            right_offset: cur_r_off,
                            right_bytes: std::mem::take(&mut cur_r_bytes),
                            differs: std::mem::take(&mut cur_differs),
                        });
                        cur_l_off = None;
                        cur_r_off = None;
                    }
                }
            }
        }
    }

    if !cur_l_bytes.is_empty() {
        while cur_l_bytes.len() < row_len {
            cur_l_bytes.push(None);
            cur_r_bytes.push(None);
            cur_differs.push(false);
        }
        rows.push(HexRow {
            left_offset: cur_l_off,
            left_bytes: cur_l_bytes,
            right_offset: cur_r_off,
            right_bytes: cur_r_bytes,
            differs: cur_differs,
        });
    }

    rows
}

/// Returns the 0-indexed row for a 64-bit byte offset given the row width.
pub fn row_from_offset(offset: u64, bytes_per_row: usize) -> u64 {
    let row_len = (bytes_per_row.max(1)) as u64;
    offset / row_len
}

/// Returns the 0-indexed column within a row for a 64-bit byte offset.
pub fn col_from_offset(offset: u64, bytes_per_row: usize) -> usize {
    let row_len = (bytes_per_row.max(1)) as u64;
    (offset % row_len) as usize
}

/// Converts a 64-bit row index and column offset into an absolute 64-bit byte offset.
pub fn offset_from_row_col(row: u64, col: usize, bytes_per_row: usize) -> u64 {
    row.saturating_mul(bytes_per_row as u64)
        .saturating_add(col as u64)
}

/// Finds the index of the [`HexRow`] containing the requested 64-bit byte offset.
pub fn find_row_for_offset(rows: &[HexRow], offset: u64, is_left: bool) -> Option<usize> {
    for (idx, row) in rows.iter().enumerate() {
        let (row_off_opt, bytes) = if is_left {
            (row.left_offset, &row.left_bytes)
        } else {
            (row.right_offset, &row.right_bytes)
        };
        if let Some(row_off) = row_off_opt {
            let actual_bytes = bytes.iter().filter(|b| b.is_some()).count() as u64;
            if actual_bytes > 0
                && offset >= row_off
                && offset < row_off.saturating_add(actual_bytes)
            {
                return Some(idx);
            }
        }
    }
    None
}

/// Finds which [`HexOp`] inside a [`HexDiff`] covers the requested 64-bit byte offset.
pub fn find_op_at_offset(diff: &HexDiff, offset: u64, is_left: bool) -> Option<(usize, &HexOp)> {
    for (idx, op) in diff.ops.iter().enumerate() {
        match op {
            HexOp::Equal {
                left_offset,
                right_offset,
                len,
            } => {
                let op_off = if is_left { *left_offset } else { *right_offset };
                if offset >= op_off && offset < op_off.saturating_add(*len as u64) {
                    return Some((idx, op));
                }
            }
            HexOp::Different {
                left_offset,
                right_offset,
                len,
            } => {
                let op_off = if is_left { *left_offset } else { *right_offset };
                if offset >= op_off && offset < op_off.saturating_add(*len as u64) {
                    return Some((idx, op));
                }
            }
            HexOp::LeftOnly { left_offset, len }
                if is_left
                    && offset >= *left_offset
                    && offset < left_offset.saturating_add(*len as u64) =>
            {
                return Some((idx, op));
            }
            HexOp::RightOnly { right_offset, len }
                if !is_left
                    && offset >= *right_offset
                    && offset < right_offset.saturating_add(*len as u64) =>
            {
                return Some((idx, op));
            }
            _ => {}
        }
    }
    None
}

/// An atomic byte modification record for the hex byte-edit backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ByteEdit {
    /// In-place byte replacement at a 64-bit offset.
    SetByte {
        offset: u64,
        old_val: u8,
        new_val: u8,
    },
    /// Insertion of bytes at a 64-bit offset.
    InsertBytes { offset: u64, bytes: Vec<u8> },
    /// Deletion of bytes starting at a 64-bit offset.
    DeleteBytes { offset: u64, old_bytes: Vec<u8> },
}

/// In-memory byte editor backend with 64-bit offsets and an undo/redo stack.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ByteEditBuffer {
    bytes: Vec<u8>,
    history: Vec<ByteEdit>,
    undone: Vec<ByteEdit>,
    dirty: bool,
}

impl ByteEditBuffer {
    /// Creates a new byte buffer initialized with `bytes`.
    pub fn new(bytes: Vec<u8>) -> Self {
        Self {
            bytes,
            history: Vec::new(),
            undone: Vec::new(),
            dirty: false,
        }
    }

    /// Total length of the buffer in bytes (64-bit).
    pub fn len(&self) -> u64 {
        self.bytes.len() as u64
    }

    /// Returns true when the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// Fetches the byte at a 64-bit offset.
    pub fn get(&self, offset: u64) -> Option<u8> {
        let idx = usize::try_from(offset).ok()?;
        self.bytes.get(idx).copied()
    }

    /// Sets the byte at a 64-bit offset, recording the change in history.
    pub fn set_byte(&mut self, offset: u64, new_val: u8) -> bool {
        let idx = match usize::try_from(offset) {
            Ok(i) => i,
            Err(_) => return false,
        };
        let slot = match self.bytes.get_mut(idx) {
            Some(s) => s,
            None => return false,
        };
        let old_val = *slot;
        *slot = new_val;
        self.history.push(ByteEdit::SetByte {
            offset,
            old_val,
            new_val,
        });
        self.undone.clear();
        self.dirty = true;
        true
    }

    /// Inserts bytes at a 64-bit offset, shifting trailing bytes.
    pub fn insert(&mut self, offset: u64, data: &[u8]) -> bool {
        if data.is_empty() {
            return true;
        }
        let idx = match usize::try_from(offset) {
            Ok(i) if i <= self.bytes.len() => i,
            _ => return false,
        };
        self.bytes.splice(idx..idx, data.iter().copied());
        self.history.push(ByteEdit::InsertBytes {
            offset,
            bytes: data.to_vec(),
        });
        self.undone.clear();
        self.dirty = true;
        true
    }

    /// Deletes `count` bytes starting at a 64-bit offset.
    pub fn delete(&mut self, offset: u64, count: u64) -> bool {
        if count == 0 {
            return true;
        }
        let start = match usize::try_from(offset) {
            Ok(i) if i < self.bytes.len() => i,
            _ => return false,
        };
        let end = match usize::try_from(offset.saturating_add(count)) {
            Ok(e) => e.min(self.bytes.len()),
            Err(_) => self.bytes.len(),
        };
        let old_bytes = self.bytes.get(start..end).unwrap_or(&[]).to_vec();
        self.bytes.drain(start..end);
        self.history
            .push(ByteEdit::DeleteBytes { offset, old_bytes });
        self.undone.clear();
        self.dirty = true;
        true
    }

    /// Undoes the most recent byte edit.
    pub fn undo(&mut self) -> bool {
        let edit = match self.history.pop() {
            Some(e) => e,
            None => return false,
        };
        match &edit {
            ByteEdit::SetByte {
                offset, old_val, ..
            } => {
                if let Some(slot) = usize::try_from(*offset)
                    .ok()
                    .and_then(|idx| self.bytes.get_mut(idx))
                {
                    *slot = *old_val;
                }
            }
            ByteEdit::InsertBytes { offset, bytes } => {
                if let Ok(start) = usize::try_from(*offset) {
                    let end = start.saturating_add(bytes.len()).min(self.bytes.len());
                    self.bytes.drain(start..end);
                }
            }
            ByteEdit::DeleteBytes { offset, old_bytes } => {
                let valid_idx = usize::try_from(*offset)
                    .ok()
                    .filter(|&idx| idx <= self.bytes.len());
                if let Some(idx) = valid_idx {
                    self.bytes.splice(idx..idx, old_bytes.iter().copied());
                }
            }
        }
        self.undone.push(edit);
        true
    }

    /// Redoes the most recently undone byte edit.
    pub fn redo(&mut self) -> bool {
        let edit = match self.undone.pop() {
            Some(e) => e,
            None => return false,
        };
        match &edit {
            ByteEdit::SetByte {
                offset, new_val, ..
            } => {
                if let Some(slot) = usize::try_from(*offset)
                    .ok()
                    .and_then(|idx| self.bytes.get_mut(idx))
                {
                    *slot = *new_val;
                }
            }
            ByteEdit::InsertBytes { offset, bytes } => {
                let valid_idx = usize::try_from(*offset)
                    .ok()
                    .filter(|&idx| idx <= self.bytes.len());
                if let Some(idx) = valid_idx {
                    self.bytes.splice(idx..idx, bytes.iter().copied());
                }
            }
            ByteEdit::DeleteBytes { offset, old_bytes } => {
                if let Ok(start) = usize::try_from(*offset) {
                    let end = start.saturating_add(old_bytes.len()).min(self.bytes.len());
                    self.bytes.drain(start..end);
                }
            }
        }
        self.history.push(edit);
        true
    }

    /// Returns true when undo is available.
    pub fn can_undo(&self) -> bool {
        !self.history.is_empty()
    }

    /// Returns true when redo is available.
    pub fn can_redo(&self) -> bool {
        !self.undone.is_empty()
    }

    /// Returns whether any changes have been made to the buffer.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Read-only slice view of the current bytes.
    pub fn as_slice(&self) -> &[u8] {
        &self.bytes
    }

    /// Consumes the buffer and returns the underlying byte vector.
    pub fn into_vec(self) -> Vec<u8> {
        self.bytes
    }

    /// Returns the sequence of edits applied so far.
    pub fn edit_history(&self) -> &[ByteEdit] {
        &self.history
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_both_empty() {
        for mode in [
            HexAlignment::None,
            HexAlignment::Fast,
            HexAlignment::Complete,
        ] {
            let diff = diff_hex(b"", b"", mode);
            assert_eq!(diff.left_len, 0);
            assert_eq!(diff.right_len, 0);
            assert_eq!(diff.first_diff, None);
            assert_eq!(diff.differing_bytes, 0);
            assert_eq!(diff.matching_bytes, 0);
            assert!(diff.ops.is_empty());
        }
    }

    #[test]
    fn test_identical_bytes() {
        let data = b"Hello, World!";
        for mode in [
            HexAlignment::None,
            HexAlignment::Fast,
            HexAlignment::Complete,
        ] {
            let diff = diff_hex(data, data, mode);
            assert_eq!(diff.first_diff, None);
            assert_eq!(diff.differing_bytes, 0);
            assert_eq!(diff.matching_bytes, data.len());
            assert_eq!(
                diff.ops,
                vec![HexOp::Equal {
                    left_offset: 0,
                    right_offset: 0,
                    len: data.len()
                }]
            );
        }
    }

    #[test]
    fn test_single_byte_difference() {
        let a = b"abcdef";
        let b = b"abcXef";
        let diff = diff_hex(a, b, HexAlignment::None);
        assert_eq!(diff.first_diff, Some(3));
        assert_eq!(diff.differing_bytes, 1);
        assert_eq!(diff.matching_bytes, 5);

        let c_diff = diff_hex(a, b, HexAlignment::Complete);
        assert_eq!(c_diff.first_diff, Some(3));
        assert_eq!(c_diff.differing_bytes, 1);
    }

    #[test]
    fn test_insertion_alignment() {
        let a = b"AAAA12345678BBBB";
        let b = b"AAAA____12345678BBBB";

        let complete = diff_hex(a, b, HexAlignment::Complete);
        assert_eq!(complete.first_diff, Some(4));
        // complete should find the inserted 4 bytes
        assert!(
            complete
                .ops
                .iter()
                .any(|op| matches!(op, HexOp::RightOnly { len: 4, .. }))
        );

        let fast = diff_hex(a, b, HexAlignment::Fast);
        assert_eq!(fast.first_diff, Some(4));
    }

    #[test]
    fn test_deletion_alignment() {
        let a = b"AAAA____12345678BBBB";
        let b = b"AAAA12345678BBBB";

        let complete = diff_hex(a, b, HexAlignment::Complete);
        assert_eq!(complete.first_diff, Some(4));
        assert!(
            complete
                .ops
                .iter()
                .any(|op| matches!(op, HexOp::LeftOnly { len: 4, .. }))
        );
    }

    #[test]
    fn test_aligned_rows_formatting() {
        let a = b"12345678";
        let b = b"123X5678";
        let diff = diff_hex(a, b, HexAlignment::None);
        let rows = aligned_hex_rows(a, b, &diff, 8);
        assert_eq!(rows.len(), 1);
        if let Some(r0) = rows.first() {
            assert_eq!(r0.left_offset, Some(0));
            assert_eq!(r0.right_offset, Some(0));
            assert_eq!(r0.differs.get(3), Some(&true));
            assert_eq!(r0.differs.first(), Some(&false));
        }
    }

    #[test]
    fn test_hostile_truncated_inputs() {
        let zeros = vec![0u8; 1000];
        let ones = vec![1u8; 1000];
        let diff = diff_hex(&zeros, &ones, HexAlignment::None);
        assert_eq!(diff.differing_bytes, 1000);
        assert_eq!(diff.matching_bytes, 0);

        let mut truncated = zeros.clone();
        truncated.truncate(500);
        let diff_t = diff_hex(&zeros, &truncated, HexAlignment::Complete);
        assert_eq!(diff_t.first_diff, Some(500));
        assert_eq!(diff_t.differing_bytes, 500);
        assert_eq!(diff_t.matching_bytes, 500);
    }

    #[test]
    fn test_64bit_goto_helpers() {
        assert_eq!(row_from_offset(0, 16), 0);
        assert_eq!(row_from_offset(15, 16), 0);
        assert_eq!(row_from_offset(16, 16), 1);
        assert_eq!(row_from_offset(1_000_000_000_000, 16), 62_500_000_000);

        assert_eq!(col_from_offset(0, 16), 0);
        assert_eq!(col_from_offset(15, 16), 15);
        assert_eq!(col_from_offset(16, 16), 0);

        assert_eq!(offset_from_row_col(10, 5, 16), 165);

        let a = b"0123456789ABCDEFextra";
        let b = b"0123456789ABCDEFextra";
        let diff = diff_hex(a, b, HexAlignment::None);
        let rows = aligned_hex_rows(a, b, &diff, 16);
        assert_eq!(find_row_for_offset(&rows, 5, true), Some(0));
        assert_eq!(find_row_for_offset(&rows, 18, true), Some(1));
        assert_eq!(find_row_for_offset(&rows, 100, true), None);

        let op_match = find_op_at_offset(&diff, 10, true);
        assert!(op_match.is_some());
        if let Some((_, op)) = op_match {
            assert!(matches!(op, HexOp::Equal { .. }));
        }
    }

    #[test]
    fn test_byte_edit_buffer_crud_and_undo_redo() {
        let mut buf = ByteEditBuffer::new(vec![1, 2, 3, 4, 5]);
        assert_eq!(buf.len(), 5);
        assert_eq!(buf.get(2), Some(3));
        assert!(!buf.is_dirty());

        // SetByte
        assert!(buf.set_byte(2, 99));
        assert_eq!(buf.get(2), Some(99));
        assert!(buf.is_dirty());
        assert!(buf.can_undo());

        // Insert
        assert!(buf.insert(1, &[10, 20]));
        assert_eq!(buf.as_slice(), &[1, 10, 20, 2, 99, 4, 5]);
        assert_eq!(buf.len(), 7);

        // Delete
        assert!(buf.delete(3, 2)); // delete 2 bytes at offset 3 ('2', '99')
        assert_eq!(buf.as_slice(), &[1, 10, 20, 4, 5]);
        assert_eq!(buf.len(), 5);

        // Undo Delete
        assert!(buf.undo());
        assert_eq!(buf.as_slice(), &[1, 10, 20, 2, 99, 4, 5]);

        // Undo Insert
        assert!(buf.undo());
        assert_eq!(buf.as_slice(), &[1, 2, 99, 4, 5]);

        // Undo SetByte
        assert!(buf.undo());
        assert_eq!(buf.as_slice(), &[1, 2, 3, 4, 5]);
        assert!(!buf.can_undo());
        assert!(buf.can_redo());

        // Redo SetByte
        assert!(buf.redo());
        assert_eq!(buf.as_slice(), &[1, 2, 99, 4, 5]);
    }
}
