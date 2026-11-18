//! Side-by-side and unified rendering of a text comparison.

use autocompare_compare::TextComparison;
use autocompare_diff::{InlineSpan, Op};

use crate::term;

/// One aligned block of the comparison.
enum Block<'a> {
    Equal {
        left: Vec<&'a str>,
        right: Vec<&'a str>,
    },
    Deleted {
        lines: Vec<&'a str>,
    },
    Inserted {
        lines: Vec<&'a str>,
    },
    Changed {
        left: Vec<&'a str>,
        right: Vec<&'a str>,
    },
}

/// Splits the edit script into renderable blocks, pairing adjacent
/// delete/insert runs into changed blocks.
fn blocks<'a>(cmp: &'a TextComparison) -> Vec<Block<'a>> {
    let mut out = Vec::new();
    let mut left_idx = 0usize;
    let mut right_idx = 0usize;
    let mut ops = cmp.script.ops.iter().cloned().peekable();
    while let Some(op) = ops.next() {
        match op {
            Op::Equal { len } => {
                let left = slice_lines(&cmp.left_lines, left_idx, len);
                let right = slice_lines(&cmp.right_lines, right_idx, len);
                left_idx = left_idx.saturating_add(len);
                right_idx = right_idx.saturating_add(len);
                out.push(Block::Equal { left, right });
            }
            Op::Delete { len } => {
                let left = slice_lines(&cmp.left_lines, left_idx, len);
                left_idx = left_idx.saturating_add(len);
                if let Some(Op::Insert { len: ins }) = ops.peek().cloned() {
                    ops.next();
                    let right = slice_lines(&cmp.right_lines, right_idx, ins);
                    right_idx = right_idx.saturating_add(ins);
                    out.push(Block::Changed { left, right });
                } else {
                    out.push(Block::Deleted { lines: left });
                }
            }
            Op::Insert { len } => {
                let right = slice_lines(&cmp.right_lines, right_idx, len);
                right_idx = right_idx.saturating_add(len);
                if let Some(Op::Delete { len: del }) = ops.peek().cloned() {
                    ops.next();
                    let left = slice_lines(&cmp.left_lines, left_idx, del);
                    left_idx = left_idx.saturating_add(del);
                    out.push(Block::Changed { left, right });
                } else {
                    out.push(Block::Inserted { lines: right });
                }
            }
        }
    }
    out
}

fn slice_lines(lines: &[String], start: usize, len: usize) -> Vec<&str> {
    if start >= lines.len() {
        return Vec::new();
    }
    let end = start.saturating_add(len).min(lines.len());
    lines
        .get(start..end)
        .map(|slice| slice.iter().map(|line| line.as_str()).collect())
        .unwrap_or_default()
}

/// Whether the comparison found any difference.
pub fn outcome(cmp: &TextComparison) -> crate::Outcome {
    let changed = cmp
        .script
        .ops
        .iter()
        .any(|op| !matches!(op, Op::Equal { .. }));
    if changed {
        crate::Outcome::Different
    } else {
        crate::Outcome::Same
    }
}

