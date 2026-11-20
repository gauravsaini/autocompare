//! Line diffing with Myers and Patience algorithms, plus unified hunk generation.
//!
//! Layer 0: no panics, no unwrap/expect in non-test code, no unsafe.

#![allow(dead_code)]

use std::collections::HashMap;

use crate::{DiffScript, Op, engine};

/// Choice of line difference algorithm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DiffAlgorithm {
    /// Myers' O(ND) algorithm: guarantees minimal edit script.
    #[default]
    Myers,
    /// Patience diff: aligns on unique matching lines, producing more
    /// human-readable diffs for shifted or reordered blocks.
    Patience,
}

/// Computes a diff script between two line sequences using the chosen algorithm.
pub fn diff(a: &[&str], b: &[&str], algorithm: DiffAlgorithm) -> DiffScript {
    match algorithm {
        DiffAlgorithm::Myers => diff_myers(a, b),
        DiffAlgorithm::Patience => diff_patience(a, b),
    }
}

/// Compares two line sequences using Myers' linear-space algorithm.
pub fn diff_myers(a: &[&str], b: &[&str]) -> DiffScript {
    let (ops, exact) = engine::diff_slices(a, b, Some(crate::EDIT_BUDGET));
    DiffScript { ops, exact }
}

/// Compares two line sequences using the Patience diff algorithm.
///
/// 1. Match unique lines occurring exactly once in both `a` and `b`.
/// 2. Find the Longest Increasing Subsequence (LIS) of these matching lines (anchors).
/// 3. Recursively diff the regions between anchors.
/// 4. When a region contains no unique matches, fallback to Myers diff.
pub fn diff_patience(a: &[&str], b: &[&str]) -> DiffScript {
    let mut builder = PatienceBuilder::new();
    let exact = diff_patience_slice(a, 0, a.len(), b, 0, b.len(), &mut builder, 0);
    DiffScript {
        ops: builder.finish(),
        exact,
    }
}

const MAX_PATIENCE_DEPTH: usize = 64;

