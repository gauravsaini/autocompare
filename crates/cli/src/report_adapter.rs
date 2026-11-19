//! Text, folder, binary, merge and table report adapters.
//!
//! Converts text, folder, binary, merge and table outcomes to ReportInput.

use autocompare_compare::{
    BinaryComparison, FolderComparison, FolderStatus, TableComparison, TextComparison,
};
use autocompare_diff::Op;
use autocompare_reports::{DiffStats, Hunk, HunkLine, ReportInput};

/// Converts text comparison outcome to ReportInput.
pub fn text_report_input(left: &str, right: &str, cmp: &TextComparison) -> ReportInput {
    let mut lines = Vec::new();
    let mut left_idx = 0usize;
    let mut right_idx = 0usize;
    let mut additions = 0usize;
    let mut deletions = 0usize;
    let mut unchanged = 0usize;

    for op in &cmp.script.ops {
        match *op {
            Op::Equal { len } => {
                for i in 0..len {
                    let old_no = left_idx.saturating_add(i).saturating_add(1);
                    let new_no = right_idx.saturating_add(i).saturating_add(1);
                    let content = cmp
                        .left_lines
                        .get(left_idx.saturating_add(i))
                        .map(|s| s.as_str())
                        .unwrap_or("");
                    lines.push(HunkLine::unchanged(content, old_no, new_no));
                    unchanged = unchanged.saturating_add(1);
                }
                left_idx = left_idx.saturating_add(len);
                right_idx = right_idx.saturating_add(len);
            }
            Op::Delete { len } => {
                for i in 0..len {
                    let old_no = left_idx.saturating_add(i).saturating_add(1);
                    let content = cmp
                        .left_lines
                        .get(left_idx.saturating_add(i))
                        .map(|s| s.as_str())
                        .unwrap_or("");
                    lines.push(HunkLine::delete(content, old_no));
                    deletions = deletions.saturating_add(1);
                }
                left_idx = left_idx.saturating_add(len);
            }
            Op::Insert { len } => {
                for i in 0..len {
                    let new_no = right_idx.saturating_add(i).saturating_add(1);
                    let content = cmp
                        .right_lines
                        .get(right_idx.saturating_add(i))
                        .map(|s| s.as_str())
                        .unwrap_or("");
                    lines.push(HunkLine::insert(content, new_no));
                    additions = additions.saturating_add(1);
                }
                right_idx = right_idx.saturating_add(len);
            }
        }
    }

    let stats = DiffStats::new(additions, deletions, 0, unchanged);
    let hunk = Hunk::new(1, cmp.left_lines.len(), 1, cmp.right_lines.len(), lines);
    ReportInput::new(format!("{left} <-> {right}"), vec![hunk], stats)
}

/// Converts folder comparison outcome to ReportInput.
pub fn folder_report_input(left: &str, right: &str, cmp: &FolderComparison) -> ReportInput {
    let mut lines = Vec::new();
    let mut additions = 0usize;
    let mut deletions = 0usize;
    let mut modifications = 0usize;
    let mut unchanged = 0usize;

    for (idx, entry) in cmp.entries.iter().enumerate() {
        let no = idx.saturating_add(1);
        match entry.status {
            FolderStatus::Same => {
                lines.push(HunkLine::unchanged(format!("= {}", entry.rel_path), no, no));
                unchanged = unchanged.saturating_add(1);
            }
            FolderStatus::LeftOnly => {
                lines.push(HunkLine::delete(format!("- {}", entry.rel_path), no));
                deletions = deletions.saturating_add(1);
            }
            FolderStatus::RightOnly => {
                lines.push(HunkLine::insert(format!("+ {}", entry.rel_path), no));
                additions = additions.saturating_add(1);
            }
            FolderStatus::Different | FolderStatus::NewerLeft | FolderStatus::NewerRight => {
                lines.push(HunkLine::modify(
                    format!("! {} ({:?})", entry.rel_path, entry.status),
                    no,
                    no,
                ));
                modifications = modifications.saturating_add(1);
            }
        }
    }

    let stats = DiffStats::new(additions, deletions, modifications, unchanged);
    let hunk = Hunk::new(1, cmp.entries.len(), 1, cmp.entries.len(), lines);
    ReportInput::new(format!("folder: {left} <-> {right}"), vec![hunk], stats)
}

