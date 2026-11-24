//! Unified diff renderer for text comparisons.
//!
//! Formats text differences in standard patch notation with three
//! lines of surrounding context.

use autocompare_compare::TextComparison;

use crate::render_text::{Block, blocks};
use crate::term;

/// A single line within a unified diff hunk.
struct ULine<'a> {
    kind: UKind,
    old_no: Option<usize>,
    new_no: Option<usize>,
    text: &'a str,
}

enum UKind {
    Context,
    Del,
    Ins,
}

fn ulines<'a>(cmp: &'a TextComparison) -> Vec<ULine<'a>> {
    let mut out = Vec::new();
    let mut old_no = 0usize;
    let mut new_no = 0usize;
    for block in blocks(cmp) {
        match block {
            Block::Equal { left, right } => {
                for (l, _r) in left.iter().zip(right.iter()) {
                    old_no = old_no.saturating_add(1);
                    new_no = new_no.saturating_add(1);
                    out.push(ULine {
                        kind: UKind::Context,
                        old_no: Some(old_no),
                        new_no: Some(new_no),
                        text: l,
                    });
                }
            }
            Block::Deleted { lines } => {
                for l in lines {
                    old_no = old_no.saturating_add(1);
                    out.push(ULine {
                        kind: UKind::Del,
                        old_no: Some(old_no),
                        new_no: None,
                        text: l,
                    });
                }
            }
            Block::Inserted { lines } => {
                for r in lines {
                    new_no = new_no.saturating_add(1);
                    out.push(ULine {
                        kind: UKind::Ins,
                        old_no: None,
                        new_no: Some(new_no),
                        text: r,
                    });
                }
            }
            Block::Changed { left, right } => {
                for l in left {
                    old_no = old_no.saturating_add(1);
                    out.push(ULine {
                        kind: UKind::Del,
                        old_no: Some(old_no),
                        new_no: None,
                        text: l,
                    });
                }
                for r in right {
                    new_no = new_no.saturating_add(1);
                    out.push(ULine {
                        kind: UKind::Ins,
                        old_no: None,
                        new_no: Some(new_no),
                        text: r,
                    });
                }
            }
        }
    }
    out
}

/// Renders the comparison as a unified diff with three lines of context.
pub fn unified(cmp: &TextComparison, color: bool) {
    let lines = ulines(cmp);
    let ctx = 3usize;
    let changed: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| !matches!(line.kind, UKind::Context))
        .map(|(i, _)| i)
        .collect();
    let mut group_start = 0usize;
    while let Some(&first) = changed.get(group_start) {
        let mut last = first;
        let mut look = group_start.saturating_add(1);
        while let Some(&pos) = changed.get(look) {
            if pos <= last.saturating_add(2usize.saturating_mul(ctx)) {
                last = pos;
                look = look.saturating_add(1);
            } else {
                break;
            }
        }
        emit_hunk(
            &lines,
            first.saturating_sub(ctx),
            last.saturating_add(ctx),
            color,
        );
        group_start = look;
    }
}

fn emit_hunk(lines: &[ULine<'_>], start: usize, end: usize, color: bool) {
    if lines.is_empty() {
        return;
    }
    let max_idx = lines.len().saturating_sub(1);
    let start = start.min(max_idx);
    let end = end.min(max_idx);
    if end < start {
        return;
    }
    let old_start = anchor(lines, start, |l| l.old_no);
    let new_start = anchor(lines, start, |l| l.new_no);
    let mut old_count = 0usize;
    let mut new_count = 0usize;
    for line in lines.iter().take(end.saturating_add(1)).skip(start) {
        if line.old_no.is_some() {
            old_count = old_count.saturating_add(1);
        }
        if line.new_no.is_some() {
            new_count = new_count.saturating_add(1);
        }
    }
    println!("@@ -{old_start},{old_count} +{new_start},{new_count} @@");
    for line in lines.iter().take(end.saturating_add(1)).skip(start) {
        match line.kind {
            UKind::Context => println!(" {}", line.text),
            UKind::Del => println!(
                "{}",
                term::paint(color, term::RED, &format!("-{}", line.text))
            ),
            UKind::Ins => {
                println!(
                    "{}",
                    term::paint(color, term::GREEN, &format!("+{}", line.text))
                )
            }
        }
    }
}

fn anchor<F>(lines: &[ULine<'_>], start: usize, pick: F) -> usize
where
    F: Fn(&ULine<'_>) -> Option<usize>,
{
    for line in lines.iter().skip(start) {
        if let Some(no) = pick(line) {
            return no;
        }
    }
    for line in lines.iter().take(start).rev() {
        if let Some(no) = pick(line) {
            return no.saturating_add(1);
        }
    }
    0
}