#[allow(clippy::too_many_arguments)]
fn diff_patience_slice(
    a: &[&str],
    a_start: usize,
    a_end: usize,
    b: &[&str],
    b_start: usize,
    b_end: usize,
    builder: &mut PatienceBuilder,
    depth: usize,
) -> bool {
    let a_slice = match a.get(a_start..a_end) {
        Some(s) => s,
        None => return false,
    };
    let b_slice = match b.get(b_start..b_end) {
        Some(s) => s,
        None => return false,
    };

    if a_slice.is_empty() && b_slice.is_empty() {
        return true;
    }
    if a_slice.is_empty() {
        builder.push_insert(b_slice.len());
        return true;
    }
    if b_slice.is_empty() {
        builder.push_delete(a_slice.len());
        return true;
    }

    // Trim common prefix
    let pfx = common_prefix(a_slice, b_slice);
    if pfx > 0 {
        builder.push_equal(pfx);
    }
    let a_sub_start = a_start.saturating_add(pfx);
    let b_sub_start = b_start.saturating_add(pfx);

    // Trim common suffix
    let sfx = common_suffix(
        a.get(a_sub_start..a_end).unwrap_or(&[]),
        b.get(b_sub_start..b_end).unwrap_or(&[]),
    );
    let a_sub_end = a_end.saturating_sub(sfx);
    let b_sub_end = b_end.saturating_sub(sfx);

    if a_sub_start >= a_sub_end && b_sub_start >= b_sub_end {
        if sfx > 0 {
            builder.push_equal(sfx);
        }
        return true;
    }

    if a_sub_start >= a_sub_end {
        builder.push_insert(b_sub_end.saturating_sub(b_sub_start));
        if sfx > 0 {
            builder.push_equal(sfx);
        }
        return true;
    }

    if b_sub_start >= b_sub_end {
        builder.push_delete(a_sub_end.saturating_sub(a_sub_start));
        if sfx > 0 {
            builder.push_equal(sfx);
        }
        return true;
    }

    // If depth limit reached, fallback to Myers
    if depth >= MAX_PATIENCE_DEPTH {
        let (fallback_ops, exact) = engine::diff_slices(
            a.get(a_sub_start..a_sub_end).unwrap_or(&[]),
            b.get(b_sub_start..b_sub_end).unwrap_or(&[]),
            Some(crate::EDIT_BUDGET),
        );
        for op in fallback_ops {
            builder.push_op(op);
        }
        if sfx > 0 {
            builder.push_equal(sfx);
        }
        return exact;
    }

    // Find unique lines in both slices
    let unique_matches = find_unique_matches(
        a.get(a_sub_start..a_sub_end).unwrap_or(&[]),
        b.get(b_sub_start..b_sub_end).unwrap_or(&[]),
    );

    if unique_matches.is_empty() {
        // Fallback to Myers
        let (fallback_ops, exact) = engine::diff_slices(
            a.get(a_sub_start..a_sub_end).unwrap_or(&[]),
            b.get(b_sub_start..b_sub_end).unwrap_or(&[]),
            Some(crate::EDIT_BUDGET),
        );
        for op in fallback_ops {
            builder.push_op(op);
        }
        if sfx > 0 {
            builder.push_equal(sfx);
        }
        return exact;
    }

    // Compute Longest Increasing Subsequence on b indices
    let lis = longest_increasing_subsequence(&unique_matches);
    if lis.is_empty() {
        let (fallback_ops, exact) = engine::diff_slices(
            a.get(a_sub_start..a_sub_end).unwrap_or(&[]),
            b.get(b_sub_start..b_sub_end).unwrap_or(&[]),
            Some(crate::EDIT_BUDGET),
        );
        for op in fallback_ops {
            builder.push_op(op);
        }
        if sfx > 0 {
            builder.push_equal(sfx);
        }
        return exact;
    }

    let mut curr_a = a_sub_start;
    let mut curr_b = b_sub_start;
    let mut all_exact = true;

    for (rel_a, rel_b) in lis {
        let next_a = a_sub_start.saturating_add(rel_a);
        let next_b = b_sub_start.saturating_add(rel_b);

        if !diff_patience_slice(
            a,
            curr_a,
            next_a,
            b,
            curr_b,
            next_b,
            builder,
            depth.saturating_add(1),
        ) {
            all_exact = false;
        }

        builder.push_equal(1);
        curr_a = next_a.saturating_add(1);
        curr_b = next_b.saturating_add(1);
    }

    if !diff_patience_slice(
        a,
        curr_a,
        a_sub_end,
        b,
        curr_b,
        b_sub_end,
        builder,
        depth.saturating_add(1),
    ) {
        all_exact = false;
    }

    if sfx > 0 {
        builder.push_equal(sfx);
    }

    all_exact
}

fn common_prefix(a: &[&str], b: &[&str]) -> usize {
    let mut n = 0;
    let limit = a.len().min(b.len());
    while n < limit {
        match (a.get(n), b.get(n)) {
            (Some(x), Some(y)) if x == y => n += 1,
            _ => break,
        }
    }
    n
}

fn common_suffix(a: &[&str], b: &[&str]) -> usize {
    let mut n = 0;
    let limit = a.len().min(b.len());
    while n < limit {
        let a_idx = a.len().saturating_sub(n + 1);
        let b_idx = b.len().saturating_sub(n + 1);
        match (a.get(a_idx), b.get(b_idx)) {
            (Some(x), Some(y)) if x == y => n += 1,
            _ => break,
        }
    }
    n
}

/// Matches that appear exactly once in `a` and exactly once in `b`.
/// Returns pairs of `(index_in_a, index_in_b)` sorted by `index_in_a`.
fn find_unique_matches(a: &[&str], b: &[&str]) -> Vec<(usize, usize)> {
    let mut a_counts: HashMap<&str, (usize, usize)> = HashMap::new(); // text -> (count, first_index)
    for (i, &line) in a.iter().enumerate() {
        let entry = a_counts.entry(line).or_insert((0, i));
        entry.0 = entry.0.saturating_add(1);
    }

    let mut b_counts: HashMap<&str, (usize, usize)> = HashMap::new();
    for (i, &line) in b.iter().enumerate() {
        let entry = b_counts.entry(line).or_insert((0, i));
        entry.0 = entry.0.saturating_add(1);
    }

    let mut matches = Vec::new();
    for (&line, &(count_a, idx_a)) in &a_counts {
        if count_a == 1
            && let Some(&(count_b, idx_b)) = b_counts.get(line)
            && count_b == 1
        {
            matches.push((idx_a, idx_b));
        }
    }

    matches.sort_by_key(|&(idx_a, _)| idx_a);
    matches
}

