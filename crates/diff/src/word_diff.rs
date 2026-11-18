//! Word-level inline spans for one pair of lines.
//!
//! Each line is split into whitespace-delimited tokens that remember
//! their character range; the same Myers machinery used for lines
//! diffs the token texts; and Delete/Insert runs map back to one
//! [`InlineSpan`] per change, covering the first token's start through
//! the last token's end on each side. A run that exists on only one
//! side gets a zero-length range on the other side (start == end),
//! which renderers treat as "nothing to highlight".

use crate::{InlineSpan, Op, engine};

/// One whitespace-delimited token with its half-open character range.
struct Token<'a> {
    text: &'a str,
    start: usize,
    end: usize,
}

/// Character offset of token `i`, or the end anchor when the token
/// does not exist.
fn tok_start(toks: &[Token<'_>], i: usize) -> usize {
    match toks.get(i) {
        Some(t) => t.start,
        None => end_anchor(toks),
    }
}

/// Character offset one past token `i`, or the end anchor when the
/// token does not exist.
fn tok_end(toks: &[Token<'_>], i: usize) -> usize {
    match toks.get(i) {
        Some(t) => t.end,
        None => end_anchor(toks),
    }
}

/// The position "adjacent to" token index `i`: the start of the token
/// at `i` when it exists, the end of the last token otherwise, and 0
/// for a side with no tokens at all.
fn anchor(toks: &[Token<'_>], i: usize) -> usize {
    match toks.get(i) {
        Some(t) => t.start,
        None => end_anchor(toks),
    }
}

fn end_anchor(toks: &[Token<'_>]) -> usize {
    match toks.last() {
        Some(t) => t.end,
        None => 0,
    }
}

/// Splits a line into tokens on whitespace, recording each token's
/// half-open character range. Whitespace between tokens belongs to no
/// span.
fn tokenize(line: &str) -> Vec<Token<'_>> {
    let mut out: Vec<Token<'_>> = Vec::new();
    let mut cpos = 0usize; // current character index
    let mut in_token = false;
    let mut bstart = 0usize; // byte offset where the token began
    let mut cstart = 0usize; // character index where the token began
    for (boff, ch) in line.char_indices() {
        if ch.is_whitespace() {
            if in_token {
                out.push(Token {
                    text: line.get(bstart..boff).unwrap_or(""),
                    start: cstart,
                    end: cpos,
                });
                in_token = false;
            }
        } else if !in_token {
            in_token = true;
            bstart = boff;
            cstart = cpos;
        }
        cpos += 1;
    }
    if in_token {
        out.push(Token {
            text: line.get(bstart..line.len()).unwrap_or(""),
            start: cstart,
            end: cpos,
        });
    }
    out
}

/// Changed character spans for one pair of lines. Empty when the lines
/// are equal.
pub(crate) fn inline_diff(a: &str, b: &str) -> Vec<InlineSpan> {
    if a == b {
        return Vec::new();
    }
    let ta = tokenize(a);
    let tb = tokenize(b);
    let sa: Vec<&str> = ta.iter().map(|t| t.text).collect();
    let sb: Vec<&str> = tb.iter().map(|t| t.text).collect();
    let (ops, _exact) = engine::diff_slices(&sa, &sb, Some(crate::EDIT_BUDGET));
    let mut spans: Vec<InlineSpan> = Vec::new();
    let (mut ai, mut bi) = (0usize, 0usize);
    let mut idx = 0usize;
    while let Some(op) = ops.get(idx) {
        match op {
            Op::Equal { len } => {
                ai = ai.saturating_add(*len);
                bi = bi.saturating_add(*len);
            }
            Op::Delete { len } => {
                let last_a = ai.saturating_add(*len).saturating_sub(1);
                // A replace block is a Delete directly followed by an
                // Insert (canonical order).
                match ops.get(idx.saturating_add(1)) {
                    Some(Op::Insert { len: ins }) => {
                        let last_b = bi.saturating_add(*ins).saturating_sub(1);
                        spans.push(InlineSpan {
                            a_start: tok_start(&ta, ai),
                            a_end: tok_end(&ta, last_a),
                            b_start: tok_start(&tb, bi),
                            b_end: tok_end(&tb, last_b),
                        });
                        ai = ai.saturating_add(*len);
                        bi = bi.saturating_add(*ins);
                        idx += 1;
                    }
                    _ => {
                        let at = anchor(&tb, bi);
                        spans.push(InlineSpan {
                            a_start: tok_start(&ta, ai),
                            a_end: tok_end(&ta, last_a),
                            b_start: at,
                            b_end: at,
                        });
                        ai = ai.saturating_add(*len);
                    }
                }
            }
            Op::Insert { len } => {
                let last_b = bi.saturating_add(*len).saturating_sub(1);
                let at = anchor(&ta, ai);
                spans.push(InlineSpan {
                    a_start: at,
                    a_end: at,
                    b_start: tok_start(&tb, bi),
                    b_end: tok_end(&tb, last_b),
                });
                bi = bi.saturating_add(*len);
            }
        }
        idx += 1;
    }
    spans
}
