//! Listing renderer for folder comparisons.

use autocompare_compare::{FolderComparison, FolderEntry, FolderStatus};

use crate::term;

/// Whether the comparison found any difference.
pub fn outcome(cmp: &FolderComparison) -> crate::Outcome {
    let changed = cmp.entries.iter().any(|e| e.status != FolderStatus::Same);
    if changed {
        crate::Outcome::Different
    } else {
        crate::Outcome::Same
    }
}

/// Prints every entry with a status glyph, sizes and times, then a
/// one-line summary.
pub fn print(cmp: &FolderComparison, color: bool) {
    if cmp.truncated {
        eprintln!(
            "autocompare: note: the walk stopped at the depth limit; deeper entries are not listed"
        );
    }
    for entry in &cmp.entries {
        print_entry(entry, color);
    }
    let mut same = 0usize;
    let mut different = 0usize;
    let mut newer_left = 0usize;
    let mut newer_right = 0usize;
    let mut left_only = 0usize;
    let mut right_only = 0usize;
    for entry in &cmp.entries {
        match entry.status {
            FolderStatus::Same => same = same.saturating_add(1),
            FolderStatus::Different => different = different.saturating_add(1),
            FolderStatus::NewerLeft => newer_left = newer_left.saturating_add(1),
            FolderStatus::NewerRight => newer_right = newer_right.saturating_add(1),
            FolderStatus::LeftOnly => left_only = left_only.saturating_add(1),
            FolderStatus::RightOnly => right_only = right_only.saturating_add(1),
        }
    }
    println!(
        "{} entries: {} same, {} different, {} newer left, {} newer right, {} left-only, {} right-only",
        cmp.entries.len(),
        same,
        different,
        newer_left,
        newer_right,
        left_only,
        right_only
    );
}

fn print_entry(entry: &FolderEntry, color: bool) {
    let (glyph, code) = match entry.status {
        FolderStatus::Same => ("=", term::DIM),
        FolderStatus::Different => ("!", term::RED),
        FolderStatus::NewerLeft => (">", term::BLUE),
        FolderStatus::NewerRight => ("<", term::BLUE),
        FolderStatus::LeftOnly => ("L", term::RED),
        FolderStatus::RightOnly => ("R", term::GREEN),
    };
    let dir_mark = if entry.is_dir { "/" } else { "" };
    let size_l = side_size(entry.left);
    let size_r = side_size(entry.right);
    let time_l = side_time(entry.left);
    let time_r = side_time(entry.right);
    println!(
        "{} {:>10} {:>10}  {:>16} {:>16}  {}{}",
        term::paint(color, code, glyph),
        size_l,
        size_r,
        time_l,
        time_r,
        entry.rel_path,
        dir_mark
    );
}

fn side_size(info: Option<autocompare_compare::NodeInfo>) -> String {
    match info {
        Some(n) => n.size.to_string(),
        None => "-".to_string(),
    }
}

fn side_time(info: Option<autocompare_compare::NodeInfo>) -> String {
    match info {
        Some(n) => term::format_time(n.modified),
        None => "-".to_string(),
    }
}