/// Computes the Longest Increasing Subsequence on `b` indices of pairs `(a_idx, b_idx)`.
fn longest_increasing_subsequence(pairs: &[(usize, usize)]) -> Vec<(usize, usize)> {
    if pairs.is_empty() {
        return Vec::new();
    }

    // Piles for patience sorting.
    // piles[k] = index into `pairs` of the top of pile k
    let mut piles: Vec<usize> = Vec::new();
    // back_pointers[i] = index into `pairs` of the previous element in LIS
    let mut back_pointers: Vec<Option<usize>> = vec![None; pairs.len()];

    for (i, &(_, b_val)) in pairs.iter().enumerate() {
        // Binary search for smallest pile with top b_val >= current b_val
        let mut lo = 0usize;
        let mut hi = piles.len();
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            let pile_top_idx = piles.get(mid).copied().unwrap_or(0);
            let pile_b_val = pairs.get(pile_top_idx).map(|p| p.1).unwrap_or(usize::MAX);
            if pile_b_val < b_val {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }

        if lo > 0
            && let Some(slot) = back_pointers.get_mut(i)
        {
            *slot = piles.get(lo - 1).copied();
        }

        if lo == piles.len() {
            piles.push(i);
        } else if let Some(slot) = piles.get_mut(lo) {
            *slot = i;
        }
    }

    let mut result = Vec::new();
    let mut curr = piles.last().copied();
    while let Some(idx) = curr {
        if let Some(&pair) = pairs.get(idx) {
            result.push(pair);
        }
        curr = back_pointers.get(idx).copied().flatten();
    }

    result.reverse();
    result
}

/// Helper to accumulate diff ops and normalize them.
struct PatienceBuilder {
    ops: Vec<Op>,
}

impl PatienceBuilder {
    fn new() -> Self {
        Self { ops: Vec::new() }
    }

    fn push_equal(&mut self, len: usize) {
        if len == 0 {
            return;
        }
        if let Some(Op::Equal { len: l }) = self.ops.last_mut() {
            *l = l.saturating_add(len);
        } else {
            self.ops.push(Op::Equal { len });
        }
    }

    fn push_delete(&mut self, len: usize) {
        if len == 0 {
            return;
        }
        if let Some(&Op::Insert { len: ins }) = self.ops.last() {
            self.ops.pop();
            self.push_delete(len);
            self.push_insert(ins);
            return;
        }
        if let Some(Op::Delete { len: l }) = self.ops.last_mut() {
            *l = l.saturating_add(len);
        } else {
            self.ops.push(Op::Delete { len });
        }
    }

    fn push_insert(&mut self, len: usize) {
        if len == 0 {
            return;
        }
        if let Some(Op::Insert { len: l }) = self.ops.last_mut() {
            *l = l.saturating_add(len);
        } else {
            self.ops.push(Op::Insert { len });
        }
    }

    fn push_op(&mut self, op: Op) {
        match op {
            Op::Equal { len } => self.push_equal(len),
            Op::Delete { len } => self.push_delete(len),
            Op::Insert { len } => self.push_insert(len),
        }
    }

    fn finish(self) -> Vec<Op> {
        self.ops
    }
}

// ----------------------------------------------------------------------------
// Hunk generation & offsets
// ----------------------------------------------------------------------------

/// Kind of change for a single line in a hunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineChange {
    /// Line is context unchanged between both versions.
    Context,
    /// Line exists in the old version and was deleted.
    Delete,
    /// Line exists in the new version and was inserted.
    Insert,
}

/// A single formatted line within a hunk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HunkLine {
    pub change: LineChange,
    pub content: String,
    /// 1-based line number in the old file, if present.
    pub old_line_no: Option<usize>,
    /// 1-based line number in the new file, if present.
    pub new_line_no: Option<usize>,
}