/// Renders the comparison as two aligned, colored panes.
pub fn side_by_side(cmp: &TextComparison, width: usize, color: bool) {
    if !cmp.script.exact {
        eprintln!(
            "autocompare: note: inputs exceeded the edit budget, showing a coarse comparison"
        );
    }
    let pane = ((width.saturating_sub(13)) / 2).max(8);
    let mut left_no = 0usize;
    let mut right_no = 0usize;
    for block in blocks(cmp) {
        match block {
            Block::Equal { left, right } => {
                for (l, r) in left.iter().zip(right.iter()) {
                    left_no = left_no.saturating_add(1);
                    right_no = right_no.saturating_add(1);
                    print_row(
                        Some(left_no),
                        Some(right_no),
                        l,
                        r,
                        color,
                        "",
                        "",
                        &[],
                        pane,
                    );
                }
            }
            Block::Deleted { lines } => {
                for l in lines {
                    left_no = left_no.saturating_add(1);
                    print_row(
                        Some(left_no),
                        None,
                        l,
                        "",
                        color,
                        term::BG_RED,
                        "",
                        &[],
                        pane,
                    );
                }
            }
            Block::Inserted { lines } => {
                for r in lines {
                    right_no = right_no.saturating_add(1);
                    print_row(
                        None,
                        Some(right_no),
                        "",
                        r,
                        color,
                        "",
                        term::BG_GREEN,
                        &[],
                        pane,
                    );
                }
            }
            Block::Changed { left, right } => {
                let pairs = left.len().min(right.len());
                let mut i = 0usize;
                while i < pairs {
                    left_no = left_no.saturating_add(1);
                    right_no = right_no.saturating_add(1);
                    let l = left.get(i).copied().unwrap_or("");
                    let r = right.get(i).copied().unwrap_or("");
                    let spans = if color {
                        autocompare_diff::inline_diff(l, r)
                    } else {
                        Vec::new()
                    };
                    print_row(
                        Some(left_no),
                        Some(right_no),
                        l,
                        r,
                        color,
                        term::BG_YELLOW,
                        term::BG_YELLOW,
                        &spans,
                        pane,
                    );
                    i = i.saturating_add(1);
                }
                let mut extra = pairs;
                while extra < left.len() {
                    left_no = left_no.saturating_add(1);
                    let l = left.get(extra).copied().unwrap_or("");
                    print_row(
                        Some(left_no),
                        None,
                        l,
                        "",
                        color,
                        term::BG_RED,
                        "",
                        &[],
                        pane,
                    );
                    extra = extra.saturating_add(1);
                }
                let mut extra = pairs;
                while extra < right.len() {
                    right_no = right_no.saturating_add(1);
                    let r = right.get(extra).copied().unwrap_or("");
                    print_row(
                        None,
                        Some(right_no),
                        "",
                        r,
                        color,
                        "",
                        term::BG_GREEN,
                        &[],
                        pane,
                    );
                    extra = extra.saturating_add(1);
                }
            }
        }
    }
}

/// Prints one aligned row: optional line numbers, the two panes, and
/// accent codes inside changed-character spans.
#[allow(clippy::too_many_arguments)]
fn print_row(
    left_no: Option<usize>,
    right_no: Option<usize>,
    left: &str,
    right: &str,
    color: bool,
    left_bg: &str,
    right_bg: &str,
    spans: &[InlineSpan],
    pane: usize,
) {
    let lnum = number(left_no, color);
    let rnum = number(right_no, color);
    let lpane = paint_pane(left, spans, true, color, left_bg, pane);
    let rpane = paint_pane(right, spans, false, color, right_bg, pane);
    println!("{lnum}{lpane} | {rnum}{rpane}");
}

fn number(no: Option<usize>, color: bool) -> String {
    match no {
        Some(n) => term::paint(color, term::DIM, &format!("{n:>4} ")),
        None => "     ".to_string(),
    }
}

/// Renders one pane: truncates to `pane` characters, pads with spaces
/// so columns stay aligned, and accents the changed spans.
fn paint_pane(
    text: &str,
    spans: &[InlineSpan],
    take_left: bool,
    color: bool,
    bg: &str,
    pane: usize,
) -> String {
    let mut out = String::new();
    if color && !bg.is_empty() {
        out.push_str(bg);
    }
    let mut visible = 0usize;
    for (idx, ch) in text.chars().enumerate() {
        if visible >= pane {
            break;
        }
        let inside = spans.iter().any(|s| {
            let (start, end) = if take_left {
                (s.a_start, s.a_end)
            } else {
                (s.b_start, s.b_end)
            };
            idx >= start && idx < end
        });
        if color && inside {
            out.push_str(term::ACCENT);
        }
        out.push(ch);
        if color && inside {
            out.push_str(term::ACCENT_OFF);
        }
        visible = visible.saturating_add(1);
    }
    while visible < pane {
        out.push(' ');
        visible = visible.saturating_add(1);
    }
    if color {
        out.push_str(term::RESET);
    }
    out
}

/// A single line of a unified diff.
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
                for (l, r) in left.iter().zip(right.iter()) {
                    old_no = old_no.saturating_add(1);
                    new_no = new_no.saturating_add(1);
                    out.push(ULine {
                        kind: UKind::Context,
                        old_no: Some(old_no),
                        new_no: Some(new_no),
                        text: l,
                    });
                    let _ = r;
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
        let mut look = group_start + 1;
        while let Some(&pos) = changed.get(look) {
            if pos <= last.saturating_add(2 * ctx + 1) {
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
    let start = start.min(lines.len().saturating_sub(1));
    let end = end.min(lines.len().saturating_sub(1));
    if end < start {
        return;
    }
    let old_start = lines.get(start).and_then(|l| l.old_no).unwrap_or(0);
    let new_start = lines.get(start).and_then(|l| l.new_no).unwrap_or(0);
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