/// Converts binary comparison outcome to ReportInput.
pub fn hex_report_input(left: &str, right: &str, cmp: &BinaryComparison) -> ReportInput {
    let mut lines = Vec::new();
    let modifications = cmp.hunks.len();
    if cmp.hunks.is_empty() {
        lines.push(HunkLine::unchanged("Binary files are identical", 1, 1));
    } else {
        for (i, hunk) in cmp.hunks.iter().enumerate() {
            let no = i.saturating_add(1);
            lines.push(HunkLine::modify(
                format!(
                    "offset {:#010x}: left={:02x?} right={:02x?}",
                    hunk.offset, hunk.left, hunk.right
                ),
                no,
                no,
            ));
        }
    }
    let stats = DiffStats::new(
        0,
        0,
        modifications,
        if cmp.hunks.is_empty() { 1 } else { 0 },
    );
    let hunk = Hunk::new(1, lines.len(), 1, lines.len(), lines);
    ReportInput::new(format!("hex: {left} <-> {right}"), vec![hunk], stats)
}

/// Converts three-way merge outcome to ReportInput.
pub fn merge_report_input(base: &str, left: &str, right: &str, merged: &[String]) -> ReportInput {
    let mut lines = Vec::new();
    let mut modifications = 0usize;
    let mut unchanged = 0usize;

    for (i, line) in merged.iter().enumerate() {
        let no = i.saturating_add(1);
        if line.starts_with("<<<<<<<") || line.starts_with("=======") || line.starts_with(">>>>>>>")
        {
            lines.push(HunkLine::modify(line, no, no));
            modifications = modifications.saturating_add(1);
        } else {
            lines.push(HunkLine::unchanged(line, no, no));
            unchanged = unchanged.saturating_add(1);
        }
    }

    let stats = DiffStats::new(0, 0, modifications, unchanged);
    let hunk = Hunk::new(1, merged.len(), 1, merged.len(), lines);
    ReportInput::new(
        format!("merge: {base} + {left} + {right}"),
        vec![hunk],
        stats,
    )
}

/// Converts table comparison outcome to ReportInput.
pub fn table_report_input(left: &str, right: &str, cmp: &TableComparison) -> ReportInput {
    let mut lines = Vec::new();
    let mut additions = 0usize;
    let mut deletions = 0usize;
    let mut unchanged = 0usize;
    let mut l_idx = 0usize;
    let mut r_idx = 0usize;

    for op in &cmp.script.ops {
        match *op {
            Op::Equal { len } => {
                for i in 0..len {
                    let old_no = l_idx.saturating_add(i).saturating_add(1);
                    let new_no = r_idx.saturating_add(i).saturating_add(1);
                    let row = cmp
                        .rows_l
                        .get(l_idx.saturating_add(i))
                        .map(|r| r.join(","))
                        .unwrap_or_default();
                    lines.push(HunkLine::unchanged(row, old_no, new_no));
                    unchanged = unchanged.saturating_add(1);
                }
                l_idx = l_idx.saturating_add(len);
                r_idx = r_idx.saturating_add(len);
            }
            Op::Delete { len } => {
                for i in 0..len {
                    let old_no = l_idx.saturating_add(i).saturating_add(1);
                    let row = cmp
                        .rows_l
                        .get(l_idx.saturating_add(i))
                        .map(|r| r.join(","))
                        .unwrap_or_default();
                    lines.push(HunkLine::delete(row, old_no));
                    deletions = deletions.saturating_add(1);
                }
                l_idx = l_idx.saturating_add(len);
            }
            Op::Insert { len } => {
                for i in 0..len {
                    let new_no = r_idx.saturating_add(i).saturating_add(1);
                    let row = cmp
                        .rows_r
                        .get(r_idx.saturating_add(i))
                        .map(|r| r.join(","))
                        .unwrap_or_default();
                    lines.push(HunkLine::insert(row, new_no));
                    additions = additions.saturating_add(1);
                }
                r_idx = r_idx.saturating_add(len);
            }
        }
    }

    let stats = DiffStats::new(additions, deletions, 0, unchanged);
    let hunk = Hunk::new(1, cmp.rows_l.len(), 1, cmp.rows_r.len(), lines);
    ReportInput::new(format!("table: {left} <-> {right}"), vec![hunk], stats)
}