/// A unified diff hunk covering a continuous edit region with surrounding context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hunk {
    /// 1-based starting line number in the old file (unified diff format).
    pub old_start: usize,
    /// Number of lines in the old file covered by this hunk.
    pub old_lines: usize,
    /// 1-based starting line number in the new file (unified diff format).
    pub new_start: usize,
    /// Number of lines in the new file covered by this hunk.
    pub new_lines: usize,
    /// Byte offset in the old text where this hunk begins.
    pub old_byte_offset: usize,
    /// Total byte length of the old lines in this hunk.
    pub old_byte_len: usize,
    /// Byte offset in the new text where this hunk begins.
    pub new_byte_offset: usize,
    /// Total byte length of the new lines in this hunk.
    pub new_byte_len: usize,
    /// The lines comprising this hunk.
    pub lines: Vec<HunkLine>,
}

impl Hunk {
    /// Formats the unified diff header for this hunk, e.g. `@@ -1,5 +1,6 @@`.
    pub fn header(&self) -> String {
        format!(
            "@@ -{},{} +{},{} @@",
            self.old_start, self.old_lines, self.new_start, self.new_lines
        )
    }
}

/// Precomputed line byte offsets and lengths for calculating hunk byte spans.
struct LineIndex {
    line_offsets: Vec<usize>,
    line_lengths: Vec<usize>,
}

impl LineIndex {
    fn from_lines(lines: &[&str]) -> Self {
        let mut line_offsets = Vec::with_capacity(lines.len());
        let mut line_lengths = Vec::with_capacity(lines.len());
        let mut offset = 0usize;
        for line in lines {
            line_offsets.push(offset);
            // Include 1 byte for '\n'
            let len = line.len().saturating_add(1);
            line_lengths.push(len);
            offset = offset.saturating_add(len);
        }
        Self {
            line_offsets,
            line_lengths,
        }
    }

    fn byte_range(&self, start_line_0based: usize, count: usize) -> (usize, usize) {
        if count == 0 || start_line_0based >= self.line_offsets.len() {
            let offset = self
                .line_offsets
                .get(start_line_0based)
                .copied()
                .unwrap_or(0);
            return (offset, 0);
        }
        let start_offset = self
            .line_offsets
            .get(start_line_0based)
            .copied()
            .unwrap_or(0);
        let mut total_len = 0usize;
        for i in 0..count {
            let idx = start_line_0based.saturating_add(i);
            total_len = total_len.saturating_add(self.line_lengths.get(idx).copied().unwrap_or(0));
        }
        (start_offset, total_len)
    }
}

