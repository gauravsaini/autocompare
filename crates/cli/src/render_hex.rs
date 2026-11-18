//! Byte-level rendering: aligned hex and ASCII panes with differing
//! bytes accented.

use autocompare_compare::{BinaryComparison, BinaryHunk};

use crate::term;

/// Whether the comparison found any difference.
pub fn outcome(cmp: &BinaryComparison) -> crate::Outcome {
    if cmp.first_diff.is_some() {
        crate::Outcome::Different
    } else {
        crate::Outcome::Same
    }
}

pub fn print(cmp: &BinaryComparison, color: bool) {
    println!("left:  {} bytes", cmp.left_size);
    println!("right: {} bytes", cmp.right_size);
    match cmp.first_diff {
        None => {
            println!("no differences");
        }
        Some(off) => {
            println!("first difference at offset {off:#010x}");
            for hunk in &cmp.hunks {
                print_hunk(hunk, color);
            }
        }
    }
    if cmp.truncated {
        eprintln!("autocompare: note: more differing runs exist (hunk limit reached)");
    }
}

fn print_hunk(hunk: &BinaryHunk, color: bool) {
    let rows = hunk.left.len().max(hunk.right.len()).div_ceil(8);
    for row in 0..rows {
        let base = row.saturating_mul(8);
        let off = hunk.offset.saturating_add(base as u64);
        let l = hunk
            .left
            .get(base..(base + 8).min(hunk.left.len()))
            .unwrap_or_default();
        let r = hunk
            .right
            .get(base..(base + 8).min(hunk.right.len()))
            .unwrap_or_default();
        println!(
            "{off:#010x}  {} {} | {} {}",
            hex(l, r, color),
            ascii(l),
            hex(r, l, color),
            ascii(r)
        );
    }
}

/// Hex bytes of one row, with bytes that differ from the other side
/// accented.
fn hex(bytes: &[u8], other: &[u8], color: bool) -> String {
    let mut out = String::new();
    for i in 0..8 {
        out.push(' ');
        match bytes.get(i) {
            Some(b) => {
                let differs = other.get(i).map(|o| o != b).unwrap_or(false);
                let cell = format!("{b:02x}");
                if differs {
                    out.push_str(&term::paint(color, term::RED, &cell));
                } else {
                    out.push_str(&cell);
                }
            }
            None => out.push_str("  "),
        }
    }
    out
}

fn ascii(bytes: &[u8]) -> String {
    let mut out = String::new();
    for i in 0..8 {
        match bytes.get(i) {
            Some(b) if (0x20..0x7f).contains(b) => out.push(*b as char),
            Some(_) => out.push('.'),
            None => out.push(' '),
        }
    }
    out
}
