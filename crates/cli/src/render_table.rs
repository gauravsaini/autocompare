//! Delta-style rendering of a delimited table comparison: the header
//! once, then one line per row; changed cells show as `[old => new]`.

use autocompare_compare::TableComparison;
use autocompare_diff::{Op, diff_cells};

use crate::term;

/// Whether the comparison found any difference: any changed, added or
/// removed row, or a differing header.
pub fn outcome(cmp: &TableComparison) -> crate::Outcome {
    let rows_changed = cmp
        .script
        .ops
        .iter()
        .any(|op| !matches!(op, Op::Equal { .. }));
    if rows_changed || cmp.header_l != cmp.header_r {
        crate::Outcome::Different
    } else {
        crate::Outcome::Same
    }
}

/// Prints the comparison. One line per row: `=` equal (dim), `!`
/// changed (cells that differ shown as `[old => new]`, accented), `L`
/// only in the left table, `R` only in the right one.
pub fn print(cmp: &TableComparison, color: bool) {
    if !cmp.header_l.is_empty() || !cmp.header_r.is_empty() {
        if cmp.header_l == cmp.header_r {
            plain_line(color, "=", " hdr", &cmp.header_r, term::DIM);
        } else {
            changed_line(color, "!", " hdr", &cmp.header_l, &cmp.header_r);
        }
    }
    let mut lrow = 0usize;
    let mut rrow = 0usize;
    let mut ops = cmp.script.ops.iter().cloned().peekable();
    while let Some(op) = ops.next() {
        match op {
            Op::Equal { len } => {
                for _ in 0..len {
                    let cells = cmp.rows_r.get(rrow).cloned().unwrap_or_default();
                    let label = row_label(rrow.saturating_add(1));
                    lrow = lrow.saturating_add(1);
                    rrow = rrow.saturating_add(1);
                    plain_line(color, "=", &label, &cells, term::DIM);
                }
            }
            Op::Delete { len } => {
                let left_rows = rows_slice(&cmp.rows_l, lrow, len);
                let lbase = lrow;
                lrow = lrow.saturating_add(len);
                if let Some(Op::Insert { len: ins }) = ops.peek().cloned() {
                    ops.next();
                    let right_rows = rows_slice(&cmp.rows_r, rrow, ins);
                    let rbase = rrow;
                    rrow = rrow.saturating_add(ins);
                    let pairs = left_rows.len().min(right_rows.len());
                    let mut i = 0usize;
                    while i < pairs {
                        let l = left_rows.get(i).cloned().unwrap_or_default();
                        let r = right_rows.get(i).cloned().unwrap_or_default();
                        let label = row_label(rbase.saturating_add(i).saturating_add(1));
                        changed_line(color, "!", &label, &l, &r);
                        i = i.saturating_add(1);
                    }
                    let mut extra = pairs;
                    while extra < left_rows.len() {
                        let cells = left_rows.get(extra).cloned().unwrap_or_default();
                        let label = row_label(lbase.saturating_add(extra).saturating_add(1));
                        plain_line(color, "L", &label, &cells, term::RED);
                        extra = extra.saturating_add(1);
                    }
                    let mut extra = pairs;
                    while extra < right_rows.len() {
                        let cells = right_rows.get(extra).cloned().unwrap_or_default();
                        let label = row_label(rbase.saturating_add(extra).saturating_add(1));
                        plain_line(color, "R", &label, &cells, term::GREEN);
                        extra = extra.saturating_add(1);
                    }
                } else {
                    let mut i = 0usize;
                    while let Some(cells) = left_rows.get(i) {
                        let label = row_label(lbase.saturating_add(i).saturating_add(1));
                        plain_line(color, "L", &label, cells, term::RED);
                        i = i.saturating_add(1);
                    }
                }
            }
            Op::Insert { len } => {
                let right_rows = rows_slice(&cmp.rows_r, rrow, len);
                let rbase = rrow;
                rrow = rrow.saturating_add(len);
                let mut i = 0usize;
                while let Some(cells) = right_rows.get(i) {
                    let label = row_label(rbase.saturating_add(i).saturating_add(1));
                    plain_line(color, "R", &label, cells, term::GREEN);
                    i = i.saturating_add(1);
                }
            }
        }
    }
}

fn rows_slice(rows: &[Vec<String>], start: usize, len: usize) -> Vec<Vec<String>> {
    if start >= rows.len() {
        return Vec::new();
    }
    let end = start.saturating_add(len).min(rows.len());
    rows.get(start..end)
        .map(|slice| slice.to_vec())
        .unwrap_or_default()
}

fn row_label(n: usize) -> String {
    format!("{n:>4}")
}

fn plain_line(color: bool, glyph: &str, label: &str, cells: &[String], code: &'static str) {
    println!(
        "{} {} | {}",
        term::paint(color, code, glyph),
        label,
        cells.join(" | ")
    );
}

fn changed_line(color: bool, glyph: &str, label: &str, left: &[String], right: &[String]) {
    let changed = diff_cells(left, right);
    let mut cells = Vec::new();
    let mut idx = 0usize;
    loop {
        if left.get(idx).is_none() && right.get(idx).is_none() {
            break;
        }
        let is_changed = changed.contains(&idx);
        let text = if is_changed {
            format!(
                "[{} => {}]",
                left.get(idx).cloned().unwrap_or_default(),
                right.get(idx).cloned().unwrap_or_default()
            )
        } else {
            right.get(idx).cloned().unwrap_or_default()
        };
        if is_changed {
            cells.push(term::paint(color, term::ACCENT, &text));
        } else {
            cells.push(text);
        }
        idx = idx.saturating_add(1);
    }
    println!(
        "{} {} | {}",
        term::paint(color, term::YELLOW, glyph),
        label,
        cells.join(" | ")
    );
}