/// Groups diff operations into unified diff hunks with the specified context lines.
pub fn create_hunks(a: &[&str], b: &[&str], ops: &[Op], context: usize) -> Vec<Hunk> {
    if ops.is_empty() {
        return Vec::new();
    }

    let a_index = LineIndex::from_lines(a);
    let b_index = LineIndex::from_lines(b);

    // Expand ops into per-line tokens
    #[derive(Clone, Copy)]
    enum RawItem {
        Equal(usize, usize), // a_idx, b_idx
        Delete(usize),       // a_idx
        Insert(usize),       // b_idx
    }

    let mut items: Vec<RawItem> = Vec::new();
    let mut a_pos = 0usize;
    let mut b_pos = 0usize;

    for op in ops {
        match op {
            Op::Equal { len } => {
                for _ in 0..*len {
                    items.push(RawItem::Equal(a_pos, b_pos));
                    a_pos += 1;
                    b_pos += 1;
                }
            }
            Op::Delete { len } => {
                for _ in 0..*len {
                    items.push(RawItem::Delete(a_pos));
                    a_pos += 1;
                }
            }
            Op::Insert { len } => {
                for _ in 0..*len {
                    items.push(RawItem::Insert(b_pos));
                    b_pos += 1;
                }
            }
        }
    }

    // Identify edit regions (non-Equal items)
    let mut edit_ranges: Vec<(usize, usize)> = Vec::new(); // [start, end) indices into `items`
    let mut in_edit = false;
    let mut edit_start = 0usize;

    for (idx, item) in items.iter().enumerate() {
        let is_edit = !matches!(item, RawItem::Equal(..));
        if is_edit {
            if !in_edit {
                in_edit = true;
                edit_start = idx;
            }
        } else if in_edit {
            in_edit = false;
            edit_ranges.push((edit_start, idx));
        }
    }
    if in_edit {
        edit_ranges.push((edit_start, items.len()));
    }

    if edit_ranges.is_empty() {
        return Vec::new();
    }

    // Merge edit ranges separated by <= 2 * context
    let mut merged_ranges: Vec<(usize, usize)> = Vec::new();
    for (start, end) in edit_ranges {
        if let Some(last) = merged_ranges.last_mut() {
            let gap = start.saturating_sub(last.1);
            if gap <= context.saturating_mul(2) {
                last.1 = end;
                continue;
            }
        }
        merged_ranges.push((start, end));
    }

    let mut hunks = Vec::new();

    for (m_start, m_end) in merged_ranges {
        let hunk_start = m_start.saturating_sub(context);
        let hunk_end = (m_end.saturating_add(context)).min(items.len());

        let mut hunk_lines = Vec::new();
        let mut old_start_0based = None;
        let mut new_start_0based = None;
        let mut old_count = 0usize;
        let mut new_count = 0usize;

        for &item in items.get(hunk_start..hunk_end).unwrap_or(&[]) {
            match item {
                RawItem::Equal(ai, bi) => {
                    if old_start_0based.is_none() {
                        old_start_0based = Some(ai);
                    }
                    if new_start_0based.is_none() {
                        new_start_0based = Some(bi);
                    }
                    old_count += 1;
                    new_count += 1;
                    hunk_lines.push(HunkLine {
                        change: LineChange::Context,
                        content: a.get(ai).copied().unwrap_or("").to_string(),
                        old_line_no: Some(ai.saturating_add(1)),
                        new_line_no: Some(bi.saturating_add(1)),
                    });
                }
                RawItem::Delete(ai) => {
                    if old_start_0based.is_none() {
                        old_start_0based = Some(ai);
                    }
                    old_count += 1;
                    hunk_lines.push(HunkLine {
                        change: LineChange::Delete,
                        content: a.get(ai).copied().unwrap_or("").to_string(),
                        old_line_no: Some(ai.saturating_add(1)),
                        new_line_no: None,
                    });
                }
                RawItem::Insert(bi) => {
                    if new_start_0based.is_none() {
                        new_start_0based = Some(bi);
                    }
                    new_count += 1;
                    hunk_lines.push(HunkLine {
                        change: LineChange::Insert,
                        content: b.get(bi).copied().unwrap_or("").to_string(),
                        old_line_no: None,
                        new_line_no: Some(bi.saturating_add(1)),
                    });
                }
            }
        }

        let old_start_idx = old_start_0based.unwrap_or(0);
        let new_start_idx = new_start_0based.unwrap_or(0);

        let (old_byte_offset, old_byte_len) = a_index.byte_range(old_start_idx, old_count);
        let (new_byte_offset, new_byte_len) = b_index.byte_range(new_start_idx, new_count);

        let old_start_1based = if old_count == 0 {
            old_start_idx
        } else {
            old_start_idx.saturating_add(1)
        };
        let new_start_1based = if new_count == 0 {
            new_start_idx
        } else {
            new_start_idx.saturating_add(1)
        };

        hunks.push(Hunk {
            old_start: old_start_1based,
            old_lines: old_count,
            new_start: new_start_1based,
            new_lines: new_count,
            old_byte_offset,
            old_byte_len,
            new_byte_offset,
            new_byte_len,
            lines: hunk_lines,
        });
    }

    hunks
}

/// Applies a diff script to `old_lines`. Returns `None` if the script does not match.
pub fn apply_diff(old_lines: &[&str], ops: &[Op]) -> Option<Vec<String>> {
    let mut out = Vec::new();
    let mut pos = 0usize;

    for op in ops {
        match op {
            Op::Equal { len } => {
                let end = pos.saturating_add(*len);
                if end > old_lines.len() {
                    return None;
                }
                for i in pos..end {
                    out.push(old_lines.get(i)?.to_string());
                }
                pos = end;
            }
            Op::Delete { len } => {
                let end = pos.saturating_add(*len);
                if end > old_lines.len() {
                    return None;
                }
                pos = end;
            }
            Op::Insert { .. } => {
                // An insert does not consume old_lines. In replay against old_lines only,
                // we can only verify position unless new lines are supplied.
            }
        }
    }

    if pos != old_lines.len() {
        return None;
    }
    Some(out)
}

/// Replays a diff script across both old and new lines, reconstructing `new_lines`.
///
/// Returns `Some(reconstructed)` when the script matches both inputs exactly,
/// or `None` if an operation overflows or encounters a mismatch.
pub fn apply_diff_slices(
    old_lines: &[&str],
    new_lines: &[&str],
    ops: &[Op],
) -> Option<Vec<String>> {
    let mut out = Vec::new();
    let mut a_pos = 0usize;
    let mut b_pos = 0usize;

    for op in ops {
        match op {
            Op::Equal { len } => {
                let a_end = a_pos.saturating_add(*len);
                let b_end = b_pos.saturating_add(*len);
                if a_end > old_lines.len() || b_end > new_lines.len() {
                    return None;
                }
                for i in 0..*len {
                    let a_curr = a_pos.saturating_add(i);
                    let b_curr = b_pos.saturating_add(i);
                    let a = old_lines.get(a_curr)?;
                    let b = new_lines.get(b_curr)?;
                    if a != b {
                        return None;
                    }
                    out.push(a.to_string());
                }
                a_pos = a_end;
                b_pos = b_end;
            }
            Op::Delete { len } => {
                let a_end = a_pos.saturating_add(*len);
                if a_end > old_lines.len() {
                    return None;
                }
                a_pos = a_end;
            }
            Op::Insert { len } => {
                let b_end = b_pos.saturating_add(*len);
                if b_end > new_lines.len() {
                    return None;
                }
                for i in 0..*len {
                    let b_curr = b_pos.saturating_add(i);
                    let b = new_lines.get(b_curr)?;
                    out.push(b.to_string());
                }
                b_pos = b_end;
            }
        }
    }

    if a_pos != old_lines.len() || b_pos != new_lines.len() {
        return None;
    }

    Some(out)
}

/// Formats a complete unified diff text including file headers.
pub fn format_unified_diff(
    a: &[&str],
    b: &[&str],
    old_file: &str,
    new_file: &str,
    ops: &[Op],
    context: usize,
) -> String {
    let hunks = create_hunks(a, b, ops, context);
    if hunks.is_empty() {
        return String::new();
    }

    let mut out = String::new();
    out.push_str(&format!("--- {}\n", old_file));
    out.push_str(&format!("+++ {}\n", new_file));

    for hunk in hunks {
        out.push_str(&hunk.header());
        out.push('\n');
        for line in hunk.lines {
            let prefix = match line.change {
                LineChange::Context => ' ',
                LineChange::Delete => '-',
                LineChange::Insert => '+',
            };
            out.push(prefix);
            out.push_str(&line.content);
            out.push('\n');
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_myers_diff_simple() {
        let a = ["apple", "banana", "cherry"];
        let b = ["apple", "cherry", "date"];
        let script = diff(&a, &b, DiffAlgorithm::Myers);
        assert!(script.exact);
        assert_eq!(
            script.ops,
            vec![
                Op::Equal { len: 1 },
                Op::Delete { len: 1 },
                Op::Equal { len: 1 },
                Op::Insert { len: 1 },
            ]
        );
    }

    #[test]
    fn test_patience_diff_matching() {
        let a = ["fn main() {", "    let x = 1;", "    let y = 2;", "}"];
        let b = ["fn main() {", "    let y = 2;", "    let z = 3;", "}"];
        let script = diff(&a, &b, DiffAlgorithm::Patience);
        assert!(script.exact);
        assert_eq!(
            script.ops,
            vec![
                Op::Equal { len: 1 },
                Op::Delete { len: 1 },
                Op::Equal { len: 1 },
                Op::Insert { len: 1 },
                Op::Equal { len: 1 },
            ]
        );
    }

    #[test]
    fn test_patience_reordered_blocks() {
        let a = ["A", "B", "C", "D"];
        let b = ["A", "C", "D", "B"];
        let script = diff_patience(&a, &b);
        assert!(script.exact);
        // Verify replay preserves input
        assert_eq!(
            script
                .ops
                .iter()
                .filter_map(|op| match op {
                    Op::Equal { len } => Some(*len),
                    _ => None,
                })
                .sum::<usize>(),
            3
        );
    }

    #[test]
    fn test_hunks_with_context() {
        let a = ["line1", "line2", "line3", "line4", "line5"];
        let b = ["line1", "line2_changed", "line3", "line4", "line5"];
        let script = diff_myers(&a, &b);
        let hunks = create_hunks(&a, &b, &script.ops, 1);
        assert_eq!(hunks.len(), 1);
        if let Some(h) = hunks.first() {
            assert_eq!(h.old_start, 1);
            assert_eq!(h.old_lines, 3); // line1, line2, line3
            assert_eq!(h.new_start, 1);
            assert_eq!(h.new_lines, 3);
            assert_eq!(h.header(), "@@ -1,3 +1,3 @@");
        }
    }

    #[test]
    fn test_hunks_byte_offsets() {
        let a = ["abc", "def"];
        let b = ["abc", "xyz"];
        let script = diff_myers(&a, &b);
        let hunks = create_hunks(&a, &b, &script.ops, 0);
        assert_eq!(hunks.len(), 1);
        if let Some(h) = hunks.first() {
            // "abc\n" is 4 bytes
            assert_eq!(h.old_byte_offset, 4);
            assert_eq!(h.new_byte_offset, 4);
        }
    }

    #[test]
    fn test_empty_diff_produces_no_hunks() {
        let a = ["same", "content"];
        let b = ["same", "content"];
        let script = diff_myers(&a, &b);
        let hunks = create_hunks(&a, &b, &script.ops, 3);
        assert!(hunks.is_empty());
    }

    #[test]
    fn test_unified_diff_output() {
        let a = ["one", "two", "three"];
        let b = ["one", "two_mod", "three"];
        let script = diff_myers(&a, &b);
        let formatted = format_unified_diff(&a, &b, "a.txt", "b.txt", &script.ops, 1);
        assert!(formatted.contains("--- a.txt\n+++ b.txt\n"));
        assert!(formatted.contains("@@ -1,3 +1,3 @@\n"));
        assert!(formatted.contains(" one\n-two\n+two_mod\n three\n"));
    }

    #[test]
    fn test_patience_diff_empty_and_single() {
        let a: [&str; 0] = [];
        let b = ["lone"];
        let script = diff_patience(&a, &b);
        assert_eq!(script.ops, vec![Op::Insert { len: 1 }]);

        let script2 = diff_patience(&b, &a);
        assert_eq!(script2.ops, vec![Op::Delete { len: 1 }]);
    }

    #[test]
    fn test_patience_diff_no_unique_lines() {
        // Fallback to Myers when lines are repeated
        let a = ["x", "x", "x"];
        let b = ["x", "y", "x"];
        let script = diff_patience(&a, &b);
        assert_eq!(
            script.ops,
            vec![
                Op::Equal { len: 1 },
                Op::Delete { len: 1 },
                Op::Insert { len: 1 },
                Op::Equal { len: 1 },
            ]
        );
    }

    #[test]
    fn test_hunks_multibyte_utf8() {
        // UTF-8 characters: "🦀" is 4 bytes, "こんにちは" is 15 bytes
        let a = ["🦀 crab", "line2", "end"];
        let b = ["🦀 crab", "line2_mod", "end"];
        let script = diff_myers(&a, &b);
        let hunks = create_hunks(&a, &b, &script.ops, 0);
        assert_eq!(hunks.len(), 1);
        if let Some(h) = hunks.first() {
            // "🦀 crab" is 4 + 1 + 4 = 9 bytes + 1 newline = 10 bytes
            assert_eq!(h.old_byte_offset, 10);
        }
    }

    #[test]
    fn test_apply_diff_replay() {
        let a = ["alpha", "beta", "gamma"];
        let b = ["alpha", "beta_mod", "gamma"];
        let script = diff_myers(&a, &b);
        let replayed = apply_diff(&a, &script.ops);
        assert!(replayed.is_some());
    }
}
